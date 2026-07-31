use ordered_float::OrderedFloat;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum IRType {
    Int,
    Float,
    String,
    Bool,
    Array,
    Void,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum IRConst {
    Int(i64),
    Float(OrderedFloat<f64>),
    Str(String),
    Array(Vec<Operand>),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Operand {
    Temp(usize, IRType),
    Var(String),
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
    Mod,
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
    LAnd,
    LOr,
    Xor,
    Range,
    Neg,
    FNeg,
    Inc,
    Dec,
    SizeOf,
    Not,
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
    ByteAccess,
    ByteAssign,
    Label(String),
    Malloc,
    StoreAt,
    LoadAt,
    StrCat,
    Lea,
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
