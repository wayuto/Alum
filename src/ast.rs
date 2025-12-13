use crate::token::{Literal, TokenType, VarType};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Program {
    pub body: Vec<Expr>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Stmt {
    pub body: Vec<Expr>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Expr {
    Stmt(Stmt),
    Val(Val),
    Var(Var),
    ArrayAccess(ArrayAccess),
    VarDecl(VarDecl),
    VarMod(VarMod),
    BinOp(BinOp),
    UnaryOp(UnaryOp),
    If(If),
    While(While),
    FuncDecl(FuncDecl),
    FuncCall(FuncCall),
    Return(Return),
    Label(Label),
    Goto(Goto),
    Extern(Extern),
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Val {
    pub value: Literal,
    pub typ: VarType,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Var {
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct VarDecl {
    pub name: String,
    pub value: Box<Expr>,
    pub typ: VarType,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct VarMod {
    pub name: String,
    pub value: Box<Expr>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct BinOp {
    pub left: Box<Expr>,
    pub right: Box<Expr>,
    pub operator: TokenType,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct UnaryOp {
    pub argument: Box<Expr>,
    pub operator: TokenType,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct If {
    pub condition: Box<Expr>,
    pub then: Box<Expr>,
    pub else_branch: Option<Box<Expr>>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct While {
    pub condition: Box<Expr>,
    pub body: Box<Expr>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct FuncDecl {
    pub name: String,
    pub params: Vec<String>,
    pub body: Box<Expr>,
    pub is_pub: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct FuncCall {
    pub name: String,
    pub args: Vec<Expr>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Return {
    pub value: Option<Box<Expr>>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Label {
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Goto {
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ArrayAccess {
    pub array: String,
    pub offset: Box<Expr>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Extern {
    pub func: String,
}
