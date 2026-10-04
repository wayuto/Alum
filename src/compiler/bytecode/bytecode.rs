use std::hash::{Hash, Hasher};

#[derive(Debug, Clone)]
pub enum Value {
    Int(i64),
    Float(f64),
    Bool(bool),
    Str(String),
    Array(Vec<Value>),
    Fn(u32, u32),
    Void,
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Float(a), Value::Float(b)) => a.to_bits() == b.to_bits(),
            (Value::Int(a), Value::Int(b)) => a == b,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Str(a), Value::Str(b)) => a == b,
            (Value::Array(a), Value::Array(b)) => a == b,
            (Value::Fn(a1, a2), Value::Fn(b1, b2)) => a1 == b1 && a2 == b2,
            (Value::Void, Value::Void) => true,
            _ => false,
        }
    }
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
            Value::Array(a) => {
                5u8.hash(state);
                a.len().hash(state);
                for e in a {
                    e.hash(state);
                }
            }
            Value::Fn(addr, arity) => {
                6u8.hash(state);
                addr.hash(state);
                arity.hash(state);
            }
            Value::Void => 4u8.hash(state),
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
#[repr(u8)]
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
    INC,
    DEC,
    LOGNOT,
    LOGAND,
    LOGOR,
    LOGXOR,
    SHL,
    SHR,
    BNOT,
    EQ,
    NE,
    GT,
    GE,
    LT,
    LE,
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
    HALT,
    TAILCALL,
    NEWARRAY,
    ARRAYFILL,
    ARRAYGET,
    ARRAYSET,
    MAKEFUNC,
    CALLVALUE,
    CALLNATIVE,
    I2F,
    F2I,
    ARRAYLEN,
}

impl TryFrom<u8> for Op {
    type Error = u8;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        const OP_LIST: &[Op] = &[
            Op::LOADCONST,
            Op::LOADVAR,
            Op::STOREVAR,
            Op::ADD,
            Op::SUB,
            Op::MUL,
            Op::DIV,
            Op::MOD,
            Op::NEG,
            Op::INC,
            Op::DEC,
            Op::LOGNOT,
            Op::LOGAND,
            Op::LOGOR,
            Op::LOGXOR,
            Op::SHL,
            Op::SHR,
            Op::BNOT,
            Op::EQ,
            Op::NE,
            Op::GT,
            Op::GE,
            Op::LT,
            Op::LE,
            Op::FADD,
            Op::FSUB,
            Op::FMUL,
            Op::FDIV,
            Op::FNEG,
            Op::FEQ,
            Op::FNE,
            Op::FGT,
            Op::FGE,
            Op::FLT,
            Op::FLE,
            Op::POP,
            Op::JUMP,
            Op::JUMPIFFALSE,
            Op::CALL,
            Op::RET,
            Op::HALT,
            Op::TAILCALL,
            Op::NEWARRAY,
            Op::ARRAYFILL,
            Op::ARRAYGET,
            Op::ARRAYSET,
            Op::MAKEFUNC,
            Op::CALLVALUE,
            Op::CALLNATIVE,
            Op::I2F,
            Op::F2I,
            Op::ARRAYLEN,
        ];
        if let Some(op) = OP_LIST.get(value as usize) {
            Ok(op.clone())
        } else {
            Err(value)
        }
    }
}
