use crate::compiler::Span;
use std::fmt;

#[derive(Debug, Clone)]
pub struct Program {
    pub body: Vec<Expr>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Type {
    Named(String),
    Array(Box<Type>, usize),
    Pointer(Box<Type>),
    Function(Vec<Box<Type>>, Box<Type>),
    TypeVar(usize),
    Auto,
    #[allow(dead_code)]
    Gen,
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Type::Named(name) => write!(f, "{}", name),
            Type::Array(inner, len) => {
                if *len == 0 {
                    write!(f, "{}[]", inner)
                } else {
                    write!(f, "{}[{}]", inner, len)
                }
            }
            Type::Pointer(inner) => write!(f, "*{}", inner),
            Type::Function(params, ret) => {
                let param_str: Vec<String> = params.iter().map(|p| p.to_string()).collect();
                write!(f, "{}({})", ret, param_str.join(", "))
            }
            Type::TypeVar(id) => write!(f, "T{}", id),
            Type::Auto => write!(f, "auto"),
            Type::Gen => write!(f, "gen"),
        }
    }
}

impl Expr {
    pub fn span(&self) -> Span {
        match self {
            Expr::Int(_, s)
            | Expr::Float(_, s)
            | Expr::Bool(_, s)
            | Expr::String(_, s)
            | Expr::Nil(s)
            | Expr::Add(_, _, s)
            | Expr::Sub(_, _, s)
            | Expr::Mul(_, _, s)
            | Expr::Div(_, _, s)
            | Expr::Mod(_, _, s)
            | Expr::FAdd(_, _, s)
            | Expr::FSub(_, _, s)
            | Expr::FMul(_, _, s)
            | Expr::FDiv(_, _, s)
            | Expr::Eq(_, _, s)
            | Expr::Ne(_, _, s)
            | Expr::Lt(_, _, s)
            | Expr::Le(_, _, s)
            | Expr::Gt(_, _, s)
            | Expr::Ge(_, _, s)
            | Expr::FEq(_, _, s)
            | Expr::FNe(_, _, s)
            | Expr::FLt(_, _, s)
            | Expr::FLe(_, _, s)
            | Expr::FGt(_, _, s)
            | Expr::FGe(_, _, s)
            | Expr::And(_, _, s)
            | Expr::Or(_, _, s)
            | Expr::Not(_, s)
            | Expr::StrCat(_, _, s)
            | Expr::Var(_, s)
            | Expr::VarDecl(_, _, _, s)
            | Expr::VarAssign(_, _, s)
            | Expr::FuncDecl(_, _, _, _, s)
            | Expr::Extern(_, _, _, s)
            | Expr::Call(_, _, s)
            | Expr::Return(_, s)
            | Expr::If(_, _, _, s)
            | Expr::While(_, _, s)
            | Expr::Break(s)
            | Expr::Continue(s)
            | Expr::Block(_, s)
            | Expr::Index(_, _, s)
            | Expr::IndexAssign(_, _, s)
            | Expr::ArrayLiteral(_, s)
            | Expr::ArrayFill(_, _, s)
            | Expr::Range(_, _, s)
            | Expr::For(_, _, _, s)
            | Expr::TypeDef(s)
            | Expr::Struct(_, _, s)
            | Expr::StructLiteral(_, _, s)
            | Expr::MemberAccess(_, _, s)
            | Expr::MemberAssign(_, _, _, s)
            | Expr::Lambda(_, _, _, s)
            | Expr::AddressOf(_, s)
            | Expr::Deref(_, s)
            | Expr::DerefAssign(_, _, s) => *s,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Expr {
    Int(isize, Span),
    Float(f64, Span),
    Bool(bool, Span),
    String(String, Span),
    Nil(Span),
    Add(Box<Expr>, Box<Expr>, Span),
    Sub(Box<Expr>, Box<Expr>, Span),
    Mul(Box<Expr>, Box<Expr>, Span),
    Div(Box<Expr>, Box<Expr>, Span),
    Mod(Box<Expr>, Box<Expr>, Span),
    FAdd(Box<Expr>, Box<Expr>, Span),
    FSub(Box<Expr>, Box<Expr>, Span),
    FMul(Box<Expr>, Box<Expr>, Span),
    FDiv(Box<Expr>, Box<Expr>, Span),
    Eq(Box<Expr>, Box<Expr>, Span),
    Ne(Box<Expr>, Box<Expr>, Span),
    Lt(Box<Expr>, Box<Expr>, Span),
    Le(Box<Expr>, Box<Expr>, Span),
    Gt(Box<Expr>, Box<Expr>, Span),
    Ge(Box<Expr>, Box<Expr>, Span),
    FEq(Box<Expr>, Box<Expr>, Span),
    FNe(Box<Expr>, Box<Expr>, Span),
    FLt(Box<Expr>, Box<Expr>, Span),
    FLe(Box<Expr>, Box<Expr>, Span),
    FGt(Box<Expr>, Box<Expr>, Span),
    FGe(Box<Expr>, Box<Expr>, Span),
    And(Box<Expr>, Box<Expr>, Span),
    Or(Box<Expr>, Box<Expr>, Span),
    Not(Box<Expr>, Span),
    StrCat(Box<Expr>, Box<Expr>, Span),
    Var(String, Span),
    VarDecl(String, Type, Box<Expr>, Span),
    VarAssign(String, Box<Expr>, Span),
    FuncDecl(String, Vec<(String, Type)>, Type, Box<Expr>, Span),
    Extern(String, Vec<(String, Type)>, Type, Span),
    Call(Box<Expr>, Vec<Expr>, Span),
    Return(Box<Expr>, Span),
    If(Box<Expr>, Box<Expr>, Option<Box<Expr>>, Span),
    While(Box<Expr>, Box<Expr>, Span),
    Break(Span),
    Continue(Span),
    Block(Vec<Expr>, Span),
    Index(Box<Expr>, Box<Expr>, Span),
    IndexAssign(Box<Expr>, Box<Expr>, Span),
    ArrayLiteral(Vec<Expr>, Span),
    ArrayFill(Type, Box<Expr>, Span),
    Range(Box<Expr>, Box<Expr>, Span),
    For(String, Box<Expr>, Box<Expr>, Span),
    TypeDef(Span),
    Struct(String, Vec<(String, Type)>, Span),
    StructLiteral(String, Vec<(String, Expr)>, Span),
    MemberAccess(Box<Expr>, String, Span),
    MemberAssign(Box<Expr>, String, Box<Expr>, Span),
    Lambda(Vec<(String, Type)>, Box<Expr>, Type, Span),
    AddressOf(Box<Expr>, Span),
    Deref(Box<Expr>, Span),
    DerefAssign(Box<Expr>, Box<Expr>, Span),
}

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
            Expr::And(l, r, _) => {
                write!(f, "{}And(", indent_str)?;
                write!(f, "\n")?;
                l.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n")?;
                r.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n{})", indent_str)
            }
            Expr::Or(l, r, _) => {
                write!(f, "{}Or(", indent_str)?;
                write!(f, "\n")?;
                l.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n")?;
                r.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n{})", indent_str)
            }
            Expr::Not(e, _) => {
                write!(f, "{}Not(", indent_str)?;
                write!(f, "\n")?;
                e.fmt_with_indent(f, indent + 1)?;
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
