use super::codegen::{AsmCodeGen, m_base_disp, m_rbp, parse_reg};
use super::error::CodeGenError;
use super::regalloc;
use crate::compiler::{
    codegen::asm::*,
    irgen::ir::{IRFunction, IRType, Instruction, Op, Operand as IROperand},
};
use std::collections::{HashMap, HashSet};

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

        Op::FLt => (Jcc::Ja, Jcc::Jbe),
        Op::FLe => (Jcc::Jae, Jcc::Jb),
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
        self.stack_arg_plan.clear();
        self.cur_inst_idx = 0;
        self.call_stack_bytes = 0;

        let alloc = regalloc::allocate_registers(&func, &self.program.constants);
        self.alloc_regs = alloc.registers;
        self.spill_vars = alloc.spill_offsets;
        self.used_callee_saved = alloc.used_callee_saved;
        let stack_size = alloc.stack_size;

        let is_leaf = alloc.is_leaf;
        let mut lea_used: std::collections::HashSet<String> = std::collections::HashSet::new();
        for inst in &func.instructions {
            if matches!(inst.op, Op::Lea)
                && let Some(IROperand::Var(name)) = &inst.src1
            {
                lea_used.insert(name.clone());
            }
        }
        let mut stack_params: HashSet<String> = HashSet::new();
        {
            let mut n_int = 0usize;
            let mut n_flt = 0usize;
            for (param, ty) in &func.params {
                let on_stack = if matches!(ty, IRType::Float) {
                    n_flt >= 8
                } else {
                    n_int >= 6
                };
                if matches!(ty, IRType::Float) {
                    n_flt += 1;
                } else {
                    n_int += 1;
                }
                if on_stack {
                    if let IROperand::Var(name) = param {
                        stack_params.insert(name.clone());
                    }
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
        let frame_less = final_stack_size == 0
            && xmm_saved.is_empty()
            && self.used_callee_saved.is_empty()
            && stack_params.is_empty();

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

            if used_regs.len() % 2 == 1 {
                self.push_text(Asm::Sub(Operand::Reg(Reg::Rsp), Operand::Imm(8)));
            }

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

        let saved_base = 16 + 8 * self.used_callee_saved.len() as i32;
        let mut int_idx = 0;
        let mut flt_idx = 0;
        let mut stack_idx = 0i32;

        let mut int_moves: Vec<(Reg, Reg)> = Vec::new();
        let mut flt_moves: Vec<(Reg, Reg)> = Vec::new();
        let mut stack_to_reg: Vec<(Reg, Operand)> = Vec::new();
        for (param, ty) in &func.params {
            let k = param.key();
            let is_float = matches!(ty, IRType::Float);
            let on_stack = if is_float { flt_idx >= 8 } else { int_idx >= 6 };
            if !on_stack {
                let arg_reg = if is_float {
                    let r = parse_reg(&self.flt_arg_reg[flt_idx]);
                    flt_idx += 1;
                    r
                } else {
                    let r = parse_reg(&self.arg_reg[int_idx]);
                    int_idx += 1;
                    r
                };
                match alloc_regs.get(&k) {
                    Some(alloc_reg) => {
                        if *alloc_reg != arg_reg {
                            if is_float {
                                flt_moves.push((*alloc_reg, arg_reg));
                            } else {
                                int_moves.push((*alloc_reg, arg_reg));
                            }
                        } else {
                            self.regs.insert(*alloc_reg, param.clone());
                        }
                    }
                    None => {
                        let off = self.get_offset(param)?;
                        if is_float {
                            self.push_text(Asm::Movsd(m_rbp(off), Operand::Reg(arg_reg)));
                        } else {
                            self.push_text(Asm::Mov(m_rbp(off), Operand::Reg(arg_reg)));
                        }
                        self.regs.insert(arg_reg, param.clone());
                    }
                }
            } else {
                let src = m_base_disp(Reg::Rbp, saved_base + 8 * stack_idx);
                stack_idx += 1;
                if let Some(alloc_reg) = alloc_regs.get(&k) {
                    stack_to_reg.push((*alloc_reg, src));
                    self.regs.insert(*alloc_reg, param.clone());
                } else {
                    let off = self.get_offset(param)?;
                    self.push_text(Asm::Mov(Operand::Reg(Reg::Rax), src));
                    self.push_text(Asm::Mov(m_rbp(off), Operand::Reg(Reg::Rax)));
                }
            }
        }
        self.emit_param_moves(&int_moves, false);
        self.emit_param_moves(&flt_moves, true);

        for (reg, src) in stack_to_reg {
            self.push_text(Asm::Mov(Operand::Reg(reg), src));
        }
        let xmm_spill_slots = xmm_saved.clone();

        let insts = &func.instructions;

        let mut stack_arg_plan: HashMap<usize, (usize, usize)> = HashMap::new();
        for (ci, inst) in insts.iter().enumerate() {
            if !matches!(inst.op, Op::Call) {
                continue;
            }
            let mut stack_args: Vec<usize> = Vec::new();
            let mut j = ci;
            while j > 0 {
                let is_stack_arg = match insts[j - 1].op {
                    Op::Arg(n) => n >= 6,
                    Op::FArg(n) => n >= 8,
                    _ => false,
                };
                if is_stack_arg {
                    stack_args.push(j - 1);
                    j -= 1;
                } else {
                    break;
                }
            }
            stack_args.reverse();
            let total = stack_args.len();
            for (slot, &idx) in stack_args.iter().enumerate() {
                stack_arg_plan.insert(idx, (total, slot));
            }
        }
        self.stack_arg_plan = stack_arg_plan;

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
            self.cur_inst_idx = i;
            if self.try_fuse_compare_jump(insts, i, &temp_use_count)? {
                i += 2;
                continue;
            }
            match &code.op {
                Op::Call => {
                    for (r, off) in &xmm_spill_slots {
                        self.push_text(Asm::Movsd(m_rbp(*off), Operand::Reg(*r)));
                    }
                    self.compile_code(code.clone())?;
                    for (r, off) in xmm_spill_slots.iter().rev() {
                        self.push_text(Asm::Movsd(Operand::Reg(*r), m_rbp(*off)));
                    }
                    self.invalidate_cached_reg(Reg::Rax);
                }
                Op::Return(reg_name) => {
                    if let Some(ref val) = code.src1 {
                        self.load(val, parse_reg(reg_name))?;
                    } else {
                        let reg = parse_reg(reg_name);
                        if reg == Reg::Rax {
                            self.push_text(Asm::Xor(
                                Operand::Reg(Reg::Rax),
                                Operand::Reg(Reg::Rax),
                            ));
                        }
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

    fn emit_param_moves(&mut self, moves: &[(Reg, Reg)], is_float: bool) {
        let has_hazard = moves
            .iter()
            .enumerate()
            .any(|(i, (dst, _))| moves.iter().skip(i + 1).any(|(_, src)| src == dst));
        if !has_hazard {
            for (dst, src) in moves {
                if is_float {
                    self.push_text(Asm::Movsd(Operand::Reg(*dst), Operand::Reg(*src)));
                } else {
                    self.push_text(Asm::Mov(Operand::Reg(*dst), Operand::Reg(*src)));
                }
            }
            return;
        }
        for (_, src) in moves.iter().rev() {
            self.push_text(Asm::Push(*src));
        }
        for (i, (dst, _)) in moves.iter().enumerate() {
            let src_mem = Operand::Mem(Mem {
                base: Some(Reg::Rsp),
                index: None,
                scale: 0,
                disp: (i * 8) as i32,
            });
            if is_float {
                self.push_text(Asm::Movsd(Operand::Reg(*dst), src_mem));
            } else {
                self.push_text(Asm::Mov(Operand::Reg(*dst), src_mem));
            }
        }
        self.push_text(Asm::Add(
            Operand::Reg(Reg::Rsp),
            Operand::Imm((moves.len() * 8) as i64),
        ));
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

            if matches!(cmp.op, Op::FLt | Op::FLe) {
                self.push_text(Asm::Ucomisd(Reg::Xmm1, Reg::Xmm0));
            } else {
                self.push_text(Asm::Ucomisd(Reg::Xmm0, Reg::Xmm1));
            }
            if matches!(cmp.op, Op::FEq | Op::FNe) {
                if matches!(cmp.op, Op::FEq) == jump_if_true {
                    let skip = self.new_label("fcmp_nan");
                    self.push_text(Asm::Jp(skip.clone()));
                    cc.emit(self, &label);
                    self.push_text(Asm::Label(skip));
                    cc.emit(self, &label);
                } else {
                    self.push_text(Asm::Jp(label.clone()));
                    self.push_text(Asm::Jne(label.clone()));
                }
            } else {
                cc.emit(self, &label);
            }
            self.invalidate_cached_reg(Reg::Xmm0);
            self.invalidate_cached_reg(Reg::Xmm1);
        } else {
            self.load(a, Reg::Rax)?;
            self.load(b, Reg::Rbx)?;
            self.push_text(Asm::Cmp(Operand::Reg(Reg::Rax), Operand::Reg(Reg::Rbx)));
            self.invalidate_cached_reg(Reg::Rax);
            self.invalidate_cached_reg(Reg::Rbx);
            cc.emit(self, &label);
        }
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
