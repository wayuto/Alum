use super::context::{Context, Symbol};
use super::ir::{IRFunction, IRType, Instruction, Op, Operand};
use crate::compiler::{
    codegen::CodeGenError,
    irgen::IRGen,
    parser::{Expr, Type},
};
use std::mem::take;

impl IRGen {
    pub(super) fn func_decl(
        &mut self,
        name: String,
        params: Vec<(String, Type)>,
        ret_type: Type,
    ) -> Result<(), CodeGenError> {
        let ir_params: Vec<(Operand, IRType)> = params
            .iter()
            .map(|(name, typ)| (Operand::Var(name.clone()), Context::type2ir_type(typ)))
            .collect();
        let ir_ret_type = Context::type2ir_type(&ret_type);
        self.functions.push(IRFunction {
            name,
            params: ir_params,
            ret_type: ir_ret_type,
            instructions: Vec::new(),
            is_pub: true,
            is_external: false,
        });
        Ok(())
    }

    pub(super) fn compile_fn(
        &mut self,
        name: String,
        params: Vec<(String, Type)>,
        body: Expr,
    ) -> Result<(), CodeGenError> {
        let func = self.find_func(&name)?;

        let mut ctx = Context::new(name.clone());
        ctx.enter_scope();

        for (i, (param, ty)) in func.params.iter().enumerate() {
            if let Operand::Var(pname) = param {
                if let Some(scope) = ctx.scope.last_mut() {
                    scope.insert(
                        pname.clone(),
                        Symbol {
                            name: pname.clone(),
                            ir_type: ty.clone(),
                        },
                    );
                }
                if let Some((_, hty)) = params.get(i) {
                    ctx.var_types.insert(pname.clone(), hty.clone());
                }
            }
        }

        let last_op = self.compile_expr(body, &mut ctx)?;
        ctx.exit_scope()?;

        let last_inst_op = ctx.instructions.last().map(|i| i.op.clone());
        let last_is_return = matches!(last_inst_op, Some(Op::Return(_)));

        if !last_is_return {
            let reg = if func.ret_type == IRType::Float {
                "xmm0".to_string()
            } else {
                "rax".to_string()
            };
            ctx.instructions.push(Instruction {
                op: Op::Return(reg),
                dst: None,
                src1: Some(last_op),
                src2: None,
            });
        }

        if let Some(f) = self.functions.iter_mut().rev().find(|f| f.name == name) {
            f.instructions = take(&mut ctx.instructions);
        }
        Ok(())
    }

    pub(super) fn extern_decl(
        &mut self,
        name: String,
        params: Vec<(String, Type)>,
        ret_type: Type,
    ) -> Result<(), CodeGenError> {
        let ir_params: Vec<(Operand, IRType)> = params
            .into_iter()
            .enumerate()
            .map(|(i, (_, typ))| {
                let param_name = format!("a{}", i);
                (Operand::Var(param_name), Context::type2ir_type(&typ))
            })
            .collect();
        let ir_ret_type = Context::type2ir_type(&ret_type);
        let signature = IRFunction {
            name: name.clone(),
            params: ir_params,
            ret_type: ir_ret_type,
            instructions: Vec::new(),
            is_pub: false,
            is_external: true,
        };
        let already_defined = self
            .functions
            .iter()
            .any(|f| f.name == name && !f.is_external);
        if !already_defined {
            self.functions.push(signature);
        }
        Ok(())
    }

    pub(super) fn find_func(&self, name: &str) -> Result<IRFunction, CodeGenError> {
        for func in self.functions.iter().rev() {
            if func.name == *name {
                return Ok(func.to_owned());
            }
        }
        Err(CodeGenError::UndefinedFunction {
            name: name.to_string(),
            span: crate::compiler::Span::new(0, 0),
        })
    }

    pub(super) fn monomorphize(
        &mut self,
        name: &str,
        type_args: &[Type],
    ) -> Result<String, CodeGenError> {
        let mangled = format!(
            "{}_{}",
            name,
            type_args
                .iter()
                .map(|t| t.mangle())
                .collect::<Vec<_>>()
                .join("_")
        );

        if self.find_func(&mangled).is_ok() {
            return Ok(mangled);
        }

        let (type_params, params, ret_type, body) = self
            .generic_funcs
            .get(name)
            .ok_or_else(|| CodeGenError::UndefinedFunction {
                name: name.to_string(),
                span: crate::compiler::Span::new(0, 0),
            })?
            .clone();

        if type_params.len() != type_args.len() {
            return Err(CodeGenError::TypeError {
                message: format!(
                    "expected {} type arguments for '{}', got {}",
                    type_params.len(),
                    name,
                    type_args.len()
                ),
            });
        }

        let concrete_params: Vec<(String, Type)> = params
            .iter()
            .map(|(n, t)| (n.clone(), t.substitute(type_args)))
            .collect();
        let concrete_ret = ret_type.substitute(type_args);

        let concrete_body = substitute_expr(*body, type_args);
        let mut lambda_map = std::collections::HashMap::new();
        let concrete_body =
            super::lambda::hoist_lambdas(concrete_body, &mut self.lambda_counter, &mut lambda_map);
        let lambda_funcs: Vec<Expr> = lambda_map.into_values().collect();

        for lambda in &lambda_funcs {
            if let Expr::FuncDecl(name, _, params, ret_type, _, _) = lambda {
                self.func_decl(name.clone(), params.clone(), ret_type.clone())?;
            }
        }
        for lambda in &lambda_funcs {
            if let Expr::FuncDecl(name, _, params, _, body, _) = lambda {
                self.compile_fn(name.clone(), params.clone(), body.as_ref().clone())?;
            }
        }

        self.mono_in_progress.push(mangled.clone());
        self.func_decl(mangled.clone(), concrete_params.clone(), concrete_ret)?;
        self.compile_fn(mangled.clone(), concrete_params, concrete_body)?;
        self.mono_in_progress.pop();

        Ok(mangled)
    }
}

pub(super) fn substitute_expr(expr: Expr, args: &[Type]) -> Expr {
    use Expr::*;
    let sub_box = |e: Box<Expr>| Box::new(substitute_expr(*e, args));
    let sub_val = |e: Expr| substitute_expr(e, args);
    match expr {
        VarDecl(name, ty, value, span) => VarDecl(name, ty.substitute(args), sub_box(value), span),
        FuncDecl(name, type_params, params, ret_type, body, span) => {
            let params = if type_params.is_empty() {
                params
                    .into_iter()
                    .map(|(n, t)| (n, t.substitute(args)))
                    .collect()
            } else {
                params
            };
            let ret_type = if type_params.is_empty() {
                ret_type.substitute(args)
            } else {
                ret_type
            };
            FuncDecl(name, type_params, params, ret_type, sub_box(body), span)
        }
        Extern(name, params, ret_type, span) => Extern(
            name,
            params
                .into_iter()
                .map(|(n, t)| (n, t.substitute(args)))
                .collect(),
            ret_type.substitute(args),
            span,
        ),
        Call(callee, type_args, call_args, span) => Call(
            sub_box(callee),
            type_args.into_iter().map(|t| t.substitute(args)).collect(),
            call_args.into_iter().map(sub_val).collect(),
            span,
        ),
        ArrayFill(ty, len, span) => ArrayFill(ty.substitute(args), sub_box(len), span),
        Struct(name, type_params, fields, span) => {
            let fields = if type_params.is_empty() {
                fields
                    .into_iter()
                    .map(|(n, t)| (n, t.substitute(args)))
                    .collect()
            } else {
                fields
            };
            Struct(name, type_params, fields, span)
        }
        StructLiteral(name, type_args, fields, span) => StructLiteral(
            name,
            type_args.into_iter().map(|t| t.substitute(args)).collect(),
            fields.into_iter().map(|(n, e)| (n, sub_val(e))).collect(),
            span,
        ),
        Lambda(params, body, ret_type, span) => Lambda(
            params
                .into_iter()
                .map(|(n, t)| (n, t.substitute(args)))
                .collect(),
            sub_box(body),
            ret_type.substitute(args),
            span,
        ),
        Add(l, r, span) => Add(sub_box(l), sub_box(r), span),
        Sub(l, r, span) => Sub(sub_box(l), sub_box(r), span),
        Mul(l, r, span) => Mul(sub_box(l), sub_box(r), span),
        Div(l, r, span) => Div(sub_box(l), sub_box(r), span),
        Mod(l, r, span) => Mod(sub_box(l), sub_box(r), span),
        FAdd(l, r, span) => FAdd(sub_box(l), sub_box(r), span),
        FSub(l, r, span) => FSub(sub_box(l), sub_box(r), span),
        FMul(l, r, span) => FMul(sub_box(l), sub_box(r), span),
        FDiv(l, r, span) => FDiv(sub_box(l), sub_box(r), span),
        Eq(l, r, span) => Eq(sub_box(l), sub_box(r), span),
        Ne(l, r, span) => Ne(sub_box(l), sub_box(r), span),
        Lt(l, r, span) => Lt(sub_box(l), sub_box(r), span),
        Le(l, r, span) => Le(sub_box(l), sub_box(r), span),
        Gt(l, r, span) => Gt(sub_box(l), sub_box(r), span),
        Ge(l, r, span) => Ge(sub_box(l), sub_box(r), span),
        FEq(l, r, span) => FEq(sub_box(l), sub_box(r), span),
        FNe(l, r, span) => FNe(sub_box(l), sub_box(r), span),
        FLt(l, r, span) => FLt(sub_box(l), sub_box(r), span),
        FLe(l, r, span) => FLe(sub_box(l), sub_box(r), span),
        FGt(l, r, span) => FGt(sub_box(l), sub_box(r), span),
        FGe(l, r, span) => FGe(sub_box(l), sub_box(r), span),
        Xor(l, r, span) => Xor(sub_box(l), sub_box(r), span),
        LAnd(l, r, span) => LAnd(sub_box(l), sub_box(r), span),
        LOr(l, r, span) => LOr(sub_box(l), sub_box(r), span),
        StrCat(l, r, span) => StrCat(sub_box(l), sub_box(r), span),
        Neg(e, span) => Neg(sub_box(e), span),
        FNeg(e, span) => FNeg(sub_box(e), span),
        Not(e, span) => Not(sub_box(e), span),
        VarAssign(name, value, span) => VarAssign(name, sub_box(value), span),
        AddAssign(name, value, span) => AddAssign(name, sub_box(value), span),
        SubAssign(name, value, span) => SubAssign(name, sub_box(value), span),
        Return(value, span) => Return(sub_box(value), span),
        If(cond, then_branch, else_branch, span) => If(
            sub_box(cond),
            sub_box(then_branch),
            else_branch.map(sub_box),
            span,
        ),
        While(cond, body, span) => While(sub_box(cond), sub_box(body), span),
        Block(body, span) => Block(body.into_iter().map(sub_val).collect(), span),
        Index(arr, idx, span) => Index(sub_box(arr), sub_box(idx), span),
        IndexAssign(arr, idx, span) => IndexAssign(sub_box(arr), sub_box(idx), span),
        ArrayLiteral(elements, span) => {
            ArrayLiteral(elements.into_iter().map(sub_val).collect(), span)
        }
        Range(start, end, span) => Range(sub_box(start), sub_box(end), span),
        For(var, iter, body, span) => For(var, sub_box(iter), sub_box(body), span),
        MemberAccess(obj, field, span) => MemberAccess(sub_box(obj), field, span),
        MemberAssign(obj, field, value, span) => {
            MemberAssign(sub_box(obj), field, sub_box(value), span)
        }
        AddressOf(e, span) => AddressOf(sub_box(e), span),
        Deref(e, span) => Deref(sub_box(e), span),
        DerefAssign(ptr, value, span) => DerefAssign(sub_box(ptr), sub_box(value), span),
        other => other,
    }
}
