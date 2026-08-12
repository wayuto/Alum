use super::codegen::{AsmCodeGen, m_rbp, parse_reg};
use super::error::CodeGenError;
use super::regalloc;
use crate::compiler::{
    codegen::asm::*,
    irgen::ir::{IRFunction, IRType, Op, Operand as IROperand},
};

impl AsmCodeGen {
    pub(super) fn compile_fn(&mut self, func: IRFunction) -> Result<(), CodeGenError> {
        if func.is_external {
            if !self.internals.contains(&func.name) {
                self.push_text(Asm::Extern(func.name.clone()));
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

        let mut offset = stack_size;
        for (param, _) in &func.params {
            match param {
                IROperand::Var(name) => {
                    if !self.vars.contains_key(name) {
                        offset += 8;
                        self.vars.insert(name.clone(), offset);
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
                                offset += 8;
                                self.vars.insert(name.clone(), offset);
                            }
                        }
                        IROperand::Temp(id, _) => {
                            let temp_key = format!("_tmp_{}", id);
                            if !self.spill_vars.contains_key(&temp_key)
                                && !self.vars.contains_key(&temp_key)
                            {
                                let needs_slot = match self.alloc_regs.get(&temp_key) {
                                    Some(reg) => reg.reg_id() < 12,
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

        if func.is_pub {
            self.push_text(Asm::Global(func.name.clone()));
        }
        self.push_text(Asm::Label(func.name.clone()));
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

        self.curr_fn = func.name.clone();
        self.ret_label = format!(".L_{}_exit", func.name);

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
        for code in insts.iter() {
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
        }

        self.push_text(Asm::Label(self.ret_label.clone()));
        self.push_text(Asm::Mov(Operand::Reg(Reg::Rsp), Operand::Reg(Reg::Rbp)));
        let used_regs_rev: Vec<Reg> = self.used_callee_saved.iter().rev().copied().collect();
        for reg in &used_regs_rev {
            self.push_text(Asm::Pop(*reg));
        }
        self.push_text(Asm::Pop(Reg::Rbp));
        self.push_text(Asm::Ret);
        Ok(())
    }
}
