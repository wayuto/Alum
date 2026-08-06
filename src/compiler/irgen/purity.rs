use crate::compiler::{codegen::CodeGenError, parser::Expr};
use std::collections::{HashMap, HashSet};

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
            if let Err(what) = classify(
                name,
                func_body,
                &decls,
                &globals,
                &mut in_progress,
                &mut memo,
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

fn op_err(fn_name: &str, what: &str, _span: crate::compiler::Span) -> CodeGenError {
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
) -> Result<(), String> {
    use Expr::*;

    match expr {
        Int(..) | Float(..) | Bool(..) | String(..) | Nil(_) | Var(..) | Break(_) | Continue(_)
        | TypeDef(_) | Struct(..) | Union(..) | Enum(..) => Ok(()),

        Call(callee, _, args, _) => {
            if let Some(name) = callee_name(callee) {
                if !is_pure(name, decls, globals, in_progress, memo) {
                    return Err(format!("call to '{}'", name));
                }
            }
            classify(fn_name, callee, decls, globals, in_progress, memo)?;
            for arg in args {
                classify(fn_name, arg, decls, globals, in_progress, memo)?;
            }
            Ok(())
        }

        Block(stmts, _) => {
            for s in stmts {
                classify(fn_name, s, decls, globals, in_progress, memo)?;
            }
            Ok(())
        }
        If(cond, then_e, else_e, _) => {
            classify(fn_name, cond, decls, globals, in_progress, memo)?;
            classify(fn_name, then_e, decls, globals, in_progress, memo)?;
            if let Some(e) = else_e {
                classify(fn_name, e, decls, globals, in_progress, memo)?;
            }
            Ok(())
        }
        While(cond, body, _) => {
            classify(fn_name, cond, decls, globals, in_progress, memo)?;
            classify(fn_name, body, decls, globals, in_progress, memo)
        }
        For(_, iter, body, _) => {
            classify(fn_name, iter, decls, globals, in_progress, memo)?;
            classify(fn_name, body, decls, globals, in_progress, memo)
        }
        Range(l, r, _) => {
            classify(fn_name, l, decls, globals, in_progress, memo)?;
            classify(fn_name, r, decls, globals, in_progress, memo)
        }
        Match(scrutinee, arms, default, _) => {
            classify(fn_name, scrutinee, decls, globals, in_progress, memo)?;
            for (pat, arm) in arms {
                classify(fn_name, pat, decls, globals, in_progress, memo)?;
                classify(fn_name, arm, decls, globals, in_progress, memo)?;
            }
            if let Some(d) = default {
                classify(fn_name, d, decls, globals, in_progress, memo)?;
            }
            Ok(())
        }
        Return(value, _) => classify(fn_name, value, decls, globals, in_progress, memo),
        Lambda(_, lbody, _, _) => classify(fn_name, lbody, decls, globals, in_progress, memo),
        FuncDecl(_, _, _, _, _, nested, _) => {
            classify(fn_name, nested, decls, globals, in_progress, memo)
        }

        GlobalVar(name, _, _, _, _) => Err(format!("declare global variable '{}'", name)),
        ExternVar(name, _, _) => Err(format!("declare extern variable '{}'", name)),
        VarDecl(_, _, value, _) => classify(fn_name, value, decls, globals, in_progress, memo),
        ConstDecl(_, _, value, _, _) => classify(fn_name, value, decls, globals, in_progress, memo),

        Not(e, _) | Neg(e, _) | FNeg(e, _) | AddressOf(e, _) | Deref(e, _) => {
            classify(fn_name, e, decls, globals, in_progress, memo)
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
        | DerefAssign(l, r, _) => {
            classify(fn_name, l, decls, globals, in_progress, memo)?;
            classify(fn_name, r, decls, globals, in_progress, memo)
        }
        IndexAssign(obj, value, _) => {
            classify(fn_name, obj, decls, globals, in_progress, memo)?;
            classify(fn_name, value, decls, globals, in_progress, memo)
        }
        MemberAccess(obj, _, _) => classify(fn_name, obj, decls, globals, in_progress, memo),
        MemberAssign(obj, _, value, _) => {
            classify(fn_name, obj, decls, globals, in_progress, memo)?;
            classify(fn_name, value, decls, globals, in_progress, memo)
        }
        VarAssign(name, value, _) => {
            if globals.contains(name) {
                return Err(format!("write to global '{}'", name));
            }
            classify(fn_name, value, decls, globals, in_progress, memo)
        }
        AddAssign(name, value, _) | SubAssign(name, value, _) => {
            if globals.contains(name) {
                return Err(format!("write to global '{}'", name));
            }
            classify(fn_name, value, decls, globals, in_progress, memo)
        }
        Inc(name, _) | Dec(name, _) => {
            if globals.contains(name) {
                return Err(format!("write to global '{}'", name));
            }
            Ok(())
        }
        ArrayLiteral(items, _) => {
            for it in items {
                classify(fn_name, it, decls, globals, in_progress, memo)?;
            }
            Ok(())
        }
        ArrayFill(_, size, _) => classify(fn_name, size, decls, globals, in_progress, memo),
        StructLiteral(_, _, fields, _) => {
            for (_, v) in fields {
                classify(fn_name, v, decls, globals, in_progress, memo)?;
            }
            Ok(())
        }
        UnionLiteral(_, _, fields, _) => {
            for (_, v) in fields {
                classify(fn_name, v, decls, globals, in_progress, memo)?;
            }
            Ok(())
        }
        FString(parts, _) => {
            for p in parts {
                classify(fn_name, p, decls, globals, in_progress, memo)?;
            }
            Ok(())
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
    let Some((_, is_external, func_body)) = decls.get(name) else {
        memo.insert(name.to_string(), false);
        return false;
    };
    if *is_external {
        memo.insert(name.to_string(), false);
        return false;
    }
    in_progress.insert(name.to_string());
    let r = classify(name, func_body, decls, globals, in_progress, memo).is_ok();
    in_progress.remove(name);
    memo.insert(name.to_string(), r);
    r
}

fn callee_name(callee: &Expr) -> Option<&str> {
    match callee {
        Expr::Var(name, _) => Some(name),
        _ => None,
    }
}
