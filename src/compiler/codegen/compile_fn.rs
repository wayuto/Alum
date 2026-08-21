use super::codegen::{AsmCodeGen, m_rbp, parse_reg};
use super::error::CodeGenError;
use super::regalloc;
use crate::compiler::{
    codegen::asm::*,
    irgen::ir::{IRFunction, IRType, Instruction, Op, Operand as IROperand},
};
use std::collections::HashMap;

#[derive(Clone, Copy)]
enum Jcc {
    Je,
    Jne,
    Jl,
    Jle,
    Jg,
    Jge,
    Ja,
    Jae,
    Jb,
    Jbe,
}

impl Jcc {
    fn emit(self, asm_gen: &mut AsmCodeGen, label: &str) {
        let asm = match self {
            Jcc::Je => Asm::Je(label.to_string()),
            Jcc::Jne => Asm::Jne(label.to_string()),
            Jcc::Jl => Asm::Jl(label.to_string()),
            Jcc::Jle => Asm::Jle(label.to_string()),
            Jcc::Jg => Asm::Jg(label.to_string()),
            Jcc::Jge => Asm::Jge(label.to_string()),
            Jcc::Ja => Asm::Ja(label.to_string()),
            Jcc::Jae => Asm::Jae(label.to_string()),
            Jcc::Jb => Asm::Jb(label.to_string()),
            Jcc::Jbe => Asm::Jbe(label.to_string()),
        };
        asm_gen.push_text(asm);
    }
}

fn cmp_to_jcc(op: &Op, jump_if_true: bool) -> Option<Jcc> {
    let (t, f) = match op {
        Op::Eq => (Jcc::Je, Jcc::Jne),
        Op::Ne => (Jcc::Jne, Jcc::Je),
        Op::Lt => (Jcc::Jl, Jcc::Jge),
        Op::Le => (Jcc::Jle, Jcc::Jg),
        Op::Gt => (Jcc::Jg, Jcc::Jle),
        Op::Ge => (Jcc::Jge, Jcc::Jl),
        Op::FEq => (Jcc::Je, Jcc::Jne),
        Op::FNe => (Jcc::Jne, Jcc::Je),
        Op::FLt => (Jcc::Jb, Jcc::Jae),
        Op::FLe => (Jcc::Jbe, Jcc::Ja),
        Op::FGt => (Jcc::Ja, Jcc::Jbe),
        Op::FGe => (Jcc::Jae, Jcc::Jb),
        _ => return None,
    };
    Some(if jump_if_true { t } else { f })
}

impl AsmCodeGen {
    pub(super) fn compile_fn(&mut self, func: IRFunction) -> Result<(), CodeGenError> {
        if func.is_external {
            if !self.internals.contains(&func.name) {
                let sym = self
                    .extern_link
                    .get(&func.name)
                    .cloned()
                    .unwrap_or_else(|| func.name.clone());
                self.push_text(Asm::Extern(sym));
            }
            return Ok(());
        }

        self.vars.clear();
        self.alloc_regs.clear();
        self.spill_vars.clear();
        self.regs.clear();

        let alloc = regalloc::allocate_registers(&func, &self.program.constants);
        self.alloc_regs = alloc.registers;
        self.spill_vars = alloc.spill_offsets;
        self.used_callee_saved = alloc.used_callee_saved;
        let stack_size = alloc.stack_size;

        let is_leaf = !func.instructions.iter().any(|i| {
            matches!(
                i.op,
                Op::Call
                    | Op::Malloc
                    | Op::Free
                    | Op::StrCat
                    | Op::StrByte
                    | Op::Range
                    | Op::StrEq
                    | Op::StrNe
                    | Op::StrLt
                    | Op::StrLe
                    | Op::StrGt
                    | Op::StrGe
            )
        });
        let mut lea_used: std::collections::HashSet<String> = std::collections::HashSet::new();
        for inst in &func.instructions {
            if matches!(inst.op, Op::Lea) {
                if let Some(IROperand::Var(name)) = &inst.src1 {
                    lea_used.insert(name.clone());
                }
            }
        }

        let mut offset = stack_size;
        for (param, _) in &func.params {
            match param {
                IROperand::Var(name) => {
                    if !self.vars.contains_key(name) {
                        let needs_slot = !is_leaf
                            || !self.alloc_regs.contains_key(name)
                            || lea_used.contains(name);
                        if needs_slot {
                            offset += 8;
                            self.vars.insert(name.clone(), offset);
                        }
                    }
                }
                _ => {}
            }
        }
        for inst in &func.instructions {
            let mut register_op = |op_opt: &Option<IROperand>| {
                if let Some(op) = op_opt {
                    match op {
                        IROperand::Var(name) => {
                            if !self.vars.contains_key(name) {
                                let needs_slot = !is_leaf
                                    || !self.alloc_regs.contains_key(name)
                                    || lea_used.contains(name);
                                if needs_slot {
                                    offset += 8;
                                    self.vars.insert(name.clone(), offset);
                                }
                            }
                        }
                        IROperand::Temp(id, _) => {
                            let temp_key = format!("_tmp_{}", id);
                            if !self.spill_vars.contains_key(&temp_key)
                                && !self.vars.contains_key(&temp_key)
                            {
                                let needs_slot = match self.alloc_regs.get(&temp_key) {
                                    Some(reg) => !is_leaf && reg.reg_id() < 12,
                                    None => true,
                                };
                                if needs_slot {
                                    offset += 8;
                                    self.vars.insert(temp_key, offset);
                                }
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

        for inst in &func.instructions {
            if matches!(inst.op, Op::StrCat) {
                for src in [&inst.src1, &inst.src2] {
                    if let Some(IROperand::Temp(id, _)) = src {
                        let temp_key = format!("_tmp_{}", id);
                        if !self.vars.contains_key(&temp_key) {
                            offset += 8;
                            self.vars.insert(temp_key, offset);
                        }
                    }
                }
            }
        }

        let frame_size = ((offset + 15) & !15).max(stack_size);
        let final_stack_size = if frame_size > 0 {
            (frame_size + 15) & !15
        } else {
            0
        };
        let xmm_saved = alloc.xmm_saved;
        let frame_less =
            final_stack_size == 0 && xmm_saved.is_empty() && self.used_callee_saved.is_empty();

        if func.is_pub {
            self.push_text(Asm::Global(func.name.clone()));
        }
        for alias in &func.aliases {
            self.push_text(Asm::Global(alias.clone()));
            self.push_text(Asm::Label(alias.clone()));
        }
        self.push_text(Asm::Label(func.name.clone()));
        if !frame_less {
            self.push_text(Asm::Push(Reg::Rbp));
            let used_regs = self.used_callee_saved.clone();
            for reg in &used_regs {
                self.push_text(Asm::Push(*reg));
            }
            self.push_text(Asm::Mov(Operand::Reg(Reg::Rbp), Operand::Reg(Reg::Rsp)));

            if final_stack_size > 0 {
                self.push_text(Asm::Sub(
                    Operand::Reg(Reg::Rsp),
                    Operand::Imm(final_stack_size as i64),
                ));
            }
        }

        self.curr_fn = func.name.clone();
        self.ret_label = format!(".L_{}_exit", func.name);

        if !frame_less {
            for (reg, off) in &xmm_saved {
                self.push_text(Asm::Movsd(m_rbp(*off), Operand::Reg(*reg)));
            }
        }

        let alloc_regs = self.alloc_regs.clone();
        let mut int_idx = 0;
        let mut flt_idx = 0;
        for (param, ty) in &func.params {
            let k = param.key();
            if let Some(alloc_reg) = alloc_regs.get(&k) {
                if matches!(ty, IRType::Float) {
                    if flt_idx < 8 {
                        let arg_reg = parse_reg(&format!("xmm{}", flt_idx));
                        if *alloc_reg != arg_reg {
                            self.push_text(Asm::Movsd(
                                Operand::Reg(*alloc_reg),
                                Operand::Reg(arg_reg),
                            ));
                        }
                        self.regs.insert(*alloc_reg, param.clone());
                        flt_idx += 1;
                    }
                } else {
                    if int_idx < 6 {
                        let arg_reg = parse_reg(&self.arg_reg[int_idx]);
                        if *alloc_reg != arg_reg {
                            self.push_text(Asm::Mov(
                                Operand::Reg(*alloc_reg),
                                Operand::Reg(arg_reg),
                            ));
                        }
                        self.regs.insert(*alloc_reg, param.clone());
                        int_idx += 1;
                    }
                }
            } else {
                let off = self.get_offset(param)?;
                if matches!(ty, IRType::Float) {
                    if flt_idx < 8 {
                        let arg_reg = parse_reg(&format!("xmm{}", flt_idx));
                        self.push_text(Asm::Movsd(m_rbp(off), Operand::Reg(arg_reg)));
                        self.regs.insert(arg_reg, param.clone());
                        flt_idx += 1;
                    }
                } else {
                    if int_idx < 6 {
                        let arg_reg = parse_reg(&self.arg_reg[int_idx]);
                        self.push_text(Asm::Mov(m_rbp(off), Operand::Reg(arg_reg)));
                        self.regs.insert(arg_reg, param.clone());
                        int_idx += 1;
                    }
                }
            }
        }

        let insts = &func.instructions;

        let mut temp_use_count: HashMap<usize, usize> = HashMap::new();
        for inst in insts.iter() {
            for op in [inst.src1.as_ref(), inst.src2.as_ref()]
                .into_iter()
                .flatten()
            {
                if let IROperand::Temp(id, _) = op {
                    *temp_use_count.entry(*id).or_insert(0) += 1;
                }
            }
        }

        let mut i = 0;
        while i < insts.len() {
            let code = &insts[i];
            if self.try_fuse_compare_jump(insts, i, &temp_use_count)? {
                i += 2;
                continue;
            }
            match &code.op {
                Op::Return(reg_name) => {
                    if let Some(ref val) = code.src1 {
                        self.load(val, parse_reg(reg_name))?;
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
            i += 1;
        }

        self.push_text(Asm::Label(self.ret_label.clone()));
        if frame_less {
            self.push_text(Asm::Ret);
        } else {
            for (reg, off) in xmm_saved.iter().rev() {
                self.push_text(Asm::Movsd(Operand::Reg(*reg), m_rbp(*off)));
            }
            self.push_text(Asm::Mov(Operand::Reg(Reg::Rsp), Operand::Reg(Reg::Rbp)));
            let used_regs_rev: Vec<Reg> = self.used_callee_saved.iter().rev().copied().collect();
            for reg in &used_regs_rev {
                self.push_text(Asm::Pop(*reg));
            }
            self.push_text(Asm::Pop(Reg::Rbp));
            self.push_text(Asm::Ret);
        }
        Ok(())
    }

    fn try_fuse_compare_jump(
        &mut self,
        insts: &[Instruction],
        i: usize,
        temp_use_count: &HashMap<usize, usize>,
    ) -> Result<bool, CodeGenError> {
        let cmp = &insts[i];
        let Some(IROperand::Temp(tid, _)) = &cmp.dst else {
            return Ok(false);
        };
        if temp_use_count.get(tid).copied().unwrap_or(0) != 1 {
            return Ok(false);
        }
        let Some(jmp) = insts.get(i + 1) else {
            return Ok(false);
        };
        if !matches!(&jmp.src1, Some(IROperand::Temp(t, _)) if t == tid) {
            return Ok(false);
        }
        let (jump_if_true, label) = match &jmp.op {
            Op::JumpIfTrue => (true, jmp_label(jmp)?),
            Op::JumpIfFalse => (false, jmp_label(jmp)?),
            _ => return Ok(false),
        };
        let Some(a) = &cmp.src1 else {
            return Ok(false);
        };
        let Some(b) = &cmp.src2 else {
            return Ok(false);
        };
        let Some(cc) = cmp_to_jcc(&cmp.op, jump_if_true) else {
            return Ok(false);
        };

        if matches!(
            &cmp.op,
            Op::FEq | Op::FNe | Op::FGt | Op::FGe | Op::FLt | Op::FLe
        ) {
            self.load(a, Reg::Xmm0)?;
            self.load(b, Reg::Xmm1)?;
            self.push_text(Asm::Ucomisd(Reg::Xmm0, Reg::Xmm1));
            self.invalidate_cached_reg(Reg::Xmm0);
            self.invalidate_cached_reg(Reg::Xmm1);
        } else {
            self.load(a, Reg::Rax)?;
            self.load(b, Reg::Rbx)?;
            self.push_text(Asm::Cmp(Operand::Reg(Reg::Rax), Operand::Reg(Reg::Rbx)));
            self.invalidate_cached_reg(Reg::Rax);
            self.invalidate_cached_reg(Reg::Rbx);
        }
        cc.emit(self, &label);
        Ok(true)
    }
}

fn jmp_label(jmp: &Instruction) -> Result<String, CodeGenError> {
    match &jmp.src2 {
        Some(IROperand::Label(l)) => Ok(l.clone()),
        _ => Err(CodeGenError::InvalidOperand {
            message: "conditional jump requires a label target".to_string(),
        }),
    }
}
