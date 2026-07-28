use std::fmt;
use crate::compiler::parser::{Expr, Program};

impl fmt::Display for Program {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for expr in &self.body {
            writeln!(f, "{}", expr)?;
        }
        Ok(())
    }
}

impl fmt::Display for Expr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.fmt_with_indent(f, 0)
    }
}

impl Expr {
    fn fmt_with_indent(&self, f: &mut fmt::Formatter<'_>, indent: usize) -> fmt::Result {
        let indent_str = "  ".repeat(indent);
        match self {
            Expr::Int(n, _) => write!(f, "{}Int({})", indent_str, n),
            Expr::Float(n, _) => write!(f, "{}Float({})", indent_str, n),
            Expr::Bool(b, _) => write!(f, "{}Bool({})", indent_str, b),
            Expr::String(s, _) => write!(f, "{}String(\"{}\")", indent_str, s),
            Expr::Nil(_) => write!(f, "{}Nil", indent_str),
            Expr::Add(l, r, _) => {
                write!(f, "{}Add(", indent_str)?;
                write!(f, "\n")?;
                l.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n")?;
                r.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n{})", indent_str)
            }
            Expr::Sub(l, r, _) => {
                write!(f, "{}Sub(", indent_str)?;
                write!(f, "\n")?;
                l.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n")?;
                r.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n{})", indent_str)
            }
            Expr::Mul(l, r, _) => {
                write!(f, "{}Mul(", indent_str)?;
                write!(f, "\n")?;
                l.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n")?;
                r.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n{})", indent_str)
            }
            Expr::Div(l, r, _) => {
                write!(f, "{}Div(", indent_str)?;
                write!(f, "\n")?;
                l.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n")?;
                r.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n{})", indent_str)
            }
            Expr::Mod(l, r, _) => {
                write!(f, "{}Mod(", indent_str)?;
                write!(f, "\n")?;
                l.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n")?;
                r.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n{})", indent_str)
            }
            Expr::FAdd(l, r, _) => {
                write!(f, "{}FAdd(", indent_str)?;
                write!(f, "\n")?;
                l.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n")?;
                r.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n{})", indent_str)
            }
            Expr::FSub(l, r, _) => {
                write!(f, "{}FSub(", indent_str)?;
                write!(f, "\n")?;
                l.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n")?;
                r.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n{})", indent_str)
            }
            Expr::FMul(l, r, _) => {
                write!(f, "{}FMul(", indent_str)?;
                write!(f, "\n")?;
                l.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n")?;
                r.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n{})", indent_str)
            }
            Expr::FDiv(l, r, _) => {
                write!(f, "{}FDiv(", indent_str)?;
                write!(f, "\n")?;
                l.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n")?;
                r.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n{})", indent_str)
            }
            Expr::Eq(l, r, _) => {
                write!(f, "{}Eq(", indent_str)?;
                write!(f, "\n")?;
                l.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n")?;
                r.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n{})", indent_str)
            }
            Expr::Ne(l, r, _) => {
                write!(f, "{}Ne(", indent_str)?;
                write!(f, "\n")?;
                l.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n")?;
                r.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n{})", indent_str)
            }
            Expr::Lt(l, r, _) => {
                write!(f, "{}Lt(", indent_str)?;
                write!(f, "\n")?;
                l.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n")?;
                r.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n{})", indent_str)
            }
            Expr::Le(l, r, _) => {
                write!(f, "{}Le(", indent_str)?;
                write!(f, "\n")?;
                l.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n")?;
                r.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n{})", indent_str)
            }
            Expr::Gt(l, r, _) => {
                write!(f, "{}Gt(", indent_str)?;
                write!(f, "\n")?;
                l.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n")?;
                r.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n{})", indent_str)
            }
            Expr::Ge(l, r, _) => {
                write!(f, "{}Ge(", indent_str)?;
                write!(f, "\n")?;
                l.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n")?;
                r.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n{})", indent_str)
            }
            Expr::FEq(l, r, _) => {
                write!(f, "{}FEq(", indent_str)?;
                write!(f, "\n")?;
                l.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n")?;
                r.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n{})", indent_str)
            }
            Expr::FNe(l, r, _) => {
                write!(f, "{}FNe(", indent_str)?;
                write!(f, "\n")?;
                l.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n")?;
                r.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n{})", indent_str)
            }
            Expr::FLt(l, r, _) => {
                write!(f, "{}FLt(", indent_str)?;
                write!(f, "\n")?;
                l.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n")?;
                r.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n{})", indent_str)
            }
            Expr::FLe(l, r, _) => {
                write!(f, "{}FLe(", indent_str)?;
                write!(f, "\n")?;
                l.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n")?;
                r.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n{})", indent_str)
            }
            Expr::FGt(l, r, _) => {
                write!(f, "{}FGt(", indent_str)?;
                write!(f, "\n")?;
                l.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n")?;
                r.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n{})", indent_str)
            }
            Expr::FGe(l, r, _) => {
                write!(f, "{}FGe(", indent_str)?;
                write!(f, "\n")?;
                l.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n")?;
                r.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n{})", indent_str)
            }
            Expr::Neg(e, _) => {
                write!(f, "{}Neg(", indent_str)?;
                write!(f, "\n")?;
                e.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n{})", indent_str)
            }
            Expr::FNeg(e, _) => {
                write!(f, "{}FNeg(", indent_str)?;
                write!(f, "\n")?;
                e.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n{})", indent_str)
            }
            Expr::Not(e, _) => {
                write!(f, "{}Not(", indent_str)?;
                write!(f, "\n")?;
                e.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n{})", indent_str)
            }
            Expr::Inc(name, _) => write!(f, "{}Inc(\"{}\")", indent_str, name),
            Expr::Dec(name, _) => write!(f, "{}Dec(\"{}\")", indent_str, name),
            Expr::Xor(l, r, _) => {
                write!(f, "{}Xor(", indent_str)?;
                write!(f, "\n")?;
                l.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n")?;
                r.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n{})", indent_str)
            }
            Expr::LAnd(l, r, _) => {
                write!(f, "{}LAnd(", indent_str)?;
                write!(f, "\n")?;
                l.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n")?;
                r.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n{})", indent_str)
            }
            Expr::LOr(l, r, _) => {
                write!(f, "{}LOr(", indent_str)?;
                write!(f, "\n")?;
                l.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n")?;
                r.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n{})", indent_str)
            }
            Expr::AddAssign(name, val, _) => {
                write!(f, "{}AddAssign(\"{}\"", indent_str, name)?;
                write!(f, "\n")?;
                val.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n{})", indent_str)
            }
            Expr::SubAssign(name, val, _) => {
                write!(f, "{}SubAssign(\"{}\"", indent_str, name)?;
                write!(f, "\n")?;
                val.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n{})", indent_str)
            }
            Expr::StrCat(l, r, _) => {
                write!(f, "{}StrCat(", indent_str)?;
                write!(f, "\n")?;
                l.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n")?;
                r.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n{})", indent_str)
            }
            Expr::Var(name, _) => write!(f, "{}Var(\"{}\")", indent_str, name),
            Expr::VarDecl(name, ty, val, _) => {
                write!(f, "{}VarDecl(\"{}\": {} =", indent_str, name, ty)?;
                write!(f, "\n")?;
                val.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n{})", indent_str)
            }
            Expr::VarAssign(name, val, _) => {
                write!(f, "{}VarAssign(\"{}\" =", indent_str, name)?;
                write!(f, "\n")?;
                val.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n{})", indent_str)
            }
            Expr::FuncDecl(name, params, ret_type, body, _) => {
                let param_str: Vec<String> = params
                    .iter()
                    .map(|(n, t)| format!("{}: {}", n, t))
                    .collect();
                write!(
                    f,
                    "{}FuncDecl(\"{}\" ({}) -> {}",
                    indent_str,
                    name,
                    param_str.join(", "),
                    ret_type
                )?;
                write!(f, "\n")?;
                body.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n{})", indent_str)
            }
            Expr::Extern(name, params, ret_type, _) => {
                let param_str: Vec<String> = params
                    .iter()
                    .map(|(n, t)| format!("{}: {}", n, t))
                    .collect();
                write!(
                    f,
                    "{}Extern(\"{}\" ({}) -> {})",
                    indent_str,
                    name,
                    param_str.join(", "),
                    ret_type
                )
            }
            Expr::Call(func, args, _) => {
                write!(f, "{}Call(", indent_str)?;
                write!(f, "\n")?;
                func.fmt_with_indent(f, indent + 1)?;
                for arg in args {
                    write!(f, ",")?;
                    write!(f, "\n")?;
                    arg.fmt_with_indent(f, indent + 1)?;
                }
                write!(f, "\n{})", indent_str)
            }
            Expr::Return(e, _) => {
                write!(f, "{}Return(", indent_str)?;
                write!(f, "\n")?;
                e.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n{})", indent_str)
            }
            Expr::If(cond, then_branch, else_branch, _) => {
                write!(f, "{}If(", indent_str)?;
                cond.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n{}Then:", indent_str)?;
                then_branch.fmt_with_indent(f, indent + 1)?;
                if let Some(else_branch) = else_branch {
                    write!(f, "\n{}Else:", indent_str)?;
                    else_branch.fmt_with_indent(f, indent + 1)?;
                }
                write!(f, "\n{})", indent_str)
            }
            Expr::While(cond, body, _) => {
                write!(f, "{}While(", indent_str)?;
                cond.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n{}Body:", indent_str)?;
                body.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n{})", indent_str)
            }
            Expr::Break(_) => write!(f, "{}Break", indent_str),
            Expr::Continue(_) => write!(f, "{}Continue", indent_str),
            Expr::Block(exprs, _) => {
                write!(f, "{}Block(", indent_str)?;
                for (i, expr) in exprs.iter().enumerate() {
                    write!(f, "\n")?;
                    expr.fmt_with_indent(f, indent + 1)?;
                    if i < exprs.len() - 1 {
                        write!(f, ",")?;
                    }
                }
                write!(f, "\n{})", indent_str)
            }
            Expr::Index(arr, idx, _) => {
                write!(f, "{}Index(", indent_str)?;
                write!(f, "\n")?;
                arr.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n")?;
                idx.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n{})", indent_str)
            }
            Expr::IndexAssign(arr, val, _) => {
                write!(f, "{}IndexAssign(", indent_str)?;
                write!(f, "\n")?;
                arr.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n")?;
                val.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n{})", indent_str)
            }
            Expr::ArrayLiteral(elements, _) => {
                write!(f, "{}ArrayLiteral[", indent_str)?;
                for (i, elem) in elements.iter().enumerate() {
                    write!(f, "\n")?;
                    elem.fmt_with_indent(f, indent + 1)?;
                    if i < elements.len() - 1 {
                        write!(f, ",")?;
                    }
                }
                write!(f, "\n{}]", indent_str)
            }
            Expr::ArrayFill(ty, len, _) => {
                write!(f, "{}ArrayFill[{}; ", indent_str, ty)?;
                len.fmt_with_indent(f, 0)?;
                write!(f, "]")
            }
            Expr::Range(start, end, _) => {
                write!(f, "{}Range(", indent_str)?;
                start.fmt_with_indent(f, 0)?;
                write!(f, "..")?;
                end.fmt_with_indent(f, 0)?;
                write!(f, ")")
            }
            Expr::For(var, array, body, _) => {
                write!(f, "{}For(\"{}\" in ", indent_str, var)?;
                array.fmt_with_indent(f, 0)?;
                write!(f, "\n{}Body:", indent_str)?;
                body.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n{})", indent_str)
            }
            Expr::TypeDef(_) => write!(f, "{}TypeDef", indent_str),
            Expr::Struct(name, fields, _) => {
                let field_str: Vec<String> = fields
                    .iter()
                    .map(|(n, t)| format!("{}: {}", n, t))
                    .collect();
                write!(
                    f,
                    "{}Struct(\"{}\" {{ {} }})",
                    indent_str,
                    name,
                    field_str.join(", ")
                )
            }
            Expr::StructLiteral(name, fields, _) => {
                write!(f, "{}StructLiteral(\"{}\" {{", indent_str, name)?;
                for (i, (fname, fval)) in fields.iter().enumerate() {
                    write!(f, "\n  {}{}: ", indent_str, fname)?;
                    write!(f, "\n")?;
                    fval.fmt_with_indent(f, indent + 2)?;
                    if i < fields.len() - 1 {
                        write!(f, ",")?;
                    }
                }
                write!(f, "\n{}}})", indent_str)
            }
            Expr::MemberAccess(obj, field, _) => {
                write!(f, "{}MemberAccess(", indent_str)?;
                write!(f, "\n")?;
                obj.fmt_with_indent(f, indent + 1)?;
                write!(f, " .{}", field)?;
                write!(f, "\n{})", indent_str)
            }
            Expr::MemberAssign(obj, field, val, _) => {
                write!(f, "{}MemberAssign(", indent_str)?;
                write!(f, "\n")?;
                obj.fmt_with_indent(f, indent + 1)?;
                write!(f, " .{} =", field)?;
                write!(f, "\n")?;
                val.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n{})", indent_str)
            }
            Expr::Lambda(params, body, ret_type, _) => {
                let param_str: Vec<String> = params
                    .iter()
                    .map(|(n, t)| format!("{}: {}", n, t))
                    .collect();
                write!(
                    f,
                    "{}Lambda({}) -> {}",
                    indent_str,
                    param_str.join(", "),
                    ret_type
                )?;
                write!(f, "\n")?;
                body.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n{})", indent_str)
            }
            Expr::AddressOf(expr, _) => {
                write!(f, "{}AddressOf(", indent_str)?;
                write!(f, "\n")?;
                expr.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n{})", indent_str)
            }
            Expr::Deref(expr, _) => {
                write!(f, "{}Deref(", indent_str)?;
                write!(f, "\n")?;
                expr.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n{})", indent_str)
            }
            Expr::DerefAssign(ptr, val, _) => {
                write!(f, "{}DerefAssign(", indent_str)?;
                write!(f, "\n")?;
                ptr.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n")?;
                val.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n{})", indent_str)
            }
        }
    }
}
