#[derive(Debug, Clone)]
pub struct Program {
    pub body: Vec<Expr>,
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub enum Type {
    Int,
    Bool,
    Void,
}

#[derive(Debug, Clone)]
pub enum Expr {
    Int(isize),
    Bool(bool),
    Add(Box<Expr>, Box<Expr>),
    Sub(Box<Expr>, Box<Expr>),
    Mul(Box<Expr>, Box<Expr>),
    Div(Box<Expr>, Box<Expr>),
    Var(String),
    VarDecl(String, Type, Box<Expr>),
    FuncDecl(String, Vec<(String, Type)>, Type, Box<Expr>),
    FuncCall(String, Vec<Expr>),
    Return(Box<Expr>),
    If(Box<Expr>, Box<Expr>, Option<Box<Expr>>),
    While(Box<Expr>, Box<Expr>),
    Stmt(Vec<Expr>),
}
