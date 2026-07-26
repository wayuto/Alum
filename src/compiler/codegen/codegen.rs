use super::ir::{IRConst, IRFunction, IRProgram, IRType, Instruction, Op, Operand};
use super::types::CodeGenError;
use ordered_float::OrderedFloat;
use std::collections::{HashMap, HashSet};
use std::mem::take;

macro_rules! assemble {
    ($buf:expr, $fmt:literal $(, $arg:expr)* $(,)?) => {
        $buf.push_str(&format!(concat!($fmt, "\n") $(, $arg)*))
    };
}

pub struct AsmCodeGen {
    program: IRProgram,
    text: String,
    data: String,
    vars: HashMap<String, usize>,
    lbl_cnt: usize,
    str_cache: HashMap<String, String>,
    flt_cache: HashMap<OrderedFloat<f64>, String>,
    arg_reg: Vec<String>,
    flt_arg_reg: Vec<String>,
    ret_label: String,
    regs: HashMap<String, Option<Operand>>,
    curr_fn: String,
    curr_flt_reg: usize,
    internals: HashSet<String>,
}

impl AsmCodeGen {
    pub fn new(program: IRProgram) -> Self {
        let internals = program
            .functions
            .iter()
            .filter(|f| !f.is_external)
            .map(|f| f.name.clone())
            .collect();
        Self {
            program,
            text: String::new(),
            data: String::new(),
            vars: HashMap::new(),
            lbl_cnt: 0,
            str_cache: HashMap::new(),
            flt_cache: HashMap::new(),
            arg_reg: vec![
                "rdi".to_string(),
                "rsi".to_string(),
                "rdx".to_string(),
                "rcx".to_string(),
                "r8".to_string(),
                "r9".to_string(),
            ],
            flt_arg_reg: vec![
                "xmm0".to_string(),
                "xmm1".to_string(),
                "xmm2".to_string(),
                "xmm3".to_string(),
                "xmm4".to_string(),
                "xmm5".to_string(),
                "xmm6".to_string(),
                "xmm7".to_string(),
                "xmm8".to_string(),
                "xmm9".to_string(),
                "xmm10".to_string(),
                "xmm11".to_string(),
                "xmm12".to_string(),
                "xmm13".to_string(),
                "xmm14".to_string(),
                "xmm15".to_string(),
            ],
            ret_label: String::new(),
            regs: HashMap::new(),
            curr_fn: String::new(),
            curr_flt_reg: 0,
            internals,
        }
    }

    pub fn compile(&mut self) -> Result<String, CodeGenError> {
        assemble!(self.text, "section .text");
        for func in &self.program.functions {
            if func.is_external {
                assemble!(self.text, "extern {}", func.name);
            }
        }
        assemble!(self.text, "extern malloc");
        assemble!(self.text, "extern strlen");
        assemble!(self.text, "extern memcpy");
        assemble!(self.text, "extern strcpy");
        assemble!(self.data, "section .data");
        assemble!(self.data, "align 16");
        assemble!(self.data, "neg_mask: dq 0x8000000000000000, 0");

        for func in take(&mut self.program.functions) {
            self.compile_fn(func)?;
        }

        Ok(take(&mut self.data) + &self.text)
    }

    fn compile_code(&mut self, code: Instruction) -> Result<(), CodeGenError> {
        match code.op {
            Op::Move => {
                let src = code
                    .src1
                    .as_ref()
                    .ok_or_else(|| CodeGenError::MissingOperand {
                        message: "Move operation requires src1".to_string(),
                    })?;
                let dst = code
                    .dst
                    .as_ref()
                    .ok_or_else(|| CodeGenError::MissingOperand {
                        message: "Move operation requires dst".to_string(),
                    })?;
                self.load(src, "rax")?;
                if match src {
                    Operand::Var(_) | Operand::Temp(_, _) => {
                        self.get_offset(src)? != self.get_offset(dst)?
                    }
                    _ => true,
                } {
                    assemble!(self.text, "mov [rbp - {}], rax", self.get_offset(dst)?);
                }
                self.regs.insert("rax".to_string(), Some(dst.clone()));
                Ok(())
            }
            Op::FMove => {
                let src = code
                    .src1
                    .as_ref()
                    .ok_or_else(|| CodeGenError::MissingOperand {
                        message: "FMove operation requires src1".to_string(),
                    })?;
                let dst = code
                    .dst
                    .as_ref()
                    .ok_or_else(|| CodeGenError::MissingOperand {
                        message: "FMove operation requires dst".to_string(),
                    })?;
                self.load(src, "xmm0")?;
                if match src {
                    Operand::Var(_) | Operand::Temp(_, _) => {
                        self.get_offset(src)? != self.get_offset(dst)?
                    }
                    _ => true,
                } {
                    assemble!(self.text, "movsd [rbp - {}], xmm0", self.get_offset(dst)?);
                }
                self.regs.insert("xmm0".to_string(), Some(dst.clone()));
                Ok(())
            }
            Op::Load | Op::Store => {
                let src = code
                    .src1
                    .as_ref()
                    .ok_or_else(|| CodeGenError::MissingOperand {
                        message: "Load/Store operation requires src1".to_string(),
                    })?;
                let dst = code
                    .dst
                    .as_ref()
                    .ok_or_else(|| CodeGenError::MissingOperand {
                        message: "Load/Store operation requires dst".to_string(),
                    })?;
                self.load(src, "rax")?;
                assemble!(self.text, "mov [rbp - {}], rax", self.get_offset(dst)?);
                self.regs.insert("rax".to_string(), Some(dst.clone()));
                Ok(())
            }
            Op::FLoad | Op::FStore => {
                let src = code
                    .src1
                    .as_ref()
                    .ok_or_else(|| CodeGenError::MissingOperand {
                        message: "FLoad/FStore operation requires src1".to_string(),
                    })?;
                let dst = code
                    .dst
                    .as_ref()
                    .ok_or_else(|| CodeGenError::MissingOperand {
                        message: "FLoad/FStore operation requires dst".to_string(),
                    })?;
                self.load(src, "xmm0")?;
                assemble!(self.text, "movsd [rbp - {}], xmm0", self.get_offset(dst)?);
                self.regs.insert("xmm0".to_string(), Some(dst.clone()));
                Ok(())
            }
            Op::Add
            | Op::Sub
            | Op::Mul
            | Op::Div
            | Op::And
            | Op::Or
            | Op::LAnd
            | Op::LOr
            | Op::Xor => {
                let dst = code
                    .dst
                    .as_ref()
                    .ok_or_else(|| CodeGenError::MissingOperand {
                        message: "Binary operation requires dst".to_string(),
                    })?;
                let src1 = code
                    .src1
                    .as_ref()
                    .ok_or_else(|| CodeGenError::MissingOperand {
                        message: "Binary operation requires src1".to_string(),
                    })?;
                let src2 = code
                    .src2
                    .as_ref()
                    .ok_or_else(|| CodeGenError::MissingOperand {
                        message: "Binary operation requires src2".to_string(),
                    })?;
                let asm_op = self.get_asm_op(&code.op).to_string();
                self.load(src1, "rax")?;
                match src2 {
                    Operand::ConstIdx(idx) => {
                        if let IRConst::Int(v) = &self.program.constants[*idx] {
                            assemble!(self.text, "{} rax, {}", asm_op, v);
                        }
                    }
                    Operand::Var(_) | Operand::Temp(_, _) => {
                        let off = self.get_offset(src2)?;
                        if matches!(code.op, Op::Div) {
                            self.load(src2, "rbx")?;
                            assemble!(self.text, "cqo");
                            assemble!(self.text, "idiv rbx");
                        } else {
                            assemble!(self.text, "{} rax, qword [rbp - {}]", asm_op, off);
                        }
                    }
                    _ => {
                        self.load(src2, "rbx")?;
                        assemble!(self.text, "{} rax, rbx", asm_op);
                    }
                }
                assemble!(self.text, "mov [rbp - {}], rax", self.get_offset(dst)?);
                self.regs.remove("rax");
                self.regs.remove("rdx");
                if matches!(code.op, Op::Div) {
                    self.regs.remove("rbx");
                }
                self.regs.insert("rax".to_string(), Some(dst.clone()));
                Ok(())
            }
            Op::Mod => {
                let dst = code
                    .dst
                    .as_ref()
                    .ok_or_else(|| CodeGenError::MissingOperand {
                        message: "Mod operation requires dst".to_string(),
                    })?;
                let src1 = code
                    .src1
                    .as_ref()
                    .ok_or_else(|| CodeGenError::MissingOperand {
                        message: "Mod operation requires src1".to_string(),
                    })?;
                let src2 = code
                    .src2
                    .as_ref()
                    .ok_or_else(|| CodeGenError::MissingOperand {
                        message: "Mod operation requires src2".to_string(),
                    })?;
                self.load(src1, "rax")?;
                self.load(src2, "rbx")?;
                assemble!(self.text, "cqo");
                assemble!(self.text, "idiv rbx");
                assemble!(self.text, "mov [rbp - {}], rdx", self.get_offset(dst)?);
                self.regs.clear();
                self.regs.insert("rdx".to_string(), Some(dst.clone()));
                Ok(())
            }
            Op::FAdd | Op::FSub | Op::FMul | Op::FDiv => {
                let dst = code
                    .dst
                    .as_ref()
                    .ok_or_else(|| CodeGenError::MissingOperand {
                        message: "Float binary operation requires dst".to_string(),
                    })?;
                let src1 = code
                    .src1
                    .as_ref()
                    .ok_or_else(|| CodeGenError::MissingOperand {
                        message: "Float binary operation requires src1".to_string(),
                    })?;
                let src2 = code
                    .src2
                    .as_ref()
                    .ok_or_else(|| CodeGenError::MissingOperand {
                        message: "Float binary operation requires src2".to_string(),
                    })?;
                let fasm_op = self.get_fasm_op(&code.op).to_string();
                self.load(src1, "xmm0")?;
                match src2 {
                    Operand::ConstIdx(idx) => {
                        if let IRConst::Float(f) = &self.program.constants[*idx] {
                            let lbl = self.alloc_flt(*f);
                            assemble!(self.text, "{} xmm0, [rel {}]", fasm_op, lbl);
                        }
                    }
                    Operand::Var(_) | Operand::Temp(_, _) => {
                        let off = self.get_offset(src2)?;
                        assemble!(self.text, "{} xmm0, qword [rbp - {}]", fasm_op, off);
                    }
                    _ => {
                        self.load(src2, "xmm1")?;
                        assemble!(self.text, "{} xmm0, xmm1", fasm_op);
                    }
                }
                assemble!(self.text, "movsd [rbp - {}], xmm0", self.get_offset(dst)?);
                self.regs.insert("xmm0".to_string(), Some(dst.clone()));
                Ok(())
            }
            Op::Eq | Op::Ne | Op::Gt | Op::Ge | Op::Lt | Op::Le => {
                let dst = code
                    .dst
                    .as_ref()
                    .ok_or_else(|| CodeGenError::MissingOperand {
                        message: "Comparison operation requires dst".to_string(),
                    })?;
                let src1 = code
                    .src1
                    .as_ref()
                    .ok_or_else(|| CodeGenError::MissingOperand {
                        message: "Comparison operation requires src1".to_string(),
                    })?;
                let src2 = code
                    .src2
                    .as_ref()
                    .ok_or_else(|| CodeGenError::MissingOperand {
                        message: "Comparison operation requires src2".to_string(),
                    })?;
                self.load(src1, "rax")?;
                self.load(src2, "rbx")?;
                assemble!(self.text, "cmp rax, rbx");
                let set_op = match code.op {
                    Op::Eq => "sete",
                    Op::Ne => "setne",
                    Op::Gt => "setg",
                    Op::Ge => "setge",
                    Op::Lt => "setl",
                    Op::Le => "setle",
                    _ => unreachable!(),
                };
                assemble!(self.text, "{} al", set_op);
                assemble!(self.text, "movzx eax, al");
                assemble!(self.text, "mov [rbp - {}], rax", self.get_offset(dst)?);
                self.regs.clear();
                self.regs.insert("rax".to_string(), Some(dst.clone()));
                Ok(())
            }
            Op::FEq | Op::FNe | Op::FGt | Op::FGe | Op::FLt | Op::FLe => {
                let dst = code
                    .dst
                    .as_ref()
                    .ok_or_else(|| CodeGenError::MissingOperand {
                        message: "Float comparison operation requires dst".to_string(),
                    })?;
                let src1 = code
                    .src1
                    .as_ref()
                    .ok_or_else(|| CodeGenError::MissingOperand {
                        message: "Float comparison operation requires src1".to_string(),
                    })?;
                let src2 = code
                    .src2
                    .as_ref()
                    .ok_or_else(|| CodeGenError::MissingOperand {
                        message: "Float comparison operation requires src2".to_string(),
                    })?;
                self.load(src1, "xmm0")?;
                self.load(src2, "xmm1")?;
                assemble!(self.text, "ucomisd xmm0, xmm1");
                let set_op = match code.op {
                    Op::FEq => "sete",
                    Op::FNe => "setne",
                    Op::FGt => "seta",
                    Op::FGe => "setae",
                    Op::FLt => "setb",
                    Op::FLe => "setbe",
                    _ => unreachable!(),
                };
                assemble!(self.text, "{} al", set_op);
                assemble!(self.text, "movzx eax, al");
                assemble!(self.text, "mov [rbp - {}], rax", self.get_offset(dst)?);
                self.regs.clear();
                self.regs.insert("rax".to_string(), Some(dst.clone()));
                Ok(())
            }
            Op::Neg | Op::Inc | Op::Dec | Op::SizeOf => {
                let dst = code
                    .dst
                    .as_ref()
                    .ok_or_else(|| CodeGenError::MissingOperand {
                        message: "Unary operation requires dst".to_string(),
                    })?;
                let src1 = code
                    .src1
                    .as_ref()
                    .ok_or_else(|| CodeGenError::MissingOperand {
                        message: "Unary operation requires src1".to_string(),
                    })?;
                self.load(src1, "rax")?;
                match code.op {
                    Op::Neg => assemble!(self.text, "neg rax"),
                    Op::Inc => assemble!(self.text, "inc rax"),
                    Op::Dec => assemble!(self.text, "dec rax"),
                    Op::SizeOf => assemble!(self.text, "mov rax, [rax]"),
                    _ => unreachable!(),
                }
                assemble!(self.text, "mov [rbp - {}], rax", self.get_offset(dst)?);
                self.regs.clear();
                self.regs.insert("rax".to_string(), Some(dst.clone()));
                Ok(())
            }
            Op::Not => {
                let dst = code
                    .dst
                    .as_ref()
                    .ok_or_else(|| CodeGenError::MissingOperand {
                        message: "Not operation requires dst".to_string(),
                    })?;
                let src1 = code
                    .src1
                    .as_ref()
                    .ok_or_else(|| CodeGenError::MissingOperand {
                        message: "Not operation requires src1".to_string(),
                    })?;
                self.load(src1, "rax")?;
                assemble!(self.text, "xor rax, 1");
                assemble!(self.text, "mov [rbp - {}], rax", self.get_offset(dst)?);
                self.regs.clear();
                self.regs.insert("rax".to_string(), Some(dst.clone()));
                Ok(())
            }
            Op::FNeg => {
                let dst = code
                    .dst
                    .as_ref()
                    .ok_or_else(|| CodeGenError::MissingOperand {
                        message: "FNeg operation requires dst".to_string(),
                    })?;
                let src1 = code
                    .src1
                    .as_ref()
                    .ok_or_else(|| CodeGenError::MissingOperand {
                        message: "FNeg operation requires src1".to_string(),
                    })?;
                self.load(src1, "xmm0")?;
                assemble!(self.text, "xorpd xmm0, oword [rel neg_mask]");
                assemble!(self.text, "movsd [rbp - {}], xmm0", self.get_offset(dst)?);
                self.regs.clear();
                self.regs.insert("xmm0".to_string(), Some(dst.clone()));
                Ok(())
            }
            Op::Range => {
                let dst = code
                    .dst
                    .as_ref()
                    .ok_or_else(|| CodeGenError::MissingOperand {
                        message: "Range operation requires dst".to_string(),
                    })?;
                let src1 = code
                    .src1
                    .as_ref()
                    .ok_or_else(|| CodeGenError::MissingOperand {
                        message: "Range operation requires src1".to_string(),
                    })?;
                let src2 = code
                    .src2
                    .as_ref()
                    .ok_or_else(|| CodeGenError::MissingOperand {
                        message: "Range operation requires src2".to_string(),
                    })?;
                self.load(src1, "rdi")?;
                self.load(src2, "rsi")?;
                let end_lbl = self.new_label("range_end");
                let fill_lbl = self.new_label("range_fill");
                let skip_lbl = self.new_label("range_skip");
                assemble!(self.text, "push rdi");
                assemble!(self.text, "push rsi");
                assemble!(self.text, "mov rax, rsi");
                assemble!(self.text, "sub rax, rdi");
                assemble!(self.text, "cmp rax, 0");
                assemble!(self.text, "jge near {}", skip_lbl);
                assemble!(self.text, "xor rax, rax");
                assemble!(self.text, "{}:", skip_lbl);
                assemble!(self.text, "push rax");
                assemble!(self.text, "lea rdi, [rax * 8 + 8]");
                assemble!(self.text, "call malloc wrt ..plt");
                assemble!(self.text, "pop rdx");
                assemble!(self.text, "mov [rax], rdx");
                assemble!(self.text, "xor rcx, rcx");
                assemble!(self.text, "pop rsi");
                assemble!(self.text, "pop rdi");
                assemble!(self.text, "{}:", fill_lbl);
                assemble!(self.text, "cmp rcx, rdx");
                assemble!(self.text, "jge near {}", end_lbl);
                assemble!(self.text, "mov r8, rdi");
                assemble!(self.text, "add r8, rcx");
                assemble!(self.text, "mov [rax + rcx * 8 + 8], r8");
                assemble!(self.text, "inc rcx");
                assemble!(self.text, "jmp near {}", fill_lbl);
                assemble!(self.text, "{}:", end_lbl);
                assemble!(self.text, "mov [rbp - {}], rax", self.get_offset(dst)?);
                self.regs.clear();
                self.regs.insert("rax".to_string(), Some(dst.clone()));
                Ok(())
            }
            Op::Arg(n) => {
                let op = code
                    .src1
                    .as_ref()
                    .ok_or_else(|| CodeGenError::MissingOperand {
                        message: "Arg operation requires src1".to_string(),
                    })?;
                if n < 6 {
                    let reg = self.arg_reg[n].clone();
                    self.load(op, &reg)?;
                } else {
                    self.load(op, "rax")?;
                    assemble!(self.text, "push rax");
                }
                Ok(())
            }
            Op::FArg(n) => {
                let op = code
                    .src1
                    .as_ref()
                    .ok_or_else(|| CodeGenError::MissingOperand {
                        message: "FArg operation requires src1".to_string(),
                    })?;
                if n < 8 {
                    self.curr_flt_reg = n + 1;
                    let reg = self.flt_arg_reg[n].clone();
                    self.load(op, &reg)?;
                } else {
                    self.curr_flt_reg = 8;
                    self.load(op, "xmm0")?;
                    assemble!(self.text, "sub rsp, 8");
                    assemble!(self.text, "movsd [rsp], xmm0");
                }
                Ok(())
            }
            Op::Call => {
                let dst = code
                    .dst
                    .as_ref()
                    .ok_or_else(|| CodeGenError::MissingOperand {
                        message: "Call operation requires dst".to_string(),
                    })?;
                let src1 = code
                    .src1
                    .as_ref()
                    .ok_or_else(|| CodeGenError::MissingOperand {
                        message: "Call operation requires src1".to_string(),
                    })?;
                match src1 {
                    Operand::Function(name) => {
                        if self.curr_flt_reg > 0 {
                            assemble!(self.text, "mov al, {}", self.curr_flt_reg);
                        } else {
                            assemble!(self.text, "xor al, al");
                        }
                        self.curr_flt_reg = 0;
                        if self.internals.contains(name) {
                            assemble!(self.text, "call {}", name);
                        } else {
                            assemble!(self.text, "call {} wrt ..plt", name);
                        }
                    }
                    _ => {
                        self.load(src1, "rax")?;
                        assemble!(self.text, "call rax");
                    }
                }
                let caller_saved = ["rax", "rcx", "rdx", "rsi", "rdi", "r8", "r9", "r10", "r11"];
                for reg in caller_saved {
                    self.regs.remove(reg);
                }
                for i in 0..16 {
                    self.regs.remove(&format!("xmm{}", i));
                }
                let is_float = match dst {
                    Operand::Temp(_, IRType::Float) => true,
                    _ => false,
                };
                if is_float {
                    assemble!(self.text, "movsd [rbp - {}], xmm0", self.get_offset(dst)?);
                    self.regs.insert("xmm0".to_string(), Some(dst.clone()));
                } else {
                    assemble!(self.text, "mov [rbp - {}], rax", self.get_offset(dst)?);
                    self.regs.insert("rax".to_string(), Some(dst.clone()));
                }
                Ok(())
            }
            Op::Label(lbl) => {
                assemble!(self.text, "{}:", lbl);
                self.regs.clear();
                Ok(())
            }
            Op::Jump => {
                let src1 = code
                    .src1
                    .as_ref()
                    .ok_or_else(|| CodeGenError::MissingOperand {
                        message: "Jump operation requires src1".to_string(),
                    })?;
                if let Operand::Label(lbl) = src1 {
                    assemble!(self.text, "jmp near {}", lbl);
                }
                Ok(())
            }
            Op::JumpIfFalse => {
                let src1 = code
                    .src1
                    .as_ref()
                    .ok_or_else(|| CodeGenError::MissingOperand {
                        message: "JumpIfFalse operation requires src1".to_string(),
                    })?;
                let src2 = code
                    .src2
                    .as_ref()
                    .ok_or_else(|| CodeGenError::MissingOperand {
                        message: "JumpIfFalse operation requires src2".to_string(),
                    })?;
                let lbl = match src2 {
                    Operand::Label(s) => s,
                    _ => {
                        return Err(CodeGenError::InvalidOperand {
                            message: "JumpIfFalse src2 must be a Label".to_string(),
                        });
                    }
                };
                self.load(src1, "rax")?;
                assemble!(self.text, "cmp rax, 0");
                assemble!(self.text, "je near {}", lbl);
                Ok(())
            }
            Op::ArrayAccess => {
                let dst = code
                    .dst
                    .as_ref()
                    .ok_or_else(|| CodeGenError::MissingOperand {
                        message: "ArrayAccess operation requires dst".to_string(),
                    })?;
                let src1 = code
                    .src1
                    .as_ref()
                    .ok_or_else(|| CodeGenError::MissingOperand {
                        message: "ArrayAccess operation requires src1".to_string(),
                    })?;
                let src2 = code
                    .src2
                    .as_ref()
                    .ok_or_else(|| CodeGenError::MissingOperand {
                        message: "ArrayAccess operation requires src2".to_string(),
                    })?;
                self.load(src1, "r10")?;
                self.load(src2, "rcx")?;
                assemble!(self.text, "lea rax, [r10 + rcx * 8 + 8]");
                assemble!(self.text, "mov rax, [rax]");
                assemble!(self.text, "mov [rbp - {}], rax", self.get_offset(dst)?);
                self.regs.clear();
                self.regs.insert("rax".to_string(), Some(dst.clone()));
                Ok(())
            }
            Op::ArrayAssign => {
                let dst = code
                    .dst
                    .as_ref()
                    .ok_or_else(|| CodeGenError::MissingOperand {
                        message: "ArrayAssign operation requires dst".to_string(),
                    })?;
                let src1 = code
                    .src1
                    .as_ref()
                    .ok_or_else(|| CodeGenError::MissingOperand {
                        message: "ArrayAssign operation requires src1".to_string(),
                    })?;
                let src2 = code
                    .src2
                    .as_ref()
                    .ok_or_else(|| CodeGenError::MissingOperand {
                        message: "ArrayAssign operation requires src2".to_string(),
                    })?;
                self.load(dst, "r10")?;
                self.load(src1, "rcx")?;
                self.load(src2, "rax")?;
                assemble!(self.text, "lea rdx, [r10 + rcx * 8 + 8]");
                assemble!(self.text, "mov [rdx], rax");
                Ok(())
            }
            Op::Lea => {
                let dst = code
                    .dst
                    .as_ref()
                    .ok_or_else(|| CodeGenError::MissingOperand {
                        message: "Lea operation requires dst".to_string(),
                    })?;
                let src1 = code
                    .src1
                    .as_ref()
                    .ok_or_else(|| CodeGenError::MissingOperand {
                        message: "Lea operation requires src1".to_string(),
                    })?;
                let offset = self.get_offset(src1)?;
                assemble!(self.text, "lea rax, [rbp - {}]", offset);
                assemble!(self.text, "mov [rbp - {}], rax", self.get_offset(dst)?);
                self.regs.clear();
                self.regs.insert("rax".to_string(), Some(dst.clone()));
                Ok(())
            }
            Op::Malloc => {
                let dst = code
                    .dst
                    .as_ref()
                    .ok_or_else(|| CodeGenError::MissingOperand {
                        message: "Malloc operation requires dst".to_string(),
                    })?;
                let src1 = code
                    .src1
                    .as_ref()
                    .ok_or_else(|| CodeGenError::MissingOperand {
                        message: "Malloc operation requires src1".to_string(),
                    })?;
                self.load(src1, "rdi")?;
                assemble!(self.text, "call malloc wrt ..plt");
                assemble!(self.text, "mov [rbp - {}], rax", self.get_offset(dst)?);
                self.regs.clear();
                self.regs.insert("rax".to_string(), Some(dst.clone()));
                Ok(())
            }
            Op::StoreAt => {
                let dst = code
                    .dst
                    .as_ref()
                    .ok_or_else(|| CodeGenError::MissingOperand {
                        message: "StoreAt operation requires dst".to_string(),
                    })?;
                let src1 = code
                    .src1
                    .as_ref()
                    .ok_or_else(|| CodeGenError::MissingOperand {
                        message: "StoreAt operation requires src1".to_string(),
                    })?;
                let src2 = code
                    .src2
                    .as_ref()
                    .ok_or_else(|| CodeGenError::MissingOperand {
                        message: "StoreAt operation requires src2".to_string(),
                    })?;
                self.load(dst, "r10")?;
                self.load(src2, "rax")?;
                match src1 {
                    Operand::ConstIdx(idx) => {
                        if let IRConst::Int(offset) = &self.program.constants[*idx] {
                            if *offset == 0 {
                                assemble!(self.text, "mov [r10], rax");
                            } else {
                                assemble!(self.text, "mov [r10 + {}], rax", offset);
                            }
                        }
                    }
                    _ => {
                        self.load(src1, "r11")?;
                        assemble!(self.text, "add r10, r11");
                        assemble!(self.text, "mov [r10], rax");
                    }
                }
                Ok(())
            }
            Op::LoadAt => {
                let dst = code
                    .dst
                    .as_ref()
                    .ok_or_else(|| CodeGenError::MissingOperand {
                        message: "LoadAt operation requires dst".to_string(),
                    })?;
                let src1 = code
                    .src1
                    .as_ref()
                    .ok_or_else(|| CodeGenError::MissingOperand {
                        message: "LoadAt operation requires src1".to_string(),
                    })?;
                let src2 = code
                    .src2
                    .as_ref()
                    .ok_or_else(|| CodeGenError::MissingOperand {
                        message: "LoadAt operation requires src2".to_string(),
                    })?;
                self.load(src1, "r10")?;
                match src2 {
                    Operand::ConstIdx(idx) => {
                        if let IRConst::Int(offset) = &self.program.constants[*idx] {
                            if *offset == 0 {
                                assemble!(self.text, "mov rax, [r10]");
                            } else {
                                assemble!(self.text, "mov rax, [r10 + {}]", offset);
                            }
                        }
                    }
                    _ => {
                        self.load(src2, "r11")?;
                        assemble!(self.text, "add r10, r11");
                        assemble!(self.text, "mov rax, [r10]");
                    }
                }
                assemble!(self.text, "mov [rbp - {}], rax", self.get_offset(dst)?);
                self.regs.clear();
                self.regs.insert("rax".to_string(), Some(dst.clone()));
                Ok(())
            }
            Op::StrLen => {
                let dst = code
                    .dst
                    .as_ref()
                    .ok_or_else(|| CodeGenError::MissingOperand {
                        message: "StrLen operation requires dst".to_string(),
                    })?;
                let src1 = code
                    .src1
                    .as_ref()
                    .ok_or_else(|| CodeGenError::MissingOperand {
                        message: "StrLen operation requires src1".to_string(),
                    })?;
                self.load(src1, "rdi")?;
                assemble!(self.text, "call strlen wrt ..plt");
                assemble!(self.text, "mov [rbp - {}], rax", self.get_offset(dst)?);
                self.regs.clear();
                self.regs.insert("rax".to_string(), Some(dst.clone()));
                Ok(())
            }
            Op::MemCopy => {
                let dst_op = code
                    .dst
                    .as_ref()
                    .ok_or_else(|| CodeGenError::MissingOperand {
                        message: "MemCopy operation requires dst".to_string(),
                    })?;
                let src1 = code
                    .src1
                    .as_ref()
                    .ok_or_else(|| CodeGenError::MissingOperand {
                        message: "MemCopy operation requires src1".to_string(),
                    })?;
                let src2 = code
                    .src2
                    .as_ref()
                    .ok_or_else(|| CodeGenError::MissingOperand {
                        message: "MemCopy operation requires src2".to_string(),
                    })?;
                self.load(dst_op, "rdi")?;
                self.load(src1, "rsi")?;
                self.load(src2, "rdx")?;
                assemble!(self.text, "call memcpy wrt ..plt");
                Ok(())
            }
            Op::StrCat => {
                let dst = code
                    .dst
                    .as_ref()
                    .ok_or_else(|| CodeGenError::MissingOperand {
                        message: "StrCat operation requires dst".to_string(),
                    })?;
                let src1 = code
                    .src1
                    .as_ref()
                    .ok_or_else(|| CodeGenError::MissingOperand {
                        message: "StrCat operation requires src1".to_string(),
                    })?;
                let src2 = code
                    .src2
                    .as_ref()
                    .ok_or_else(|| CodeGenError::MissingOperand {
                        message: "StrCat operation requires src2".to_string(),
                    })?;
                let off_tmp = self.load_to_temp(src1)?;
                assemble!(self.text, "mov rdi, [rbp - {}]", off_tmp);
                assemble!(self.text, "call strlen wrt ..plt");
                assemble!(self.text, "mov rbx, rax");
                let off_tmp2 = self.load_to_temp(src2)?;
                assemble!(self.text, "mov rdi, [rbp - {}]", off_tmp2);
                assemble!(self.text, "call strlen wrt ..plt");
                assemble!(self.text, "lea rdi, [rbx + rax + 1]");
                assemble!(self.text, "call malloc wrt ..plt");
                assemble!(self.text, "mov r15, rax");
                assemble!(self.text, "mov rdi, r15");
                assemble!(self.text, "mov rsi, [rbp - {}]", off_tmp);
                assemble!(self.text, "call strcpy wrt ..plt");
                assemble!(self.text, "mov rdi, r15");
                assemble!(self.text, "call strlen wrt ..plt");
                assemble!(self.text, "lea rdi, [r15 + rax]");
                assemble!(self.text, "mov rsi, [rbp - {}]", off_tmp2);
                assemble!(self.text, "call strcpy wrt ..plt");
                assemble!(self.text, "mov [rbp - {}], r15", self.get_offset(dst)?);
                self.regs.clear();
                self.regs.insert("r15".to_string(), Some(dst.clone()));
                Ok(())
            }
            Op::Return(reg) => {
                if let Some(ref val) = code.src1 {
                    self.load(val, &reg)?;
                }
                assemble!(self.text, "jmp near {}", self.ret_label);
                Ok(())
            }
        }
    }

    fn load_to_temp(&self, op: &Operand) -> Result<usize, CodeGenError> {
        match op {
            Operand::Var(_) | Operand::Temp(_, _) => self.get_offset(op),
            _ => Err(CodeGenError::InvalidOperand {
                message: "load_to_temp not supported".to_string(),
            }),
        }
    }

    fn compile_fn(&mut self, func: IRFunction) -> Result<(), CodeGenError> {
        if func.is_external {
            assemble!(self.text, "extern {}", func.name);
            return Ok(());
        }

        self.vars.clear();
        self.regs.clear();
        let mut offset = 0;

        for (param, _) in &func.params {
            if let Operand::Var(name) = param {
                if !self.vars.contains_key(name) {
                    offset += 8;
                    self.vars.insert(name.clone(), offset);
                }
            }
        }
        for inst in &func.instructions {
            let mut register_op = |op_opt: &Option<Operand>| {
                if let Some(op) = op_opt {
                    match op {
                        Operand::Var(name) => {
                            if !self.vars.contains_key(name) {
                                offset += 8;
                                self.vars.insert(name.clone(), offset);
                            }
                        }
                        Operand::Temp(id, _) => {
                            let temp_key = format!("_tmp_{}", id);
                            if !self.vars.contains_key(&temp_key) {
                                offset += 8;
                                self.vars.insert(temp_key, offset);
                            }
                        }
                        _ => {}
                    }
                }
            };
            register_op(&inst.dst);
            register_op(&inst.src1);
            register_op(&inst.src2);
        }

        let stack_size = (offset + 15) & !15;
        if func.is_pub {
            assemble!(self.text, "global {}", func.name);
        }
        assemble!(self.text, "{}:", func.name);
        assemble!(self.text, "push rbp");
        assemble!(self.text, "mov rbp, rsp");
        if stack_size > 0 {
            assemble!(self.text, "sub rsp, {}", stack_size);
        }

        self.curr_fn = func.name.clone();
        self.ret_label = format!(".L_{}_exit", func.name);

        let mut int_idx = 0;
        let mut flt_idx = 0;
        for (param, ty) in &func.params {
            let off = self.get_offset(param)?;
            if matches!(ty, IRType::Float) {
                if flt_idx < 8 {
                    let reg = format!("xmm{}", flt_idx);
                    assemble!(self.text, "movsd [rbp - {}], {}", off, reg);
                    self.regs.insert(reg, Some(param.clone()));
                    flt_idx += 1;
                }
            } else {
                if int_idx < 6 {
                    let reg = self.arg_reg[int_idx].clone();
                    assemble!(self.text, "mov [rbp - {}], {}", off, reg);
                    self.regs.insert(reg, Some(param.clone()));
                    int_idx += 1;
                }
            }
        }

        let insts = &func.instructions;
        for code in insts.iter() {
            match &code.op {
                Op::Return(reg_name) => {
                    if let Some(ref val) = code.src1 {
                        self.load(val, reg_name)?;
                    }
                    assemble!(self.text, "jmp near {}", self.ret_label);
                }
                Op::Label(name) => {
                    assemble!(self.text, "{}:", name);
                    self.regs.clear();
                }
                _ => {
                    self.compile_code(code.clone())?;
                }
            }
        }

        assemble!(self.text, "{}:", self.ret_label);
        assemble!(self.text, "leave");
        assemble!(self.text, "ret");
        Ok(())
    }

    fn load(&mut self, op: &Operand, reg: &str) -> Result<(), CodeGenError> {
        if let Some(Some(cached_op)) = self.regs.get(reg) {
            if cached_op == op {
                return Ok(());
            }
        }

        match op {
            Operand::ConstIdx(idx) => {
                let constant = &self.program.constants[*idx];
                match constant {
                    IRConst::Int(v) => assemble!(self.text, "mov {}, {}", reg, v),
                    IRConst::Float(f) => {
                        let lbl = self.alloc_flt(*f);
                        if reg.starts_with("xmm") {
                            assemble!(self.text, "movsd {}, [rel {}]", reg, lbl);
                        } else {
                            assemble!(self.text, "mov {}, [rel {}]", reg, lbl);
                        }
                    }
                    IRConst::Str(s) => {
                        let lbl = self.alloc_str(s.clone());
                        assemble!(self.text, "lea {}, [rel {}]", reg, lbl);
                    }
                    IRConst::Array(len, arr) => {
                        self.alloc_arr(*len, arr.clone(), reg)?;
                    }
                }
            }
            Operand::Var(_) | Operand::Temp(_, _) => {
                let off = self.get_offset(op)?;
                if reg.starts_with("xmm") {
                    assemble!(self.text, "movsd {}, qword [rbp - {}]", reg, off);
                } else {
                    assemble!(self.text, "mov {}, [rbp - {}]", reg, off);
                }
            }
            Operand::Function(name) => {
                assemble!(self.text, "lea {}, [rel {}]", reg, name);
            }
            _ => {}
        }

        self.regs.insert(reg.to_string(), Some(op.clone()));
        Ok(())
    }

    fn new_label(&mut self, name: &str) -> String {
        let lbl = format!(".{}_{:X}", name, self.lbl_cnt);
        self.lbl_cnt += 1;
        lbl
    }

    fn alloc_str(&mut self, s: String) -> String {
        if let Some(lbl) = self.str_cache.get(&s) {
            return lbl.clone();
        } else {
            let lbl = format!("L.S.{}", self.lbl_cnt);
            self.str_cache.insert(s.clone(), lbl.clone());
            self.lbl_cnt += 1;
            let bytes = s.as_bytes();
            if bytes.is_empty() {
                assemble!(self.data, "{} db 0", lbl);
            } else {
                assemble!(
                    self.data,
                    "{} db {}, 0",
                    lbl,
                    bytes
                        .iter()
                        .map(|b| b.to_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                );
            }
            lbl
        }
    }

    fn alloc_flt(&mut self, f: OrderedFloat<f64>) -> String {
        if let Some(lbl) = self.flt_cache.get(&f) {
            return lbl.clone();
        } else {
            let lbl = format!("L.F.{}", self.lbl_cnt);
            self.flt_cache.insert(f, lbl.clone());
            self.lbl_cnt += 1;
            assemble!(self.data, "{} dq 0x{:x}", lbl, f.into_inner().to_bits());
            lbl
        }
    }

    fn get_offset(&self, op: &Operand) -> Result<usize, CodeGenError> {
        match op {
            Operand::Var(name) => self
                .vars
                .get(name)
                .ok_or_else(|| CodeGenError::MissingOperand {
                    message: format!("variable '{}' not found in stack frame", name),
                })
                .map(|v| *v),
            Operand::Temp(id, _) => {
                let key = format!("_tmp_{}", id);
                self.vars
                    .get(&key)
                    .ok_or_else(|| CodeGenError::MissingOperand {
                        message: format!("temporary '{}' not found in stack frame", key),
                    })
                    .map(|v| *v)
            }
            _ => Err(CodeGenError::InvalidOperand {
                message: "Not a stack operand".to_string(),
            }),
        }
    }

    fn alloc_arr(&mut self, len: usize, arr: Vec<Operand>, reg: &str) -> Result<(), CodeGenError> {
        let size = (len * 8 + 8 + 15) & !15;
        assemble!(self.text, "sub rsp, {}", size);
        assemble!(self.text, "mov r10, rsp");
        assemble!(self.text, "mov rax, {}", len);
        assemble!(self.text, "mov [r10], rax");
        for (i, op) in arr.iter().enumerate() {
            self.load(op, "rax")?;
            assemble!(self.text, "mov [r10 + {}], rax", 8 + i * 8);
        }
        assemble!(self.text, "mov {}, r10", reg);
        self.regs.clear();
        Ok(())
    }

    fn get_asm_op(&self, op: &Op) -> &str {
        match op {
            Op::Add => "add",
            Op::Sub => "sub",
            Op::Mul => "imul",
            Op::Div => "idiv",
            Op::LAnd => "and",
            Op::LOr => "or",
            Op::Xor => "xor",
            _ => "",
        }
    }

    fn get_fasm_op(&self, op: &Op) -> &str {
        match op {
            Op::FAdd => "addsd",
            Op::FSub => "subsd",
            Op::FMul => "mulsd",
            Op::FDiv => "divsd",
            _ => "",
        }
    }
}
