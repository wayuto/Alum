use crate::compiler::{
    Span,
    irgen::IRGen,
    parser::{Expr, FuncAttrs, Program},
};
use std::collections::HashMap;

pub(super) fn hoist_lambdas(
    expr: Expr,
    lambda_counter: &mut u32,
    lambda_map: &mut HashMap<String, Expr>,
) -> Expr {
    match expr {
        Expr::Lambda(params, body, ret_type, _) => {
            let lambda_name = format!("_lambda_{}", lambda_counter);
            *lambda_counter += 1;

            let body = hoist_lambdas(*body, lambda_counter, lambda_map);

            let lambda_func = Expr::FuncDecl(
                lambda_name.clone(),
                FuncAttrs::default(),
                Vec::new(),
                params,
                ret_type,
                Box::new(body),
                Span::new(0, 0),
            );
            lambda_map.insert(lambda_name.clone(), lambda_func);

            Expr::Var(lambda_name, Span::new(0, 0))
        }
        Expr::FuncDecl(name, attrs, type_params, params, ret_type, body, span) => {
            if !type_params.is_empty() {
                return Expr::FuncDecl(name, attrs, type_params, params, ret_type, body, span);
            }
            Expr::FuncDecl(
                name,
                attrs,
                type_params,
                params,
                ret_type,
                Box::new(hoist_lambdas(*body, lambda_counter, lambda_map)),
                span,
            )
        }
        Expr::Block(body, _) => Expr::Block(
            body.into_iter()
                .map(|e| hoist_lambdas(e, lambda_counter, lambda_map))
                .collect(),
            Span::new(0, 0),
        ),
        Expr::Add(l, r, _) => Expr::Add(
            Box::new(hoist_lambdas(*l, lambda_counter, lambda_map)),
            Box::new(hoist_lambdas(*r, lambda_counter, lambda_map)),
            Span::new(0, 0),
        ),
        Expr::Sub(l, r, _) => Expr::Sub(
            Box::new(hoist_lambdas(*l, lambda_counter, lambda_map)),
            Box::new(hoist_lambdas(*r, lambda_counter, lambda_map)),
            Span::new(0, 0),
        ),
        Expr::Mul(l, r, _) => Expr::Mul(
            Box::new(hoist_lambdas(*l, lambda_counter, lambda_map)),
            Box::new(hoist_lambdas(*r, lambda_counter, lambda_map)),
            Span::new(0, 0),
        ),
        Expr::Div(l, r, _) => Expr::Div(
            Box::new(hoist_lambdas(*l, lambda_counter, lambda_map)),
            Box::new(hoist_lambdas(*r, lambda_counter, lambda_map)),
            Span::new(0, 0),
        ),
        Expr::Mod(l, r, _) => Expr::Mod(
            Box::new(hoist_lambdas(*l, lambda_counter, lambda_map)),
            Box::new(hoist_lambdas(*r, lambda_counter, lambda_map)),
            Span::new(0, 0),
        ),
        Expr::FAdd(l, r, _) => Expr::FAdd(
            Box::new(hoist_lambdas(*l, lambda_counter, lambda_map)),
            Box::new(hoist_lambdas(*r, lambda_counter, lambda_map)),
            Span::new(0, 0),
        ),
        Expr::FSub(l, r, _) => Expr::FSub(
            Box::new(hoist_lambdas(*l, lambda_counter, lambda_map)),
            Box::new(hoist_lambdas(*r, lambda_counter, lambda_map)),
            Span::new(0, 0),
        ),
        Expr::FMul(l, r, _) => Expr::FMul(
            Box::new(hoist_lambdas(*l, lambda_counter, lambda_map)),
            Box::new(hoist_lambdas(*r, lambda_counter, lambda_map)),
            Span::new(0, 0),
        ),
        Expr::FDiv(l, r, _) => Expr::FDiv(
            Box::new(hoist_lambdas(*l, lambda_counter, lambda_map)),
            Box::new(hoist_lambdas(*r, lambda_counter, lambda_map)),
            Span::new(0, 0),
        ),
        Expr::Eq(l, r, _) => Expr::Eq(
            Box::new(hoist_lambdas(*l, lambda_counter, lambda_map)),
            Box::new(hoist_lambdas(*r, lambda_counter, lambda_map)),
            Span::new(0, 0),
        ),
        Expr::Ne(l, r, _) => Expr::Ne(
            Box::new(hoist_lambdas(*l, lambda_counter, lambda_map)),
            Box::new(hoist_lambdas(*r, lambda_counter, lambda_map)),
            Span::new(0, 0),
        ),
        Expr::Lt(l, r, _) => Expr::Lt(
            Box::new(hoist_lambdas(*l, lambda_counter, lambda_map)),
            Box::new(hoist_lambdas(*r, lambda_counter, lambda_map)),
            Span::new(0, 0),
        ),
        Expr::Le(l, r, _) => Expr::Le(
            Box::new(hoist_lambdas(*l, lambda_counter, lambda_map)),
            Box::new(hoist_lambdas(*r, lambda_counter, lambda_map)),
            Span::new(0, 0),
        ),
        Expr::Gt(l, r, _) => Expr::Gt(
            Box::new(hoist_lambdas(*l, lambda_counter, lambda_map)),
            Box::new(hoist_lambdas(*r, lambda_counter, lambda_map)),
            Span::new(0, 0),
        ),
        Expr::Ge(l, r, _) => Expr::Ge(
            Box::new(hoist_lambdas(*l, lambda_counter, lambda_map)),
            Box::new(hoist_lambdas(*r, lambda_counter, lambda_map)),
            Span::new(0, 0),
        ),
        Expr::FEq(l, r, _) => Expr::FEq(
            Box::new(hoist_lambdas(*l, lambda_counter, lambda_map)),
            Box::new(hoist_lambdas(*r, lambda_counter, lambda_map)),
            Span::new(0, 0),
        ),
        Expr::FNe(l, r, _) => Expr::FNe(
            Box::new(hoist_lambdas(*l, lambda_counter, lambda_map)),
            Box::new(hoist_lambdas(*r, lambda_counter, lambda_map)),
            Span::new(0, 0),
        ),
        Expr::FLt(l, r, _) => Expr::FLt(
            Box::new(hoist_lambdas(*l, lambda_counter, lambda_map)),
            Box::new(hoist_lambdas(*r, lambda_counter, lambda_map)),
            Span::new(0, 0),
        ),
        Expr::FLe(l, r, _) => Expr::FLe(
            Box::new(hoist_lambdas(*l, lambda_counter, lambda_map)),
            Box::new(hoist_lambdas(*r, lambda_counter, lambda_map)),
            Span::new(0, 0),
        ),
        Expr::FGt(l, r, _) => Expr::FGt(
            Box::new(hoist_lambdas(*l, lambda_counter, lambda_map)),
            Box::new(hoist_lambdas(*r, lambda_counter, lambda_map)),
            Span::new(0, 0),
        ),
        Expr::FGe(l, r, _) => Expr::FGe(
            Box::new(hoist_lambdas(*l, lambda_counter, lambda_map)),
            Box::new(hoist_lambdas(*r, lambda_counter, lambda_map)),
            Span::new(0, 0),
        ),
        Expr::Not(e, _) => Expr::Not(
            Box::new(hoist_lambdas(*e, lambda_counter, lambda_map)),
            Span::new(0, 0),
        ),
        Expr::StrCat(l, r, _) => Expr::StrCat(
            Box::new(hoist_lambdas(*l, lambda_counter, lambda_map)),
            Box::new(hoist_lambdas(*r, lambda_counter, lambda_map)),
            Span::new(0, 0),
        ),
        Expr::VarDecl(name, ty, val, _) => Expr::VarDecl(
            name,
            ty,
            Box::new(hoist_lambdas(*val, lambda_counter, lambda_map)),
            Span::new(0, 0),
        ),
        Expr::ConstDecl(name, ty, val, is_pub, _) => Expr::ConstDecl(
            name,
            ty,
            Box::new(hoist_lambdas(*val, lambda_counter, lambda_map)),
            is_pub,
            Span::new(0, 0),
        ),
        Expr::GlobalVar(name, is_pub, ty, val, _) => Expr::GlobalVar(
            name,
            is_pub,
            ty,
            val.map(|v| Box::new(hoist_lambdas(*v, lambda_counter, lambda_map))),
            Span::new(0, 0),
        ),
        Expr::VarAssign(name, val, _) => Expr::VarAssign(
            name,
            Box::new(hoist_lambdas(*val, lambda_counter, lambda_map)),
            Span::new(0, 0),
        ),
        Expr::AddAssign(name, val, _) => Expr::AddAssign(
            name,
            Box::new(hoist_lambdas(*val, lambda_counter, lambda_map)),
            Span::new(0, 0),
        ),
        Expr::SubAssign(name, val, _) => Expr::SubAssign(
            name,
            Box::new(hoist_lambdas(*val, lambda_counter, lambda_map)),
            Span::new(0, 0),
        ),
        Expr::MulAssign(name, val, _) => Expr::MulAssign(
            name,
            Box::new(hoist_lambdas(*val, lambda_counter, lambda_map)),
            Span::new(0, 0),
        ),
        Expr::DivAssign(name, val, _) => Expr::DivAssign(
            name,
            Box::new(hoist_lambdas(*val, lambda_counter, lambda_map)),
            Span::new(0, 0),
        ),
        Expr::ModAssign(name, val, _) => Expr::ModAssign(
            name,
            Box::new(hoist_lambdas(*val, lambda_counter, lambda_map)),
            Span::new(0, 0),
        ),
        Expr::AndAssign(name, val, _) => Expr::AndAssign(
            name,
            Box::new(hoist_lambdas(*val, lambda_counter, lambda_map)),
            Span::new(0, 0),
        ),
        Expr::OrAssign(name, val, _) => Expr::OrAssign(
            name,
            Box::new(hoist_lambdas(*val, lambda_counter, lambda_map)),
            Span::new(0, 0),
        ),
        Expr::XorAssign(name, val, _) => Expr::XorAssign(
            name,
            Box::new(hoist_lambdas(*val, lambda_counter, lambda_map)),
            Span::new(0, 0),
        ),
        Expr::ShlAssign(name, val, _) => Expr::ShlAssign(
            name,
            Box::new(hoist_lambdas(*val, lambda_counter, lambda_map)),
            Span::new(0, 0),
        ),
        Expr::ShrAssign(name, val, _) => Expr::ShrAssign(
            name,
            Box::new(hoist_lambdas(*val, lambda_counter, lambda_map)),
            Span::new(0, 0),
        ),
        Expr::Call(func, type_args, args, _) => Expr::Call(
            Box::new(hoist_lambdas(*func, lambda_counter, lambda_map)),
            type_args,
            args.into_iter()
                .map(|a| hoist_lambdas(a, lambda_counter, lambda_map))
                .collect(),
            Span::new(0, 0),
        ),
        Expr::Return(e, _) => Expr::Return(
            Box::new(hoist_lambdas(*e, lambda_counter, lambda_map)),
            Span::new(0, 0),
        ),
        Expr::If(cond, then_branch, else_branch, _) => Expr::If(
            Box::new(hoist_lambdas(*cond, lambda_counter, lambda_map)),
            Box::new(hoist_lambdas(*then_branch, lambda_counter, lambda_map)),
            else_branch.map(|e| Box::new(hoist_lambdas(*e, lambda_counter, lambda_map))),
            Span::new(0, 0),
        ),
        Expr::While(cond, body, _) => Expr::While(
            Box::new(hoist_lambdas(*cond, lambda_counter, lambda_map)),
            Box::new(hoist_lambdas(*body, lambda_counter, lambda_map)),
            Span::new(0, 0),
        ),
        Expr::For(var, array, body, _) => Expr::For(
            var,
            Box::new(hoist_lambdas(*array, lambda_counter, lambda_map)),
            Box::new(hoist_lambdas(*body, lambda_counter, lambda_map)),
            Span::new(0, 0),
        ),
        Expr::Index(arr, idx, _) => Expr::Index(
            Box::new(hoist_lambdas(*arr, lambda_counter, lambda_map)),
            Box::new(hoist_lambdas(*idx, lambda_counter, lambda_map)),
            Span::new(0, 0),
        ),
        Expr::IndexAssign(arr, val, _) => Expr::IndexAssign(
            Box::new(hoist_lambdas(*arr, lambda_counter, lambda_map)),
            Box::new(hoist_lambdas(*val, lambda_counter, lambda_map)),
            Span::new(0, 0),
        ),
        Expr::ArrayLiteral(elements, _) => Expr::ArrayLiteral(
            elements
                .into_iter()
                .map(|e| hoist_lambdas(e, lambda_counter, lambda_map))
                .collect(),
            Span::new(0, 0),
        ),
        Expr::ArrayFill(ty, len, _) => Expr::ArrayFill(
            ty,
            Box::new(hoist_lambdas(*len, lambda_counter, lambda_map)),
            Span::new(0, 0),
        ),
        Expr::Range(start, end, _) => Expr::Range(
            Box::new(hoist_lambdas(*start, lambda_counter, lambda_map)),
            Box::new(hoist_lambdas(*end, lambda_counter, lambda_map)),
            Span::new(0, 0),
        ),
        Expr::StructLiteral(name, type_args, fields, _) => Expr::StructLiteral(
            name,
            type_args,
            fields
                .into_iter()
                .map(|(n, e)| (n, hoist_lambdas(e, lambda_counter, lambda_map)))
                .collect(),
            Span::new(0, 0),
        ),
        Expr::UnionLiteral(name, type_args, fields, _) => Expr::UnionLiteral(
            name,
            type_args,
            fields
                .into_iter()
                .map(|(n, e)| (n, hoist_lambdas(e, lambda_counter, lambda_map)))
                .collect(),
            Span::new(0, 0),
        ),
        Expr::MemberAccess(obj, field, _) => Expr::MemberAccess(
            Box::new(hoist_lambdas(*obj, lambda_counter, lambda_map)),
            field,
            Span::new(0, 0),
        ),
        Expr::MemberAssign(obj, field, val, _) => Expr::MemberAssign(
            Box::new(hoist_lambdas(*obj, lambda_counter, lambda_map)),
            field,
            Box::new(hoist_lambdas(*val, lambda_counter, lambda_map)),
            Span::new(0, 0),
        ),
        Expr::AddressOf(expr, _) => Expr::AddressOf(
            Box::new(hoist_lambdas(*expr, lambda_counter, lambda_map)),
            Span::new(0, 0),
        ),
        Expr::Deref(expr, _) => Expr::Deref(
            Box::new(hoist_lambdas(*expr, lambda_counter, lambda_map)),
            Span::new(0, 0),
        ),
        Expr::DerefAssign(ptr, val, _) => Expr::DerefAssign(
            Box::new(hoist_lambdas(*ptr, lambda_counter, lambda_map)),
            Box::new(hoist_lambdas(*val, lambda_counter, lambda_map)),
            Span::new(0, 0),
        ),
        Expr::Match(target, branches, default, _) => Expr::Match(
            Box::new(hoist_lambdas(*target, lambda_counter, lambda_map)),
            branches
                .into_iter()
                .map(|(pat, arm)| {
                    (
                        hoist_lambdas(pat, lambda_counter, lambda_map),
                        hoist_lambdas(arm, lambda_counter, lambda_map),
                    )
                })
                .collect(),
            default.map(|d| Box::new(hoist_lambdas(*d, lambda_counter, lambda_map))),
            Span::new(0, 0),
        ),
        Expr::BNot(e, _) => Expr::BNot(
            Box::new(hoist_lambdas(*e, lambda_counter, lambda_map)),
            Span::new(0, 0),
        ),
        Expr::Neg(e, _) => Expr::Neg(
            Box::new(hoist_lambdas(*e, lambda_counter, lambda_map)),
            Span::new(0, 0),
        ),
        Expr::FNeg(e, _) => Expr::FNeg(
            Box::new(hoist_lambdas(*e, lambda_counter, lambda_map)),
            Span::new(0, 0),
        ),
        Expr::Xor(l, r, _) => Expr::Xor(
            Box::new(hoist_lambdas(*l, lambda_counter, lambda_map)),
            Box::new(hoist_lambdas(*r, lambda_counter, lambda_map)),
            Span::new(0, 0),
        ),
        Expr::LAnd(l, r, _) => Expr::LAnd(
            Box::new(hoist_lambdas(*l, lambda_counter, lambda_map)),
            Box::new(hoist_lambdas(*r, lambda_counter, lambda_map)),
            Span::new(0, 0),
        ),
        Expr::LOr(l, r, _) => Expr::LOr(
            Box::new(hoist_lambdas(*l, lambda_counter, lambda_map)),
            Box::new(hoist_lambdas(*r, lambda_counter, lambda_map)),
            Span::new(0, 0),
        ),
        Expr::Shl(l, r, _) => Expr::Shl(
            Box::new(hoist_lambdas(*l, lambda_counter, lambda_map)),
            Box::new(hoist_lambdas(*r, lambda_counter, lambda_map)),
            Span::new(0, 0),
        ),
        Expr::Shr(l, r, _) => Expr::Shr(
            Box::new(hoist_lambdas(*l, lambda_counter, lambda_map)),
            Box::new(hoist_lambdas(*r, lambda_counter, lambda_map)),
            Span::new(0, 0),
        ),
        Expr::Cast(e, ty, _) => Expr::Cast(
            Box::new(hoist_lambdas(*e, lambda_counter, lambda_map)),
            ty,
            Span::new(0, 0),
        ),
        _ => expr,
    }
}

impl IRGen {
    pub(super) fn lambda2function(&mut self, program: Program) -> Program {
        let mut new_body = Vec::new();
        let mut lambda_map: HashMap<String, Expr> = HashMap::new();

        for expr in program.body {
            let processed = hoist_lambdas(expr, &mut self.lambda_counter, &mut lambda_map);
            new_body.push(processed);
        }

        let lambda_funcs: Vec<Expr> = lambda_map.into_values().collect();
        new_body.splice(0..0, lambda_funcs);

        Program { body: new_body }
    }
}
