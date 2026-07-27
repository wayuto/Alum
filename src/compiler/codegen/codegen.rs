use super::ir::Operand as IROperand;
use super::ir::{IRConst, IRFunction, IRProgram, IRType, Instruction, Op};
use super::types::CodeGenError;
use crate::compiler::codegen::asm::*;
use ordered_float::OrderedFloat;
use std::collections::{HashMap, HashSet};
use std::mem::take;

fn parse_reg(s: &str) -> Reg {
    match s {
        "rax" => Reg::Rax,
        "rbx" => Reg::Rbx,
        "rcx" => Reg::Rcx,
        "rdx" => Reg::Rdx,
        "rsi" => Reg::Rsi,
        "rdi" => Reg::Rdi,
        "r8" => Reg::R8,
        "r9" => Reg::R9,
        "r10" => Reg::R10,
        "r11" => Reg::R11,
        "r15" => Reg::R15,
        "rsp" => Reg::Rsp,
        "rbp" => Reg::Rbp,
        "xmm0" => Reg::Xmm0,
        "xmm1" => Reg::Xmm1,
        "xmm2" => Reg::Xmm2,
        "xmm3" => Reg::Xmm3,
        "xmm4" => Reg::Xmm4,
        "xmm5" => Reg::Xmm5,
        "xmm6" => Reg::Xmm6,
        "xmm7" => Reg::Xmm7,
        "xmm8" => Reg::Xmm8,
        "xmm9" => Reg::Xmm9,
        "xmm10" => Reg::Xmm10,
        "xmm11" => Reg::Xmm11,
        "xmm12" => Reg::Xmm12,
        "xmm13" => Reg::Xmm13,
        "xmm14" => Reg::Xmm14,
        "xmm15" => Reg::Xmm15,
        _ => panic!("unknown register: {}", s),
    }
}

fn m_rbp(off: usize) -> Operand {
    Operand::Mem(Mem {
        base: Some(Reg::Rbp),
        index: None,
        scale: 0,
        disp: -(off as i32),
        size: None,
    })
}

fn m_rbp_q(off: usize) -> Operand {
    Operand::Mem(Mem {
        base: Some(Reg::Rbp),
        index: None,
        scale: 0,
        disp: -(off as i32),
        size: Some(Size::QWord),
    })
}

fn m_base(base: Reg) -> Operand {
    Operand::Mem(Mem {
        base: Some(base),
        index: None,
        scale: 0,
        disp: 0,
        size: None,
    })
}

fn m_base_disp(base: Reg, disp: i32) -> Operand {
    Operand::Mem(Mem {
        base: Some(base),
        index: None,
        scale: 0,
        disp,
        size: None,
    })
}

fn m_sib(base: Reg, index: Reg, scale: u8, disp: i32) -> Operand {
    Operand::Mem(Mem {
        base: Some(base),
        index: Some(index),
        scale,
        disp,
        size: None,
    })
}

fn rel(lbl: String) -> Operand {
    Operand::DataLabel(lbl)
}

pub struct AsmCodeGen {
    program: IRProgram,
    text_asms: Vec<Asm>,
    data_asms: Vec<Asm>,
    vars: HashMap<String, usize>,
    lbl_cnt: usize,
    str_cache: HashMap<String, String>,
    flt_cache: HashMap<OrderedFloat<f64>, String>,
    arg_reg: Vec<String>,
    flt_arg_reg: Vec<String>,
    ret_label: String,
    regs: HashMap<String, Option<IROperand>>,
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
            text_asms: Vec::new(),
            data_asms: Vec::new(),
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

    pub fn compile(&mut self) -> Result<Vec<Asm>, CodeGenError> {
        self.text_asms.push(Asm::Section(Section::Text));
        for func in &self.program.functions {
            if func.is_external {
                let has_internal = self
                    .program
                    .functions
                    .iter()
                    .any(|f| f.name == func.name && !f.is_external);
                if !has_internal {
                    self.text_asms.push(Asm::Extern(func.name.clone()));
                } else {
                }
            }
        }
        self.text_asms.push(Asm::Extern("malloc".to_string()));
        self.text_asms.push(Asm::Extern("strlen".to_string()));
        self.text_asms.push(Asm::Extern("memcpy".to_string()));
        self.text_asms.push(Asm::Extern("strcpy".to_string()));

        self.data_asms.push(Asm::Section(Section::Data));
        self.data_asms.push(Asm::Align(16));
        self.data_asms.push(Asm::Label("neg_mask".to_string()));
        self.data_asms.push(Asm::Dq(vec![0x8000000000000000, 0]));

        for func in take(&mut self.program.functions) {
            self.compile_fn(func)?;
        }

        let defined: Vec<String> = self
            .text_asms
            .iter()
            .filter_map(|a| {
                if let Asm::Global(n) = a {
                    Some(n.clone())
                } else {
                    None
                }
            })
            .collect();
        self.text_asms.retain(|a| {
            if let Asm::Extern(n) = a {
                !defined.contains(n)
            } else {
                true
            }
        });

        let mut all = Vec::new();
        all.extend(take(&mut self.data_asms));
        all.extend(take(&mut self.text_asms));
        Ok(all)
    }

    fn push_text(&mut self, a: Asm) {
        self.text_asms.push(a);
    }

    fn push_data(&mut self, a: Asm) {
        self.data_asms.push(a);
    }

    fn compile_code(&mut self, code: Instruction) -> Result<(), CodeGenError> {
        match code.op {
            Op::Move => {
                let src = code.src1.as_ref().unwrap();
                let dst = code.dst.as_ref().unwrap();
                self.load(src, "rax")?;
                if match src {
                    IROperand::Var(_) | IROperand::Temp(_, _) => {
                        self.get_offset(src)? != self.get_offset(dst)?
                    }
                    _ => true,
                } {
                    self.store_dst(dst, Reg::Rax)?;
                } else {
                    self.regs.insert("rax".to_string(), Some(dst.clone()));
                }
                Ok(())
            }
            Op::FMove => {
                let src = code.src1.as_ref().unwrap();
                let dst = code.dst.as_ref().unwrap();
                self.load(src, "xmm0")?;
                if match src {
                    IROperand::Var(_) | IROperand::Temp(_, _) => {
                        self.get_offset(src)? != self.get_offset(dst)?
                    }
                    _ => true,
                } {
                    self.store_dst_xmm(dst, Reg::Xmm0)?;
                } else {
                    self.regs.insert("xmm0".to_string(), Some(dst.clone()));
                }
                Ok(())
            }
            Op::Load | Op::Store => {
                let src = code.src1.as_ref().unwrap();
                let dst = code.dst.as_ref().unwrap();
                self.load(src, "rax")?;
                self.store_dst(dst, Reg::Rax)?;
                Ok(())
            }
            Op::FLoad | Op::FStore => {
                let src = code.src1.as_ref().unwrap();
                let dst = code.dst.as_ref().unwrap();
                self.load(src, "xmm0")?;
                self.store_dst_xmm(dst, Reg::Xmm0)?;
                Ok(())
            }
            Op::Add | Op::Sub | Op::Mul | Op::Div | Op::LAnd | Op::LOr | Op::Xor => {
                let dst = code.dst.as_ref().unwrap();
                let src1 = code.src1.as_ref().unwrap();
                let src2 = code.src2.as_ref().unwrap();
                self.load(src1, "rax")?;
                if matches!(code.op, Op::Div) {
                    self.load(src2, "rbx")?;
                    self.push_text(Asm::Cqo);
                    self.push_text(Asm::Idiv(Reg::Rbx));
                } else {
                    let asm_op = match code.op {
                        Op::Add => Asm::Add,
                        Op::Sub => Asm::Sub,
                        Op::Mul => Asm::Imul,
                        Op::LAnd => Asm::And,
                        Op::LOr => Asm::Or,
                        Op::Xor => Asm::Xor,
                        _ => unreachable!(),
                    };
                    match src2 {
                        IROperand::ConstIdx(idx) => {
                            if let IRConst::Int(v) = &self.program.constants[*idx] {
                                self.push_text(asm_op(Operand::Reg(Reg::Rax), Operand::Imm(*v)));
                            }
                        }
                        IROperand::Var(_) | IROperand::Temp(_, _) => {
                            self.push_text(asm_op(
                                Operand::Reg(Reg::Rax),
                                m_rbp_q(self.get_offset(src2)?),
                            ));
                        }
                        _ => {
                            self.load(src2, "rbx")?;
                            self.push_text(asm_op(Operand::Reg(Reg::Rax), Operand::Reg(Reg::Rbx)));
                        }
                    }
                }
                self.store_dst(dst, Reg::Rax)?;
                self.regs.remove("rax");
                self.regs.remove("rdx");
                if matches!(code.op, Op::Div) {
                    self.regs.remove("rbx");
                }
                self.regs.insert("rax".to_string(), Some(dst.clone()));
                Ok(())
            }
            Op::Mod => {
                let dst = code.dst.as_ref().unwrap();
                let src1 = code.src1.as_ref().unwrap();
                let src2 = code.src2.as_ref().unwrap();
                self.load(src1, "rax")?;
                self.load(src2, "rbx")?;
                self.push_text(Asm::Cqo);
                self.push_text(Asm::Idiv(Reg::Rbx));
                self.invalidate_cached_operand(dst, Some("rdx"));
                self.push_text(Asm::Mov(
                    m_rbp(self.get_offset(dst)?),
                    Operand::Reg(Reg::Rdx),
                ));
                self.invalidate_cached_reg("rax");
                self.invalidate_cached_reg("rdx");
                self.invalidate_cached_reg("rbx");
                self.regs.insert("rdx".to_string(), Some(dst.clone()));
                Ok(())
            }
            Op::FAdd | Op::FSub | Op::FMul | Op::FDiv => {
                let dst = code.dst.as_ref().unwrap();
                let src1 = code.src1.as_ref().unwrap();
                let src2 = code.src2.as_ref().unwrap();
                let asm_op = match code.op {
                    Op::FAdd => Asm::Addsd,
                    Op::FSub => Asm::Subsd,
                    Op::FMul => Asm::Mulsd,
                    Op::FDiv => Asm::Divsd,
                    _ => unreachable!(),
                };
                self.load(src1, "xmm0")?;
                let src2_op = match src2 {
                    IROperand::ConstIdx(idx) => {
                        if let IRConst::Float(f) = &self.program.constants[*idx] {
                            let lbl = self.alloc_flt(*f);
                            rel(lbl)
                        } else {
                            unreachable!()
                        }
                    }
                    IROperand::Var(_) | IROperand::Temp(_, _) => m_rbp_q(self.get_offset(src2)?),
                    _ => {
                        self.load(src2, "xmm1")?;
                        Operand::Reg(Reg::Xmm1)
                    }
                };
                self.push_text(asm_op(Operand::Reg(Reg::Xmm0), src2_op));
                self.push_text(Asm::Movsd(
                    m_rbp(self.get_offset(dst)?),
                    Operand::Reg(Reg::Xmm0),
                ));
                self.regs.insert("xmm0".to_string(), Some(dst.clone()));
                Ok(())
            }
            Op::Eq | Op::Ne | Op::Gt | Op::Ge | Op::Lt | Op::Le => {
                let dst = code.dst.as_ref().unwrap();
                let src1 = code.src1.as_ref().unwrap();
                let src2 = code.src2.as_ref().unwrap();
                self.load(src1, "rax")?;
                self.load(src2, "rbx")?;
                self.push_text(Asm::Cmp(Operand::Reg(Reg::Rax), Operand::Reg(Reg::Rbx)));
                let set_op = match code.op {
                    Op::Eq => Asm::Sete,
                    Op::Ne => Asm::Setne,
                    Op::Gt => Asm::Setg,
                    Op::Ge => Asm::Setge,
                    Op::Lt => Asm::Setl,
                    Op::Le => Asm::Setle,
                    _ => unreachable!(),
                };
                self.push_text(set_op(Reg::Rax));
                self.push_text(Asm::Movzx(Reg::Rax, Reg::Rax));
                self.push_text(Asm::Mov(
                    m_rbp(self.get_offset(dst)?),
                    Operand::Reg(Reg::Rax),
                ));
                self.invalidate_cached_reg("rax");
                self.regs.insert("rax".to_string(), Some(dst.clone()));
                Ok(())
            }
            Op::FEq | Op::FNe | Op::FGt | Op::FGe | Op::FLt | Op::FLe => {
                let dst = code.dst.as_ref().unwrap();
                let src1 = code.src1.as_ref().unwrap();
                let src2 = code.src2.as_ref().unwrap();
                self.load(src1, "xmm0")?;
                self.load(src2, "xmm1")?;
                self.push_text(Asm::Ucomisd(Reg::Xmm0, Reg::Xmm1));
                let set_op = match code.op {
                    Op::FEq => Asm::Sete,
                    Op::FNe => Asm::Setne,
                    Op::FGt => Asm::Seta,
                    Op::FGe => Asm::Setae,
                    Op::FLt => Asm::Setb,
                    Op::FLe => Asm::Setbe,
                    _ => unreachable!(),
                };
                self.push_text(set_op(Reg::Rax));
                self.push_text(Asm::Movzx(Reg::Rax, Reg::Rax));
                self.push_text(Asm::Mov(
                    m_rbp(self.get_offset(dst)?),
                    Operand::Reg(Reg::Rax),
                ));
                self.invalidate_cached_reg("rax");
                self.regs.insert("rax".to_string(), Some(dst.clone()));
                Ok(())
            }
            Op::Neg | Op::Inc | Op::Dec | Op::SizeOf => {
                let dst = code.dst.as_ref().unwrap();
                let src1 = code.src1.as_ref().unwrap();
                self.load(src1, "rax")?;
                match code.op {
                    Op::Neg => self.push_text(Asm::Neg(Reg::Rax)),
                    Op::Inc => self.push_text(Asm::Inc(Reg::Rax)),
                    Op::Dec => self.push_text(Asm::Dec(Reg::Rax)),
                    Op::SizeOf => {
                        self.push_text(Asm::Mov(Operand::Reg(Reg::Rax), m_base_disp(Reg::Rax, -8)))
                    }
                    _ => unreachable!(),
                }
                self.push_text(Asm::Mov(
                    m_rbp(self.get_offset(dst)?),
                    Operand::Reg(Reg::Rax),
                ));
                self.invalidate_cached_reg("rax");
                self.regs.insert("rax".to_string(), Some(dst.clone()));
                Ok(())
            }
            Op::Not => {
                let dst = code.dst.as_ref().unwrap();
                let src1 = code.src1.as_ref().unwrap();
                self.load(src1, "rax")?;
                self.push_text(Asm::Xor(Operand::Reg(Reg::Rax), Operand::Imm(1)));
                self.push_text(Asm::Mov(
                    m_rbp(self.get_offset(dst)?),
                    Operand::Reg(Reg::Rax),
                ));
                self.invalidate_cached_reg("rax");
                self.regs.insert("rax".to_string(), Some(dst.clone()));
                Ok(())
            }
            Op::FNeg => {
                let dst = code.dst.as_ref().unwrap();
                let src1 = code.src1.as_ref().unwrap();
                self.load(src1, "xmm0")?;
                self.push_text(Asm::Xorpd(Reg::Xmm0, rel("neg_mask".to_string())));
                self.push_text(Asm::Movsd(
                    m_rbp(self.get_offset(dst)?),
                    Operand::Reg(Reg::Xmm0),
                ));
                self.invalidate_cached_reg("xmm0");
                self.regs.insert("xmm0".to_string(), Some(dst.clone()));
                Ok(())
            }
            Op::Range => {
                let dst = code.dst.as_ref().unwrap();
                let src1 = code.src1.as_ref().unwrap();
                let src2 = code.src2.as_ref().unwrap();
                self.load(src1, "rdi")?;
                self.load(src2, "rsi")?;
                let end_lbl = self.new_label("range_end");
                let fill_lbl = self.new_label("range_fill");
                let skip_lbl = self.new_label("range_skip");
                self.push_text(Asm::Push(Reg::Rdi));
                self.push_text(Asm::Push(Reg::Rsi));
                self.push_text(Asm::Mov(Operand::Reg(Reg::Rax), Operand::Reg(Reg::Rsi)));
                self.push_text(Asm::Sub(Operand::Reg(Reg::Rax), Operand::Reg(Reg::Rdi)));
                self.push_text(Asm::Cmp(Operand::Reg(Reg::Rax), Operand::Imm(0)));
                self.push_text(Asm::Jge(skip_lbl.clone()));
                self.push_text(Asm::Xor(Operand::Reg(Reg::Rax), Operand::Reg(Reg::Rax)));
                self.push_text(Asm::Label(skip_lbl));
                self.push_text(Asm::Push(Reg::Rax));
                self.push_text(Asm::Sub(Operand::Reg(Reg::Rsp), Operand::Imm(8)));
                self.push_text(Asm::Lea(
                    Operand::Reg(Reg::Rdi),
                    Operand::Mem(Mem {
                        base: None,
                        index: Some(Reg::Rax),
                        scale: 8,
                        disp: 8,
                        size: None,
                    }),
                ));
                self.push_text(Asm::Call(Operand::PLT("malloc".to_string())));
                self.push_text(Asm::Add(Operand::Reg(Reg::Rsp), Operand::Imm(8)));
                self.push_text(Asm::Pop(Reg::Rdx));
                self.push_text(Asm::Mov(m_base(Reg::Rax), Operand::Reg(Reg::Rdx)));
                self.push_text(Asm::Add(Operand::Reg(Reg::Rax), Operand::Imm(8)));
                self.push_text(Asm::Xor(Operand::Reg(Reg::Rcx), Operand::Reg(Reg::Rcx)));
                self.push_text(Asm::Pop(Reg::Rsi));
                self.push_text(Asm::Pop(Reg::Rdi));
                self.push_text(Asm::Label(fill_lbl.clone()));
                self.push_text(Asm::Cmp(Operand::Reg(Reg::Rcx), Operand::Reg(Reg::Rdx)));
                self.push_text(Asm::Jge(end_lbl.clone()));
                self.push_text(Asm::Mov(Operand::Reg(Reg::R8), Operand::Reg(Reg::Rdi)));
                self.push_text(Asm::Add(Operand::Reg(Reg::R8), Operand::Reg(Reg::Rcx)));
                self.push_text(Asm::Mov(
                    m_sib(Reg::Rax, Reg::Rcx, 8, 0),
                    Operand::Reg(Reg::R8),
                ));
                self.push_text(Asm::Inc(Reg::Rcx));
                self.push_text(Asm::Jmp(fill_lbl));
                self.push_text(Asm::Label(end_lbl));
                self.push_text(Asm::Mov(
                    m_rbp(self.get_offset(dst)?),
                    Operand::Reg(Reg::Rax),
                ));
                self.invalidate_cached_reg("rax");
                self.regs.insert("rax".to_string(), Some(dst.clone()));
                Ok(())
            }
            Op::Arg(n) => {
                let op = code.src1.as_ref().unwrap();
                if n < 6 {
                    let reg = self.arg_reg[n].clone();
                    self.load(op, &reg)?;
                } else {
                    self.load(op, "rax")?;
                    self.push_text(Asm::Push(Reg::Rax));
                }
                Ok(())
            }
            Op::FArg(n) => {
                let op = code.src1.as_ref().unwrap();
                if n < 8 {
                    self.curr_flt_reg = n + 1;
                    let reg = self.flt_arg_reg[n].clone();
                    self.load(op, &reg)?;
                } else {
                    self.curr_flt_reg = 8;
                    self.load(op, "xmm0")?;
                    self.push_text(Asm::Sub(Operand::Reg(Reg::Rsp), Operand::Imm(8)));
                    self.push_text(Asm::Movsd(m_base(Reg::Rsp), Operand::Reg(Reg::Xmm0)));
                }
                Ok(())
            }
            Op::Call => {
                let dst = code.dst.as_ref().unwrap();
                let src1 = code.src1.as_ref().unwrap();
                match src1 {
                    IROperand::Function(name) => {
                        if self.curr_flt_reg > 0 {
                            self.push_text(Asm::Mov(
                                Operand::Reg(Reg::Rax),
                                Operand::Imm(self.curr_flt_reg as i64),
                            ));
                        } else {
                            self.push_text(Asm::Xor(
                                Operand::Reg(Reg::Rax),
                                Operand::Reg(Reg::Rax),
                            ));
                        }
                        self.curr_flt_reg = 0;
                        if self.internals.contains(name) {
                            self.push_text(Asm::Call(Operand::Label(name.clone())));
                        } else {
                            self.push_text(Asm::Call(Operand::PLT(name.clone())));
                        }
                    }
                    _ => {
                        self.load(src1, "rax")?;
                        self.push_text(Asm::Call(Operand::Reg(Reg::Rax)));
                    }
                }
                let caller_saved = ["rax", "rcx", "rdx", "rsi", "rdi", "r8", "r9", "r10", "r11"];
                for reg in caller_saved {
                    self.invalidate_cached_reg(reg);
                }
                // Preserve non-volatile general purpose registers and any xmm values that are not caller-saved.
                self.invalidate_cached_reg("xmm0");
                self.invalidate_cached_reg("xmm1");
                self.invalidate_cached_reg("xmm2");
                self.invalidate_cached_reg("xmm3");
                self.invalidate_cached_reg("xmm4");
                self.invalidate_cached_reg("xmm5");
                self.invalidate_cached_reg("xmm6");
                self.invalidate_cached_reg("xmm7");
                let is_float = match dst {
                    IROperand::Temp(_, IRType::Float) => true,
                    _ => false,
                };
                if is_float {
                    self.push_text(Asm::Movsd(
                        m_rbp(self.get_offset(dst)?),
                        Operand::Reg(Reg::Xmm0),
                    ));
                    self.regs.insert("xmm0".to_string(), Some(dst.clone()));
                } else {
                    self.push_text(Asm::Mov(
                        m_rbp(self.get_offset(dst)?),
                        Operand::Reg(Reg::Rax),
                    ));
                    self.regs.insert("rax".to_string(), Some(dst.clone()));
                }
                Ok(())
            }
            Op::Label(lbl) => {
                self.push_text(Asm::Label(lbl));
                self.regs.clear();
                Ok(())
            }
            Op::Jump => {
                if let IROperand::Label(lbl) = code.src1.as_ref().unwrap() {
                    self.push_text(Asm::Jmp(lbl.clone()));
                }
                Ok(())
            }
            Op::JumpIfFalse => {
                let src1 = code.src1.as_ref().unwrap();
                let lbl = match code.src2.as_ref().unwrap() {
                    IROperand::Label(s) => s.clone(),
                    _ => {
                        return Err(CodeGenError::InvalidOperand {
                            message: "JumpIfFalse src2 must be a Label".to_string(),
                        });
                    }
                };
                self.load(src1, "rax")?;
                self.push_text(Asm::Cmp(Operand::Reg(Reg::Rax), Operand::Imm(0)));
                self.push_text(Asm::Je(lbl));
                Ok(())
            }
            Op::ArrayAccess => {
                let dst = code.dst.as_ref().unwrap();
                let src1 = code.src1.as_ref().unwrap();
                let src2 = code.src2.as_ref().unwrap();
                self.load(src1, "r10")?;
                self.load(src2, "rcx")?;
                self.push_text(Asm::Lea(
                    Operand::Reg(Reg::Rax),
                    m_sib(Reg::R10, Reg::Rcx, 8, 0),
                ));
                self.push_text(Asm::Mov(Operand::Reg(Reg::Rax), m_base(Reg::Rax)));
                self.push_text(Asm::Mov(
                    m_rbp(self.get_offset(dst)?),
                    Operand::Reg(Reg::Rax),
                ));
                self.invalidate_cached_reg("rax");
                self.regs.insert("rax".to_string(), Some(dst.clone()));
                Ok(())
            }
            Op::ArrayAssign => {
                let dst = code.dst.as_ref().unwrap();
                let src1 = code.src1.as_ref().unwrap();
                let src2 = code.src2.as_ref().unwrap();
                self.load(dst, "r10")?;
                self.load(src1, "rcx")?;
                self.load(src2, "rax")?;
                self.push_text(Asm::Lea(
                    Operand::Reg(Reg::Rdx),
                    m_sib(Reg::R10, Reg::Rcx, 8, 0),
                ));
                self.push_text(Asm::Mov(m_base(Reg::Rdx), Operand::Reg(Reg::Rax)));
                Ok(())
            }
            Op::Lea => {
                let dst = code.dst.as_ref().unwrap();
                let src1 = code.src1.as_ref().unwrap();
                let offset = self.get_offset(src1)?;
                self.push_text(Asm::Lea(Operand::Reg(Reg::Rax), m_rbp(offset)));
                self.push_text(Asm::Mov(
                    m_rbp(self.get_offset(dst)?),
                    Operand::Reg(Reg::Rax),
                ));
                self.invalidate_volatile_registers();
                self.regs.insert("rax".to_string(), Some(dst.clone()));
                Ok(())
            }
            Op::Malloc => {
                let dst = code.dst.as_ref().unwrap();
                let src1 = code.src1.as_ref().unwrap();
                self.load(src1, "rdi")?;
                self.push_text(Asm::Call(Operand::PLT("malloc".to_string())));
                self.push_text(Asm::Mov(
                    m_rbp(self.get_offset(dst)?),
                    Operand::Reg(Reg::Rax),
                ));
                self.invalidate_cached_reg("rax");
                self.regs.insert("rax".to_string(), Some(dst.clone()));
                Ok(())
            }
            Op::StoreAt => {
                let dst = code.dst.as_ref().unwrap();
                let src1 = code.src1.as_ref().unwrap();
                let src2 = code.src2.as_ref().unwrap();
                self.load(dst, "r10")?;
                self.load(src2, "rax")?;
                match src1 {
                    IROperand::ConstIdx(idx) => {
                        if let IRConst::Int(offset) = &self.program.constants[*idx] {
                            if *offset == 0 {
                                self.push_text(Asm::Mov(m_base(Reg::R10), Operand::Reg(Reg::Rax)));
                            } else {
                                self.push_text(Asm::Mov(
                                    m_base_disp(Reg::R10, *offset as i32),
                                    Operand::Reg(Reg::Rax),
                                ));
                            }
                        }
                    }
                    _ => {
                        self.load(src1, "r11")?;
                        self.push_text(Asm::Add(Operand::Reg(Reg::R10), Operand::Reg(Reg::R11)));
                        self.push_text(Asm::Mov(m_base(Reg::R10), Operand::Reg(Reg::Rax)));
                    }
                }
                Ok(())
            }
            Op::LoadAt => {
                let dst = code.dst.as_ref().unwrap();
                let src1 = code.src1.as_ref().unwrap();
                let src2 = code.src2.as_ref().unwrap();
                self.load(src1, "r10")?;
                let src_op = match src2 {
                    IROperand::ConstIdx(idx) => {
                        if let IRConst::Int(offset) = &self.program.constants[*idx] {
                            if *offset == 0 {
                                m_base(Reg::R10)
                            } else {
                                m_base_disp(Reg::R10, *offset as i32)
                            }
                        } else {
                            unreachable!()
                        }
                    }
                    _ => {
                        self.load(src2, "r11")?;
                        self.push_text(Asm::Add(Operand::Reg(Reg::R10), Operand::Reg(Reg::R11)));
                        m_base(Reg::R10)
                    }
                };
                self.push_text(Asm::Mov(Operand::Reg(Reg::Rax), src_op));
                self.push_text(Asm::Mov(
                    m_rbp(self.get_offset(dst)?),
                    Operand::Reg(Reg::Rax),
                ));
                self.regs.clear();
                self.regs.insert("rax".to_string(), Some(dst.clone()));
                Ok(())
            }
            Op::StrCat => {
                let dst = code.dst.as_ref().unwrap();
                let src1 = code.src1.as_ref().unwrap();
                let src2 = code.src2.as_ref().unwrap();
                let off_tmp = self.load2temp(src1)?;
                self.push_text(Asm::Mov(Operand::Reg(Reg::Rdi), m_rbp(off_tmp)));
                self.push_text(Asm::Call(Operand::PLT("strlen".to_string())));
                self.push_text(Asm::Mov(Operand::Reg(Reg::Rbx), Operand::Reg(Reg::Rax)));
                let off_tmp2 = self.load2temp(src2)?;
                self.push_text(Asm::Mov(Operand::Reg(Reg::Rdi), m_rbp(off_tmp2)));
                self.push_text(Asm::Call(Operand::PLT("strlen".to_string())));
                self.push_text(Asm::Lea(
                    Operand::Reg(Reg::Rdi),
                    Operand::Mem(Mem {
                        base: Some(Reg::Rbx),
                        index: Some(Reg::Rax),
                        scale: 1,
                        disp: 1,
                        size: None,
                    }),
                ));
                self.push_text(Asm::Call(Operand::PLT("malloc".to_string())));
                self.push_text(Asm::Mov(Operand::Reg(Reg::R15), Operand::Reg(Reg::Rax)));
                self.push_text(Asm::Mov(Operand::Reg(Reg::Rdi), Operand::Reg(Reg::R15)));
                self.push_text(Asm::Mov(Operand::Reg(Reg::Rsi), m_rbp(off_tmp)));
                self.push_text(Asm::Call(Operand::PLT("strcpy".to_string())));
                self.push_text(Asm::Mov(Operand::Reg(Reg::Rdi), Operand::Reg(Reg::R15)));
                self.push_text(Asm::Call(Operand::PLT("strlen".to_string())));
                self.push_text(Asm::Lea(
                    Operand::Reg(Reg::Rdi),
                    Operand::Mem(Mem {
                        base: Some(Reg::R15),
                        index: Some(Reg::Rax),
                        scale: 1,
                        disp: 0,
                        size: None,
                    }),
                ));
                self.push_text(Asm::Mov(Operand::Reg(Reg::Rsi), m_rbp(off_tmp2)));
                self.push_text(Asm::Call(Operand::PLT("strcpy".to_string())));
                self.push_text(Asm::Mov(
                    m_rbp(self.get_offset(dst)?),
                    Operand::Reg(Reg::R15),
                ));
                self.invalidate_volatile_registers();
                self.regs.insert("r15".to_string(), Some(dst.clone()));
                Ok(())
            }
            Op::Return(reg) => {
                if let Some(ref val) = code.src1 {
                    self.load(val, &reg)?;
                }
                self.push_text(Asm::Jmp(self.ret_label.clone()));
                Ok(())
            }
        }
    }

    fn load2temp(&self, op: &IROperand) -> Result<usize, CodeGenError> {
        match op {
            IROperand::Var(_) | IROperand::Temp(_, _) => self.get_offset(op),
            _ => Err(CodeGenError::InvalidOperand {
                message: "load2temp not supported".to_string(),
            }),
        }
    }

    fn compile_fn(&mut self, func: IRFunction) -> Result<(), CodeGenError> {
        if func.is_external {
            self.push_text(Asm::Extern(func.name.clone()));
            return Ok(());
        }

        self.vars.clear();
        self.regs.clear();
        let mut offset = 0;

        for (param, _) in &func.params {
            if let IROperand::Var(name) = param {
                if !self.vars.contains_key(name) {
                    offset += 8;
                    self.vars.insert(name.clone(), offset);
                }
            }
        }
        for inst in &func.instructions {
            let mut register_op = |op_opt: &Option<IROperand>| {
                if let Some(op) = op_opt {
                    match op {
                        IROperand::Var(name) => {
                            if !self.vars.contains_key(name) {
                                offset += 8;
                                self.vars.insert(name.clone(), offset);
                            }
                        }
                        IROperand::Temp(id, _) => {
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
            self.push_text(Asm::Global(func.name.clone()));
        }
        self.push_text(Asm::Label(func.name.clone()));
        self.push_text(Asm::Push(Reg::Rbp));
        self.push_text(Asm::Mov(Operand::Reg(Reg::Rbp), Operand::Reg(Reg::Rsp)));
        if stack_size > 0 {
            self.push_text(Asm::Sub(
                Operand::Reg(Reg::Rsp),
                Operand::Imm(stack_size as i64),
            ));
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
                    self.push_text(Asm::Movsd(m_rbp(off), Operand::Reg(parse_reg(&reg))));
                    self.regs.insert(reg, Some(param.clone()));
                    flt_idx += 1;
                }
            } else {
                if int_idx < 6 {
                    let reg = self.arg_reg[int_idx].clone();
                    self.push_text(Asm::Mov(m_rbp(off), Operand::Reg(parse_reg(&reg))));
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
                    self.push_text(Asm::Jmp(self.ret_label.clone()));
                }
                Op::Label(name) => {
                    self.push_text(Asm::Label(name.clone()));
                    self.regs.clear();
                }
                _ => {
                    self.compile_code(code.clone())?;
                }
            }
        }

        self.push_text(Asm::Label(self.ret_label.clone()));
        self.push_text(Asm::Leave);
        self.push_text(Asm::Ret);
        Ok(())
    }

    fn load(&mut self, op: &IROperand, reg: &str) -> Result<(), CodeGenError> {
        if let Some(Some(cached_op)) = self.regs.get(reg) {
            if cached_op == op {
                return Ok(());
            }
        }

        let dst_reg = parse_reg(reg);
        let is_xmm = dst_reg.is_xmm();
        if let Some((src_reg, _)) = self.regs.iter().find(|(src, cached_op)| {
            *src != reg && cached_op.as_ref() == Some(op) && parse_reg(src).is_xmm() == is_xmm
        }) {
            let src_reg = parse_reg(src_reg);
            if is_xmm {
                self.push_text(Asm::Movsd(Operand::Reg(dst_reg), Operand::Reg(src_reg)));
            } else {
                self.push_text(Asm::Mov(Operand::Reg(dst_reg), Operand::Reg(src_reg)));
            }
            self.regs.insert(reg.to_string(), Some(op.clone()));
            return Ok(());
        }

        match op {
            IROperand::ConstIdx(idx) => {
                let constant = &self.program.constants[*idx];
                match constant {
                    IRConst::Int(v) => {
                        self.push_text(Asm::Mov(Operand::Reg(dst_reg), Operand::Imm(*v)));
                    }
                    IRConst::Float(f) => {
                        let lbl = self.alloc_flt(*f);
                        if dst_reg.is_xmm() {
                            self.push_text(Asm::Movsd(Operand::Reg(dst_reg), rel(lbl)));
                        } else {
                            self.push_text(Asm::Mov(Operand::Reg(dst_reg), rel(lbl)));
                        }
                    }
                    IRConst::Str(s) => {
                        let lbl = self.alloc_str(s.clone());
                        self.push_text(Asm::Lea(Operand::Reg(dst_reg), rel(lbl)));
                    }
                    IRConst::Array(len, arr) => {
                        self.alloc_arr(*len, arr.clone(), reg)?;
                    }
                }
            }
            IROperand::Var(_) | IROperand::Temp(_, _) => {
                let off = self.get_offset(op)?;
                if dst_reg.is_xmm() {
                    self.push_text(Asm::Movsd(Operand::Reg(dst_reg), m_rbp_q(off)));
                } else {
                    self.push_text(Asm::Mov(Operand::Reg(dst_reg), m_rbp(off)));
                }
            }
            IROperand::Function(name) => {
                self.push_text(Asm::Lea(Operand::Reg(dst_reg), rel(name.clone())));
            }
            _ => {}
        }

        self.regs.insert(reg.to_string(), Some(op.clone()));
        Ok(())
    }

    fn invalidate_cached_operand(&mut self, op: &IROperand, keep_reg: Option<&str>) {
        self.regs.retain(|reg, cached| {
            if let Some(cached_op) = cached {
                if cached_op == op && keep_reg != Some(reg.as_str()) {
                    return false;
                }
            }
            true
        });
    }

    fn invalidate_cached_reg(&mut self, reg: &str) {
        self.regs.remove(reg);
    }

    fn invalidate_caller_saved_regs(&mut self) {
        for reg in ["rax", "rcx", "rdx", "rsi", "rdi", "r8", "r9", "r10", "r11"] {
            self.invalidate_cached_reg(reg);
        }
    }

    fn invalidate_caller_saved_xmm(&mut self) {
        for i in 0..16 {
            self.invalidate_cached_reg(&format!("xmm{}", i));
        }
    }

    fn invalidate_volatile_registers(&mut self) {
        self.invalidate_caller_saved_regs();
        self.invalidate_caller_saved_xmm();
    }

    fn store_dst(&mut self, dst: &IROperand, reg: Reg) -> Result<(), CodeGenError> {
        self.invalidate_cached_operand(dst, Some(reg.to_string().as_str()));
        self.push_text(Asm::Mov(m_rbp(self.get_offset(dst)?), Operand::Reg(reg)));
        self.regs.insert(reg.to_string(), Some(dst.clone()));
        Ok(())
    }

    fn store_dst_xmm(&mut self, dst: &IROperand, reg: Reg) -> Result<(), CodeGenError> {
        self.invalidate_cached_operand(dst, Some(reg.to_string().as_str()));
        self.push_text(Asm::Movsd(m_rbp(self.get_offset(dst)?), Operand::Reg(reg)));
        self.regs.insert(reg.to_string(), Some(dst.clone()));
        Ok(())
    }

    fn new_label(&mut self, name: &str) -> String {
        let lbl = format!(".{}_{}_{}", self.curr_fn, name, self.lbl_cnt);
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
            self.push_data(Asm::Label(lbl.clone()));
            if bytes.is_empty() {
                self.push_data(Asm::Db(vec![0]));
            } else {
                let mut db_bytes = bytes.to_vec();
                db_bytes.push(0);
                self.push_data(Asm::Db(db_bytes));
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
            self.push_data(Asm::Label(lbl.clone()));
            self.push_data(Asm::Dq(vec![f.into_inner().to_bits()]));
            lbl
        }
    }

    fn get_offset(&self, op: &IROperand) -> Result<usize, CodeGenError> {
        match op {
            IROperand::Var(name) => self
                .vars
                .get(name)
                .ok_or_else(|| CodeGenError::MissingOperand {
                    message: format!("variable '{}' not found in stack frame", name),
                })
                .map(|v| *v),
            IROperand::Temp(id, _) => {
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

    fn alloc_arr(
        &mut self,
        _len: usize,
        arr: Vec<IROperand>,
        reg: &str,
    ) -> Result<(), CodeGenError> {
        let size = (_len * 8 + 15) & !15;
        let dst_reg = parse_reg(reg);
        self.push_text(Asm::Sub(Operand::Reg(Reg::Rsp), Operand::Imm(size as i64)));
        self.push_text(Asm::Mov(Operand::Reg(Reg::R10), Operand::Reg(Reg::Rsp)));
        for (i, op) in arr.iter().enumerate() {
            self.load(op, "rax")?;
            self.push_text(Asm::Mov(
                m_base_disp(Reg::R10, (i * 8) as i32),
                Operand::Reg(Reg::Rax),
            ));
        }
        self.push_text(Asm::Mov(Operand::Reg(dst_reg), Operand::Reg(Reg::R10)));
        self.regs.clear();
        Ok(())
    }
}
