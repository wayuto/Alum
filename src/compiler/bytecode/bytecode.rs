use std::hash::{Hash, Hasher};

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Int(i64),
    Float(f64),
    Bool(bool),
    Str(String),
    Void,
}

impl Eq for Value {}

impl Hash for Value {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            Value::Int(i) => {
                0u8.hash(state);
                i.hash(state);
            }
            Value::Float(f) => {
                1u8.hash(state);
                f.to_bits().hash(state);
            }
            Value::Bool(b) => {
                2u8.hash(state);
                b.hash(state);
            }
            Value::Str(s) => {
                3u8.hash(state);
                s.hash(state);
            }
            Value::Void => 4u8.hash(state),
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum Op {
    LOADCONST,
    LOADVAR,
    STOREVAR,
    ADD,
    SUB,
    MUL,
    DIV,
    MOD,
    NEG,
    POS,
    INC,
    DEC,
    LOGNOT,
    LOGAND,
    LOGOR,
    LOGXOR,
    EQ,
    NE,
    GT,
    GE,
    LT,
    LE,
    AND,
    OR,
    FADD,
    FSUB,
    FMUL,
    FDIV,
    FNEG,
    FEQ,
    FNE,
    FGT,
    FGE,
    FLT,
    FLE,
    POP,
    JUMP,
    JUMPIFFALSE,
    CALL,
    RET,
    EXIT,
    HALT,
}
impl Op {
    pub fn from_u8(op: u8) -> Option<Self> {
        match op {
            0 => Some(Op::LOADCONST),
            1 => Some(Op::LOADVAR),
            2 => Some(Op::STOREVAR),
            3 => Some(Op::ADD),
            4 => Some(Op::SUB),
            5 => Some(Op::MUL),
            6 => Some(Op::DIV),
            7 => Some(Op::MOD),
            8 => Some(Op::NEG),
            9 => Some(Op::POS),
            10 => Some(Op::INC),
            11 => Some(Op::DEC),
            12 => Some(Op::LOGNOT),
            13 => Some(Op::LOGAND),
            14 => Some(Op::LOGOR),
            15 => Some(Op::LOGXOR),
            16 => Some(Op::EQ),
            17 => Some(Op::NE),
            18 => Some(Op::GT),
            19 => Some(Op::GE),
            20 => Some(Op::LT),
            21 => Some(Op::LE),
            22 => Some(Op::AND),
            23 => Some(Op::OR),
            24 => Some(Op::FADD),
            25 => Some(Op::FSUB),
            26 => Some(Op::FMUL),
            27 => Some(Op::FDIV),
            28 => Some(Op::FNEG),
            29 => Some(Op::FEQ),
            30 => Some(Op::FNE),
            31 => Some(Op::FGT),
            32 => Some(Op::FGE),
            33 => Some(Op::FLT),
            34 => Some(Op::FLE),
            35 => Some(Op::POP),
            36 => Some(Op::JUMP),
            37 => Some(Op::JUMPIFFALSE),
            38 => Some(Op::CALL),
            39 => Some(Op::RET),
            40 => Some(Op::EXIT),
            41 => Some(Op::HALT),
            _ => None,
        }
    }

    #[allow(dead_code)]
    pub fn to_str(op: Op) -> String {
        match op {
            Op::LOADCONST => "LOAD_CONST".to_string(),
            Op::LOADVAR => "LOAD_VAR".to_string(),
            Op::STOREVAR => "STORE_VAR".to_string(),
            Op::ADD => "ADD".to_string(),
            Op::SUB => "SUB".to_string(),
            Op::MUL => "MUL".to_string(),
            Op::DIV => "DIV".to_string(),
            Op::MOD => "MOD".to_string(),
            Op::NEG => "NEG".to_string(),
            Op::POS => "POS".to_string(),
            Op::INC => "INC".to_string(),
            Op::DEC => "DEC".to_string(),
            Op::LOGNOT => "LOG_NOT".to_string(),
            Op::LOGAND => "LOG_AND".to_string(),
            Op::LOGOR => "LOG_OR".to_string(),
            Op::LOGXOR => "LOG_XOR".to_string(),
            Op::EQ => "EQ".to_string(),
            Op::NE => "NE".to_string(),
            Op::GT => "GT".to_string(),
            Op::GE => "GE".to_string(),
            Op::LT => "LT".to_string(),
            Op::LE => "LE".to_string(),
            Op::AND => "AND".to_string(),
            Op::OR => "OR".to_string(),
            Op::FADD => "F_ADD".to_string(),
            Op::FSUB => "F_SUB".to_string(),
            Op::FMUL => "F_MUL".to_string(),
            Op::FDIV => "F_DIV".to_string(),
            Op::FNEG => "F_NEG".to_string(),
            Op::FEQ => "F_EQ".to_string(),
            Op::FNE => "F_NE".to_string(),
            Op::FGT => "F_GT".to_string(),
            Op::FGE => "F_GE".to_string(),
            Op::FLT => "F_LT".to_string(),
            Op::FLE => "F_LE".to_string(),
            Op::POP => "POP".to_string(),
            Op::JUMP => "JUMP".to_string(),
            Op::JUMPIFFALSE => "JUMP_IF_FALSE".to_string(),
            Op::CALL => "CALL".to_string(),
            Op::RET => "RET".to_string(),
            Op::EXIT => "EXIT".to_string(),
            Op::HALT => "HALT".to_string(),
        }
    }

    #[allow(dead_code)]
    pub fn operand_count(&self) -> usize {
        match self {
            Op::LOADCONST => 1,
            Op::LOADVAR => 1,
            Op::STOREVAR => 1,
            Op::ADD
            | Op::SUB
            | Op::MUL
            | Op::DIV
            | Op::MOD
            | Op::NEG
            | Op::POS
            | Op::INC
            | Op::DEC
            | Op::LOGNOT
            | Op::LOGAND
            | Op::LOGOR
            | Op::LOGXOR
            | Op::EQ
            | Op::NE
            | Op::GT
            | Op::GE
            | Op::LT
            | Op::LE
            | Op::AND
            | Op::OR
            | Op::FADD
            | Op::FSUB
            | Op::FMUL
            | Op::FDIV
            | Op::FNEG
            | Op::FEQ
            | Op::FNE
            | Op::FGT
            | Op::FGE
            | Op::FLT
            | Op::FLE => 0,
            Op::POP => 0,
            Op::JUMP => 2,
            Op::JUMPIFFALSE => 2,
            Op::CALL => 3,
            Op::RET => 0,
            Op::EXIT => 0,
            Op::HALT => 0,
        }
    }
}
