#[derive(Debug, Clone)]
pub struct Program {
    pub body: Vec<Expr>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Type {
    Named(String),
    Array(Box<Type>),
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
    Eq(Box<Expr>, Box<Expr>),
    Ne(Box<Expr>, Box<Expr>),
    Lt(Box<Expr>, Box<Expr>),
    Le(Box<Expr>, Box<Expr>),
    Gt(Box<Expr>, Box<Expr>),
    Ge(Box<Expr>, Box<Expr>),
    And(Box<Expr>, Box<Expr>),
    Or(Box<Expr>, Box<Expr>),
    Not(Box<Expr>),
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
    Stmt(Vec<Expr>),
    Index(Box<Expr>, Box<Expr>),
    IndexAssign(Box<Expr>, Box<Expr>),
    ArrayLiteral(Vec<Expr>),
    ArrayFill(Type, Box<Expr>),
    For(String, Box<Expr>, Box<Expr>, Box<Expr>),
    TypeDef,
}
