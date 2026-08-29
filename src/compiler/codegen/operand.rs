use super::codegen::{AsmCodeGen, m_base, m_base_disp, m_rbp, m_rbp_q, rel};
use super::error::CodeGenError;
use crate::compiler::{
    codegen::asm::*,
    irgen::ir::{IRConst, Operand as IROperand},
};
use ordered_float::OrderedFloat;

impl AsmCodeGen {
    pub(super) fn load2temp(&mut self, op: &IROperand) -> Result<usize, CodeGenError> {
        match op {
            IROperand::Var(_) | IROperand::Temp(_, _) => {
                let k = op.key();
                if let Some(reg) = self.alloc_regs.get(&k) {
                    let off = self.get_offset(op)?;
                    self.push_text(Asm::Mov(m_rbp(off), Operand::Reg(*reg)));
                }
                self.get_offset(op)
            }
            _ => Err(CodeGenError::InvalidOperand {
                message: "load2temp not supported".to_string(),
            }),
        }
    }

    pub(super) fn get_location(&mut self, op: &IROperand) -> Result<Operand, CodeGenError> {
        match op {
            IROperand::ConstIdx(idx) => {
                let constant = &self.program.constants[*idx];
                match constant {
                    IRConst::Int(v) => Ok(Operand::Imm(*v)),
                    IRConst::Float(f) => {
                        let lbl = self.alloc_flt(*f);
                        Ok(rel(lbl))
                    }
                    IRConst::Str(s) => {
                        let lbl = self.alloc_str(s.clone());
                        Ok(rel(lbl))
                    }
                    IRConst::Array(_) => Err(CodeGenError::InvalidOperand {
                        message: "array constant as location".to_string(),
                    }),
                }
            }
            IROperand::Var(_) | IROperand::Temp(_, _) => {
                let k = op.key();
                if let Some(reg) = self.alloc_regs.get(&k) {
                    Ok(Operand::Reg(*reg))
                } else if let Some(off) = self.spill_vars.get(&k) {
                    Ok(m_rbp_q(*off))
                } else {
                    let off = self.get_offset(op)?;
                    Ok(m_rbp_q(off))
                }
            }
            IROperand::Function(name) => Ok(rel(name.clone())),
            IROperand::Label(name) => Ok(Operand::Label(name.clone())),
            IROperand::Global(name) => {
                self.invalidate_cached_reg(Reg::R10);
                self.push_text(Asm::Mov(Operand::Reg(Reg::R10), rel(name.clone())));
                Ok(m_base(Reg::R10))
            }
        }
    }

    pub(super) fn load(&mut self, op: &IROperand, reg: Reg) -> Result<(), CodeGenError> {
        if let Some(cached_op) = self.regs.get(&reg) {
            if cached_op == op {
                return Ok(());
            }
        }

        let dst_reg = reg;
        let is_xmm = dst_reg.is_xmm();

        if let Some((&src_reg, _)) = self
            .regs
            .iter()
            .find(|(src, cached_op)| *src != &reg && **cached_op == *op && src.is_xmm() == is_xmm)
        {
            if is_xmm {
                self.push_text(Asm::Movsd(Operand::Reg(dst_reg), Operand::Reg(src_reg)));
            } else {
                self.push_text(Asm::Mov(Operand::Reg(dst_reg), Operand::Reg(src_reg)));
            }
            self.regs.insert(reg, op.clone());
            return Ok(());
        }

        if matches!(op, IROperand::Var(_) | IROperand::Temp(_, _)) {
            let k = op.key();
            if let Some(alloc_reg) = self.alloc_regs.get(&k) {
                if *alloc_reg == dst_reg {
                    self.regs.insert(reg, op.clone());
                    return Ok(());
                }
                if is_xmm {
                    self.push_text(Asm::Movsd(Operand::Reg(dst_reg), Operand::Reg(*alloc_reg)));
                } else {
                    self.push_text(Asm::Mov(Operand::Reg(dst_reg), Operand::Reg(*alloc_reg)));
                }
                self.regs.insert(reg, op.clone());
                return Ok(());
            }
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
                            self.push_text(Asm::Mov(Operand::Reg(dst_reg), m_base(dst_reg)));
                        }
                    }
                    IRConst::Str(s) => {
                        let lbl = self.alloc_str(s.clone());
                        self.push_text(Asm::Lea(Operand::Reg(dst_reg), rel(lbl)));
                    }
                    IRConst::Array(arr) => {
                        self.alloc_arr(arr.len(), arr.clone(), reg)?;
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
            IROperand::Global(name) => {
                if is_xmm {
                    self.push_text(Asm::Mov(Operand::Reg(Reg::R10), rel(name.clone())));
                    self.push_text(Asm::Movsd(Operand::Reg(dst_reg), m_base(Reg::R10)));
                    self.invalidate_cached_reg(Reg::R10);
                } else {
                    self.push_text(Asm::Mov(Operand::Reg(dst_reg), rel(name.clone())));
                    self.push_text(Asm::Mov(Operand::Reg(dst_reg), m_base(dst_reg)));
                }
                return Ok(());
            }
            _ => {}
        }

        self.regs.insert(reg, op.clone());
        Ok(())
    }

    pub(super) fn invalidate_cached_operand(&mut self, op: &IROperand, keep_reg: Option<Reg>) {
        self.regs
            .retain(|reg, cached| !(cached == op && Some(*reg) != keep_reg));
    }

    pub(super) fn invalidate_cached_reg(&mut self, reg: Reg) {
        self.regs.remove(&reg);
    }

    pub(super) fn invalidate_caller_saved_regs(&mut self) {
        for reg in &[
            Reg::Rax,
            Reg::Rcx,
            Reg::Rdx,
            Reg::Rsi,
            Reg::Rdi,
            Reg::R8,
            Reg::R9,
            Reg::R10,
            Reg::R11,
        ] {
            self.invalidate_cached_reg(*reg);
        }

        self.alloc_regs
            .retain(|_, reg| reg.is_xmm() || !reg.is_caller_saved_gp());
    }

    pub(super) fn invalidate_caller_saved_xmm(&mut self) {
        for reg in &[
            Reg::Xmm0,
            Reg::Xmm1,
            Reg::Xmm2,
            Reg::Xmm3,
            Reg::Xmm4,
            Reg::Xmm5,
            Reg::Xmm6,
            Reg::Xmm7,
        ] {
            self.invalidate_cached_reg(*reg);
        }
    }

    pub(super) fn invalidate_volatile_registers(&mut self) {
        self.invalidate_caller_saved_regs();
        self.invalidate_caller_saved_xmm();
    }

    pub(super) fn store_dst(&mut self, dst: &IROperand, reg: Reg) -> Result<(), CodeGenError> {
        self.store_dst_inner(dst, reg, false)
    }

    pub(super) fn store_dst_xmm(&mut self, dst: &IROperand, reg: Reg) -> Result<(), CodeGenError> {
        self.store_dst_inner(dst, reg, true)
    }

    fn store_dst_inner(
        &mut self,
        dst: &IROperand,
        reg: Reg,
        is_float: bool,
    ) -> Result<(), CodeGenError> {
        let mov = |to, from| {
            if is_float {
                Asm::Movsd(to, from)
            } else {
                Asm::Mov(to, from)
            }
        };
        let k = dst.key();
        let alloc_reg = self.alloc_regs.get(&k).copied();
        if let Some(alloc_reg) = alloc_reg {
            self.invalidate_cached_operand(dst, Some(alloc_reg));
            if alloc_reg != reg {
                self.push_text(mov(Operand::Reg(alloc_reg), Operand::Reg(reg)));
            }
            self.regs.insert(alloc_reg, dst.clone());

            if alloc_reg.is_caller_saved_gp() && self.has_slot(dst) {
                self.push_text(mov(m_rbp(self.get_offset(dst)?), Operand::Reg(reg)));
            }
            return Ok(());
        }
        self.invalidate_cached_operand(dst, Some(reg));
        self.push_text(mov(m_rbp(self.get_offset(dst)?), Operand::Reg(reg)));
        self.regs.insert(reg, dst.clone());
        Ok(())
    }

    pub(super) fn store_global(&mut self, dst: &IROperand, reg: Reg) -> Result<(), CodeGenError> {
        self.store_global_inner(dst, reg, false)
    }

    pub(super) fn store_global_xmm(
        &mut self,
        dst: &IROperand,
        reg: Reg,
    ) -> Result<(), CodeGenError> {
        self.store_global_inner(dst, reg, true)
    }

    fn store_global_inner(
        &mut self,
        dst: &IROperand,
        reg: Reg,
        is_float: bool,
    ) -> Result<(), CodeGenError> {
        let kind = if is_float { "global_xmm" } else { "global" };
        let IROperand::Global(name) = dst else {
            return Err(CodeGenError::InvalidOperand {
                message: format!("store_{kind} requires a global operand"),
            });
        };
        let mov = |to, from| {
            if is_float {
                Asm::Movsd(to, from)
            } else {
                Asm::Mov(to, from)
            }
        };
        self.invalidate_cached_reg(Reg::R10);
        self.push_text(Asm::Mov(Operand::Reg(Reg::R10), rel(name.clone())));
        self.push_text(mov(m_base(Reg::R10), Operand::Reg(reg)));
        self.regs.remove(&reg);
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
            let mut db_bytes = s.as_bytes().to_vec();
            db_bytes.push(0);
            self.push_data(Asm::Label(lbl.clone()));
            self.push_data(Asm::Db(db_bytes));
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

    pub(super) fn has_slot(&self, op: &IROperand) -> bool {
        let k = op.key();
        if self.spill_vars.contains_key(&k) {
            return true;
        }
        match op {
            IROperand::Var(name) => self.vars.contains_key(name),
            IROperand::Temp(id, _) => self.vars.contains_key(&format!("_tmp_{}", id)),
            _ => false,
        }
    }

    pub(super) fn get_offset(&self, op: &IROperand) -> Result<usize, CodeGenError> {
        let k = op.key();
        if let Some(off) = self.spill_vars.get(&k) {
            return Ok(*off);
        }
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
        len: usize,
        arr: Vec<IROperand>,
        reg: Reg,
    ) -> Result<(), CodeGenError> {
        let size = (len * 8 + 15) & !15;

        self.push_text(Asm::Push(Reg::Rdi));
        self.push_text(Asm::Mov(
            Operand::Reg(Reg::Rdi),
            Operand::Imm((size + 8) as i64),
        ));

        self.push_text(Asm::Sub(Operand::Reg(Reg::Rsp), Operand::Imm(8)));
        self.push_text(Asm::Call(Operand::PLT("malloc".to_string())));
        self.push_text(Asm::Add(Operand::Reg(Reg::Rsp), Operand::Imm(8)));
        self.push_text(Asm::Pop(Reg::Rdi));
        self.invalidate_volatile_registers();
        self.push_text(Asm::Mov(m_base_disp(Reg::Rax, 0), Operand::Imm(len as i64)));
        self.push_text(Asm::Push(Reg::Rax));

        self.push_text(Asm::Sub(Operand::Reg(Reg::Rsp), Operand::Imm(8)));
        for (i, op) in arr.iter().enumerate() {
            self.load(op, Reg::Rax)?;
            self.push_text(Asm::Mov(Operand::Reg(Reg::R11), m_base_disp(Reg::Rsp, 8)));
            self.push_text(Asm::Mov(
                m_base_disp(Reg::R11, ((i + 1) * 8) as i32),
                Operand::Reg(Reg::Rax),
            ));
        }
        self.push_text(Asm::Mov(Operand::Reg(reg), m_base_disp(Reg::Rsp, 8)));
        self.push_text(Asm::Add(Operand::Reg(Reg::Rsp), Operand::Imm(16)));
        self.regs.clear();
        Ok(())
    }
}
