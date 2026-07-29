use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Reg {
    Rax,
    Rbx,
    Rcx,
    Rdx,
    Rsi,
    Rdi,
    R8,
    R9,
    R10,
    R11,
    R12,
    R13,
    R14,
    R15,
    Rsp,
    Rbp,
    Xmm0,
    Xmm1,
    Xmm2,
    Xmm3,
    Xmm4,
    Xmm5,
    Xmm6,
    Xmm7,
    Xmm8,
    Xmm9,
    Xmm10,
    Xmm11,
    Xmm12,
    Xmm13,
    Xmm14,
    Xmm15,
}

impl fmt::Display for Reg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Reg::Rax => write!(f, "rax"),
            Reg::Rbx => write!(f, "rbx"),
            Reg::Rcx => write!(f, "rcx"),
            Reg::Rdx => write!(f, "rdx"),
            Reg::Rsi => write!(f, "rsi"),
            Reg::Rdi => write!(f, "rdi"),
            Reg::R8 => write!(f, "r8"),
            Reg::R9 => write!(f, "r9"),
            Reg::R10 => write!(f, "r10"),
            Reg::R11 => write!(f, "r11"),
            Reg::R12 => write!(f, "r12"),
            Reg::R13 => write!(f, "r13"),
            Reg::R14 => write!(f, "r14"),
            Reg::R15 => write!(f, "r15"),
            Reg::Rsp => write!(f, "rsp"),
            Reg::Rbp => write!(f, "rbp"),
            Reg::Xmm0 => write!(f, "xmm0"),
            Reg::Xmm1 => write!(f, "xmm1"),
            Reg::Xmm2 => write!(f, "xmm2"),
            Reg::Xmm3 => write!(f, "xmm3"),
            Reg::Xmm4 => write!(f, "xmm4"),
            Reg::Xmm5 => write!(f, "xmm5"),
            Reg::Xmm6 => write!(f, "xmm6"),
            Reg::Xmm7 => write!(f, "xmm7"),
            Reg::Xmm8 => write!(f, "xmm8"),
            Reg::Xmm9 => write!(f, "xmm9"),
            Reg::Xmm10 => write!(f, "xmm10"),
            Reg::Xmm11 => write!(f, "xmm11"),
            Reg::Xmm12 => write!(f, "xmm12"),
            Reg::Xmm13 => write!(f, "xmm13"),
            Reg::Xmm14 => write!(f, "xmm14"),
            Reg::Xmm15 => write!(f, "xmm15"),
        }
    }
}

impl Reg {
    pub fn is_xmm(self) -> bool {
        matches!(
            self,
            Reg::Xmm0
                | Reg::Xmm1
                | Reg::Xmm2
                | Reg::Xmm3
                | Reg::Xmm4
                | Reg::Xmm5
                | Reg::Xmm6
                | Reg::Xmm7
                | Reg::Xmm8
                | Reg::Xmm9
                | Reg::Xmm10
                | Reg::Xmm11
                | Reg::Xmm12
                | Reg::Xmm13
                | Reg::Xmm14
                | Reg::Xmm15
        )
    }

    pub fn reg_id(self) -> u8 {
        match self {
            Reg::Rax => 0,
            Reg::Rcx => 1,
            Reg::Rdx => 2,
            Reg::Rbx => 3,
            Reg::Rsp => 4,
            Reg::Rbp => 5,
            Reg::Rsi => 6,
            Reg::Rdi => 7,
            Reg::R8 => 8,
            Reg::R9 => 9,
            Reg::R10 => 10,
            Reg::R11 => 11,
            Reg::R12 => 12,
            Reg::R13 => 13,
            Reg::R14 => 14,
            Reg::R15 => 15,
            Reg::Xmm0 => 0,
            Reg::Xmm1 => 1,
            Reg::Xmm2 => 2,
            Reg::Xmm3 => 3,
            Reg::Xmm4 => 4,
            Reg::Xmm5 => 5,
            Reg::Xmm6 => 6,
            Reg::Xmm7 => 7,
            Reg::Xmm8 => 8,
            Reg::Xmm9 => 9,
            Reg::Xmm10 => 10,
            Reg::Xmm11 => 11,
            Reg::Xmm12 => 12,
            Reg::Xmm13 => 13,
            Reg::Xmm14 => 14,
            Reg::Xmm15 => 15,
        }
    }

    pub fn rex_b(self) -> bool {
        matches!(self, Reg::R8 | Reg::R9 | Reg::R10 | Reg::R11 | Reg::R12 | Reg::R13 | Reg::R14 | Reg::R15)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Size {
    QWord,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Mem {
    pub base: Option<Reg>,
    pub index: Option<Reg>,
    pub scale: u8,
    pub disp: i32,
    pub size: Option<Size>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Operand {
    Reg(Reg),
    Mem(Mem),
    Imm(i64),
    Label(String),
    PLT(String),
    DataLabel(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Section {
    Text,
    Data,
}

#[derive(Debug, Clone)]
pub enum Asm {
    Section(Section),
    Align(u32),
    Global(String),
    Extern(String),
    Label(String),

    Dq(Vec<u64>),
    Db(Vec<u8>),

    Mov(Operand, Operand),
    Movzx(Reg, Reg),
    Lea(Operand, Operand),
    Add(Operand, Operand),
    Sub(Operand, Operand),
    Imul(Operand, Operand),
    Idiv(Reg),
    Cqo,
    Neg(Reg),
    Inc(Reg),
    Dec(Reg),
    Xor(Operand, Operand),
    Or(Operand, Operand),
    And(Operand, Operand),
    Cmp(Operand, Operand),
    Push(Reg),
    Pop(Reg),
    Call(Operand),
    Ret,
    Jmp(String),
    Je(String),
    Jge(String),

    Sete(Reg),
    Setne(Reg),
    Setg(Reg),
    Setge(Reg),
    Setl(Reg),
    Setle(Reg),
    Seta(Reg),
    Setae(Reg),
    Setb(Reg),
    Setbe(Reg),

    Movsd(Operand, Operand),
    Addsd(Operand, Operand),
    Subsd(Operand, Operand),
    Mulsd(Operand, Operand),
    Divsd(Operand, Operand),
    Ucomisd(Reg, Reg),
    Xorpd(Reg, Operand),
}
