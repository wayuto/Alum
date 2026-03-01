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

#[derive(Debug, Clone)]
pub enum Expr {
    Int(isize),
    Float(f64),
    Bool(bool),
    String(String),
    Nil,
    Add(Box<Expr>, Box<Expr>),
    Sub(Box<Expr>, Box<Expr>),
    Mul(Box<Expr>, Box<Expr>),
    Div(Box<Expr>, Box<Expr>),
    Mod(Box<Expr>, Box<Expr>),
    FAdd(Box<Expr>, Box<Expr>),
    FSub(Box<Expr>, Box<Expr>),
    FMul(Box<Expr>, Box<Expr>),
    FDiv(Box<Expr>, Box<Expr>),
    Eq(Box<Expr>, Box<Expr>),
    Ne(Box<Expr>, Box<Expr>),
    Lt(Box<Expr>, Box<Expr>),
    Le(Box<Expr>, Box<Expr>),
    Gt(Box<Expr>, Box<Expr>),
    Ge(Box<Expr>, Box<Expr>),
    FEq(Box<Expr>, Box<Expr>),
    FNe(Box<Expr>, Box<Expr>),
    FLt(Box<Expr>, Box<Expr>),
    FLe(Box<Expr>, Box<Expr>),
    FGt(Box<Expr>, Box<Expr>),
    FGe(Box<Expr>, Box<Expr>),
    And(Box<Expr>, Box<Expr>),
    Or(Box<Expr>, Box<Expr>),
    Not(Box<Expr>),
    StrCat(Box<Expr>, Box<Expr>),
    Var(String),
    VarDecl(String, Type, Box<Expr>),
    VarAssign(String, Box<Expr>),
    FuncDecl(String, Vec<(String, Type)>, Type, Box<Expr>),
    Extern(String, Vec<(String, Type)>, Type),
    Call(Box<Expr>, Vec<Expr>),
    Return(Box<Expr>),
    If(Box<Expr>, Box<Expr>, Option<Box<Expr>>),
    While(Box<Expr>, Box<Expr>),
    Break,
    Continue,
    Block(Vec<Expr>),
    Index(Box<Expr>, Box<Expr>),
    IndexAssign(Box<Expr>, Box<Expr>),
    ArrayLiteral(Vec<Expr>),
    ArrayFill(Type, Box<Expr>),
    Range(Box<Expr>, Box<Expr>),
    For(String, Box<Expr>, Box<Expr>),
    TypeDef,
    Struct(String, Vec<(String, Type)>),
    StructLiteral(String, Vec<(String, Expr)>),
    MemberAccess(Box<Expr>, String),
    MemberAssign(Box<Expr>, String, Box<Expr>),
    Lambda(Vec<(String, Type)>, Box<Expr>, Type),
    AddressOf(Box<Expr>),
    Deref(Box<Expr>),
    DerefAssign(Box<Expr>, Box<Expr>),
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
            Expr::Int(n) => write!(f, "{}Int({})", indent_str, n),
            Expr::Float(n) => write!(f, "{}Float({})", indent_str, n),
            Expr::Bool(b) => write!(f, "{}Bool({})", indent_str, b),
            Expr::String(s) => write!(f, "{}String(\"{}\")", indent_str, s),
            Expr::Nil => write!(f, "{}Nil", indent_str),
            Expr::Add(l, r) => {
                write!(f, "{}Add(", indent_str)?;
                write!(f, "\n")?;
                l.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n")?;
                r.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n{})", indent_str)
            }
            Expr::Sub(l, r) => {
                write!(f, "{}Sub(", indent_str)?;
                write!(f, "\n")?;
                l.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n")?;
                r.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n{})", indent_str)
            }
            Expr::Mul(l, r) => {
                write!(f, "{}Mul(", indent_str)?;
                write!(f, "\n")?;
                l.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n")?;
                r.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n{})", indent_str)
            }
            Expr::Div(l, r) => {
                write!(f, "{}Div(", indent_str)?;
                write!(f, "\n")?;
                l.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n")?;
                r.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n{})", indent_str)
            }
            Expr::Mod(l, r) => {
                write!(f, "{}Mod(", indent_str)?;
                write!(f, "\n")?;
                l.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n")?;
                r.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n{})", indent_str)
            }
            Expr::FAdd(l, r) => {
                write!(f, "{}FAdd(", indent_str)?;
                write!(f, "\n")?;
                l.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n")?;
                r.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n{})", indent_str)
            }
            Expr::FSub(l, r) => {
                write!(f, "{}FSub(", indent_str)?;
                write!(f, "\n")?;
                l.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n")?;
                r.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n{})", indent_str)
            }
            Expr::FMul(l, r) => {
                write!(f, "{}FMul(", indent_str)?;
                write!(f, "\n")?;
                l.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n")?;
                r.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n{})", indent_str)
            }
            Expr::FDiv(l, r) => {
                write!(f, "{}FDiv(", indent_str)?;
                write!(f, "\n")?;
                l.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n")?;
                r.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n{})", indent_str)
            }
            Expr::Eq(l, r) => {
                write!(f, "{}Eq(", indent_str)?;
                write!(f, "\n")?;
                l.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n")?;
                r.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n{})", indent_str)
            }
            Expr::Ne(l, r) => {
                write!(f, "{}Ne(", indent_str)?;
                write!(f, "\n")?;
                l.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n")?;
                r.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n{})", indent_str)
            }
            Expr::Lt(l, r) => {
                write!(f, "{}Lt(", indent_str)?;
                write!(f, "\n")?;
                l.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n")?;
                r.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n{})", indent_str)
            }
            Expr::Le(l, r) => {
                write!(f, "{}Le(", indent_str)?;
                write!(f, "\n")?;
                l.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n")?;
                r.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n{})", indent_str)
            }
            Expr::Gt(l, r) => {
                write!(f, "{}Gt(", indent_str)?;
                write!(f, "\n")?;
                l.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n")?;
                r.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n{})", indent_str)
            }
            Expr::Ge(l, r) => {
                write!(f, "{}Ge(", indent_str)?;
                write!(f, "\n")?;
                l.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n")?;
                r.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n{})", indent_str)
            }
            Expr::FEq(l, r) => {
                write!(f, "{}FEq(", indent_str)?;
                write!(f, "\n")?;
                l.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n")?;
                r.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n{})", indent_str)
            }
            Expr::FNe(l, r) => {
                write!(f, "{}FNe(", indent_str)?;
                write!(f, "\n")?;
                l.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n")?;
                r.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n{})", indent_str)
            }
            Expr::FLt(l, r) => {
                write!(f, "{}FLt(", indent_str)?;
                write!(f, "\n")?;
                l.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n")?;
                r.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n{})", indent_str)
            }
            Expr::FLe(l, r) => {
                write!(f, "{}FLe(", indent_str)?;
                write!(f, "\n")?;
                l.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n")?;
                r.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n{})", indent_str)
            }
            Expr::FGt(l, r) => {
                write!(f, "{}FGt(", indent_str)?;
                write!(f, "\n")?;
                l.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n")?;
                r.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n{})", indent_str)
            }
            Expr::FGe(l, r) => {
                write!(f, "{}FGe(", indent_str)?;
                write!(f, "\n")?;
                l.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n")?;
                r.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n{})", indent_str)
            }
            Expr::And(l, r) => {
                write!(f, "{}And(", indent_str)?;
                write!(f, "\n")?;
                l.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n")?;
                r.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n{})", indent_str)
            }
            Expr::Or(l, r) => {
                write!(f, "{}Or(", indent_str)?;
                write!(f, "\n")?;
                l.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n")?;
                r.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n{})", indent_str)
            }
            Expr::Not(e) => {
                write!(f, "{}Not(", indent_str)?;
                write!(f, "\n")?;
                e.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n{})", indent_str)
            }
            Expr::StrCat(l, r) => {
                write!(f, "{}StrCat(", indent_str)?;
                write!(f, "\n")?;
                l.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n")?;
                r.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n{})", indent_str)
            }
            Expr::Var(name) => write!(f, "{}Var(\"{}\")", indent_str, name),
            Expr::VarDecl(name, ty, val) => {
                write!(f, "{}VarDecl(\"{}\": {} =", indent_str, name, ty)?;
                write!(f, "\n")?;
                val.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n{})", indent_str)
            }
            Expr::VarAssign(name, val) => {
                write!(f, "{}VarAssign(\"{}\" =", indent_str, name)?;
                write!(f, "\n")?;
                val.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n{})", indent_str)
            }
            Expr::FuncDecl(name, params, ret_type, body) => {
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
            Expr::Extern(name, params, ret_type) => {
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
            Expr::Call(func, args) => {
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
            Expr::Return(e) => {
                write!(f, "{}Return(", indent_str)?;
                write!(f, "\n")?;
                e.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n{})", indent_str)
            }
            Expr::If(cond, then_branch, else_branch) => {
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
            Expr::While(cond, body) => {
                write!(f, "{}While(", indent_str)?;
                cond.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n{}Body:", indent_str)?;
                body.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n{})", indent_str)
            }
            Expr::Break => write!(f, "{}Break", indent_str),
            Expr::Continue => write!(f, "{}Continue", indent_str),
            Expr::Block(exprs) => {
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
            Expr::Index(arr, idx) => {
                write!(f, "{}Index(", indent_str)?;
                write!(f, "\n")?;
                arr.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n")?;
                idx.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n{})", indent_str)
            }
            Expr::IndexAssign(arr, val) => {
                write!(f, "{}IndexAssign(", indent_str)?;
                write!(f, "\n")?;
                arr.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n")?;
                val.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n{})", indent_str)
            }
            Expr::ArrayLiteral(elements) => {
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
            Expr::ArrayFill(ty, len) => {
                write!(f, "{}ArrayFill[{}; ", indent_str, ty)?;
                len.fmt_with_indent(f, 0)?;
                write!(f, "]")
            }
            Expr::Range(start, end) => {
                write!(f, "{}Range(", indent_str)?;
                start.fmt_with_indent(f, 0)?;
                write!(f, "..")?;
                end.fmt_with_indent(f, 0)?;
                write!(f, ")")
            }
            Expr::For(var, array, body) => {
                write!(f, "{}For(\"{}\" in ", indent_str, var)?;
                array.fmt_with_indent(f, 0)?;
                write!(f, "\n{}Body:", indent_str)?;
                body.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n{})", indent_str)
            }
            Expr::TypeDef => write!(f, "{}TypeDef", indent_str),
            Expr::Struct(name, fields) => {
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
            Expr::StructLiteral(name, fields) => {
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
            Expr::MemberAccess(obj, field) => {
                write!(f, "{}MemberAccess(", indent_str)?;
                write!(f, "\n")?;
                obj.fmt_with_indent(f, indent + 1)?;
                write!(f, " .{}", field)?;
                write!(f, "\n{})", indent_str)
            }
            Expr::MemberAssign(obj, field, val) => {
                write!(f, "{}MemberAssign(", indent_str)?;
                write!(f, "\n")?;
                obj.fmt_with_indent(f, indent + 1)?;
                write!(f, " .{} =", field)?;
                write!(f, "\n")?;
                val.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n{})", indent_str)
            }
            Expr::Lambda(params, body, ret_type) => {
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
            Expr::AddressOf(expr) => {
                write!(f, "{}AddressOf(", indent_str)?;
                write!(f, "\n")?;
                expr.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n{})", indent_str)
            }
            Expr::Deref(expr) => {
                write!(f, "{}Deref(", indent_str)?;
                write!(f, "\n")?;
                expr.fmt_with_indent(f, indent + 1)?;
                write!(f, "\n{})", indent_str)
            }
            Expr::DerefAssign(ptr, val) => {
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
