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
    Global(String),
}

impl Operand {
    pub fn key(&self) -> String {
        match self {
            Operand::Var(name) => name.clone(),
            Operand::Temp(id, _) => format!("_tmp_{}", id),
            _ => panic!("unsupported operand key: {:?}", self),
        }
    }
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
    StrEq,
    StrNe,
    StrLt,
    StrLe,
    StrGt,
    StrGe,
    Lt,
    FLt,
    Le,
    FLe,
    LAnd,
    LOr,
    Xor,
    Shl,
    Shr,
    BNot,
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
    GlobLoad,
    FGlobLoad,
    GlobStore,
    FGlobStore,
    Call,
    Arg(usize),
    FArg(usize),
    Return(String),
    Jump,
    JumpIfFalse,
    JumpIfTrue,
    ArrayAccess,
    ArrayAssign,
    ByteAccess,
    ByteAssign,
    Label(String),
    Malloc,
    Free,
    StoreAt,
    LoadAt,
    StrCat,
    StrByte,
    Lea,
    IntToFloat,
    FloatToInt,
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
pub struct IRGlobalVar {
    pub name: String,
    pub value: Option<IRConst>,
    pub is_pub: bool,
}

#[derive(Debug, Clone)]
pub struct IRProgram {
    pub functions: Vec<IRFunction>,
    pub constants: Vec<IRConst>,
    pub extern_vars: Vec<String>,
    pub global_vars: Vec<IRGlobalVar>,
}
