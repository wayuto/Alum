use super::context::{Context, Symbol};
use super::ir::{IRFunction, IRType, Instruction, Op, Operand};
use crate::compiler::{
    Span,
    codegen::CodeGenError,
    irgen::IRGen,
    parser::{Expr, FuncAttrs, Type},
};
use std::mem::take;

impl IRGen {
    pub(super) fn func_decl(
        &mut self,
        name: String,
        attrs: FuncAttrs,
        params: Vec<(String, Type)>,
        ret_type: Type,
    ) -> Result<(), CodeGenError> {
        let ir_params: Vec<(Operand, IRType)> = params
            .iter()
            .map(|(name, typ)| (Operand::Var(name.clone()), Context::type_to_ir_type(typ)))
            .collect();
        let ir_ret_type = Context::type_to_ir_type(&ret_type);
        let is_pub = attrs.is_pub || name == "main";
        self.functions.push(IRFunction {
            name,
            aliases: Vec::new(),
            params: ir_params,
            ret_type: ir_ret_type,
            instructions: Vec::new(),
            is_pub,
            is_external: attrs.is_external,
            link_name: attrs.link_name,
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
                let slot = if ctx.var_slots.contains_key(pname) {
                    let depth = ctx.var_slots.get(pname).map(|v| v.len()).unwrap_or(0);
                    format!("{}${}", pname, depth)
                } else {
                    pname.clone()
                };
                if let Some(scope) = ctx.scope.last_mut() {
                    scope.insert(
                        pname.clone(),
                        Symbol {
                            ir_type: ty.clone(),
                            slot: slot.clone(),
                        },
                    );
                }
                ctx.var_slots
                    .entry(pname.clone())
                    .or_insert_with(Vec::new)
                    .push(slot);
                if let Some((_, hty)) = params.get(i) {
                    ctx.var_types.insert(pname.clone(), hty.clone());
                }
            }
        }

        let last_op = self.compile_expr(body, &mut ctx)?;
        self.emit_scope_frees(&mut ctx)?;
        ctx.exit_scope()?;
        if std::env::var("ALC_DEBUG_IR").is_ok() {
            eprintln!("=== IR {} ===", name);
            for (i, inst) in ctx.instructions.iter().enumerate() {
                eprintln!("{:3} {:?}", i, inst);
            }
        }

        let last_inst_op = ctx.instructions.last().map(|i| i.op.clone());
        let last_is_return = matches!(last_inst_op, Some(Op::Return(_)));

        if !last_is_return {
            let last_is_void = matches!(
                ctx.get_operand_type(&last_op, &self.constants),
                Ok(IRType::Void)
            );
            let reg = if func.ret_type == IRType::Float {
                "xmm0".to_string()
            } else {
                "rax".to_string()
            };
            ctx.instructions.push(Instruction {
                op: Op::Return(reg),
                dst: None,
                src1: if last_is_void { None } else { Some(last_op) },
                src2: None,
            });
        }

        if let Some(f) = self.functions.iter_mut().rev().find(|f| f.name == name) {
            f.instructions = take(&mut ctx.instructions);
        }
        Ok(())
    }

    pub(super) fn lookup_func(&self, name: &str) -> Option<&IRFunction> {
        self.functions
            .iter()
            .rev()
            .find(|f| f.name == *name || f.aliases.iter().any(|a| a == name))
    }
    pub(super) fn has_func(&self, name: &str) -> bool {
        self.lookup_func(name).is_some()
    }

    pub(super) fn find_func(&self, name: &str) -> Result<IRFunction, CodeGenError> {
        self.lookup_func(name)
            .cloned()
            .ok_or_else(|| CodeGenError::UndefinedFunction {
                name: name.to_string(),
                span: Span::new(0, 0),
            })
    }

    pub(super) fn monomorphize(
        &mut self,
        name: &str,
        type_args: &[Type],
    ) -> Result<String, CodeGenError> {
        const MAX_MONO_DEPTH: usize = 64;
        if self.mono_depth >= MAX_MONO_DEPTH {
            return Err(CodeGenError::TypeError {
                message: format!(
                    "generic instantiation of '{}' exceeds maximum depth {} (recursive generic instantiation?)",
                    name, MAX_MONO_DEPTH
                ),
            });
        }
        self.mono_depth += 1;
        let result = self.monomorphize_inner(name, type_args);
        self.mono_depth -= 1;
        result
    }

    fn monomorphize_inner(
        &mut self,
        name: &str,
        type_args: &[Type],
    ) -> Result<String, CodeGenError> {
        let mangled = format!(
            "__mono_{}_{}",
            name,
            type_args
                .iter()
                .map(|t| t.mangle())
                .collect::<Vec<_>>()
                .join("_")
        );

        if self.has_func(&mangled) {
            return Ok(mangled);
        }

        if self.functions.len() > 20_000 {
            return Err(CodeGenError::TypeError {
                message: format!(
                    "too many generic instantiations (>20000); possible recursive generic '{}' with type args {:?}",
                    name, type_args
                ),
            });
        }

        let (type_params, params, ret_type, body) = self
            .generic_funcs
            .get(name)
            .ok_or_else(|| CodeGenError::UndefinedFunction {
                name: name.to_string(),
                span: Span::new(0, 0),
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
            if let Expr::FuncDecl(name, _, _, params, ret_type, _, _) = lambda {
                self.func_decl(
                    name.clone(),
                    FuncAttrs::default(),
                    params.clone(),
                    ret_type.clone(),
                )?;
            }
        }
        for lambda in &lambda_funcs {
            if let Expr::FuncDecl(name, _, _, params, _, body, _) = lambda {
                self.compile_fn(name.clone(), params.clone(), body.as_ref().clone())?;
            }
        }

        self.func_decl(
            mangled.clone(),
            FuncAttrs::default(),
            concrete_params.clone(),
            concrete_ret,
        )?;
        self.compile_fn(mangled.clone(), concrete_params, concrete_body)?;

        Ok(mangled)
    }
}

pub(super) fn substitute_expr(expr: Expr, args: &[Type]) -> Expr {
    use Expr::*;
    let sub_box = |e: Box<Expr>| Box::new(substitute_expr(*e, args));
    let sub_val = |e: Expr| substitute_expr(e, args);
    match expr {
        VarDecl(name, ty, value, span) => VarDecl(name, ty.substitute(args), sub_box(value), span),
        ConstDecl(name, ty, value, is_pub, span) => {
            ConstDecl(name, ty.substitute(args), sub_box(value), is_pub, span)
        }
        GlobalVar(name, is_pub, ty, value, span) => GlobalVar(
            name,
            is_pub,
            ty.substitute(args),
            value.map(|v| sub_box(v)),
            span,
        ),
        FuncDecl(name, attrs, type_params, params, ret_type, body, span) => {
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
            FuncDecl(
                name,
                attrs,
                type_params,
                params,
                ret_type,
                sub_box(body),
                span,
            )
        }
        ExternVar(name, ty, span) => ExternVar(name, ty.substitute(args), span),
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
        Union(name, type_params, fields, span) => {
            let fields = if type_params.is_empty() {
                fields
                    .into_iter()
                    .map(|(n, t)| (n, t.substitute(args)))
                    .collect()
            } else {
                fields
            };
            Union(name, type_params, fields, span)
        }
        UnionLiteral(name, type_args, fields, span) => UnionLiteral(
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

        Match(target, branches, default, span) => Match(
            sub_box(target),
            branches
                .into_iter()
                .map(|(c, v)| (sub_val(c), sub_val(v)))
                .collect(),
            default.map(sub_box),
            span,
        ),
        Cast(inner, ty, span) => Cast(sub_box(inner), ty.substitute(args), span),
        FString(parts, span) => FString(parts.into_iter().map(sub_val).collect(), span),
        Inc(name, span) => Inc(name, span),
        Dec(name, span) => Dec(name, span),
        MulAssign(name, value, span) => MulAssign(name, sub_box(value), span),
        DivAssign(name, value, span) => DivAssign(name, sub_box(value), span),
        ModAssign(name, value, span) => ModAssign(name, sub_box(value), span),
        AndAssign(name, value, span) => AndAssign(name, sub_box(value), span),
        OrAssign(name, value, span) => OrAssign(name, sub_box(value), span),
        XorAssign(name, value, span) => XorAssign(name, sub_box(value), span),
        ShlAssign(name, value, span) => ShlAssign(name, sub_box(value), span),
        ShrAssign(name, value, span) => ShrAssign(name, sub_box(value), span),
        other => other,
    }
}
