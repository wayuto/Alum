use ordered_float::OrderedFloat;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum IRType {
    Int,
    Float,
    String,
    Bool,
    Array(Option<usize>),
    Void,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum IRConst {
    Int(i64),
    Float(OrderedFloat<f64>),
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
    FAdd,
    Sub,
    FSub,
    Mul,
    FMul,
    Div,
    FDiv,
    Eq,
    FEq,
    Ne,
    FNe,
    Gt,
    FGt,
    Ge,
    FGe,
    Lt,
    FLt,
    Le,
    FLe,
    And,
    Or,
    LAnd,
    LOr,
    Xor,
    Range,
    Neg,
    FNeg,
    Inc,
    Dec,
    SizeOf,
    Move,
    FMove,
    Load,
    FLoad,
    Store,
    FStore,
    Call,
    Arg(usize),
    FArg(usize),
    Return(String),
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
