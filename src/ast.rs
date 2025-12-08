use crate::token::{Literal, TokenType};

#[derive(Debug, Clone)]
pub struct Program {
    pub body: Vec<Expr>,
}

#[derive(Debug, Clone)]
pub struct Stmt {
    pub body: Vec<Expr>,
}

#[derive(Debug, Clone)]
pub enum Expr {
    Stmt(Stmt),
    Val(Val),
    Var(Var),
    VarDecl(VarDecl),
    VarMod(VarMod),
    BinOp(BinOp),
    UnaryOp(UnaryOp),
    If(If),
    While(While),
    FuncDecl(FuncDecl),
    FuncCall(FuncCall),
    Return(Return),
    Out(Out),
    In(In),
    Label(Label),
    Goto(Goto),
    Extern(Extern),
    Exit(Exit),
}

#[derive(Debug, Clone)]
pub struct Val {
    pub value: Literal,
}

#[derive(Debug, Clone)]
pub struct Var {
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct VarDecl {
    pub name: String,
    pub value: Box<Expr>,
}

#[derive(Debug, Clone)]
pub struct VarMod {
    pub name: String,
    pub value: Box<Expr>,
}

#[derive(Debug, Clone)]
pub struct BinOp {
    pub left: Box<Expr>,
    pub right: Box<Expr>,
    pub operator: TokenType,
}

#[derive(Debug, Clone)]
pub struct UnaryOp {
    pub argument: Box<Expr>,
    pub operator: TokenType,
}

#[derive(Debug, Clone)]
pub struct If {
    pub condition: Box<Expr>,
    pub then: Box<Expr>,
    pub else_branch: Option<Box<Expr>>,
}

#[derive(Debug, Clone)]
pub struct While {
    pub condition: Box<Expr>,
    pub body: Box<Expr>,
}

#[derive(Debug, Clone)]
pub struct FuncDecl {
    pub name: String,
    pub params: Vec<String>,
    pub body: Box<Expr>,
}

#[derive(Debug, Clone)]
pub struct FuncCall {
    pub name: String,
    pub args: Vec<Expr>,
}

#[derive(Debug, Clone)]
pub struct Return {
    pub value: Option<Box<Expr>>,
}

#[derive(Debug, Clone)]
pub struct Out {
    pub value: Box<Expr>,
}

#[derive(Debug, Clone)]
pub struct In {
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct Label {
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct Goto {
    pub label: String,
}

#[derive(Debug, Clone)]
pub struct Exit {
    pub code: Box<Expr>,
}

#[derive(Debug, Clone)]
pub struct Extern {
    pub func: String,
}
