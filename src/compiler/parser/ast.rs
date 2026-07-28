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
            | Expr::Neg(_, s)
            | Expr::FNeg(_, s)
            | Expr::Inc(_, s)
            | Expr::Dec(_, s)
            | Expr::Xor(_, _, s)
            | Expr::LAnd(_, _, s)
            | Expr::LOr(_, _, s)
            | Expr::AddAssign(_, _, s)
            | Expr::SubAssign(_, _, s)
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
    Neg(Box<Expr>, Span),
    FNeg(Box<Expr>, Span),
    Not(Box<Expr>, Span),
    Inc(String, Span),
    Dec(String, Span),
    Xor(Box<Expr>, Box<Expr>, Span),
    LAnd(Box<Expr>, Box<Expr>, Span),
    LOr(Box<Expr>, Box<Expr>, Span),
    AddAssign(String, Box<Expr>, Span),
    SubAssign(String, Box<Expr>, Span),
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
