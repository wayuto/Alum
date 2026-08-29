use crate::compiler::parser::{Expr, Program};
use std::fmt;

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
    fn fmt_bin(
        f: &mut fmt::Formatter<'_>,
        indent: usize,
        tag: &str,
        l: &Expr,
        r: &Expr,
    ) -> fmt::Result {
        let pad = "  ".repeat(indent);
        write!(f, "{pad}{tag}(\n")?;
        l.fmt_with_indent(f, indent + 1)?;
        write!(f, "\n")?;
        r.fmt_with_indent(f, indent + 1)?;
        write!(f, "\n{pad})")
    }
    fn fmt_un(f: &mut fmt::Formatter<'_>, indent: usize, tag: &str, e: &Expr) -> fmt::Result {
        let pad = "  ".repeat(indent);
        write!(f, "{pad}{tag}(\n")?;
        e.fmt_with_indent(f, indent + 1)?;
        write!(f, "\n{pad})")
    }

    fn fmt_with_indent(&self, f: &mut fmt::Formatter<'_>, indent: usize) -> fmt::Result {
        let indent_str = "  ".repeat(indent);
        match self {
            Expr::Int(n, _) => write!(f, "{}Int({})", indent_str, n),
            Expr::Char(c, _) => write!(f, "{}Char('{}')", indent_str, *c as char),
            Expr::Float(n, _) => write!(f, "{}Float({})", indent_str, n),
            Expr::Bool(b, _) => write!(f, "{}Bool({})", indent_str, b),
            Expr::String(s, _) => write!(f, "{}String(\"{}\")", indent_str, s),
            Expr::Nil(_) => write!(f, "{}Nil", indent_str),
            Expr::Add(l, r, _) => Self::fmt_bin(f, indent, "Add", l, r),
            Expr::Sub(l, r, _) => Self::fmt_bin(f, indent, "Sub", l, r),
            Expr::Mul(l, r, _) => Self::fmt_bin(f, indent, "Mul", l, r),
            Expr::Div(l, r, _) => Self::fmt_bin(f, indent, "Div", l, r),
            Expr::Mod(l, r, _) => Self::fmt_bin(f, indent, "Mod", l, r),
            Expr::FAdd(l, r, _) => Self::fmt_bin(f, indent, "FAdd", l, r),
            Expr::FSub(l, r, _) => Self::fmt_bin(f, indent, "FSub", l, r),
            Expr::FMul(l, r, _) => Self::fmt_bin(f, indent, "FMul", l, r),
            Expr::FDiv(l, r, _) => Self::fmt_bin(f, indent, "FDiv", l, r),
            Expr::Eq(l, r, _) => Self::fmt_bin(f, indent, "Eq", l, r),
            Expr::Ne(l, r, _) => Self::fmt_bin(f, indent, "Ne", l, r),
            Expr::Lt(l, r, _) => Self::fmt_bin(f, indent, "Lt", l, r),
            Expr::Le(l, r, _) => Self::fmt_bin(f, indent, "Le", l, r),
            Expr::Gt(l, r, _) => Self::fmt_bin(f, indent, "Gt", l, r),
            Expr::Ge(l, r, _) => Self::fmt_bin(f, indent, "Ge", l, r),
            Expr::FEq(l, r, _) => Self::fmt_bin(f, indent, "FEq", l, r),
            Expr::FNe(l, r, _) => Self::fmt_bin(f, indent, "FNe", l, r),
            Expr::FLt(l, r, _) => Self::fmt_bin(f, indent, "FLt", l, r),
            Expr::FLe(l, r, _) => Self::fmt_bin(f, indent, "FLe", l, r),
            Expr::FGt(l, r, _) => Self::fmt_bin(f, indent, "FGt", l, r),
            Expr::FGe(l, r, _) => Self::fmt_bin(f, indent, "FGe", l, r),
            Expr::Neg(e, _) => Self::fmt_un(f, indent, "Neg", e),
            Expr::FNeg(e, _) => Self::fmt_un(f, indent, "FNeg", e),
            Expr::Not(e, _) => Self::fmt_un(f, indent, "Not", e),
            Expr::Inc(name, _) => write!(f, "{}Inc(\"{}\")", indent_str, name),
            Expr::Dec(name, _) => write!(f, "{}Dec(\"{}\")", indent_str, name),
            Expr::Xor(l, r, _) => Self::fmt_bin(f, indent, "Xor", l, r),
            Expr::BAnd(l, r, _) => Self::fmt_bin(f, indent, "BAnd", l, r),
            Expr::BOr(l, r, _) => Self::fmt_bin(f, indent, "BOr", l, r),
            Expr::Shl(l, r, _) => Self::fmt_bin(f, indent, "Shl", l, r),
            Expr::Shr(l, r, _) => Self::fmt_bin(f, indent, "Shr", l, r),
            Expr::BNot(e, _) => Self::fmt_un(f, indent, "BNot", e),
            Expr::LAnd(l, r, _) => Self::fmt_bin(f, indent, "LAnd", l, r),
            Expr::LOr(l, r, _) => Self::fmt_bin(f, indent, "LOr", l, r),
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
            Expr::MulAssign(name, val, _) => {
                write!(f, "{}MulAssign(\"{}\"", indent_str, name)?;
                write!(f, "\n")?;
                val.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n{})", indent_str)
            }
            Expr::DivAssign(name, val, _) => {
                write!(f, "{}DivAssign(\"{}\"", indent_str, name)?;
                write!(f, "\n")?;
                val.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n{})", indent_str)
            }
            Expr::ModAssign(name, val, _) => {
                write!(f, "{}ModAssign(\"{}\"", indent_str, name)?;
                write!(f, "\n")?;
                val.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n{})", indent_str)
            }
            Expr::AndAssign(name, val, _) => {
                write!(f, "{}AndAssign(\"{}\"", indent_str, name)?;
                write!(f, "\n")?;
                val.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n{})", indent_str)
            }
            Expr::OrAssign(name, val, _) => {
                write!(f, "{}OrAssign(\"{}\"", indent_str, name)?;
                write!(f, "\n")?;
                val.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n{})", indent_str)
            }
            Expr::XorAssign(name, val, _) => {
                write!(f, "{}XorAssign(\"{}\"", indent_str, name)?;
                write!(f, "\n")?;
                val.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n{})", indent_str)
            }
            Expr::ShlAssign(name, val, _) => {
                write!(f, "{}ShlAssign(\"{}\"", indent_str, name)?;
                write!(f, "\n")?;
                val.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n{})", indent_str)
            }
            Expr::ShrAssign(name, val, _) => {
                write!(f, "{}ShrAssign(\"{}\"", indent_str, name)?;
                write!(f, "\n")?;
                val.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n{})", indent_str)
            }
            Expr::StrCat(l, r, _) => Self::fmt_bin(f, indent, "StrCat", l, r),
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
            Expr::ConstDecl(name, ty, val, is_pub, _) => {
                write!(
                    f,
                    "{}ConstDecl{}(\"{}\": {} =",
                    indent_str,
                    if *is_pub { "(pub)" } else { "" },
                    name,
                    ty
                )?;
                write!(f, "\n")?;
                val.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n{})", indent_str)
            }
            Expr::GlobalVar(name, is_pub, ty, val, _) => {
                write!(
                    f,
                    "{}GlobalVar{}(\"{}\": {}",
                    indent_str,
                    if *is_pub { "(pub)" } else { "" },
                    name,
                    ty
                )?;
                if let Some(val) = val {
                    write!(f, " =")?;
                    write!(f, "\n")?;
                    val.fmt_with_indent(f, indent + 1)?;
                    write!(f, "\n{})", indent_str)?;
                } else {
                    write!(f, ")")?;
                }
                Ok(())
            }
            Expr::FuncDecl(name, attrs, type_params, params, ret_type, body, _) => {
                let param_str: Vec<String> = params
                    .iter()
                    .map(|(n, t)| format!("{}: {}", n, t))
                    .collect();
                let tp_str = if type_params.is_empty() {
                    String::new()
                } else {
                    format!("<{}>", type_params.join(", "))
                };
                let attr_str: Vec<String> = std::iter::empty()
                    .chain(attrs.is_pub.then(|| "pub".to_string()))
                    .chain(attrs.is_external.then(|| "extern".to_string()))
                    .chain(attrs.is_pure.then(|| "pure".to_string()))
                    .collect();
                let ann_str = if attr_str.is_empty() {
                    String::new()
                } else {
                    format!("({})", attr_str.join(", "))
                };
                write!(
                    f,
                    "{}FuncDecl{}(\"{}{}\" ({}) -> {}",
                    indent_str,
                    ann_str,
                    name,
                    tp_str,
                    param_str.join(", "),
                    ret_type
                )?;
                write!(f, "\n")?;
                body.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n{})", indent_str)
            }
            Expr::ExternVar(name, ty, _) => {
                write!(f, "{}ExternVar(\"{}\": {})", indent_str, name, ty)
            }
            Expr::Call(func, _, args, _) => {
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
            Expr::Return(e, _) => Self::fmt_un(f, indent, "Return", e),
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
            Expr::Break(value, _) => match value {
                Some(v) => {
                    write!(f, "{}Break ", indent_str)?;
                    v.fmt_with_indent(f, 0)
                }
                None => write!(f, "{}Break", indent_str),
            },
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
            Expr::Match(target, branches, default, _) => {
                write!(f, "{}Match(", indent_str)?;
                target.fmt_with_indent(f, 0)?;
                write!(f, " {{")?;
                for (case, guard, value) in branches {
                    write!(f, "\n{}  Case:", indent_str)?;
                    case.fmt_with_indent(f, 0)?;
                    if let Some(g) = guard {
                        write!(f, " if")?;
                        g.fmt_with_indent(f, 0)?;
                    }
                    write!(f, " =>")?;
                    value.fmt_with_indent(f, indent + 2)?;
                }
                if let Some(d) = default {
                    write!(f, "\n{}  Default =>", indent_str)?;
                    d.fmt_with_indent(f, indent + 2)?;
                }
                write!(f, "\n{}}})", indent_str)
            }
            Expr::Struct(name, type_params, fields, _) => {
                let field_str: Vec<String> = fields
                    .iter()
                    .map(|(n, t)| format!("{}: {}", n, t))
                    .collect();
                let tp_str = if type_params.is_empty() {
                    String::new()
                } else {
                    format!("<{}>", type_params.join(", "))
                };
                write!(
                    f,
                    "{}Struct(\"{}{}\" {{ {} }})",
                    indent_str,
                    name,
                    tp_str,
                    field_str.join(", ")
                )
            }
            Expr::StructLiteral(name, type_args, fields, _) => {
                let ta_str = if type_args.is_empty() {
                    String::new()
                } else {
                    let args: Vec<String> = type_args.iter().map(|t| t.to_string()).collect();
                    format!("<{}>", args.join(", "))
                };
                write!(f, "{}StructLiteral(\"{}{}\" {{", indent_str, name, ta_str)?;
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
            Expr::Union(name, type_params, fields, _) => {
                let field_str: Vec<String> = fields
                    .iter()
                    .map(|(n, t)| format!("{}: {}", n, t))
                    .collect();
                let tp_str = if type_params.is_empty() {
                    String::new()
                } else {
                    format!("<{}>", type_params.join(", "))
                };
                write!(
                    f,
                    "{}Union(\"{}{}\" {{ {} }})",
                    indent_str,
                    name,
                    tp_str,
                    field_str.join(", ")
                )
            }
            Expr::UnionLiteral(name, type_args, fields, _) => {
                let ta_str = if type_args.is_empty() {
                    String::new()
                } else {
                    let args: Vec<String> = type_args.iter().map(|t| t.to_string()).collect();
                    format!("<{}>", args.join(", "))
                };
                write!(f, "{}UnionLiteral(\"{}{}\" {{", indent_str, name, ta_str)?;
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
            Expr::Enum(name, members, _) => {
                let member_str: Vec<String> = members
                    .iter()
                    .map(|(n, v)| format!("{} = {}", n, v))
                    .collect();
                write!(
                    f,
                    "{}Enum(\"{}\" {{ {} }})",
                    indent_str,
                    name,
                    member_str.join(", ")
                )
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
            Expr::Cast(expr, ty, _) => {
                write!(f, "{}Cast({} ->", indent_str, ty)?;
                write!(f, "\n")?;
                expr.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n{})", indent_str)
            }
            Expr::FString(parts, _) => {
                write!(f, "{}FString(", indent_str)?;
                for part in parts {
                    write!(f, "\n")?;
                    part.fmt_with_indent(f, indent + 1)?;
                }
                write!(f, "\n{})", indent_str)
            }
        }
    }
}
