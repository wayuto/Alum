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
        Expr::Lambda {
            params,
            body,
            return_type: ret_type,
            ..
        } => {
            let lambda_name = format!("_lambda_{}", lambda_counter);
            *lambda_counter += 1;

            let body = hoist_lambdas(*body, lambda_counter, lambda_map);

            let lambda_func = Expr::FuncDecl {
                name: lambda_name.clone(),
                attrs: FuncAttrs::default(),
                type_params: Vec::new(),
                params,
                return_type: ret_type,
                body: Box::new(body),
                span: Span::new(0, 0),
            };
            lambda_map.insert(lambda_name.clone(), lambda_func);

            Expr::Var {
                name: lambda_name,
                span: Span::new(0, 0),
            }
        }
        Expr::FuncDecl {
            name,
            attrs,
            type_params,
            params,
            return_type: ret_type,
            body,
            span,
        } => {
            if !type_params.is_empty() {
                return Expr::FuncDecl {
                    name,
                    attrs,
                    type_params,
                    params,
                    return_type: ret_type,
                    body,
                    span,
                };
            }
            Expr::FuncDecl {
                name,
                attrs,
                type_params,
                params,
                return_type: ret_type,
                body: Box::new(hoist_lambdas(*body, lambda_counter, lambda_map)),
                span,
            }
        }
        Expr::Block { stmts: body, .. } => Expr::Block {
            stmts: body
                .into_iter()
                .map(|e| hoist_lambdas(e, lambda_counter, lambda_map))
                .collect(),
            span: Span::new(0, 0),
        },
        Expr::Add {
            left: l, right: r, ..
        } => Expr::Add {
            left: Box::new(hoist_lambdas(*l, lambda_counter, lambda_map)),
            right: Box::new(hoist_lambdas(*r, lambda_counter, lambda_map)),
            span: Span::new(0, 0),
        },
        Expr::Sub {
            left: l, right: r, ..
        } => Expr::Sub {
            left: Box::new(hoist_lambdas(*l, lambda_counter, lambda_map)),
            right: Box::new(hoist_lambdas(*r, lambda_counter, lambda_map)),
            span: Span::new(0, 0),
        },
        Expr::Mul {
            left: l, right: r, ..
        } => Expr::Mul {
            left: Box::new(hoist_lambdas(*l, lambda_counter, lambda_map)),
            right: Box::new(hoist_lambdas(*r, lambda_counter, lambda_map)),
            span: Span::new(0, 0),
        },
        Expr::Div {
            left: l, right: r, ..
        } => Expr::Div {
            left: Box::new(hoist_lambdas(*l, lambda_counter, lambda_map)),
            right: Box::new(hoist_lambdas(*r, lambda_counter, lambda_map)),
            span: Span::new(0, 0),
        },
        Expr::Mod {
            left: l, right: r, ..
        } => Expr::Mod {
            left: Box::new(hoist_lambdas(*l, lambda_counter, lambda_map)),
            right: Box::new(hoist_lambdas(*r, lambda_counter, lambda_map)),
            span: Span::new(0, 0),
        },
        Expr::FAdd {
            left: l, right: r, ..
        } => Expr::FAdd {
            left: Box::new(hoist_lambdas(*l, lambda_counter, lambda_map)),
            right: Box::new(hoist_lambdas(*r, lambda_counter, lambda_map)),
            span: Span::new(0, 0),
        },
        Expr::FSub {
            left: l, right: r, ..
        } => Expr::FSub {
            left: Box::new(hoist_lambdas(*l, lambda_counter, lambda_map)),
            right: Box::new(hoist_lambdas(*r, lambda_counter, lambda_map)),
            span: Span::new(0, 0),
        },
        Expr::FMul {
            left: l, right: r, ..
        } => Expr::FMul {
            left: Box::new(hoist_lambdas(*l, lambda_counter, lambda_map)),
            right: Box::new(hoist_lambdas(*r, lambda_counter, lambda_map)),
            span: Span::new(0, 0),
        },
        Expr::FDiv {
            left: l, right: r, ..
        } => Expr::FDiv {
            left: Box::new(hoist_lambdas(*l, lambda_counter, lambda_map)),
            right: Box::new(hoist_lambdas(*r, lambda_counter, lambda_map)),
            span: Span::new(0, 0),
        },
        Expr::Eq {
            left: l, right: r, ..
        } => Expr::Eq {
            left: Box::new(hoist_lambdas(*l, lambda_counter, lambda_map)),
            right: Box::new(hoist_lambdas(*r, lambda_counter, lambda_map)),
            span: Span::new(0, 0),
        },
        Expr::Ne {
            left: l, right: r, ..
        } => Expr::Ne {
            left: Box::new(hoist_lambdas(*l, lambda_counter, lambda_map)),
            right: Box::new(hoist_lambdas(*r, lambda_counter, lambda_map)),
            span: Span::new(0, 0),
        },
        Expr::Lt {
            left: l, right: r, ..
        } => Expr::Lt {
            left: Box::new(hoist_lambdas(*l, lambda_counter, lambda_map)),
            right: Box::new(hoist_lambdas(*r, lambda_counter, lambda_map)),
            span: Span::new(0, 0),
        },
        Expr::Le {
            left: l, right: r, ..
        } => Expr::Le {
            left: Box::new(hoist_lambdas(*l, lambda_counter, lambda_map)),
            right: Box::new(hoist_lambdas(*r, lambda_counter, lambda_map)),
            span: Span::new(0, 0),
        },
        Expr::Gt {
            left: l, right: r, ..
        } => Expr::Gt {
            left: Box::new(hoist_lambdas(*l, lambda_counter, lambda_map)),
            right: Box::new(hoist_lambdas(*r, lambda_counter, lambda_map)),
            span: Span::new(0, 0),
        },
        Expr::Ge {
            left: l, right: r, ..
        } => Expr::Ge {
            left: Box::new(hoist_lambdas(*l, lambda_counter, lambda_map)),
            right: Box::new(hoist_lambdas(*r, lambda_counter, lambda_map)),
            span: Span::new(0, 0),
        },
        Expr::FEq {
            left: l, right: r, ..
        } => Expr::FEq {
            left: Box::new(hoist_lambdas(*l, lambda_counter, lambda_map)),
            right: Box::new(hoist_lambdas(*r, lambda_counter, lambda_map)),
            span: Span::new(0, 0),
        },
        Expr::FNe {
            left: l, right: r, ..
        } => Expr::FNe {
            left: Box::new(hoist_lambdas(*l, lambda_counter, lambda_map)),
            right: Box::new(hoist_lambdas(*r, lambda_counter, lambda_map)),
            span: Span::new(0, 0),
        },
        Expr::FLt {
            left: l, right: r, ..
        } => Expr::FLt {
            left: Box::new(hoist_lambdas(*l, lambda_counter, lambda_map)),
            right: Box::new(hoist_lambdas(*r, lambda_counter, lambda_map)),
            span: Span::new(0, 0),
        },
        Expr::FLe {
            left: l, right: r, ..
        } => Expr::FLe {
            left: Box::new(hoist_lambdas(*l, lambda_counter, lambda_map)),
            right: Box::new(hoist_lambdas(*r, lambda_counter, lambda_map)),
            span: Span::new(0, 0),
        },
        Expr::FGt {
            left: l, right: r, ..
        } => Expr::FGt {
            left: Box::new(hoist_lambdas(*l, lambda_counter, lambda_map)),
            right: Box::new(hoist_lambdas(*r, lambda_counter, lambda_map)),
            span: Span::new(0, 0),
        },
        Expr::FGe {
            left: l, right: r, ..
        } => Expr::FGe {
            left: Box::new(hoist_lambdas(*l, lambda_counter, lambda_map)),
            right: Box::new(hoist_lambdas(*r, lambda_counter, lambda_map)),
            span: Span::new(0, 0),
        },
        Expr::Not { expr: e, .. } => Expr::Not {
            expr: Box::new(hoist_lambdas(*e, lambda_counter, lambda_map)),
            span: Span::new(0, 0),
        },
        Expr::StrCat {
            left: l, right: r, ..
        } => Expr::StrCat {
            left: Box::new(hoist_lambdas(*l, lambda_counter, lambda_map)),
            right: Box::new(hoist_lambdas(*r, lambda_counter, lambda_map)),
            span: Span::new(0, 0),
        },
        Expr::VarDecl {
            name,
            ty,
            value: val,
            ..
        } => Expr::VarDecl {
            name,
            ty,
            value: Box::new(hoist_lambdas(*val, lambda_counter, lambda_map)),
            span: Span::new(0, 0),
        },
        Expr::ConstDecl {
            name,
            ty,
            value: val,
            is_pub,
            ..
        } => Expr::ConstDecl {
            name,
            ty,
            value: Box::new(hoist_lambdas(*val, lambda_counter, lambda_map)),
            is_pub,
            span: Span::new(0, 0),
        },
        Expr::GlobalVar {
            name,
            is_pub,
            ty,
            value: val,
            ..
        } => Expr::GlobalVar {
            name,
            is_pub,
            ty,
            value: val.map(|v| Box::new(hoist_lambdas(*v, lambda_counter, lambda_map))),
            span: Span::new(0, 0),
        },
        Expr::VarAssign {
            name, value: val, ..
        } => Expr::VarAssign {
            name,
            value: Box::new(hoist_lambdas(*val, lambda_counter, lambda_map)),
            span: Span::new(0, 0),
        },
        Expr::AddAssign {
            name, value: val, ..
        } => Expr::AddAssign {
            name,
            value: Box::new(hoist_lambdas(*val, lambda_counter, lambda_map)),
            span: Span::new(0, 0),
        },
        Expr::SubAssign {
            name, value: val, ..
        } => Expr::SubAssign {
            name,
            value: Box::new(hoist_lambdas(*val, lambda_counter, lambda_map)),
            span: Span::new(0, 0),
        },
        Expr::MulAssign {
            name, value: val, ..
        } => Expr::MulAssign {
            name,
            value: Box::new(hoist_lambdas(*val, lambda_counter, lambda_map)),
            span: Span::new(0, 0),
        },
        Expr::DivAssign {
            name, value: val, ..
        } => Expr::DivAssign {
            name,
            value: Box::new(hoist_lambdas(*val, lambda_counter, lambda_map)),
            span: Span::new(0, 0),
        },
        Expr::ModAssign {
            name, value: val, ..
        } => Expr::ModAssign {
            name,
            value: Box::new(hoist_lambdas(*val, lambda_counter, lambda_map)),
            span: Span::new(0, 0),
        },
        Expr::AndAssign {
            name, value: val, ..
        } => Expr::AndAssign {
            name,
            value: Box::new(hoist_lambdas(*val, lambda_counter, lambda_map)),
            span: Span::new(0, 0),
        },
        Expr::OrAssign {
            name, value: val, ..
        } => Expr::OrAssign {
            name,
            value: Box::new(hoist_lambdas(*val, lambda_counter, lambda_map)),
            span: Span::new(0, 0),
        },
        Expr::XorAssign {
            name, value: val, ..
        } => Expr::XorAssign {
            name,
            value: Box::new(hoist_lambdas(*val, lambda_counter, lambda_map)),
            span: Span::new(0, 0),
        },
        Expr::ShlAssign {
            name, value: val, ..
        } => Expr::ShlAssign {
            name,
            value: Box::new(hoist_lambdas(*val, lambda_counter, lambda_map)),
            span: Span::new(0, 0),
        },
        Expr::ShrAssign {
            name, value: val, ..
        } => Expr::ShrAssign {
            name,
            value: Box::new(hoist_lambdas(*val, lambda_counter, lambda_map)),
            span: Span::new(0, 0),
        },
        Expr::Call {
            callee: func,
            type_args,
            args,
            ..
        } => Expr::Call {
            callee: Box::new(hoist_lambdas(*func, lambda_counter, lambda_map)),
            type_args,
            args: args
                .into_iter()
                .map(|a| hoist_lambdas(a, lambda_counter, lambda_map))
                .collect(),
            span: Span::new(0, 0),
        },
        Expr::Return { value: e, .. } => Expr::Return {
            value: Box::new(hoist_lambdas(*e, lambda_counter, lambda_map)),
            span: Span::new(0, 0),
        },
        Expr::If {
            cond,
            then_branch,
            else_branch,
            ..
        } => Expr::If {
            cond: Box::new(hoist_lambdas(*cond, lambda_counter, lambda_map)),
            then_branch: Box::new(hoist_lambdas(*then_branch, lambda_counter, lambda_map)),
            else_branch: else_branch
                .map(|e| Box::new(hoist_lambdas(*e, lambda_counter, lambda_map))),
            span: Span::new(0, 0),
        },
        Expr::While { cond, body, .. } => Expr::While {
            cond: Box::new(hoist_lambdas(*cond, lambda_counter, lambda_map)),
            body: Box::new(hoist_lambdas(*body, lambda_counter, lambda_map)),
            span: Span::new(0, 0),
        },
        Expr::For {
            var,
            iterable: array,
            body,
            ..
        } => Expr::For {
            var,
            iterable: Box::new(hoist_lambdas(*array, lambda_counter, lambda_map)),
            body: Box::new(hoist_lambdas(*body, lambda_counter, lambda_map)),
            span: Span::new(0, 0),
        },
        Expr::Index {
            array: arr,
            index: idx,
            ..
        } => Expr::Index {
            array: Box::new(hoist_lambdas(*arr, lambda_counter, lambda_map)),
            index: Box::new(hoist_lambdas(*idx, lambda_counter, lambda_map)),
            span: Span::new(0, 0),
        },
        Expr::IndexAssign {
            target: arr,
            value: val,
            ..
        } => Expr::IndexAssign {
            target: Box::new(hoist_lambdas(*arr, lambda_counter, lambda_map)),
            value: Box::new(hoist_lambdas(*val, lambda_counter, lambda_map)),
            span: Span::new(0, 0),
        },
        Expr::ArrayLiteral { elements, .. } => Expr::ArrayLiteral {
            elements: elements
                .into_iter()
                .map(|e| hoist_lambdas(e, lambda_counter, lambda_map))
                .collect(),
            span: Span::new(0, 0),
        },
        Expr::ArrayFill {
            elem_type: ty, len, ..
        } => Expr::ArrayFill {
            elem_type: ty,
            len: Box::new(hoist_lambdas(*len, lambda_counter, lambda_map)),
            span: Span::new(0, 0),
        },
        Expr::Range {
            start,
            end,
            inclusive,
            ..
        } => Expr::Range {
            start: Box::new(hoist_lambdas(*start, lambda_counter, lambda_map)),
            end: Box::new(hoist_lambdas(*end, lambda_counter, lambda_map)),
            inclusive,
            span: Span::new(0, 0),
        },
        Expr::StructLiteral {
            name,
            type_args,
            fields,
            ..
        } => Expr::StructLiteral {
            name,
            type_args,
            fields: fields
                .into_iter()
                .map(|(n, e)| (n, hoist_lambdas(e, lambda_counter, lambda_map)))
                .collect(),
            span: Span::new(0, 0),
        },
        Expr::UnionLiteral {
            name,
            type_args,
            fields,
            ..
        } => Expr::UnionLiteral {
            name,
            type_args,
            fields: fields
                .into_iter()
                .map(|(n, e)| (n, hoist_lambdas(e, lambda_counter, lambda_map)))
                .collect(),
            span: Span::new(0, 0),
        },
        Expr::MemberAccess { obj, field, .. } => Expr::MemberAccess {
            obj: Box::new(hoist_lambdas(*obj, lambda_counter, lambda_map)),
            field,
            span: Span::new(0, 0),
        },
        Expr::MemberAssign {
            obj,
            field,
            value: val,
            ..
        } => Expr::MemberAssign {
            obj: Box::new(hoist_lambdas(*obj, lambda_counter, lambda_map)),
            field,
            value: Box::new(hoist_lambdas(*val, lambda_counter, lambda_map)),
            span: Span::new(0, 0),
        },
        Expr::AddressOf { expr, .. } => Expr::AddressOf {
            expr: Box::new(hoist_lambdas(*expr, lambda_counter, lambda_map)),
            span: Span::new(0, 0),
        },
        Expr::Deref { expr, .. } => Expr::Deref {
            expr: Box::new(hoist_lambdas(*expr, lambda_counter, lambda_map)),
            span: Span::new(0, 0),
        },
        Expr::DerefAssign {
            ptr, value: val, ..
        } => Expr::DerefAssign {
            ptr: Box::new(hoist_lambdas(*ptr, lambda_counter, lambda_map)),
            value: Box::new(hoist_lambdas(*val, lambda_counter, lambda_map)),
            span: Span::new(0, 0),
        },
        Expr::Match {
            target,
            branches,
            default,
            ..
        } => Expr::Match {
            target: Box::new(hoist_lambdas(*target, lambda_counter, lambda_map)),
            branches: branches
                .into_iter()
                .map(|(pat, guard, arm)| {
                    (
                        hoist_lambdas(pat, lambda_counter, lambda_map),
                        guard.map(|g| Box::new(hoist_lambdas(*g, lambda_counter, lambda_map))),
                        hoist_lambdas(arm, lambda_counter, lambda_map),
                    )
                })
                .collect(),
            default: default.map(|d| Box::new(hoist_lambdas(*d, lambda_counter, lambda_map))),
            span: Span::new(0, 0),
        },
        Expr::BNot { expr: e, .. } => Expr::BNot {
            expr: Box::new(hoist_lambdas(*e, lambda_counter, lambda_map)),
            span: Span::new(0, 0),
        },
        Expr::Neg { expr: e, .. } => Expr::Neg {
            expr: Box::new(hoist_lambdas(*e, lambda_counter, lambda_map)),
            span: Span::new(0, 0),
        },
        Expr::FNeg { expr: e, .. } => Expr::FNeg {
            expr: Box::new(hoist_lambdas(*e, lambda_counter, lambda_map)),
            span: Span::new(0, 0),
        },
        Expr::Xor {
            left: l, right: r, ..
        } => Expr::Xor {
            left: Box::new(hoist_lambdas(*l, lambda_counter, lambda_map)),
            right: Box::new(hoist_lambdas(*r, lambda_counter, lambda_map)),
            span: Span::new(0, 0),
        },
        Expr::BAnd {
            left: l, right: r, ..
        } => Expr::BAnd {
            left: Box::new(hoist_lambdas(*l, lambda_counter, lambda_map)),
            right: Box::new(hoist_lambdas(*r, lambda_counter, lambda_map)),
            span: Span::new(0, 0),
        },
        Expr::BOr {
            left: l, right: r, ..
        } => Expr::BOr {
            left: Box::new(hoist_lambdas(*l, lambda_counter, lambda_map)),
            right: Box::new(hoist_lambdas(*r, lambda_counter, lambda_map)),
            span: Span::new(0, 0),
        },
        Expr::LAnd {
            left: l, right: r, ..
        } => Expr::LAnd {
            left: Box::new(hoist_lambdas(*l, lambda_counter, lambda_map)),
            right: Box::new(hoist_lambdas(*r, lambda_counter, lambda_map)),
            span: Span::new(0, 0),
        },
        Expr::LOr {
            left: l, right: r, ..
        } => Expr::LOr {
            left: Box::new(hoist_lambdas(*l, lambda_counter, lambda_map)),
            right: Box::new(hoist_lambdas(*r, lambda_counter, lambda_map)),
            span: Span::new(0, 0),
        },
        Expr::Shl {
            left: l, right: r, ..
        } => Expr::Shl {
            left: Box::new(hoist_lambdas(*l, lambda_counter, lambda_map)),
            right: Box::new(hoist_lambdas(*r, lambda_counter, lambda_map)),
            span: Span::new(0, 0),
        },
        Expr::Shr {
            left: l, right: r, ..
        } => Expr::Shr {
            left: Box::new(hoist_lambdas(*l, lambda_counter, lambda_map)),
            right: Box::new(hoist_lambdas(*r, lambda_counter, lambda_map)),
            span: Span::new(0, 0),
        },
        Expr::Cast { expr: e, ty, .. } => Expr::Cast {
            expr: Box::new(hoist_lambdas(*e, lambda_counter, lambda_map)),
            ty,
            span: Span::new(0, 0),
        },
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
