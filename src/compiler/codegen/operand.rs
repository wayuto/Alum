use super::codegen::AsmCodeGen;
use super::codegen::{m_base_disp, m_rbp, m_rbp_q, parse_reg, rel};
use super::types::CodeGenError;
use crate::compiler::codegen::asm::*;
use crate::compiler::irgen::ir::{IRConst, Operand as IROperand};
use ordered_float::OrderedFloat;

impl AsmCodeGen {
    pub(super) fn load2temp(&self, op: &IROperand) -> Result<usize, CodeGenError> {
        match op {
            IROperand::Var(_) | IROperand::Temp(_, _) => self.get_offset(op),
            _ => Err(CodeGenError::InvalidOperand {
                message: "load2temp not supported".to_string(),
            }),
        }
    }

    pub(super) fn load(&mut self, op: &IROperand, reg: &str) -> Result<(), CodeGenError> {
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

    pub(super) fn invalidate_cached_operand(&mut self, op: &IROperand, keep_reg: Option<&str>) {
        self.regs.retain(|reg, cached| {
            if let Some(cached_op) = cached {
                if cached_op == op && keep_reg != Some(reg.as_str()) {
                    return false;
                }
            }
            true
        });
    }

    pub(super) fn invalidate_cached_reg(&mut self, reg: &str) {
        self.regs.remove(reg);
    }

    pub(super) fn invalidate_caller_saved_regs(&mut self) {
        for reg in ["rax", "rcx", "rdx", "rsi", "rdi", "r8", "r9", "r10", "r11"] {
            self.invalidate_cached_reg(reg);
        }
    }

    pub(super) fn invalidate_caller_saved_xmm(&mut self) {
        for i in 0..16 {
            self.invalidate_cached_reg(&format!("xmm{}", i));
        }
    }

    pub(super) fn invalidate_volatile_registers(&mut self) {
        self.invalidate_caller_saved_regs();
        self.invalidate_caller_saved_xmm();
    }

    pub(super) fn store_dst(&mut self, dst: &IROperand, reg: Reg) -> Result<(), CodeGenError> {
        self.invalidate_cached_operand(dst, Some(reg.to_string().as_str()));
        self.push_text(Asm::Mov(m_rbp(self.get_offset(dst)?), Operand::Reg(reg)));
        self.regs.insert(reg.to_string(), Some(dst.clone()));
        Ok(())
    }

    pub(super) fn store_dst_xmm(&mut self, dst: &IROperand, reg: Reg) -> Result<(), CodeGenError> {
        self.invalidate_cached_operand(dst, Some(reg.to_string().as_str()));
        self.push_text(Asm::Movsd(m_rbp(self.get_offset(dst)?), Operand::Reg(reg)));
        self.regs.insert(reg.to_string(), Some(dst.clone()));
        Ok(())
    }

    pub(super) fn new_label(&mut self, name: &str) -> String {
        let lbl = format!(".{}_{}_{}", self.curr_fn, name, self.lbl_cnt);
        self.lbl_cnt += 1;
        lbl
    }

    pub(super) fn alloc_str(&mut self, s: String) -> String {
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

    pub(super) fn alloc_flt(&mut self, f: OrderedFloat<f64>) -> String {
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

    pub(super) fn get_offset(&self, op: &IROperand) -> Result<usize, CodeGenError> {
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

    pub(super) fn alloc_arr(
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
