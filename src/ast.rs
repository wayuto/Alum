use crate::token::Literal;

pub struct Program {
    pub body: Vec<Expr>,    
}

pub struct Stmt {
    pub body: Vec<Expr>,
}

pub enum Expr {
    Stmt(Stmt),
    Val(Val),
    Var(Var),
    VarDecl(Box<VarDecl>),
    VarMod(Box<VarMod>),
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
    Exit(Exit),
}

pub struct Val {
    pub value: Literal,
}

pub struct Var {
    pub name: String,
}

pub struct VarDecl {
    pub name: String,
    pub value: Expr,
}

pub struct VarMod {
    pub name: String,
    pub value: Expr,
}

pub struct BinOp {
    pub left: Box<Expr>,
    pub right: Box<Expr>,
    pub operator: String,
}

pub struct UnaryOp {
    pub argument: Box<Expr>,
    pub operator: String,
}

pub struct If {
    pub condition: Box<Expr>,
    pub then: Box<Expr>,
    pub else_branch: Option<Box<Expr>>,
}

pub struct While {
    pub condition: Box<Expr>,
    pub body: Box<Expr>,
}

pub struct FuncDecl {
    pub name: String,
    pub params: Vec<String>,
    pub body: Box<Expr>,
}

pub struct FuncCall {
    pub name: String,
    pub args: Vec<Expr>,
}

pub struct Return {
    pub value: Option<Box<Expr>>,
}

pub struct Out {
    pub value: Box<Expr>,
}

pub struct In {
    pub name: String
}

pub struct Label {
    pub name: String,
}

pub struct Goto {
    pub label: String,
}

pub struct Exit {
    pub code: Option<Box<Expr>>,
}