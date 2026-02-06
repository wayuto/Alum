#[derive(Debug, Clone)]
pub struct Program {
    pub body: Vec<Expr>,
}

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum Type {
    Int,
    Float,
    Bool,
    String,
    Array(Box<Type>),
    Void,
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
    Var(String),
    VarDecl(String, Type, Box<Expr>),
    VarAssign(String, Box<Expr>),
    FuncDecl(String, Vec<(String, Type)>, Type, Box<Expr>),
    Extern(String, Vec<(String, Type)>, Type),
    Call(Box<Expr>, Vec<Expr>),
    Return(Box<Expr>),
    If(Box<Expr>, Box<Expr>, Option<Box<Expr>>),
    While(Box<Expr>, Box<Expr>),
    Stmt(Vec<Expr>),
    Index(Box<Expr>, Box<Expr>),
    ArrayLiteral(Vec<Expr>),
    ArrayFill(Type, Box<Expr>),
    For(String, Box<Expr>, Box<Expr>, Box<Expr>),
}
