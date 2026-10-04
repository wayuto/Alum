use crate::compiler::{Span, codegen::CodeGenError, parser::Expr};
use std::collections::{HashMap, HashSet};

const LAMBDA_MARKER: &str = "\u{03bb}";
const IMPURE_LAMBDA_MARKER: &str = "!\u{03bb}";

pub fn check_pure_functions(body: &[Expr]) -> Result<(), CodeGenError> {
    let mut decls: HashMap<&str, (bool, bool, &Expr)> = HashMap::new();
    for expr in body {
        if let Expr::FuncDecl {
            name,
            attrs,
            type_params: _,
            params: _,
            return_type: _,
            body: func_body,
            ..
        } = expr
        {
            decls.insert(name, (attrs.is_pure, attrs.is_external, func_body));
        }
    }

    let globals: HashSet<String> = body
        .iter()
        .filter_map(|e| match e {
            Expr::GlobalVar { name, .. } => Some(name.to_string()),
            _ => None,
        })
        .collect();

    let mut in_progress: HashSet<String> = HashSet::new();
    let mut memo: HashMap<String, bool> = HashMap::new();

    for expr in body {
        let Expr::FuncDecl {
            name,
            attrs,
            type_params: _,
            params: _,
            return_type: _,
            body: func_body,
            span,
        } = expr
        else {
            continue;
        };
        if attrs.is_external {
            continue;
        }

        if attrs.is_pure {
            let mut bound: HashMap<String, String> = HashMap::new();
            if let Err(what) = classify(
                name,
                func_body,
                &decls,
                &globals,
                &mut in_progress,
                &mut memo,
                &mut bound,
            ) {
                return Err(op_err(name, &what, *span));
            }
        } else if name != "main"
            && !name.starts_with("_lambda")
            && classify(
                name,
                func_body,
                &decls,
                &globals,
                &mut in_progress,
                &mut memo,
                &mut HashMap::new(),
            )
            .is_ok()
        {
            eprintln!(
                "warning: function '{}' has a pure body but is not marked `fun(pure)`",
                name
            );
        }
    }
    Ok(())
}

fn op_err(fn_name: &str, what: &str, _span: Span) -> CodeGenError {
    CodeGenError::NameError {
        message: format!("pure function '{}' may not {}", fn_name, what),
    }
}

fn classify(
    fn_name: &str,
    expr: &Expr,
    decls: &HashMap<&str, (bool, bool, &Expr)>,
    globals: &HashSet<String>,
    in_progress: &mut HashSet<String>,
    memo: &mut HashMap<String, bool>,
    bound: &mut HashMap<String, String>,
) -> Result<(), String> {
    use Expr::*;

    match expr {
        Var { name, .. } if globals.contains(name) => {
            Err(format!("read mutable global '{}'", name))
        }
        Int { .. }
        | Float { .. }
        | Char { .. }
        | Bool { .. }
        | String { .. }
        | Nil(_)
        | Var { .. }
        | Continue(_)
        | TypeDef(_)
        | Struct { .. }
        | Union { .. }
        | Enum { .. } => Ok(()),
        Break { value: v, .. } => {
            if let Some(v) = v {
                classify(fn_name, v, decls, globals, in_progress, memo, bound)?;
            }
            Ok(())
        }

        Call {
            callee,
            type_args: _,
            args,
            ..
        } => {
            let Some(name) = callee_name(callee) else {
                return Err("indirect call (callee is not a function name)".to_string());
            };
            if let Some(target) = bound.get(name) {
                if let Some(lambda_name) = target.strip_prefix(IMPURE_LAMBDA_MARKER) {
                    return Err(format!("call to impure lambda '{lambda_name}'"));
                }
                if target != LAMBDA_MARKER && !is_pure(target, decls, globals, in_progress, memo) {
                    return Err(format!("call to '{target}'"));
                }
            } else if !is_pure(name, decls, globals, in_progress, memo) {
                return Err(format!("call to '{name}'"));
            }
            for arg in args {
                classify(fn_name, arg, decls, globals, in_progress, memo, bound)?;
            }
            Ok(())
        }

        Block { stmts, .. } => {
            let saved = bound.clone();
            for s in stmts {
                classify(fn_name, s, decls, globals, in_progress, memo, bound)?;
            }
            *bound = saved;
            Ok(())
        }
        If {
            cond,
            then_branch: then_e,
            else_branch: else_e,
            ..
        } => {
            classify(fn_name, cond, decls, globals, in_progress, memo, bound)?;
            let saved = bound.clone();
            classify(fn_name, then_e, decls, globals, in_progress, memo, bound)?;
            *bound = saved;
            if let Some(e) = else_e {
                let saved = bound.clone();
                classify(fn_name, e, decls, globals, in_progress, memo, bound)?;
                *bound = saved;
            }
            Ok(())
        }
        While { cond, body, .. } => {
            classify(fn_name, cond, decls, globals, in_progress, memo, bound)?;
            let saved = bound.clone();
            let r = classify(fn_name, body, decls, globals, in_progress, memo, bound);
            *bound = saved;
            r
        }
        For {
            var: _,
            iterable: iter,
            body,
            ..
        } => {
            classify(fn_name, iter, decls, globals, in_progress, memo, bound)?;
            let saved = bound.clone();
            let r = classify(fn_name, body, decls, globals, in_progress, memo, bound);
            *bound = saved;
            r
        }
        Range {
            start: l, end: r, ..
        } => {
            classify(fn_name, l, decls, globals, in_progress, memo, bound)?;
            classify(fn_name, r, decls, globals, in_progress, memo, bound)
        }
        Match {
            target: scrutinee,
            branches: arms,
            default,
            ..
        } => {
            classify(fn_name, scrutinee, decls, globals, in_progress, memo, bound)?;
            for (pat, guard, arm) in arms {
                let saved = bound.clone();
                classify(fn_name, pat, decls, globals, in_progress, memo, bound)?;
                if let Some(guard) = guard {
                    classify(fn_name, guard, decls, globals, in_progress, memo, bound)?;
                }
                classify(fn_name, arm, decls, globals, in_progress, memo, bound)?;
                *bound = saved;
            }
            if let Some(d) = default {
                let saved = bound.clone();
                classify(fn_name, d, decls, globals, in_progress, memo, bound)?;
                *bound = saved;
            }
            Ok(())
        }
        Return { value, .. } => classify(fn_name, value, decls, globals, in_progress, memo, bound),
        Lambda {
            params: _,
            body: lbody,
            ..
        } => check_lambda_body(fn_name, lbody, decls, globals, in_progress, memo),
        FuncDecl {
            name: _,
            attrs: _,
            type_params: _,
            params: _,
            return_type: _,
            body: nested,
            ..
        } => classify(fn_name, nested, decls, globals, in_progress, memo, bound),

        GlobalVar { name, .. } => Err(format!("declare global variable '{}'", name)),
        ExternVar { name, .. } => Err(format!("declare extern variable '{}'", name)),
        VarDecl {
            name, ty: _, value, ..
        } => {
            let result = classify(fn_name, value, decls, globals, in_progress, memo, bound);
            if let Ok(()) = &result {
                bind_value(name, value, decls, globals, in_progress, memo, bound);
            }
            result
        }
        ConstDecl {
            name: _,
            ty: _,
            value,
            ..
        } => classify(fn_name, value, decls, globals, in_progress, memo, bound),

        Not { expr: e, .. } | BNot { expr: e, .. } | Neg { expr: e, .. } | FNeg { expr: e, .. } => {
            classify(fn_name, e, decls, globals, in_progress, memo, bound)
        }
        AddressOf { expr: e, .. } | Deref { expr: e, .. } => Err(format!(
            "dereference or take address '{}' (pointer access escapes local state)",
            match &**e {
                Var { name, .. } => name.clone(),
                _ => "<expr>".to_string(),
            }
        )),
        Add {
            left: l, right: r, ..
        }
        | Sub {
            left: l, right: r, ..
        }
        | Mul {
            left: l, right: r, ..
        }
        | Div {
            left: l, right: r, ..
        }
        | Mod {
            left: l, right: r, ..
        }
        | FAdd {
            left: l, right: r, ..
        }
        | FSub {
            left: l, right: r, ..
        }
        | FMul {
            left: l, right: r, ..
        }
        | FDiv {
            left: l, right: r, ..
        }
        | Eq {
            left: l, right: r, ..
        }
        | Ne {
            left: l, right: r, ..
        }
        | Lt {
            left: l, right: r, ..
        }
        | Le {
            left: l, right: r, ..
        }
        | Gt {
            left: l, right: r, ..
        }
        | Ge {
            left: l, right: r, ..
        }
        | FEq {
            left: l, right: r, ..
        }
        | FNe {
            left: l, right: r, ..
        }
        | FLt {
            left: l, right: r, ..
        }
        | FLe {
            left: l, right: r, ..
        }
        | FGt {
            left: l, right: r, ..
        }
        | FGe {
            left: l, right: r, ..
        }
        | Xor {
            left: l, right: r, ..
        }
        | BAnd {
            left: l, right: r, ..
        }
        | BOr {
            left: l, right: r, ..
        }
        | LAnd {
            left: l, right: r, ..
        }
        | LOr {
            left: l, right: r, ..
        }
        | Shl {
            left: l, right: r, ..
        }
        | Shr {
            left: l, right: r, ..
        }
        | StrCat {
            left: l, right: r, ..
        }
        | Index {
            array: l, index: r, ..
        } => {
            classify(fn_name, l, decls, globals, in_progress, memo, bound)?;
            classify(fn_name, r, decls, globals, in_progress, memo, bound)
        }
        DerefAssign { .. } => {
            Err("pointer store (write through pointer) in a pure function".to_string())
        }
        IndexAssign { .. } => {
            Err("array store (write through pointer) in a pure function".to_string())
        }
        MemberAccess { obj, .. } => {
            classify(fn_name, obj, decls, globals, in_progress, memo, bound)
        }
        MemberAssign {
            obj,
            field: _,
            value,
            ..
        } => {
            classify(fn_name, obj, decls, globals, in_progress, memo, bound)?;
            classify(fn_name, value, decls, globals, in_progress, memo, bound)
        }
        VarAssign { name, value, .. } => {
            if globals.contains(name) {
                return Err(format!("write to global '{}'", name));
            }
            let result = classify(fn_name, value, decls, globals, in_progress, memo, bound);
            match value.as_ref() {
                Var { .. } => {
                    bind_value(name, value, decls, globals, in_progress, memo, bound);
                }
                _ => {
                    bound.remove(name);
                }
            }
            result
        }
        AddAssign { name, value, .. }
        | SubAssign { name, value, .. }
        | MulAssign { name, value, .. }
        | DivAssign { name, value, .. }
        | ModAssign { name, value, .. }
        | AndAssign { name, value, .. }
        | OrAssign { name, value, .. }
        | XorAssign { name, value, .. }
        | ShlAssign { name, value, .. }
        | ShrAssign { name, value, .. } => {
            if globals.contains(name) {
                return Err(format!("write to global '{}'", name));
            }
            classify(fn_name, value, decls, globals, in_progress, memo, bound)
        }
        Inc { name, .. } | Dec { name, .. } => {
            if globals.contains(name) {
                return Err(format!("write to global '{}'", name));
            }
            Ok(())
        }
        ArrayLiteral {
            elements: items, ..
        } => {
            for it in items {
                classify(fn_name, it, decls, globals, in_progress, memo, bound)?;
            }
            Ok(())
        }
        ArrayFill {
            elem_type: _,
            len: size,
            ..
        } => classify(fn_name, size, decls, globals, in_progress, memo, bound),
        StructLiteral {
            name: _,
            type_args: _,
            fields,
            ..
        } => {
            for (_, v) in fields {
                classify(fn_name, v, decls, globals, in_progress, memo, bound)?;
            }
            Ok(())
        }
        UnionLiteral {
            name: _,
            type_args: _,
            fields,
            ..
        } => {
            for (_, v) in fields {
                classify(fn_name, v, decls, globals, in_progress, memo, bound)?;
            }
            Ok(())
        }
        FString { segs: parts, .. } => {
            for p in parts {
                classify(fn_name, p, decls, globals, in_progress, memo, bound)?;
            }
            Ok(())
        }
        Cast { expr: inner, .. } => {
            classify(fn_name, inner, decls, globals, in_progress, memo, bound)
        }
    }
}

fn is_pure(
    name: &str,
    decls: &HashMap<&str, (bool, bool, &Expr)>,
    globals: &HashSet<String>,
    in_progress: &mut HashSet<String>,
    memo: &mut HashMap<String, bool>,
) -> bool {
    if let Some(&r) = memo.get(name) {
        return r;
    }
    if in_progress.contains(name) {
        return true;
    }
    let Some((declared_pure, is_external, func_body)) = decls.get(name) else {
        memo.insert(name.to_string(), false);
        return false;
    };
    if *is_external {
        memo.insert(name.to_string(), *declared_pure);
        return *declared_pure;
    }
    in_progress.insert(name.to_string());
    let mut bound: HashMap<String, String> = HashMap::new();
    let r = classify(
        name,
        func_body,
        decls,
        globals,
        in_progress,
        memo,
        &mut bound,
    )
    .is_ok();
    in_progress.remove(name);
    memo.insert(name.to_string(), r);
    r
}

fn bind_value(
    name: &str,
    value: &Expr,
    decls: &HashMap<&str, (bool, bool, &Expr)>,
    globals: &HashSet<String>,
    in_progress: &mut HashSet<String>,
    memo: &mut HashMap<String, bool>,
    bound: &mut HashMap<String, String>,
) {
    match value {
        Expr::Var { name: v, .. } => {
            if let Some(target) = bound.get(v) {
                bound.insert(name.to_string(), target.clone());
            } else if v.starts_with("_lambda_") {
                if is_pure(v, decls, globals, in_progress, memo) {
                    bound.insert(name.to_string(), v.clone());
                } else {
                    bound.insert(name.to_string(), format!("{IMPURE_LAMBDA_MARKER}{v}"));
                }
            } else if is_pure(v, decls, globals, in_progress, memo) {
                bound.insert(name.to_string(), v.clone());
            }
        }
        Expr::Lambda {
            params: _,
            body: lbody,
            ..
        } => {
            let mut inner: HashMap<String, String> = HashMap::new();
            if classify(
                "lambda",
                lbody,
                decls,
                globals,
                in_progress,
                memo,
                &mut inner,
            )
            .is_ok()
            {
                bound.insert(name.to_string(), LAMBDA_MARKER.to_string());
            }
        }
        _ => {}
    }
}

fn check_lambda_body(
    fn_name: &str,
    lbody: &Expr,
    decls: &HashMap<&str, (bool, bool, &Expr)>,
    globals: &HashSet<String>,
    in_progress: &mut HashSet<String>,
    memo: &mut HashMap<String, bool>,
) -> Result<(), String> {
    let mut bound: HashMap<String, String> = HashMap::new();
    classify(
        fn_name,
        lbody,
        decls,
        globals,
        in_progress,
        memo,
        &mut bound,
    )
}

fn callee_name(callee: &Expr) -> Option<&str> {
    match callee {
        Expr::Var { name, .. } => Some(name),
        _ => None,
    }
}
