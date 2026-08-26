use super::codegen::{
    AsmCodeGen, {m_base, m_base_disp, m_rbp, m_sib, parse_reg, rel},
};
use super::error::CodeGenError;
use crate::compiler::{
    codegen::asm::*,
    irgen::ir::{IRConst, IRType, Instruction, Op, Operand as IROperand},
};

impl AsmCodeGen {
    fn mem_at(&mut self, off: &IROperand, strict: bool) -> Result<Option<Operand>, CodeGenError> {
        if let IROperand::ConstIdx(idx) = off {
            return Ok(match &self.program.constants[*idx] {
                IRConst::Int(offset) => Some(if *offset == 0 {
                    m_base(Reg::R10)
                } else {
                    m_base_disp(Reg::R10, *offset as i32)
                }),
                _ if strict => unreachable!(),
                _ => None,
            });
        }
        self.load(off, Reg::R11)?;
        self.push_text(Asm::Add(Operand::Reg(Reg::R10), Operand::Reg(Reg::R11)));
        Ok(Some(m_base(Reg::R10)))
    }

    pub(super) fn compile_code(&mut self, code: Instruction) -> Result<(), CodeGenError> {
        match code.op {
            Op::Move | Op::Load | Op::Store | Op::GlobLoad => {
                let src = code.src1.as_ref().unwrap();
                let dst = code.dst.as_ref().unwrap();
                self.load(src, Reg::Rax)?;
                self.store_dst(dst, Reg::Rax)?;
                Ok(())
            }
            Op::FMove | Op::FLoad | Op::FStore | Op::FGlobLoad => {
                let src = code.src1.as_ref().unwrap();
                let dst = code.dst.as_ref().unwrap();
                self.load(src, Reg::Xmm0)?;
                self.store_dst_xmm(dst, Reg::Xmm0)?;
                Ok(())
            }
            Op::GlobStore => {
                let src = code.src1.as_ref().unwrap();
                let dst = code.dst.as_ref().unwrap();
                self.load(src, Reg::Rax)?;
                self.store_global(dst, Reg::Rax)?;
                Ok(())
            }
            Op::FGlobStore => {
                let src = code.src1.as_ref().unwrap();
                let dst = code.dst.as_ref().unwrap();
                self.load(src, Reg::Xmm0)?;
                self.store_global_xmm(dst, Reg::Xmm0)?;
                Ok(())
            }
            Op::Add | Op::Sub | Op::Mul | Op::Div | Op::LAnd | Op::LOr | Op::Xor => {
                let dst = code.dst.as_ref().unwrap();
                let src1 = code.src1.as_ref().unwrap();
                let src2 = code.src2.as_ref().unwrap();
                self.load(src1, Reg::Rax)?;
                if matches!(code.op, Op::Div) {
                    self.load(src2, Reg::Rbx)?;
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
                                if *v >= i32::MIN as i64 && *v <= i32::MAX as i64 {
                                    self.push_text(asm_op(
                                        Operand::Reg(Reg::Rax),
                                        Operand::Imm(*v),
                                    ));
                                } else {
                                    self.load(src2, Reg::R10)?;
                                    self.push_text(asm_op(
                                        Operand::Reg(Reg::Rax),
                                        Operand::Reg(Reg::R10),
                                    ));
                                    self.invalidate_cached_reg(Reg::R10);
                                }
                            }
                        }
                        IROperand::Var(_) | IROperand::Temp(_, _) => {
                            let loc = self.get_location(src2)?;
                            self.push_text(asm_op(Operand::Reg(Reg::Rax), loc));
                        }
                        _ => {
                            self.load(src2, Reg::Rbx)?;
                            self.push_text(asm_op(Operand::Reg(Reg::Rax), Operand::Reg(Reg::Rbx)));
                        }
                    }
                }
                self.store_dst(dst, Reg::Rax)?;
                self.regs.remove(&Reg::Rax);
                self.regs.remove(&Reg::Rdx);
                if matches!(code.op, Op::Div) {
                    self.regs.remove(&Reg::Rbx);
                }
                self.regs.insert(Reg::Rax, dst.clone());
                Ok(())
            }
            Op::Shl | Op::Shr => {
                let dst = code.dst.as_ref().unwrap();
                let src1 = code.src1.as_ref().unwrap();
                let src2 = code.src2.as_ref().unwrap();
                self.load(src1, Reg::Rax)?;
                self.load(src2, Reg::Rcx)?;
                if matches!(code.op, Op::Shl) {
                    self.push_text(Asm::Shl(Reg::Rax));
                } else {
                    self.push_text(Asm::Sar(Reg::Rax));
                }
                self.store_dst(dst, Reg::Rax)?;
                self.invalidate_cached_reg(Reg::Rax);
                self.invalidate_cached_reg(Reg::Rcx);
                self.regs.insert(Reg::Rax, dst.clone());
                Ok(())
            }
            Op::Mod => {
                let dst = code.dst.as_ref().unwrap();
                let src1 = code.src1.as_ref().unwrap();
                let src2 = code.src2.as_ref().unwrap();
                self.load(src1, Reg::Rax)?;
                self.load(src2, Reg::Rbx)?;
                self.push_text(Asm::Cqo);
                self.push_text(Asm::Idiv(Reg::Rbx));
                self.store_dst(dst, Reg::Rdx)?;
                self.invalidate_cached_reg(Reg::Rax);
                self.invalidate_cached_reg(Reg::Rdx);
                self.invalidate_cached_reg(Reg::Rbx);
                self.regs.insert(Reg::Rdx, dst.clone());
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
                self.load(src1, Reg::Xmm0)?;
                let src2_op = match src2 {
                    IROperand::ConstIdx(idx) => {
                        if let IRConst::Float(f) = &self.program.constants[*idx] {
                            let lbl = self.alloc_flt(*f);
                            rel(lbl)
                        } else {
                            unreachable!()
                        }
                    }
                    IROperand::Var(_) | IROperand::Temp(_, _) => self.get_location(src2)?,
                    _ => {
                        self.load(src2, Reg::Xmm1)?;
                        Operand::Reg(Reg::Xmm1)
                    }
                };
                self.push_text(asm_op(Operand::Reg(Reg::Xmm0), src2_op));
                self.store_dst_xmm(dst, Reg::Xmm0)?;
                self.regs.insert(Reg::Xmm0, dst.clone());
                Ok(())
            }
            Op::Eq | Op::Ne | Op::Gt | Op::Ge | Op::Lt | Op::Le => {
                let dst = code.dst.as_ref().unwrap();
                let src1 = code.src1.as_ref().unwrap();
                let src2 = code.src2.as_ref().unwrap();
                self.load(src1, Reg::Rax)?;
                self.load(src2, Reg::Rbx)?;
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
                self.push_text(Asm::Movzx(Reg::Rax, Operand::Reg(Reg::Rax)));
                self.store_dst(dst, Reg::Rax)?;
                self.invalidate_cached_reg(Reg::Rax);
                self.regs.insert(Reg::Rax, dst.clone());
                Ok(())
            }
            Op::FEq | Op::FNe | Op::FGt | Op::FGe | Op::FLt | Op::FLe => {
                let dst = code.dst.as_ref().unwrap();
                let src1 = code.src1.as_ref().unwrap();
                let src2 = code.src2.as_ref().unwrap();
                self.load(src1, Reg::Xmm0)?;
                self.load(src2, Reg::Xmm1)?;

                match code.op {
                    Op::FGt => {
                        self.push_text(Asm::Ucomisd(Reg::Xmm0, Reg::Xmm1));
                        self.push_text(Asm::Seta(Reg::Rax));
                    }
                    Op::FGe => {
                        self.push_text(Asm::Ucomisd(Reg::Xmm0, Reg::Xmm1));
                        self.push_text(Asm::Setae(Reg::Rax));
                    }
                    Op::FLt => {
                        self.push_text(Asm::Ucomisd(Reg::Xmm1, Reg::Xmm0));
                        self.push_text(Asm::Seta(Reg::Rax));
                    }
                    Op::FLe => {
                        self.push_text(Asm::Ucomisd(Reg::Xmm1, Reg::Xmm0));
                        self.push_text(Asm::Setae(Reg::Rax));
                    }
                    Op::FEq => {
                        self.push_text(Asm::Ucomisd(Reg::Xmm0, Reg::Xmm1));
                        self.push_text(Asm::Setnp(Reg::Rax));
                        self.push_text(Asm::Sete(Reg::Rcx));
                        self.push_text(Asm::And(Operand::Reg(Reg::Rax), Operand::Reg(Reg::Rcx)));
                    }
                    Op::FNe => {
                        self.push_text(Asm::Ucomisd(Reg::Xmm0, Reg::Xmm1));
                        self.push_text(Asm::Setne(Reg::Rax));
                        self.push_text(Asm::Setp(Reg::Rcx));
                        self.push_text(Asm::Or(Operand::Reg(Reg::Rax), Operand::Reg(Reg::Rcx)));
                    }
                    _ => unreachable!(),
                }
                self.push_text(Asm::Movzx(Reg::Rax, Operand::Reg(Reg::Rax)));
                self.store_dst(dst, Reg::Rax)?;
                self.invalidate_cached_reg(Reg::Rax);
                self.invalidate_cached_reg(Reg::Rcx);
                self.regs.insert(Reg::Rax, dst.clone());
                Ok(())
            }
            Op::StrEq | Op::StrNe | Op::StrLt | Op::StrLe | Op::StrGt | Op::StrGe => {
                let dst = code.dst.as_ref().unwrap();
                let src1 = code.src1.as_ref().unwrap();
                let src2 = code.src2.as_ref().unwrap();
                self.load(src1, Reg::Rdi)?;
                self.load(src2, Reg::Rsi)?;
                self.push_text(Asm::Call(Operand::PLT("strcmp".to_string())));
                self.push_text(Asm::Cdqe);
                self.push_text(Asm::Cmp(Operand::Reg(Reg::Rax), Operand::Imm(0)));
                let set_op = match code.op {
                    Op::StrEq => Asm::Sete,
                    Op::StrNe => Asm::Setne,
                    Op::StrLt => Asm::Setl,
                    Op::StrLe => Asm::Setle,
                    Op::StrGt => Asm::Setg,
                    Op::StrGe => Asm::Setge,
                    _ => unreachable!(),
                };
                self.push_text(set_op(Reg::Rax));
                self.push_text(Asm::Movzx(Reg::Rax, Operand::Reg(Reg::Rax)));
                self.store_dst(dst, Reg::Rax)?;
                self.invalidate_volatile_registers();
                self.regs.insert(Reg::Rax, dst.clone());
                Ok(())
            }
            Op::Neg | Op::Inc | Op::Dec | Op::SizeOf => {
                let dst = code.dst.as_ref().unwrap();
                let src1 = code.src1.as_ref().unwrap();
                self.load(src1, Reg::Rax)?;
                match code.op {
                    Op::Neg => self.push_text(Asm::Neg(Reg::Rax)),
                    Op::Inc => self.push_text(Asm::Inc(Reg::Rax)),
                    Op::Dec => self.push_text(Asm::Dec(Reg::Rax)),
                    Op::SizeOf => {
                        self.push_text(Asm::Mov(Operand::Reg(Reg::Rax), m_base(Reg::Rax)))
                    }
                    _ => unreachable!(),
                }
                self.store_dst(dst, Reg::Rax)?;
                self.invalidate_cached_reg(Reg::Rax);
                self.regs.insert(Reg::Rax, dst.clone());
                Ok(())
            }
            Op::Not => {
                let dst = code.dst.as_ref().unwrap();
                let src1 = code.src1.as_ref().unwrap();
                self.load(src1, Reg::Rax)?;
                self.push_text(Asm::Xor(Operand::Reg(Reg::Rax), Operand::Imm(1)));
                self.store_dst(dst, Reg::Rax)?;
                self.invalidate_cached_reg(Reg::Rax);
                self.regs.insert(Reg::Rax, dst.clone());
                Ok(())
            }
            Op::BNot => {
                let dst = code.dst.as_ref().unwrap();
                let src1 = code.src1.as_ref().unwrap();
                self.load(src1, Reg::Rax)?;
                self.push_text(Asm::Not(Reg::Rax));
                self.store_dst(dst, Reg::Rax)?;
                self.invalidate_cached_reg(Reg::Rax);
                self.regs.insert(Reg::Rax, dst.clone());
                Ok(())
            }
            Op::FNeg => {
                let dst = code.dst.as_ref().unwrap();
                let src1 = code.src1.as_ref().unwrap();
                self.load(src1, Reg::Xmm0)?;
                self.push_text(Asm::Xorpd(Reg::Xmm0, rel("neg_mask".to_string())));
                self.store_dst_xmm(dst, Reg::Xmm0)?;
                self.invalidate_cached_reg(Reg::Xmm0);
                self.regs.insert(Reg::Xmm0, dst.clone());
                Ok(())
            }
            Op::Range => {
                let dst = code.dst.as_ref().unwrap();
                let src1 = code.src1.as_ref().unwrap();
                let src2 = code.src2.as_ref().unwrap();
                self.load(src1, Reg::Rdi)?;
                self.load(src2, Reg::Rsi)?;
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
                    }),
                ));
                self.push_text(Asm::Call(Operand::PLT("malloc".to_string())));
                self.push_text(Asm::Add(Operand::Reg(Reg::Rsp), Operand::Imm(8)));
                self.push_text(Asm::Pop(Reg::Rdx));
                self.push_text(Asm::Mov(m_base(Reg::Rax), Operand::Reg(Reg::Rdx)));
                self.push_text(Asm::Xor(Operand::Reg(Reg::Rcx), Operand::Reg(Reg::Rcx)));
                self.push_text(Asm::Pop(Reg::Rsi));
                self.push_text(Asm::Pop(Reg::Rdi));
                self.push_text(Asm::Label(fill_lbl.clone()));
                self.push_text(Asm::Cmp(Operand::Reg(Reg::Rcx), Operand::Reg(Reg::Rdx)));
                self.push_text(Asm::Jge(end_lbl.clone()));
                self.push_text(Asm::Mov(Operand::Reg(Reg::R8), Operand::Reg(Reg::Rdi)));
                self.push_text(Asm::Add(Operand::Reg(Reg::R8), Operand::Reg(Reg::Rcx)));
                self.push_text(Asm::Mov(
                    m_sib(Reg::Rax, Reg::Rcx, 8, 8),
                    Operand::Reg(Reg::R8),
                ));
                self.push_text(Asm::Inc(Reg::Rcx));
                self.push_text(Asm::Jmp(fill_lbl));
                self.push_text(Asm::Label(end_lbl));
                self.store_dst(dst, Reg::Rax)?;
                self.invalidate_cached_reg(Reg::Rax);
                self.regs.insert(Reg::Rax, dst.clone());
                Ok(())
            }
            Op::Arg(n) => {
                let op = code.src1.as_ref().unwrap();
                if n < 6 {
                    let reg = parse_reg(&self.arg_reg[n]);
                    self.load(op, reg)?;
                } else {
                    let (total, slot) = self
                        .stack_arg_plan
                        .get(&self.cur_inst_idx)
                        .copied()
                        .unwrap_or((1, 0));
                    if slot == 0 {
                        let bytes = (total * 8 + if total % 2 == 1 { 8 } else { 0 }) as i64;
                        self.call_stack_bytes = bytes as usize;
                        self.push_text(Asm::Sub(Operand::Reg(Reg::Rsp), Operand::Imm(bytes)));
                    }
                    self.load(op, Reg::Rax)?;
                    self.push_text(Asm::Mov(
                        m_base_disp(Reg::Rsp, (slot * 8) as i32),
                        Operand::Reg(Reg::Rax),
                    ));
                }
                Ok(())
            }
            Op::FArg(n) => {
                let op = code.src1.as_ref().unwrap();
                if n < 8 {
                    self.curr_flt_reg = n + 1;
                    let reg = parse_reg(&self.flt_arg_reg[n]);
                    self.load(op, reg)?;
                } else {
                    self.curr_flt_reg = 8;

                    let (total, slot) = self
                        .stack_arg_plan
                        .get(&self.cur_inst_idx)
                        .copied()
                        .unwrap_or((1, 0));
                    if slot == 0 {
                        let bytes = (total * 8 + if total % 2 == 1 { 8 } else { 0 }) as i64;
                        self.call_stack_bytes = bytes as usize;
                        self.push_text(Asm::Sub(Operand::Reg(Reg::Rsp), Operand::Imm(bytes)));
                    }
                    self.load(op, Reg::Xmm0)?;
                    self.push_text(Asm::Movsd(
                        m_base_disp(Reg::Rsp, (slot * 8) as i32),
                        Operand::Reg(Reg::Xmm0),
                    ));
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
                            let sym = self
                                .extern_link
                                .get(name)
                                .cloned()
                                .unwrap_or_else(|| name.clone());
                            self.push_text(Asm::Call(Operand::PLT(sym)));
                        }
                    }
                    _ => {
                        self.load(src1, Reg::Rax)?;
                        self.push_text(Asm::Call(Operand::Reg(Reg::Rax)));
                    }
                }

                if self.call_stack_bytes > 0 {
                    self.push_text(Asm::Add(
                        Operand::Reg(Reg::Rsp),
                        Operand::Imm(self.call_stack_bytes as i64),
                    ));
                    self.call_stack_bytes = 0;
                }
                self.invalidate_volatile_registers();
                let is_float = match dst {
                    IROperand::Temp(_, IRType::Float) => true,
                    _ => false,
                };
                if is_float {
                    self.store_dst_xmm(dst, Reg::Xmm0)?;
                    self.regs.insert(Reg::Xmm0, dst.clone());
                } else {
                    self.store_dst(dst, Reg::Rax)?;
                    self.regs.insert(Reg::Rax, dst.clone());
                }
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
                self.load(src1, Reg::Rax)?;
                self.push_text(Asm::Cmp(Operand::Reg(Reg::Rax), Operand::Imm(0)));
                self.push_text(Asm::Je(lbl));
                Ok(())
            }
            Op::JumpIfTrue => {
                let src1 = code.src1.as_ref().unwrap();
                let lbl = match code.src2.as_ref().unwrap() {
                    IROperand::Label(s) => s.clone(),
                    _ => {
                        return Err(CodeGenError::InvalidOperand {
                            message: "JumpIfTrue src2 must be a Label".to_string(),
                        });
                    }
                };
                self.load(src1, Reg::Rax)?;
                self.push_text(Asm::Cmp(Operand::Reg(Reg::Rax), Operand::Imm(1)));
                self.push_text(Asm::Je(lbl));
                Ok(())
            }
            Op::ArrayAccess => {
                let dst = code.dst.as_ref().unwrap();
                let src1 = code.src1.as_ref().unwrap();
                let src2 = code.src2.as_ref().unwrap();
                self.load(src1, Reg::R10)?;
                self.load(src2, Reg::Rcx)?;
                self.push_text(Asm::Lea(
                    Operand::Reg(Reg::Rax),
                    m_sib(Reg::R10, Reg::Rcx, 8, 8),
                ));
                self.push_text(Asm::Mov(Operand::Reg(Reg::Rax), m_base(Reg::Rax)));
                self.store_dst(dst, Reg::Rax)?;
                self.invalidate_cached_reg(Reg::Rax);
                self.regs.insert(Reg::Rax, dst.clone());
                Ok(())
            }
            Op::ArrayAssign => {
                let dst = code.dst.as_ref().unwrap();
                let src1 = code.src1.as_ref().unwrap();
                let src2 = code.src2.as_ref().unwrap();
                self.load(dst, Reg::R10)?;
                self.load(src1, Reg::Rcx)?;
                self.load(src2, Reg::Rax)?;
                self.push_text(Asm::Lea(
                    Operand::Reg(Reg::Rdx),
                    m_sib(Reg::R10, Reg::Rcx, 8, 8),
                ));
                self.push_text(Asm::Mov(m_base(Reg::Rdx), Operand::Reg(Reg::Rax)));
                Ok(())
            }
            Op::StrByte => {
                let dst = code.dst.as_ref().unwrap();
                let src1 = code.src1.as_ref().unwrap();
                let src2 = code.src2.as_ref().unwrap();
                self.load(src1, Reg::R10)?;
                self.load(src2, Reg::R11)?;
                self.push_text(Asm::Movzx(Reg::Rax, m_sib(Reg::R10, Reg::R11, 1, 0)));
                self.push_text(Asm::Mov(Operand::Reg(Reg::R15), Operand::Reg(Reg::Rax)));
                self.push_text(Asm::Mov(Operand::Reg(Reg::Rdi), Operand::Imm(2)));
                self.push_text(Asm::Call(Operand::PLT("malloc".to_string())));
                self.push_text(Asm::Movb(m_base(Reg::Rax), Reg::R15));
                self.push_text(Asm::Mov(Operand::Reg(Reg::Rdi), Operand::Imm(0)));
                self.push_text(Asm::Movb(m_base_disp(Reg::Rax, 1), Reg::Rdi));
                self.store_dst(dst, Reg::Rax)?;
                self.invalidate_volatile_registers();
                self.regs.insert(Reg::Rax, dst.clone());
                Ok(())
            }
            Op::ByteAccess => {
                let dst = code.dst.as_ref().unwrap();
                let src1 = code.src1.as_ref().unwrap();
                let src2 = code.src2.as_ref().unwrap();
                self.load(src1, Reg::R10)?;
                self.load(src2, Reg::Rcx)?;
                self.push_text(Asm::Lea(
                    Operand::Reg(Reg::Rax),
                    m_sib(Reg::R10, Reg::Rcx, 1, 0),
                ));
                self.push_text(Asm::Movzx(Reg::Rax, m_base(Reg::Rax)));
                self.store_dst(dst, Reg::Rax)?;
                self.invalidate_cached_reg(Reg::Rax);
                self.regs.insert(Reg::Rax, dst.clone());
                Ok(())
            }
            Op::ByteAssign => {
                let dst = code.dst.as_ref().unwrap();
                let src1 = code.src1.as_ref().unwrap();
                let src2 = code.src2.as_ref().unwrap();
                self.load(dst, Reg::R10)?;
                self.load(src1, Reg::Rcx)?;
                self.load(src2, Reg::Rax)?;
                self.push_text(Asm::Lea(
                    Operand::Reg(Reg::Rdx),
                    m_sib(Reg::R10, Reg::Rcx, 1, 0),
                ));
                self.push_text(Asm::Movb(m_base(Reg::Rdx), Reg::Rax));
                Ok(())
            }
            Op::Lea => {
                let dst = code.dst.as_ref().unwrap();
                let src1 = code.src1.as_ref().unwrap();
                if let IROperand::Global(name) = src1 {
                    self.push_text(Asm::Lea(Operand::Reg(Reg::Rax), rel(name.clone())));
                } else {
                    let offset = self.get_offset(src1)?;
                    self.push_text(Asm::Lea(Operand::Reg(Reg::Rax), m_rbp(offset)));
                }
                self.store_dst(dst, Reg::Rax)?;
                self.invalidate_volatile_registers();
                self.regs.insert(Reg::Rax, dst.clone());
                Ok(())
            }
            Op::Malloc => {
                let dst = code.dst.as_ref().unwrap();
                let src1 = code.src1.as_ref().unwrap();
                self.load(src1, Reg::Rdi)?;
                self.push_text(Asm::Call(Operand::PLT("malloc".to_string())));
                self.store_dst(dst, Reg::Rax)?;
                self.invalidate_cached_reg(Reg::Rax);
                self.regs.insert(Reg::Rax, dst.clone());
                Ok(())
            }
            Op::Free => {
                let src1 = code.src1.as_ref().unwrap();
                self.load(src1, Reg::Rdi)?;
                self.push_text(Asm::Call(Operand::PLT("free".to_string())));
                self.invalidate_volatile_registers();
                self.invalidate_cached_reg(Reg::Rdi);
                Ok(())
            }
            Op::StoreAt => {
                let dst = code.dst.as_ref().unwrap();
                let src1 = code.src1.as_ref().unwrap();
                let src2 = code.src2.as_ref().unwrap();
                self.load(dst, Reg::R10)?;
                self.load(src2, Reg::Rax)?;
                if let Some(addr) = self.mem_at(src1, false)? {
                    self.push_text(Asm::Mov(addr, Operand::Reg(Reg::Rax)));
                }
                Ok(())
            }
            Op::LoadAt => {
                let dst = code.dst.as_ref().unwrap();
                let src1 = code.src1.as_ref().unwrap();
                let src2 = code.src2.as_ref().unwrap();
                self.load(src1, Reg::R10)?;
                let src_op = self.mem_at(src2, true)?.unwrap();
                self.push_text(Asm::Mov(Operand::Reg(Reg::Rax), src_op));
                self.store_dst(dst, Reg::Rax)?;
                self.regs.clear();
                self.regs.insert(Reg::Rax, dst.clone());
                Ok(())
            }
            Op::FStoreAt => {
                let dst = code.dst.as_ref().unwrap();
                let src1 = code.src1.as_ref().unwrap();
                let src2 = code.src2.as_ref().unwrap();
                self.load(dst, Reg::R10)?;
                self.load(src2, Reg::Xmm0)?;
                let addr = self.mem_at(src1, true)?.unwrap();
                self.push_text(Asm::Movsd(addr, Operand::Reg(Reg::Xmm0)));
                self.invalidate_cached_reg(Reg::Xmm0);
                Ok(())
            }
            Op::FLoadAt => {
                let dst = code.dst.as_ref().unwrap();
                let src1 = code.src1.as_ref().unwrap();
                let src2 = code.src2.as_ref().unwrap();
                self.load(src1, Reg::R10)?;
                let src_op = self.mem_at(src2, true)?.unwrap();
                self.push_text(Asm::Movsd(Operand::Reg(Reg::Xmm0), src_op));
                self.store_dst_xmm(dst, Reg::Xmm0)?;
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
                    }),
                ));
                self.push_text(Asm::Mov(Operand::Reg(Reg::Rsi), m_rbp(off_tmp2)));
                self.push_text(Asm::Call(Operand::PLT("strcpy".to_string())));
                self.store_dst(dst, Reg::R15)?;
                self.invalidate_volatile_registers();
                self.regs.insert(Reg::R15, dst.clone());
                Ok(())
            }
            Op::IntToFloat => {
                let dst = code.dst.as_ref().unwrap();
                let src = code.src1.as_ref().unwrap();
                self.load(src, Reg::Rax)?;
                self.push_text(Asm::Cvtsi2sd(Reg::Xmm0, Operand::Reg(Reg::Rax)));
                self.store_dst_xmm(dst, Reg::Xmm0)?;
                self.invalidate_cached_reg(Reg::Rax);
                self.invalidate_cached_reg(Reg::Xmm0);
                self.regs.insert(Reg::Xmm0, dst.clone());
                Ok(())
            }
            Op::FloatToInt => {
                let dst = code.dst.as_ref().unwrap();
                let src = code.src1.as_ref().unwrap();
                self.load(src, Reg::Xmm0)?;
                self.push_text(Asm::Cvttsd2si(Reg::Rax, Operand::Reg(Reg::Xmm0)));
                self.store_dst(dst, Reg::Rax)?;
                self.invalidate_cached_reg(Reg::Rax);
                self.invalidate_cached_reg(Reg::Xmm0);
                self.regs.insert(Reg::Rax, dst.clone());
                Ok(())
            }
            Op::Return(_) | Op::Label(_) => unreachable!("handled by compile_fn"),
        }
    }
}
