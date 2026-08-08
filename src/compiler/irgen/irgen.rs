use super::context::Context;
use super::ir::{IRConst, IRGlobalVar, IRProgram, IRType, Op, Operand};
use crate::compiler::{
    codegen::CodeGenError,
    irgen::IRGen,
    parser::{Expr, Primitive, Program, Type},
};
use ordered_float::OrderedFloat;
use std::collections::HashSet;
use std::mem::take;

impl IRGen {
    pub fn compile(&mut self, program: Program) -> Result<IRProgram, CodeGenError> {
        let program = self.lambda2function(program);
        self.program_body = program.body.clone();

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
                Expr::FuncDecl(name, attrs, ..) if attrs.is_pure && !attrs.is_external => {
                    Some(name.clone())
                }
                _ => None,
            })
            .collect();
        if !self.vm_expr_safe(expr, &pure_fns) {
            return None;
        }
        if self.expr_has_var(expr) {
            return None;
        }
        for decl in self.program_body.iter() {
            if let Expr::FuncDecl(_, attrs, _, _, _, body, _) = decl {
                if attrs.is_pure && !attrs.is_external && !self.vm_expr_safe(body, &pure_fns) {
                    return None;
                }
            }
        }

        let mut body: Vec<Expr> = self
            .program_body
            .iter()
            .filter(|e| {
                matches!(e, Expr::FuncDecl(_, attrs, ..) if attrs.is_pure && !attrs.is_external)
            })
            .cloned()
            .collect();
        body.push(expr.clone());
        let program = Program { body };

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let bc = Compiler::new().compile(program);
            let mut vm = GVM::new(bc);
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

    fn vm_expr_safe(&self, expr: &Expr, pure_fns: &HashSet<String>) -> bool {
        use Expr::*;

        match expr {
            Int(..) | Float(..) | Bool(..) | String(..) | Nil(_) | Var(..) | Break(_)
            | Continue(_) | TypeDef(_) | Struct(..) | Union(..) | Enum(..) | GlobalVar(..)
            | ExternVar(..) | FuncDecl(..) => true,

            Call(callee, _, args, _) => {
                match callee.as_ref() {
                    Var(name, _) => {
                        if !pure_fns.contains(name.as_str()) {
                            return false;
                        }
                    }
                    _ => return false,
                }
                if !self.vm_expr_safe(callee, pure_fns) {
                    return false;
                }
                args.iter().all(|a| self.vm_expr_safe(a, pure_fns))
            }

            Block(stmts, _) => stmts.iter().all(|s| self.vm_expr_safe(s, pure_fns)),
            If(c, t, e, _) => {
                self.vm_expr_safe(c, pure_fns)
                    && self.vm_expr_safe(t, pure_fns)
                    && e.as_ref()
                        .map(|x| self.vm_expr_safe(x, pure_fns))
                        .unwrap_or(true)
            }
            While(c, b, _) => self.vm_expr_safe(c, pure_fns) && self.vm_expr_safe(b, pure_fns),
            For(_, _, _, _) => false,
            Range(_, _, _) => false,
            Match(s, arms, d, _) => {
                if !self.vm_expr_safe(s, pure_fns) {
                    return false;
                }
                for (pat, arm) in arms {
                    if !self.vm_expr_safe(pat, pure_fns) || !self.vm_expr_safe(arm, pure_fns) {
                        return false;
                    }
                }
                d.as_ref()
                    .map(|x| self.vm_expr_safe(x, pure_fns))
                    .unwrap_or(true)
            }
            Return(v, _) => self.vm_expr_safe(v, pure_fns),
            Lambda(_, b, _, _) => self.vm_expr_safe(b, pure_fns),
            VarDecl(_, _, v, _) | ConstDecl(_, _, v, _, _) | Not(v, _) | Neg(v, _) | FNeg(v, _) => {
                self.vm_expr_safe(v, pure_fns)
            }
            AddressOf(..) => false,
            Deref(..) => false,
            VarAssign(_, v, _) | AddAssign(_, v, _) | SubAssign(_, v, _) => {
                self.vm_expr_safe(v, pure_fns)
            }
            Inc(..) | Dec(..) => true,
            IndexAssign(arr_idx, value, _) => {
                let Expr::Index(arr, idx, _) = arr_idx.as_ref() else {
                    return false;
                };
                if !matches!(arr.as_ref(), Var(..)) {
                    return false;
                }
                self.vm_expr_safe(idx, pure_fns) && self.vm_expr_safe(value, pure_fns)
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
            | StrCat(l, r, _) => self.vm_expr_safe(l, pure_fns) && self.vm_expr_safe(r, pure_fns),
            DerefAssign(..) => false,
            Index(l, r, _) => self.vm_expr_safe(l, pure_fns) && self.vm_expr_safe(r, pure_fns),
            ArrayLiteral(items, _) => items.iter().all(|it| self.vm_expr_safe(it, pure_fns)),
            ArrayFill(_, len, _) => self.vm_expr_safe(len, pure_fns),
            StructLiteral(..) => false,
            UnionLiteral(..) => false,
            MemberAccess(..) => false,
            FString(parts, _) => parts.iter().all(|p| self.vm_expr_safe(p, pure_fns)),
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
