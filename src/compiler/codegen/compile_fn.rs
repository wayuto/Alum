use super::codegen::{AsmCodeGen, m_rbp, parse_reg};
use super::error::CodeGenError;
use crate::compiler::{
    codegen::asm::*,
    irgen::ir::{IRFunction, IRType, Op, Operand as IROperand},
};

impl AsmCodeGen {
    pub(super) fn compile_fn(&mut self, func: IRFunction) -> Result<(), CodeGenError> {
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
}
