use super::ir::{IRConst, IRType, Operand};
use super::vm_safety::VmSafety;
use crate::compiler::{
    bytecode::NativeSig,
    irgen::IRGen,
    parser::{Expr, Primitive, Program, Type},
};
use ordered_float::OrderedFloat;
use std::collections::{HashMap, HashSet};

impl IRGen {
    pub(super) fn eval_const(&mut self, expr: &Expr) -> Option<(IRConst, IRType)> {
        match expr {
            Expr::Int(n, _) => Some((IRConst::Int(*n as i64), IRType::Int)),
            Expr::Float(f, _) => Some((IRConst::Float(OrderedFloat(*f)), IRType::Float)),
            Expr::String(s, _) => Some((IRConst::Str(s.clone()), IRType::String)),
            Expr::Bool(b, _) => Some((IRConst::Int(if *b { 1 } else { 0 }), IRType::Bool)),
            Expr::Nil(_) => Some((IRConst::Int(0), IRType::Int)),
            Expr::Var(name, _) => self.globals.get(name).cloned(),
            Expr::Neg(e, _) => match self.eval_const(e)? {
                (IRConst::Int(v), IRType::Int) => Some((IRConst::Int(-v), IRType::Int)),
                _ => None,
            },
            Expr::FNeg(e, _) => match self.eval_const(e)? {
                (IRConst::Float(v), IRType::Float) => Some((IRConst::Float(-v), IRType::Float)),
                _ => None,
            },
            Expr::Add(l, r, _)
            | Expr::Sub(l, r, _)
            | Expr::Mul(l, r, _)
            | Expr::Div(l, r, _)
            | Expr::Mod(l, r, _)
            | Expr::FAdd(l, r, _)
            | Expr::FSub(l, r, _)
            | Expr::FMul(l, r, _)
            | Expr::FDiv(l, r, _) => {
                let (lc, lt) = self.eval_const(l)?;
                let (rc, rt) = self.eval_const(r)?;
                if matches!(lt, IRType::Float) || matches!(rt, IRType::Float) {
                    let (a, b) = match (lc, rc) {
                        (IRConst::Float(a), IRConst::Float(b)) => (a.into_inner(), b.into_inner()),
                        _ => return None,
                    };
                    let v = match expr {
                        Expr::FAdd(..) | Expr::Add(..) => a + b,
                        Expr::FSub(..) | Expr::Sub(..) => a - b,
                        Expr::FMul(..) | Expr::Mul(..) => a * b,
                        Expr::FDiv(..) | Expr::Div(..) => a / b,
                        _ => return None,
                    };
                    Some((IRConst::Float(OrderedFloat(v)), IRType::Float))
                } else {
                    let (a, b) = match (lc, rc) {
                        (IRConst::Int(a), IRConst::Int(b)) => (a, b),
                        _ => return None,
                    };
                    let v = match expr {
                        Expr::Add(..) => a.wrapping_add(b),
                        Expr::Sub(..) => a.wrapping_sub(b),
                        Expr::Mul(..) => a.wrapping_mul(b),
                        Expr::Div(..) => {
                            if b == 0 {
                                return None;
                            }
                            a.wrapping_div(b)
                        }
                        Expr::Mod(..) => {
                            if b == 0 {
                                return None;
                            }
                            a.wrapping_rem(b)
                        }
                        _ => return None,
                    };
                    Some((IRConst::Int(v), IRType::Int))
                }
            }
            _ => self.eval_const_vm(expr),
        }
    }

    pub(super) fn eval_const_vm(&mut self, expr: &Expr) -> Option<(IRConst, IRType)> {
        use crate::compiler::bytecode::{Compiler, GVM};

        if self.expr_has_var(expr) {
            return None;
        }

        let pure_fns: HashSet<String> = self
            .program_body
            .iter()
            .filter_map(|e| match e {
                Expr::FuncDecl(name, attrs, ..)
                    if attrs.is_pure && (!attrs.is_external || self.native_resolved(name)) =>
                {
                    Some(name.clone())
                }
                _ => None,
            })
            .collect();
        let mut safety = VmSafety::new(&self.program_body, &pure_fns);
        if !safety.safe(expr) {
            return None;
        }
        for decl in self.program_body.iter() {
            if let Expr::FuncDecl(name, attrs, _, _, _, body, _) = decl {
                if (attrs.is_pure || name.starts_with("_lambda_"))
                    && (!attrs.is_external || self.native_resolved(name))
                {
                    let mut fn_safety = VmSafety::new(&self.program_body, &pure_fns);
                    if !fn_safety.safe(body) {
                        return None;
                    }
                }
            }
        }

        let mut selected: Vec<(String, Expr)> = self
            .program_body
            .iter()
            .filter_map(|e| match e {
                Expr::FuncDecl(name, attrs, ..)
                    if ((attrs.is_pure || name.starts_with("_lambda_"))
                        && (!attrs.is_external || self.native_resolved(name))) =>
                {
                    Some((name.clone(), e.clone()))
                }
                _ => None,
            })
            .collect();

        let mut body = order_vm_functions(&mut selected);
        body.push(expr.clone());
        let program = Program { body };

        let natives = self
            .natives
            .as_ref()
            .map(|t| t.entries.clone())
            .unwrap_or_default();

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let bc = Compiler::new().compile(program);
            let mut vm = GVM::new(bc, natives);
            vm.run();
            vm.result()
        }))
        .ok()
        .flatten()?;

        match result {
            crate::compiler::bytecode::Value::Int(i) => Some((IRConst::Int(i), IRType::Int)),
            crate::compiler::bytecode::Value::Float(f) => {
                Some((IRConst::Float(OrderedFloat(f)), IRType::Float))
            }
            crate::compiler::bytecode::Value::Str(s) => Some((IRConst::Str(s), IRType::String)),
            crate::compiler::bytecode::Value::Bool(b) => {
                Some((IRConst::Int(if b { 1 } else { 0 }), IRType::Bool))
            }
            crate::compiler::bytecode::Value::Array(elems) => {
                let mut operands = Vec::with_capacity(elems.len());
                for e in elems {
                    operands.push(self.vm_value_to_const(&e)?);
                }
                Some((IRConst::Array(operands), IRType::Array))
            }
            crate::compiler::bytecode::Value::Void => None,
            crate::compiler::bytecode::Value::Fn(..) => None,
        }
    }

    fn vm_value_to_const(&mut self, value: &crate::compiler::bytecode::Value) -> Option<Operand> {
        match value {
            crate::compiler::bytecode::Value::Int(i) => {
                Some(Operand::ConstIdx(self.get_const_index(IRConst::Int(*i))))
            }
            crate::compiler::bytecode::Value::Float(f) => Some(Operand::ConstIdx(
                self.get_const_index(IRConst::Float(OrderedFloat(*f))),
            )),
            crate::compiler::bytecode::Value::Str(s) => Some(Operand::ConstIdx(
                self.get_const_index(IRConst::Str(s.clone())),
            )),
            crate::compiler::bytecode::Value::Bool(b) => Some(Operand::ConstIdx(
                self.get_const_index(IRConst::Int(if *b { 1 } else { 0 })),
            )),
            crate::compiler::bytecode::Value::Array(elems) => {
                let mut operands = Vec::with_capacity(elems.len());
                for e in elems {
                    operands.push(self.vm_value_to_const(e)?);
                }
                Some(Operand::ConstIdx(
                    self.get_const_index(IRConst::Array(operands)),
                ))
            }
            crate::compiler::bytecode::Value::Void => None,
            crate::compiler::bytecode::Value::Fn(..) => None,
        }
    }

    fn expr_has_var(&self, expr: &Expr) -> bool {
        use Expr::*;
        match expr {
            Int(..) | Float(..) | Bool(..) | String(..) | Nil(_) | Break(_) | Continue(_)
            | TypeDef(_) | Struct(..) | Union(..) | Enum(..) => false,
            Var(..) => true,
            Call(f, _, args, _) => {
                if !matches!(f.as_ref(), Var(..)) && self.expr_has_var(f) {
                    return true;
                }
                args.iter().any(|a| self.expr_has_var(a))
            }
            Block(stmts, _) => stmts.iter().any(|s| self.expr_has_var(s)),
            If(c, t, e, _) => {
                self.expr_has_var(c)
                    || self.expr_has_var(t)
                    || e.as_ref().map(|x| self.expr_has_var(x)).unwrap_or(false)
            }
            While(c, b, _) => self.expr_has_var(c) || self.expr_has_var(b),
            For(_, i, b, _) => self.expr_has_var(i) || self.expr_has_var(b),
            Range(l, r, _) => self.expr_has_var(l) || self.expr_has_var(r),
            Match(s, arms, d, _) => {
                self.expr_has_var(s)
                    || arms
                        .iter()
                        .any(|(p, a)| self.expr_has_var(p) || self.expr_has_var(a))
                    || d.as_ref().map(|x| self.expr_has_var(x)).unwrap_or(false)
            }
            Return(v, _) => self.expr_has_var(v),
            Lambda(_, b, _, _) => self.expr_has_var(b),
            FuncDecl(_, _, _, _, _, b, _) => self.expr_has_var(b),
            GlobalVar(..) | ExternVar(..) => false,
            VarDecl(_, _, v, _) | ConstDecl(_, _, v, _, _) => self.expr_has_var(v),
            Not(e, _) | Neg(e, _) | FNeg(e, _) | AddressOf(e, _) | Deref(e, _) => {
                self.expr_has_var(e)
            }
            Add(l, r, _)
            | Sub(l, r, _)
            | Mul(l, r, _)
            | Div(l, r, _)
            | Mod(l, r, _)
            | FAdd(l, r, _)
            | FSub(l, r, _)
            | FMul(l, r, _)
            | FDiv(l, r, _)
            | Eq(l, r, _)
            | Ne(l, r, _)
            | Lt(l, r, _)
            | Le(l, r, _)
            | Gt(l, r, _)
            | Ge(l, r, _)
            | FEq(l, r, _)
            | FNe(l, r, _)
            | FLt(l, r, _)
            | FLe(l, r, _)
            | FGt(l, r, _)
            | FGe(l, r, _)
            | Xor(l, r, _)
            | LAnd(l, r, _)
            | LOr(l, r, _)
            | StrCat(l, r, _)
            | Index(l, r, _)
            | DerefAssign(l, r, _) => self.expr_has_var(l) || self.expr_has_var(r),
            IndexAssign(o, v, _) | MemberAssign(o, _, v, _) => {
                self.expr_has_var(o) || self.expr_has_var(v)
            }
            ArrayLiteral(items, _) => items.iter().any(|it| self.expr_has_var(it)),
            ArrayFill(_, len, _) => self.expr_has_var(len),
            StructLiteral(_, _, fields, _) | UnionLiteral(_, _, fields, _) => {
                fields.iter().any(|(_, v)| self.expr_has_var(v))
            }
            MemberAccess(o, _, _) => self.expr_has_var(o),
            Inc(..) | Dec(..) => false,
            VarAssign(_, v, _) | AddAssign(_, v, _) | SubAssign(_, v, _) => self.expr_has_var(v),
            FString(parts, _) => parts.iter().any(|p| self.expr_has_var(p)),
        }
    }
}

fn order_vm_functions(selected: &mut Vec<(String, Expr)>) -> Vec<Expr> {
    if selected.is_empty() {
        return Vec::new();
    }
    let decls: HashMap<String, Expr> = selected.drain(..).collect();
    let mut ordered: Vec<Expr> = Vec::new();
    let mut done: HashSet<String> = HashSet::new();
    let mut in_progress: HashSet<String> = HashSet::new();

    fn emit(
        name: &str,
        decls: &HashMap<String, Expr>,
        ordered: &mut Vec<Expr>,
        done: &mut HashSet<String>,
        in_progress: &mut HashSet<String>,
    ) {
        if done.contains(name) || in_progress.contains(name) {
            return;
        }
        let Some(decl) = decls.get(name) else {
            return;
        };
        in_progress.insert(name.to_string());
        let mut deps: HashSet<String> = HashSet::new();
        collect_var_refs(decl, &mut deps);
        for dep in deps {
            emit(&dep, decls, ordered, done, in_progress);
        }
        ordered.push(decl.clone());
        in_progress.remove(name);
        done.insert(name.to_string());
    }

    let names: Vec<String> = decls.keys().cloned().collect();
    for name in names {
        emit(&name, &decls, &mut ordered, &mut done, &mut in_progress);
    }
    ordered
}

fn collect_var_refs(expr: &Expr, out: &mut HashSet<String>) {
    use Expr::*;
    match expr {
        Int(..) | Float(..) | Bool(..) | String(..) | Nil(_) | Break(_) | Continue(_)
        | TypeDef(_) | Struct(..) | Union(..) | Enum(..) | GlobalVar(..) | ExternVar(..) => {}
        Var(name, _) => {
            out.insert(name.clone());
        }
        FuncDecl(_, _, _, _, _, body, _) => {
            collect_var_refs(body, out);
        }
        Call(f, _, args, _) => {
            collect_var_refs(f, out);
            for a in args {
                collect_var_refs(a, out);
            }
        }
        Block(stmts, _) => {
            for s in stmts {
                collect_var_refs(s, out);
            }
        }
        If(c, t, e, _) => {
            collect_var_refs(c, out);
            collect_var_refs(t, out);
            if let Some(x) = e {
                collect_var_refs(x, out);
            }
        }
        While(c, b, _) | Range(c, b, _) => {
            collect_var_refs(c, out);
            collect_var_refs(b, out);
        }
        For(_, i, b, _) => {
            collect_var_refs(i, out);
            collect_var_refs(b, out);
        }
        Match(s, arms, d, _) => {
            collect_var_refs(s, out);
            for (p, a) in arms {
                collect_var_refs(p, out);
                collect_var_refs(a, out);
            }
            if let Some(x) = d {
                collect_var_refs(x, out);
            }
        }
        Return(v, _) | Not(v, _) | Neg(v, _) | FNeg(v, _) | AddressOf(v, _) | Deref(v, _) => {
            collect_var_refs(v, out)
        }
        Lambda(_, b, _, _) => collect_var_refs(b, out),
        VarDecl(_, _, v, _) | ConstDecl(_, _, v, _, _) => collect_var_refs(v, out),
        Add(l, r, _)
        | Sub(l, r, _)
        | Mul(l, r, _)
        | Div(l, r, _)
        | Mod(l, r, _)
        | FAdd(l, r, _)
        | FSub(l, r, _)
        | FMul(l, r, _)
        | FDiv(l, r, _)
        | Eq(l, r, _)
        | Ne(l, r, _)
        | Lt(l, r, _)
        | Le(l, r, _)
        | Gt(l, r, _)
        | Ge(l, r, _)
        | FEq(l, r, _)
        | FNe(l, r, _)
        | FLt(l, r, _)
        | FLe(l, r, _)
        | FGt(l, r, _)
        | FGe(l, r, _)
        | Xor(l, r, _)
        | LAnd(l, r, _)
        | LOr(l, r, _)
        | StrCat(l, r, _)
        | Index(l, r, _)
        | DerefAssign(l, r, _) => {
            collect_var_refs(l, out);
            collect_var_refs(r, out);
        }
        IndexAssign(o, v, _) | MemberAssign(o, _, v, _) => {
            collect_var_refs(o, out);
            collect_var_refs(v, out);
        }
        ArrayLiteral(items, _) => {
            for it in items {
                collect_var_refs(it, out);
            }
        }
        ArrayFill(_, len, _) => collect_var_refs(len, out),
        StructLiteral(_, _, fields, _) | UnionLiteral(_, _, fields, _) => {
            for (_, v) in fields {
                collect_var_refs(v, out);
            }
        }
        MemberAccess(o, _, _) => collect_var_refs(o, out),
        Inc(..) | Dec(..) => {}
        VarAssign(_, v, _) | AddAssign(_, v, _) | SubAssign(_, v, _) => collect_var_refs(v, out),
        FString(parts, _) => {
            for p in parts {
                collect_var_refs(p, out);
            }
        }
    }
}

pub(super) fn native_sig(params: &[(String, Type)], ret_type: &Type) -> Option<NativeSig> {
    use crate::compiler::bytecode::NativeKind;
    let f = |ty: &Type| -> Option<NativeKind> {
        match ty {
            Type::Primitive(Primitive::Int) => Some(NativeKind::Int),
            Type::Primitive(Primitive::Float) => Some(NativeKind::Float),
            Type::Primitive(Primitive::Boolean) => Some(NativeKind::Bool),
            Type::Primitive(Primitive::String) => Some(NativeKind::Str),
            _ => None,
        }
    };
    let mut kinds: Vec<NativeKind> = Vec::new();
    for (_, pty) in params {
        kinds.push(f(pty)?);
    }
    let ret = f(ret_type)?;
    Some(NativeSig {
        params: Box::leak(kinds.into_boxed_slice()),
        ret,
    })
}
