use super::asm::Reg;
use crate::compiler::irgen::ir::{IRConst, IRFunction, IRType, Instruction, Op, Operand};
use std::collections::{HashMap, HashSet};

fn collect_float_vars(instructions: &[Instruction]) -> HashSet<String> {
    let mut vars = HashSet::new();
    for inst in instructions {
        let is_flt = matches!(
            inst.op,
            Op::FMove
                | Op::FLoad
                | Op::FStore
                | Op::FGlobLoad
                | Op::FGlobStore
                | Op::FAdd
                | Op::FSub
                | Op::FMul
                | Op::FDiv
                | Op::FNeg
                | Op::FEq
                | Op::FNe
                | Op::FGt
                | Op::FGe
                | Op::FLt
                | Op::FLe
                | Op::FArg(_)
        );
        if is_flt {
            for op in [inst.dst.as_ref(), inst.src1.as_ref(), inst.src2.as_ref()]
                .iter()
                .flatten()
            {
                if let Operand::Var(name) = op {
                    vars.insert(name.clone());
                }
            }
        }
    }
    vars
}

#[derive(Debug, Clone)]
struct Interval {
    vreg: String,
    is_float: bool,
    start: usize,
    end: usize,
    range_mask: u32,
}

#[derive(Debug, Clone)]
pub struct Allocation {
    pub registers: HashMap<String, Reg>,
    pub spill_offsets: HashMap<String, usize>,
    pub stack_size: usize,
    pub used_callee_saved: Vec<Reg>,
    pub xmm_saved: Vec<(Reg, usize)>,
}

fn build_label_map(instructions: &[Instruction]) -> HashMap<String, usize> {
    let mut map = HashMap::new();
    for (i, inst) in instructions.iter().enumerate() {
        if let Op::Label(lbl) = &inst.op {
            map.insert(lbl.clone(), i);
        }
    }
    map
}

fn get_successors(
    i: usize,
    inst: &Instruction,
    label_map: &HashMap<String, usize>,
    inst_count: usize,
) -> Vec<usize> {
    match &inst.op {
        Op::Jump => {
            if let Some(Operand::Label(lbl)) = &inst.src1 {
                label_map.get(lbl).map(|&t| vec![t]).unwrap_or_default()
            } else {
                vec![]
            }
        }
        Op::JumpIfFalse => {
            let mut succs = Vec::new();
            if let Some(Operand::Label(lbl)) = &inst.src2 {
                if let Some(&target) = label_map.get(lbl) {
                    succs.push(target);
                }
            }
            if i + 1 < inst_count {
                succs.push(i + 1);
            }
            succs
        }
        Op::Return(_) => vec![],
        _ => {
            if i + 1 < inst_count {
                vec![i + 1]
            } else {
                vec![]
            }
        }
    }
}

fn collect_array_element_temps(op: &Operand, constants: &[IRConst], out: &mut HashSet<String>) {
    if let Operand::ConstIdx(idx) = op {
        if let IRConst::Array(elems) = &constants[*idx] {
            for elem in elems {
                if let Operand::Temp(_, _) = elem {
                    out.insert(elem.key());
                }
            }
        }
    }
}

fn gpr_bit(reg: Reg) -> u32 {
    1u32 << reg.reg_id()
}

const R_RAX: u32 = 1 << 0;
const R_RCX: u32 = 1 << 1;
const R_RDX: u32 = 1 << 2;
const R_RBX: u32 = 1 << 3;
const R_RSI: u32 = 1 << 6;
const R_RDI: u32 = 1 << 7;
const R_R8: u32 = 1 << 8;
const R_R9: u32 = 1 << 9;
const R_R10: u32 = 1 << 10;
const R_R11: u32 = 1 << 11;
const R_R15: u32 = 1 << 15;

const ALL_VOLATILE: u32 =
    R_RAX | R_RCX | R_RDX | R_RBX | R_RSI | R_RDI | R_R8 | R_R9 | R_R10 | R_R11;

fn arg_reg_bit(n: usize) -> u32 {
    match n {
        0 => R_RDI,
        1 => R_RSI,
        2 => R_RDX,
        3 => R_RCX,
        _ => R_R8,
    }
}

fn op_clobbers(op: &Op) -> u32 {
    match op {
        Op::Move | Op::Load | Op::Store | Op::GlobLoad | Op::GlobStore => R_RAX | R_R10,
        Op::FMove | Op::FLoad | Op::FStore | Op::FGlobLoad | Op::FGlobStore => R_R10,
        Op::Add | Op::Sub | Op::Mul | Op::LAnd | Op::LOr | Op::Xor => R_RAX | R_RBX | R_R10,
        Op::Shl | Op::Shr => R_RAX | R_RCX | R_R10,
        Op::BNot => R_RAX | R_R10,
        Op::Div | Op::Mod => R_RAX | R_RDX | R_RBX | R_R10,
        Op::FAdd | Op::FSub | Op::FMul | Op::FDiv => R_R10,
        Op::Eq | Op::Ne | Op::Gt | Op::Ge | Op::Lt | Op::Le => R_RAX | R_RBX | R_R10,
        Op::FEq | Op::FNe | Op::FGt | Op::FGe | Op::FLt | Op::FLe => R_RAX | R_R10,
        Op::StrEq | Op::StrNe | Op::StrLt | Op::StrLe | Op::StrGt | Op::StrGe => ALL_VOLATILE,
        Op::Neg | Op::Inc | Op::Dec | Op::SizeOf | Op::Not => R_RAX | R_R10,
        Op::FNeg => R_R10,
        Op::Range => ALL_VOLATILE,
        Op::Arg(n) => {
            if *n < 6 {
                arg_reg_bit(*n) | R_R10
            } else {
                R_RAX | R_R10
            }
        }
        Op::FArg(n) => {
            if *n < 8 {
                R_R10
            } else {
                R_RAX | R_R10
            }
        }
        Op::Call => ALL_VOLATILE,
        Op::Jump => 0,
        Op::JumpIfFalse | Op::JumpIfTrue => R_RAX | R_R10,
        Op::ArrayAccess | Op::ByteAccess => R_RAX | R_RCX | R_R10,
        Op::ArrayAssign | Op::ByteAssign => R_RAX | R_RCX | R_RDX | R_R10,
        Op::StrByte | Op::StrCat => ALL_VOLATILE | R_R15,
        Op::Lea => R_RAX | R_R10,
        Op::Malloc | Op::Free => ALL_VOLATILE,
        Op::StoreAt | Op::LoadAt => R_RAX | R_R10 | R_R11,
        Op::IntToFloat | Op::FloatToInt => R_RAX | R_R10,
        Op::Return(_) | Op::Label(_) => 0,
    }
}

fn compute_liveness(
    instructions: &[Instruction],
    label_map: &HashMap<String, usize>,
    constants: &[IRConst],
) -> (
    Vec<HashSet<String>>,
    Vec<HashSet<String>>,
    HashMap<String, usize>,
    HashMap<String, usize>,
) {
    let inst_count = instructions.len();
    let mut live_in: Vec<HashSet<String>> = vec![HashSet::new(); inst_count];
    let mut live_out: Vec<HashSet<String>> = vec![HashSet::new(); inst_count];
    let mut def: Vec<HashSet<String>> = vec![HashSet::new(); inst_count];
    let mut use_: Vec<HashSet<String>> = vec![HashSet::new(); inst_count];

    for (i, inst) in instructions.iter().enumerate() {
        if let Some(dst) = &inst.dst {
            let dst_reads = matches!(inst.op, Op::ArrayAssign | Op::ByteAssign | Op::StoreAt);
            match dst {
                Operand::Temp(_, _) => {
                    if dst_reads {
                        use_[i].insert(dst.key());
                    } else {
                        def[i].insert(dst.key());
                    }
                }
                Operand::Var(_) => {
                    if matches!(
                        inst.op,
                        Op::Store
                            | Op::FStore
                            | Op::Move
                            | Op::FMove
                            | Op::Load
                            | Op::FLoad
                            | Op::GlobLoad
                            | Op::FGlobLoad
                    ) {
                        def[i].insert(dst.key());
                    } else {
                        use_[i].insert(dst.key());
                    }
                }
                _ => {}
            }
        }
        for src in [inst.src1.as_ref(), inst.src2.as_ref()].iter().flatten() {
            if matches!(src, Operand::Temp(_, _) | Operand::Var(_)) {
                use_[i].insert(src.key());
            }
            collect_array_element_temps(src, constants, &mut use_[i]);
        }
    }

    let mut changed = true;
    while changed {
        changed = false;
        for i in (0..inst_count).rev() {
            let succs = get_successors(i, &instructions[i], label_map, inst_count);
            let mut new_live_out = HashSet::new();
            for &succ in &succs {
                new_live_out.extend(live_in[succ].iter().cloned());
            }
            let mut new_live_in = new_live_out.clone();
            for d in &def[i] {
                new_live_in.remove(d);
            }
            new_live_in.extend(use_[i].iter().cloned());

            if new_live_in != live_in[i] {
                changed = true;
                live_in[i] = new_live_in;
                live_out[i] = new_live_out;
            }
        }
    }

    let mut first_def_or_use: HashMap<String, usize> = HashMap::new();
    let mut last_live: HashMap<String, usize> = HashMap::new();

    for (i, inst) in instructions.iter().enumerate() {
        for op in [inst.dst.as_ref(), inst.src1.as_ref(), inst.src2.as_ref()]
            .iter()
            .flatten()
        {
            if matches!(op, Operand::Temp(_, _) | Operand::Var(_)) {
                first_def_or_use.entry(op.key()).or_insert(i);
            }
            let mut array_temps = HashSet::new();
            collect_array_element_temps(op, constants, &mut array_temps);
            for k in array_temps {
                first_def_or_use.entry(k).or_insert(i);
            }
        }
        for t in &live_out[i] {
            last_live
                .entry(t.clone())
                .and_modify(|e| *e = i)
                .or_insert(i);
        }
    }

    for (i, inst) in instructions.iter().enumerate() {
        for op in [inst.dst.as_ref(), inst.src1.as_ref(), inst.src2.as_ref()]
            .iter()
            .flatten()
        {
            if matches!(op, Operand::Temp(_, _) | Operand::Var(_)) {
                let k = op.key();
                last_live
                    .entry(k)
                    .and_modify(|e| *e = (*e).max(i))
                    .or_insert(i);
            }
            let mut array_temps = HashSet::new();
            collect_array_element_temps(op, constants, &mut array_temps);
            for k in array_temps {
                last_live
                    .entry(k)
                    .and_modify(|e| *e = (*e).max(i))
                    .or_insert(i);
            }
        }
    }

    (live_in, live_out, first_def_or_use, last_live)
}

fn compute_intervals(
    instructions: &[Instruction],
    params: &[(Operand, IRType)],
    first_def_or_use: &HashMap<String, usize>,
    last_live: &HashMap<String, usize>,
    constants: &[IRConst],
) -> Vec<Interval> {
    let float_vars = collect_float_vars(instructions);
    let mut seen = HashSet::new();
    let mut intervals = Vec::new();

    let register = |op: &Operand, seen: &mut HashSet<String>, intervals: &mut Vec<Interval>| {
        let k = op.key();
        if seen.insert(k.clone()) {
            let start = first_def_or_use.get(&k).copied().unwrap_or(0);
            let end = last_live.get(&k).copied().unwrap_or(start);
            let mut range_mask = 0u32;
            for j in (start + 1)..=end {
                if j < instructions.len() {
                    range_mask |= op_clobbers(&instructions[j].op);
                }
            }
            let is_float = match op {
                Operand::Temp(_, ty) => *ty == IRType::Float,
                Operand::Var(name) => float_vars.contains(name),
                _ => false,
            };
            intervals.push(Interval {
                vreg: k,
                is_float,
                start,
                end,
                range_mask,
            });
        }
    };

    for (op, ty) in params {
        let k = op.key();
        if seen.insert(k.clone()) {
            let start = 0;
            let end = last_live.get(&k).copied().unwrap_or(start);
            let mut range_mask = 0u32;
            for j in (start + 1)..=end {
                if j < instructions.len() {
                    range_mask |= op_clobbers(&instructions[j].op);
                }
            }
            intervals.push(Interval {
                vreg: k,
                is_float: matches!(ty, IRType::Float),
                start,
                end,
                range_mask,
            });
        }
    }

    for inst in instructions {
        for op in [inst.dst.as_ref(), inst.src1.as_ref(), inst.src2.as_ref()]
            .iter()
            .flatten()
        {
            if matches!(op, Operand::Temp(_, _) | Operand::Var(_)) {
                register(op, &mut seen, &mut intervals);
            }
            let mut array_temps = HashSet::new();
            collect_array_element_temps(op, constants, &mut array_temps);
            for k in array_temps {
                if seen.insert(k.clone()) {
                    let start = first_def_or_use.get(&k).copied().unwrap_or(0);
                    let end = last_live.get(&k).copied().unwrap_or(start);
                    intervals.push(Interval {
                        vreg: k,
                        is_float: false,
                        start,
                        end,
                        range_mask: 0,
                    });
                }
            }
        }
    }

    intervals.sort_by_key(|iv| iv.start);
    intervals
}

fn alloc_pool(
    intervals: &[Interval],
    volatile: &[Reg],
    callee: &[Reg],
) -> (HashMap<String, Reg>, Vec<String>) {
    let mut allocation: HashMap<String, Reg> = HashMap::new();
    let mut spilled: Vec<String> = Vec::new();
    let mut active: Vec<(usize, String, Reg)> = Vec::new();

    for iv in intervals {
        let mut i = 0;
        while i < active.len() {
            if active[i].0 < iv.start {
                active.swap_remove(i);
            } else {
                i += 1;
            }
        }

        let used: HashSet<Reg> = active.iter().map(|(_, _, r)| *r).collect();
        let mut chosen = None;
        for reg in volatile {
            if !used.contains(reg) && (iv.range_mask & gpr_bit(*reg)) == 0 {
                chosen = Some(*reg);
                break;
            }
        }
        if chosen.is_none() {
            for reg in callee {
                if !used.contains(reg) {
                    chosen = Some(*reg);
                    break;
                }
            }
        }

        match chosen {
            Some(reg) => {
                allocation.insert(iv.vreg.clone(), reg);
                active.push((iv.end, iv.vreg.clone(), reg));
            }
            None => spilled.push(iv.vreg.clone()),
        }
    }

    (allocation, spilled)
}

pub fn allocate_registers(func: &IRFunction, program_constants: &[IRConst]) -> Allocation {
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
    let label_map = build_label_map(&func.instructions);
    let (_live_in, _live_out, first_def_or_use, last_live) =
        compute_liveness(&func.instructions, &label_map, program_constants);
    let intervals = compute_intervals(
        &func.instructions,
        &func.params,
        &first_def_or_use,
        &last_live,
        program_constants,
    );

    const VOLATILE_GPR: [Reg; 10] = [
        Reg::R8,
        Reg::R9,
        Reg::R10,
        Reg::R11,
        Reg::R15,
        Reg::Rax,
        Reg::Rcx,
        Reg::Rdx,
        Reg::Rsi,
        Reg::Rdi,
    ];
    let volatile_pool: Vec<Reg> = if is_leaf {
        VOLATILE_GPR
            .iter()
            .copied()
            .filter(|r| *r != Reg::R15)
            .collect()
    } else {
        VOLATILE_GPR.to_vec()
    };
    const CALLEE_GPR: [Reg; 3] = [Reg::R12, Reg::R13, Reg::R14];
    const FLOAT_POOL: [Reg; 8] = [
        Reg::Xmm8,
        Reg::Xmm9,
        Reg::Xmm10,
        Reg::Xmm11,
        Reg::Xmm12,
        Reg::Xmm13,
        Reg::Xmm14,
        Reg::Xmm15,
    ];

    let int_intervals: Vec<Interval> = intervals
        .iter()
        .filter(|iv| !iv.is_float)
        .cloned()
        .collect();
    let flt_intervals: Vec<Interval> = intervals.iter().filter(|iv| iv.is_float).cloned().collect();

    let (mut int_registers, mut int_spilled) =
        alloc_pool(&int_intervals, &volatile_pool, &CALLEE_GPR);
    let (mut flt_registers, flt_spilled) = alloc_pool(&flt_intervals, &[], &FLOAT_POOL);

    let must_spill: HashSet<String> = func
        .instructions
        .iter()
        .filter(|inst| matches!(inst.op, Op::Lea))
        .filter_map(|inst| inst.src1.as_ref())
        .filter(|op| matches!(op, Operand::Temp(_, _) | Operand::Var(_)))
        .map(Operand::key)
        .collect();

    for vreg in &must_spill {
        if int_registers.remove(vreg).is_some() {
            int_spilled.push(vreg.clone());
        }
    }

    let mut registers = int_registers;
    registers.extend(flt_registers.drain());

    let mut used_callee_saved: Vec<Reg> = registers
        .values()
        .copied()
        .filter(|r| !r.is_xmm() && r.reg_id() >= 12)
        .collect();
    if func
        .instructions
        .iter()
        .any(|i| matches!(i.op, Op::Div | Op::Mod | Op::StrCat))
    {
        used_callee_saved.push(Reg::Rbx);
    }
    used_callee_saved.sort_by_key(|r| r.reg_id());
    used_callee_saved.dedup();

    let mut spill_offsets: HashMap<String, usize> = HashMap::new();
    let mut offset = 0;
    for vreg in &int_spilled {
        offset += 8;
        spill_offsets.insert(vreg.clone(), offset);
    }
    for vreg in &flt_spilled {
        offset += 8;
        spill_offsets.insert(vreg.clone(), offset);
    }

    let mut xmm_saved: Vec<(Reg, usize)> = Vec::new();
    for reg in &FLOAT_POOL {
        if registers.values().any(|r| *r == *reg) {
            offset += 8;
            xmm_saved.push((*reg, offset));
        }
    }

    let stack_size = ((offset + 15) & !15).max(if is_leaf { 0 } else { 16 });

    let allocation = Allocation {
        registers,
        spill_offsets,
        stack_size,
        used_callee_saved,
        xmm_saved,
    };
    if std::env::var("ALC_DEBUG_ALLOC").is_ok() {
        eprintln!("=== ALLOC {} ===", func.name);
        let mut v: Vec<_> = allocation.registers.iter().collect();
        v.sort_by_key(|(k, _)| (*k).clone());
        for (k, r) in v {
            eprintln!("{} -> {}", k, r);
        }
        for (k, o) in &allocation.spill_offsets {
            eprintln!("SPILL {} -> {}", k, o);
        }
    }
    allocation
}
