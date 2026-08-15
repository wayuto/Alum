use crate::compiler::{
    Span,
    codegen::CodeGenError,
    parser::{Expr, Type},
};
use std::collections::{HashMap, HashSet};

const LAMBDA_MARKER: &str = "\u{03bb}";
const IMPURE_LAMBDA_MARKER: &str = "!\u{03bb}";

pub fn check_pure_functions(body: &[Expr]) -> Result<(), CodeGenError> {
    let mut decls: HashMap<&str, (bool, bool, &Expr)> = HashMap::new();
    for expr in body {
        if let Expr::FuncDecl(name, attrs, _, _, _, func_body, _) = expr {
            decls.insert(name, (attrs.is_pure, attrs.is_external, func_body));
        }
    }

    let globals: HashSet<String> = body
        .iter()
        .filter_map(|e| match e {
            Expr::GlobalVar(name, ..) | Expr::ConstDecl(name, ..) => Some(name.to_string()),
            _ => None,
        })
        .collect();

    let mut in_progress: HashSet<String> = HashSet::new();
    let mut memo: HashMap<String, bool> = HashMap::new();

    for expr in body {
        let Expr::FuncDecl(name, attrs, _, _, _, func_body, span) = expr else {
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

pub(super) fn check_lambda_params(program_body: &[Expr]) -> Result<(), CodeGenError> {
    fn check_params(kind: &str, name: &str, params: &[(String, Type)]) -> Result<(), CodeGenError> {
        for (pname, ty) in params {
            if type_has_pointer(ty) {
                return Err(CodeGenError::TypeError {
                    message: format!(
                        "{kind} '{name}' has pointer parameter '{pname}' ({kind}s may not take pointer parameters)",
                    ),
                });
            }
        }
        Ok(())
    }

    fn walk_field_value(expr: &Expr) -> Result<(), CodeGenError> {
        match expr {
            Expr::Lambda(_, body, _, _) => walk(body),
            other => walk(other),
        }
    }

    fn walk(expr: &Expr) -> Result<(), CodeGenError> {
        use Expr::*;
        match expr {
            Lambda(params, body, _, _) => {
                check_params("lambda", "<lambda>", params)?;
                walk(body)
            }
            FuncDecl(name, attrs, _, params, _, body, _) => {
                if !attrs.is_external {
                    check_params("function", name, params)?;
                }
                walk(body)
            }
            Call(f, _, args, _) => {
                walk(f)?;
                for a in args {
                    walk(a)?;
                }
                Ok(())
            }
            Block(stmts, _) => {
                for s in stmts {
                    walk(s)?;
                }
                Ok(())
            }
            If(c, t, e, _) => {
                walk(c)?;
                walk(t)?;
                if let Some(x) = e {
                    walk(x)?;
                }
                Ok(())
            }
            While(c, b, _) => {
                walk(c)?;
                walk(b)
            }
            For(_, i, b, _) => {
                walk(i)?;
                walk(b)
            }
            Range(l, r, _) => {
                walk(l)?;
                walk(r)
            }
            Match(s, arms, d, _) => {
                walk(s)?;
                for (p, a) in arms {
                    walk(p)?;
                    walk(a)?;
                }
                if let Some(x) = d {
                    walk(x)?;
                }
                Ok(())
            }
            Return(v, _)
            | Not(v, _)
            | BNot(v, _)
            | Neg(v, _)
            | FNeg(v, _)
            | AddressOf(v, _)
            | Deref(v, _) => walk(v),
            VarDecl(_, _, v, _) | ConstDecl(_, _, v, _, _) => walk(v),
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
            | Shl(l, r, _)
            | Shr(l, r, _)
            | StrCat(l, r, _)
            | Index(l, r, _)
            | DerefAssign(l, r, _) => {
                walk(l)?;
                walk(r)
            }
            IndexAssign(o, v, _) | MemberAssign(o, _, v, _) => {
                walk(o)?;
                walk(v)
            }
            ArrayLiteral(items, _) => {
                for it in items {
                    walk(it)?;
                }
                Ok(())
            }
            ArrayFill(_, len, _) => walk(len),
            StructLiteral(_, _, fields, _) | UnionLiteral(_, _, fields, _) => {
                for (_, v) in fields {
                    walk_field_value(v)?;
                }
                Ok(())
            }
            MemberAccess(o, _, _) => walk(o),
            VarAssign(_, v, _)
            | AddAssign(_, v, _)
            | SubAssign(_, v, _)
            | MulAssign(_, v, _)
            | DivAssign(_, v, _)
            | ModAssign(_, v, _)
            | AndAssign(_, v, _)
            | OrAssign(_, v, _)
            | XorAssign(_, v, _)
            | ShlAssign(_, v, _)
            | ShrAssign(_, v, _) => walk(v),
            FString(parts, _) => {
                for p in parts {
                    walk(p)?;
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }

    for expr in program_body {
        walk(expr)?;
    }
    Ok(())
}

fn type_has_pointer(ty: &Type) -> bool {
    match ty {
        Type::Pointer(_) => true,
        Type::Array(inner) => type_has_pointer(inner),
        Type::Function(params, ret) => params.iter().any(type_has_pointer) || type_has_pointer(ret),
        _ => false,
    }
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
        Int(..) | Float(..) | Bool(..) | String(..) | Nil(_) | Var(..) | Break(_) | Continue(_)
        | TypeDef(_) | Struct(..) | Union(..) | Enum(..) => Ok(()),

        Call(callee, _, args, _) => {
            if let Some(name) = callee_name(callee) {
                if let Some(target) = bound.get(name) {
                    if let Some(lambda_name) = target.strip_prefix(IMPURE_LAMBDA_MARKER) {
                        return Err(format!("call to impure lambda '{lambda_name}'"));
                    }
                    if target != LAMBDA_MARKER
                        && !is_pure(target, decls, globals, in_progress, memo)
                    {
                        return Err(format!("call to '{target}'"));
                    }
                } else if !is_pure(name, decls, globals, in_progress, memo) {
                    return Err(format!("call to '{name}'"));
                }
            }
            classify(fn_name, callee, decls, globals, in_progress, memo, bound)?;
            for arg in args {
                classify(fn_name, arg, decls, globals, in_progress, memo, bound)?;
            }
            Ok(())
        }

        Block(stmts, _) => {
            let saved = bound.clone();
            for s in stmts {
                classify(fn_name, s, decls, globals, in_progress, memo, bound)?;
            }
            *bound = saved;
            Ok(())
        }
        If(cond, then_e, else_e, _) => {
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
        While(cond, body, _) => {
            classify(fn_name, cond, decls, globals, in_progress, memo, bound)?;
            classify(fn_name, body, decls, globals, in_progress, memo, bound)
        }
        For(_, iter, body, _) => {
            classify(fn_name, iter, decls, globals, in_progress, memo, bound)?;
            classify(fn_name, body, decls, globals, in_progress, memo, bound)
        }
        Range(l, r, _) => {
            classify(fn_name, l, decls, globals, in_progress, memo, bound)?;
            classify(fn_name, r, decls, globals, in_progress, memo, bound)
        }
        Match(scrutinee, arms, default, _) => {
            classify(fn_name, scrutinee, decls, globals, in_progress, memo, bound)?;
            for (pat, arm) in arms {
                let saved = bound.clone();
                classify(fn_name, pat, decls, globals, in_progress, memo, bound)?;
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
        Return(value, _) => classify(fn_name, value, decls, globals, in_progress, memo, bound),
        Lambda(_, lbody, _, _) => {
            check_lambda_body(fn_name, lbody, decls, globals, in_progress, memo)
        }
        FuncDecl(_, _, _, _, _, nested, _) => {
            classify(fn_name, nested, decls, globals, in_progress, memo, bound)
        }

        GlobalVar(name, _, _, _, _) => Err(format!("declare global variable '{}'", name)),
        ExternVar(name, _, _) => Err(format!("declare extern variable '{}'", name)),
        VarDecl(name, _, value, _) => {
            let result = classify(fn_name, value, decls, globals, in_progress, memo, bound);
            if let Ok(()) = &result {
                bind_value(name, value, decls, globals, in_progress, memo, bound);
            }
            result
        }
        ConstDecl(_, _, value, _, _) => {
            classify(fn_name, value, decls, globals, in_progress, memo, bound)
        }

        Not(e, _) | BNot(e, _) | Neg(e, _) | FNeg(e, _) => {
            classify(fn_name, e, decls, globals, in_progress, memo, bound)
        }
        AddressOf(e, _) | Deref(e, _) => Err(format!(
            "dereference or take address '{}' (pointer access escapes local state)",
            match &**e {
                Var(name, _) => name.clone(),
                _ => "<expr>".to_string(),
            }
        )),
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
        | Shl(l, r, _)
        | Shr(l, r, _)
        | StrCat(l, r, _)
        | Index(l, r, _) => {
            classify(fn_name, l, decls, globals, in_progress, memo, bound)?;
            classify(fn_name, r, decls, globals, in_progress, memo, bound)
        }
        DerefAssign(_, _, _) => {
            Err("pointer store (write through pointer) in a pure function".to_string())
        }
        IndexAssign(_, _, _) => {
            Err("array store (write through pointer) in a pure function".to_string())
        }
        MemberAccess(obj, _, _) => classify(fn_name, obj, decls, globals, in_progress, memo, bound),
        MemberAssign(obj, _, value, _) => {
            classify(fn_name, obj, decls, globals, in_progress, memo, bound)?;
            classify(fn_name, value, decls, globals, in_progress, memo, bound)
        }
        VarAssign(name, value, _) => {
            if globals.contains(name) {
                return Err(format!("write to global '{}'", name));
            }
            let result = classify(fn_name, value, decls, globals, in_progress, memo, bound);
            match value.as_ref() {
                Var(..) => {
                    bind_value(name, value, decls, globals, in_progress, memo, bound);
                }
                _ => {
                    bound.remove(name);
                }
            }
            result
        }
        AddAssign(name, value, _)
        | SubAssign(name, value, _)
        | MulAssign(name, value, _)
        | DivAssign(name, value, _)
        | ModAssign(name, value, _)
        | AndAssign(name, value, _)
        | OrAssign(name, value, _)
        | XorAssign(name, value, _)
        | ShlAssign(name, value, _)
        | ShrAssign(name, value, _) => {
            if globals.contains(name) {
                return Err(format!("write to global '{}'", name));
            }
            classify(fn_name, value, decls, globals, in_progress, memo, bound)
        }
        Inc(name, _) | Dec(name, _) => {
            if globals.contains(name) {
                return Err(format!("write to global '{}'", name));
            }
            Ok(())
        }
        ArrayLiteral(items, _) => {
            for it in items {
                classify(fn_name, it, decls, globals, in_progress, memo, bound)?;
            }
            Ok(())
        }
        ArrayFill(_, size, _) => classify(fn_name, size, decls, globals, in_progress, memo, bound),
        StructLiteral(_, _, fields, _) => {
            for (_, v) in fields {
                classify(fn_name, v, decls, globals, in_progress, memo, bound)?;
            }
            Ok(())
        }
        UnionLiteral(_, _, fields, _) => {
            for (_, v) in fields {
                classify(fn_name, v, decls, globals, in_progress, memo, bound)?;
            }
            Ok(())
        }
        FString(parts, _) => {
            for p in parts {
                classify(fn_name, p, decls, globals, in_progress, memo, bound)?;
            }
            Ok(())
        }
        Cast(inner, _, _) => classify(fn_name, inner, decls, globals, in_progress, memo, bound),
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
    let Some((_, is_external, func_body)) = decls.get(name) else {
        memo.insert(name.to_string(), false);
        return false;
    };
    if *is_external {
        memo.insert(name.to_string(), false);
        return false;
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
        Expr::Var(v, _) => {
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
        Expr::Lambda(_, lbody, _, _) => {
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
        Expr::Var(name, _) => Some(name),
        _ => None,
    }
}
