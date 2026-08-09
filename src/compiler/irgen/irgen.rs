use super::context::Context;
use super::ir::{IRConst, IRGlobalVar, IRProgram, IRType, Op, Operand};
use crate::compiler::{
    codegen::CodeGenError,
    irgen::IRGen,
    parser::{Expr, Primitive, Program, Type},
};
use ordered_float::OrderedFloat;
use std::collections::{HashMap, HashSet};
use std::mem::take;

impl IRGen {
    fn native_resolved(&self, name: &str) -> bool {
        self.natives
            .as_ref()
            .map(|t| t.entries.contains_key(name))
            .unwrap_or(false)
    }

    pub fn compile(&mut self, program: Program) -> Result<IRProgram, CodeGenError> {
        super::purity::check_lambda_params(&program.body)?;
        let program = self.lambda2function(program);
        self.program_body = program.body.clone();

        let mut warned_extern_pure: std::collections::HashSet<String> =
            std::collections::HashSet::new();
        for expr in &self.program_body {
            if let Expr::FuncDecl(name, attrs, _, _, _, _, _) = expr {
                if attrs.is_external && attrs.is_pure {
                    if !warned_extern_pure.contains(name.as_str()) {
                        warned_extern_pure.insert(name.clone());
                        eprintln!(
                            "warning: purity of external function '{name}' cannot be verified"
                        );
                    }
                }
            }
        }

        if let Some(natives) = self.natives.as_mut() {
            for expr in &self.program_body {
                if let Expr::FuncDecl(name, attrs, _, params, ret_type, _, _) = expr {
                    if attrs.is_external && attrs.is_pure {
                        if let Some(sig) = native_sig(params, ret_type) {
                            natives.resolve(name, sig);
                        }
                    }
                }
            }
        }

        super::purity::check_pure_functions(&self.program_body)?;

        for expr in &program.body {
            match expr {
                Expr::FuncDecl(name, attrs, type_params, params, ret_type, body, _) => {
                    if type_params.is_empty() {
                        self.func_decl(
                            name.clone(),
                            attrs.clone(),
                            params.clone(),
                            ret_type.clone(),
                        )?;
                        self.func_high_returns
                            .insert(name.clone(), ret_type.clone());
                    } else {
                        self.generic_funcs.insert(
                            name.clone(),
                            (
                                type_params.clone(),
                                params.clone(),
                                ret_type.clone(),
                                body.clone(),
                            ),
                        );
                    }
                }
                Expr::ExternVar(name, ty, _) => {
                    self.extern_vars.insert(name.clone(), ty.clone());
                }
                Expr::Struct(name, type_params, fields, _) => {
                    self.structs
                        .insert(name.clone(), (type_params.clone(), fields.clone()));
                }
                Expr::Union(name, type_params, fields, _) => {
                    self.unions
                        .insert(name.clone(), (type_params.clone(), fields.clone()));
                }
                Expr::Enum(name, members, _) => {
                    self.enums.insert(name.clone(), members.clone());
                }
                _ => {}
            }
        }

        let const_decls: Vec<Expr> = program
            .body
            .iter()
            .filter(|e| matches!(e, Expr::ConstDecl(..)))
            .cloned()
            .collect();
        self.store_global_consts(&const_decls)?;
        self.store_global_vars(&program.body)?;

        for expr in program.body {
            match expr {
                Expr::FuncDecl(name, _, type_params, params, _, body, _) => {
                    if type_params.is_empty() {
                        self.compile_fn(name, params, *body)?;
                    }
                }
                Expr::ConstDecl(_, _, _, _, _) | Expr::GlobalVar(_, _, _, _, _) => {}
                Expr::Int(_, _)
                | Expr::Float(_, _)
                | Expr::Bool(_, _)
                | Expr::String(_, _)
                | Expr::Nil(_)
                | Expr::Var(_, _) => {
                    let mut ctx = Context::new("_global".to_string());
                    ctx.enter_scope();
                    self.compile_expr(
                        Expr::VarDecl(
                            "_global".to_string(),
                            Type::Primitive(Primitive::Int),
                            Box::new(expr),
                            crate::compiler::Span::new(0, 0),
                        ),
                        &mut ctx,
                    )?;
                }
                _ => {}
            }
        }

        Ok(IRProgram {
            functions: take(&mut self.functions),
            constants: take(&mut self.constants),
            extern_vars: take(&mut self.extern_vars).into_keys().collect(),
            global_vars: take(&mut self.global_emits),
        })
    }

    pub(super) fn store_global_vars(&mut self, body: &[Expr]) -> Result<(), CodeGenError> {
        for expr in body {
            match expr {
                Expr::GlobalVar(name, is_pub, ty, init, _) => {
                    let value = match init {
                        Some(init) => match self.eval_const(init) {
                            Some((cv, _)) => Some(cv),
                            None => {
                                return Err(CodeGenError::TypeError {
                                    message: format!(
                                        "initializer of global variable '{}' is not a compile-time constant",
                                        name
                                    ),
                                });
                            }
                        },
                        None => None,
                    };
                    let ir_type = if matches!(ty, Type::Unknown) {
                        value
                            .as_ref()
                            .map(|cv| match cv {
                                IRConst::Float(_) => IRType::Float,
                                _ => IRType::Int,
                            })
                            .unwrap_or(IRType::Int)
                    } else {
                        Context::type2ir_type(ty)
                    };
                    if !matches!(ir_type, IRType::Int | IRType::Float | IRType::Bool) {
                        return Err(CodeGenError::TypeError {
                            message: format!(
                                "unsupported type '{}' for global variable '{}' (only int/float/bool)",
                                ty, name
                            ),
                        });
                    }
                    self.global_storage
                        .insert(name.clone(), (ir_type.clone(), value.clone(), *is_pub));
                    self.global_emits.push(IRGlobalVar {
                        name: name.clone(),
                        value,
                        is_pub: *is_pub,
                    });
                }
                Expr::ConstDecl(name, _, _, is_pub, _) => {
                    if *is_pub {
                        if let Some((cv, ct)) = self.globals.get(name).cloned() {
                            if matches!(&cv, IRConst::Str(_) | IRConst::Array(_)) {
                                return Err(CodeGenError::TypeError {
                                    message: format!(
                                        "unsupported value for global constant '{}' (only int/float/bool can be exported)",
                                        name
                                    ),
                                });
                            }
                            let _ = ct;
                            self.global_emits.push(IRGlobalVar {
                                name: name.clone(),
                                value: Some(cv),
                                is_pub: true,
                            });
                        }
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }

    pub(super) fn store_global_consts(&mut self, exprs: &[Expr]) -> Result<(), CodeGenError> {
        let mut pending: Vec<(String, Expr)> = exprs
            .iter()
            .filter_map(|e| match e {
                Expr::ConstDecl(name, _, value, _, _) => {
                    Some((name.clone(), value.as_ref().clone()))
                }
                _ => None,
            })
            .collect();
        let mut progressed = true;
        while !pending.is_empty() && progressed {
            progressed = false;
            let mut next = Vec::new();
            for (name, value) in pending {
                if let Some(cv) = self.eval_const(&value) {
                    self.globals.insert(name, cv);
                    progressed = true;
                } else {
                    next.push((name, value));
                }
            }
            pending = next;
        }
        for (name, _) in pending {
            return Err(CodeGenError::TypeError {
                message: format!(
                    "initializer of constant '{}' is not a compile-time constant",
                    name
                ),
            });
        }
        Ok(())
    }

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
        if self.expr_has_var(expr) {
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

    pub(super) fn get_const_index(&mut self, constant: IRConst) -> usize {
        if let Some(&index) = self.constant_pool.get(&constant) {
            return index;
        }
        let index = self.constants.len();
        self.constants.push(constant.clone());
        self.constant_pool.insert(constant, index);
        index
    }

    pub(super) fn get_binop_parts(expr: Expr) -> Result<(Op, Box<Expr>, Box<Expr>), CodeGenError> {
        let _span = crate::compiler::Span::new(0, 0);
        match expr {
            Expr::Add(l, r, _) => Ok((Op::Add, l, r)),
            Expr::Sub(l, r, _) => Ok((Op::Sub, l, r)),
            Expr::Mul(l, r, _) => Ok((Op::Mul, l, r)),
            Expr::Div(l, r, _) => Ok((Op::Div, l, r)),
            Expr::Mod(l, r, _) => Ok((Op::Mod, l, r)),
            Expr::FAdd(l, r, _) => Ok((Op::FAdd, l, r)),
            Expr::FSub(l, r, _) => Ok((Op::FSub, l, r)),
            Expr::FMul(l, r, _) => Ok((Op::FMul, l, r)),
            Expr::FDiv(l, r, _) => Ok((Op::FDiv, l, r)),
            Expr::Eq(l, r, _) => Ok((Op::Eq, l, r)),
            Expr::Ne(l, r, _) => Ok((Op::Ne, l, r)),
            Expr::Lt(l, r, _) => Ok((Op::Lt, l, r)),
            Expr::Le(l, r, _) => Ok((Op::Le, l, r)),
            Expr::Gt(l, r, _) => Ok((Op::Gt, l, r)),
            Expr::Ge(l, r, _) => Ok((Op::Ge, l, r)),
            Expr::FEq(l, r, _) => Ok((Op::FEq, l, r)),
            Expr::FNe(l, r, _) => Ok((Op::FNe, l, r)),
            Expr::FLt(l, r, _) => Ok((Op::FLt, l, r)),
            Expr::FLe(l, r, _) => Ok((Op::FLe, l, r)),
            Expr::FGt(l, r, _) => Ok((Op::FGt, l, r)),
            Expr::FGe(l, r, _) => Ok((Op::FGe, l, r)),
            Expr::Xor(l, r, _) => Ok((Op::Xor, l, r)),
            Expr::LAnd(l, r, _) => Ok((Op::LAnd, l, r)),
            Expr::LOr(l, r, _) => Ok((Op::LOr, l, r)),
            Expr::StrCat(l, r, _) => Ok((Op::StrCat, l, r)),
            _ => Err(CodeGenError::UnsupportedOperation {
                message: "not a binary operation".to_string(),
            }),
        }
    }
}

const VM_LAMBDA_MARKER: &str = "\u{03bb}";

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

struct VmSafety<'a> {
    program_body: &'a [Expr],
    pure_fns: &'a HashSet<String>,
    lambda_memo: HashMap<String, bool>,
    lambda_in_progress: HashSet<String>,
    bound: Vec<HashMap<String, String>>,
}

fn native_sig(
    params: &[(String, Type)],
    ret_type: &Type,
) -> Option<crate::compiler::bytecode::NativeSig> {
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
    Some(crate::compiler::bytecode::NativeSig {
        params: Box::leak(kinds.into_boxed_slice()),
        ret,
    })
}

impl<'a> VmSafety<'a> {
    fn new(program_body: &'a [Expr], pure_fns: &'a HashSet<String>) -> Self {
        VmSafety {
            program_body,
            pure_fns,
            lambda_memo: HashMap::new(),
            lambda_in_progress: HashSet::new(),
            bound: vec![HashMap::new()],
        }
    }

    fn enter_scope(&mut self) {
        self.bound.push(HashMap::new());
    }

    fn leave_scope(&mut self) {
        self.bound.pop();
    }

    fn lookup_bound(&self, name: &str) -> Option<&String> {
        self.bound.iter().rev().find_map(|scope| scope.get(name))
    }

    fn unbind(&mut self, name: &str) {
        for scope in &mut self.bound {
            scope.remove(name);
        }
    }

    fn bind(&mut self, name: &str, value: &Expr) {
        match value {
            Expr::Var(v, _) => {
                let target = if v.starts_with("_lambda_") || self.pure_fns.contains(v) {
                    Some(v.clone())
                } else {
                    self.lookup_bound(v).cloned()
                };
                if let Some(t) = target {
                    if let Some(scope) = self.bound.last_mut() {
                        scope.insert(name.to_string(), t);
                    }
                }
            }
            Expr::Lambda(_, body, _, _) => {
                if self.safe(body) {
                    if let Some(scope) = self.bound.last_mut() {
                        scope.insert(name.to_string(), VM_LAMBDA_MARKER.to_string());
                    }
                }
            }
            _ => {}
        }
    }

    fn callee_pure(&mut self, name: &str) -> bool {
        if self.pure_fns.contains(name) || name == VM_LAMBDA_MARKER {
            return true;
        }
        if name.starts_with("_lambda_") {
            return self.lambda_is_pure(name);
        }
        false
    }

    fn lambda_is_pure(&mut self, name: &str) -> bool {
        if let Some(&r) = self.lambda_memo.get(name) {
            return r;
        }
        if self.lambda_in_progress.contains(name) {
            return true;
        }
        let Some(body) = self.find_lambda_body(name).cloned() else {
            self.lambda_memo.insert(name.to_string(), false);
            return false;
        };
        self.lambda_in_progress.insert(name.to_string());
        let saved = std::mem::take(&mut self.bound);
        let r = self.safe(&body);
        self.bound = saved;
        self.lambda_in_progress.remove(name);
        self.lambda_memo.insert(name.to_string(), r);
        r
    }

    fn find_lambda_body(&self, name: &str) -> Option<&Expr> {
        self.program_body.iter().find_map(|e| match e {
            Expr::FuncDecl(n, _, _, _, _, b, _) if n == name => Some(b.as_ref()),
            _ => None,
        })
    }

    fn safe(&mut self, expr: &Expr) -> bool {
        use Expr::*;
        match expr {
            Int(..) | Float(..) | Bool(..) | String(..) | Nil(_) | Var(..) | Break(_)
            | Continue(_) | TypeDef(_) | Struct(..) | Union(..) | Enum(..) | GlobalVar(..)
            | ExternVar(..) | FuncDecl(..) => true,

            Call(callee, _, args, _) => {
                match callee.as_ref() {
                    Var(name, _) => {
                        let bound_target = self.lookup_bound(name).cloned();
                        let pure = match &bound_target {
                            Some(t) => self.callee_pure(t),
                            None => self.callee_pure(name),
                        };
                        if !pure {
                            return false;
                        }
                    }
                    _ => {
                        if !self.safe(callee) {
                            return false;
                        }
                    }
                }
                args.iter().all(|a| self.safe(a))
            }

            Block(stmts, _) => {
                self.enter_scope();
                let r = stmts.iter().all(|s| self.safe(s));
                self.leave_scope();
                r
            }
            If(c, t, e, _) => {
                if !self.safe(c) {
                    return false;
                }
                self.enter_scope();
                let r_t = self.safe(t);
                self.leave_scope();
                if !r_t {
                    return false;
                }
                match e {
                    Some(x) => {
                        self.enter_scope();
                        let r = self.safe(x);
                        self.leave_scope();
                        r
                    }
                    None => true,
                }
            }
            While(c, b, _) => {
                if !self.safe(c) {
                    return false;
                }
                self.enter_scope();
                let r = self.safe(b);
                self.leave_scope();
                r
            }
            For(..) => false,
            Range(..) => false,
            Match(s, arms, d, _) => {
                if !self.safe(s) {
                    return false;
                }
                for (pat, arm) in arms {
                    if !self.safe(pat) {
                        return false;
                    }
                    self.enter_scope();
                    let r = self.safe(arm);
                    self.leave_scope();
                    if !r {
                        return false;
                    }
                }
                match d {
                    Some(x) => {
                        self.enter_scope();
                        let r = self.safe(x);
                        self.leave_scope();
                        r
                    }
                    None => true,
                }
            }
            Return(v, _) => self.safe(v),
            Lambda(_, b, _, _) => {
                let saved = std::mem::take(&mut self.bound);
                self.bound = vec![HashMap::new()];
                let r = self.safe(b);
                self.bound = saved;
                r
            }
            VarDecl(name, _, v, _) | ConstDecl(name, _, v, _, _) => {
                let r = self.safe(v);
                if r {
                    self.bind(name, v);
                }
                r
            }
            Not(v, _) | Neg(v, _) | FNeg(v, _) => self.safe(v),
            AddressOf(..) => false,
            Deref(..) => false,
            VarAssign(name, v, _) | AddAssign(name, v, _) | SubAssign(name, v, _) => {
                let r = self.safe(v);
                if r {
                    match v.as_ref() {
                        Var(..) | Lambda(..) => self.bind(name, v),
                        _ => self.unbind(name),
                    }
                }
                r
            }
            Inc(..) | Dec(..) => true,
            IndexAssign(arr_idx, value, _) => {
                let Expr::Index(arr, idx, _) = arr_idx.as_ref() else {
                    return false;
                };
                if !matches!(arr.as_ref(), Var(..)) {
                    return false;
                }
                self.safe(idx) && self.safe(value)
            }
            MemberAssign(..) => false,
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
            | StrCat(l, r, _) => self.safe(l) && self.safe(r),
            DerefAssign(..) => false,
            Index(l, r, _) => self.safe(l) && self.safe(r),
            ArrayLiteral(items, _) => items.iter().all(|it| self.safe(it)),
            ArrayFill(_, len, _) => self.safe(len),
            StructLiteral(..) => false,
            UnionLiteral(..) => false,
            MemberAccess(..) => false,
            FString(parts, _) => parts.iter().all(|p| self.safe(p)),
        }
    }
}
