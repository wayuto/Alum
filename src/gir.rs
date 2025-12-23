#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum IRType {
    Number,
    String,
    Bool,
    Array(Option<usize>),
    Void,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum IRConst {
    Number(i64),
    Bool(bool),
    Str(String),
    Array(usize, Vec<Operand>),
    Void,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Operand {
    Temp(usize, IRType),
    Var(String),
    Const(IRConst),
    ConstIdx(usize),
    Label(String),
    Function(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
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
    And,
    Or,
    LAnd,
    LOr,
    Xor,
    Range,
    Neg,
    Inc,
    Dec,
    SizeOf,
    Move,
    Load,
    Store,
    Call,
    Arg(usize),
    Return,
    Jump,
    JumpIfFalse,
    ArrayAccess,
    ArrayAssign,
    Label(String),
    Extern(String),
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
    pub is_external: bool,
}

#[derive(Debug, Clone)]
pub struct IRProgram {
    pub functions: Vec<IRFunction>,
    pub constants: Vec<IRConst>,
}
