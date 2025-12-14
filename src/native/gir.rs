#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum IRType {
    Number,
    String,
    Bool,
    Void,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum IRConst {
    I64(i64),
    Bool(bool),
    Str(String),
    Void,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Operand {
    Temp(usize, IRType),
    Var(String),
    Const(IRConst),
    Label(String),
    Function(String),
}

#[derive(Debug, Clone)]
pub enum Op {
    Add,
    Sub,
    Mul,
    Div,
    Eq,
    Ne,
    Gt,
    Ge,
    Lt,
    Le,
    Move,
    Load,
    Store,
    Call,
    Arg,
    Return,
    Jump,
    JumpIfFalse,
    Label,
    Nop,
}

#[derive(Debug, Clone)]
pub struct Instruction {
    pub op: Op,
    pub dst: Option<Operand>,
    pub src1: Option<Operand>,
    pub src2: Option<Operand>,
}

#[derive(Debug, Clone)]
pub struct IRFunction {
    pub name: String,
    pub params: Vec<(Operand, IRType)>,
    pub instructions: Vec<Instruction>,
    pub ret_type: IRType,
    pub is_pub: bool,
}

#[derive(Debug, Clone)]
pub struct IRProgram {
    pub functions: Vec<IRFunction>,
    pub constants: Vec<IRConst>,
}
