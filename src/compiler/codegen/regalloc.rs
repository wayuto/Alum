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

    mask_volatile: u32,
}

#[derive(Debug, Clone)]
pub struct Allocation {
    pub registers: HashMap<String, Reg>,
    pub spill_offsets: HashMap<String, usize>,
    pub stack_size: usize,
    pub used_callee_saved: Vec<Reg>,
    pub xmm_saved: Vec<(Reg, usize)>,
    pub is_leaf: bool,

    pub call_saves: HashMap<usize, (Vec<Reg>, bool)>,

    pub debug_intervals: Vec<(String, usize, usize, Reg)>,
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
        Op::TailCall => vec![],
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
const ALL_VOLATILE: u32 = R_RAX | R_RCX | R_RDX | R_RSI | R_RDI | R_R8 | R_R9 | R_R10 | R_R11;

fn arg_reg_bit(n: usize) -> u32 {
    match n {
        0 => R_RDI,
        1 => R_RSI,
        2 => R_RDX,
        3 => R_RCX,
        4 => R_R8,
        _ => R_R9,
    }
}

fn src2_clobber(src2: Option<&Operand>, constants: &[IRConst]) -> u32 {
    match src2 {
        Some(Operand::ConstIdx(idx)) => match &constants[*idx] {
            IRConst::Int(v) if *v >= i32::MIN as i64 && *v <= i32::MAX as i64 => R_RAX,
            IRConst::Int(_) => R_RAX | R_R10,
            _ => R_RAX | R_RBX | R_R10,
        },
        Some(Operand::Var(_) | Operand::Temp(_, _)) => R_RAX,
        _ => R_RAX | R_RBX | R_R10,
    }
}

fn op_clobbers(inst: &Instruction, constants: &[IRConst]) -> u32 {
    let base = match &inst.op {
        Op::Move | Op::Load | Op::Store | Op::GlobLoad | Op::GlobStore => R_RAX | R_R10,
        Op::FMove | Op::FLoad | Op::FStore | Op::FGlobLoad | Op::FGlobStore => R_R10,
        Op::Add | Op::Sub | Op::Mul | Op::LAnd | Op::LOr | Op::And | Op::Or | Op::Xor => {
            src2_clobber(inst.src2.as_ref(), constants)
        }
        Op::Shl | Op::Shr => R_RAX | R_RCX | R_R10,
        Op::BNot => R_RAX | R_R10,
        Op::Div | Op::Mod => R_RAX | R_RCX | R_RDX | R_RBX | R_R10,
        Op::FAdd | Op::FSub | Op::FMul | Op::FDiv => R_R10,
        Op::Eq | Op::Ne | Op::Gt | Op::Ge | Op::Lt | Op::Le => {
            src2_clobber(inst.src2.as_ref(), constants)
        }
        Op::FEq | Op::FNe => R_RAX | R_RCX | R_R10,
        Op::FGt | Op::FGe | Op::FLt | Op::FLe => R_RAX | R_R10,
        Op::StrEq | Op::StrNe | Op::StrLt | Op::StrLe | Op::StrGt | Op::StrGe => ALL_VOLATILE,
        Op::Neg | Op::Inc | Op::Dec | Op::SizeOf | Op::Not => R_RAX | R_R10,
        Op::FNeg => R_R10,
        Op::Range(..) => ALL_VOLATILE,
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
        Op::TailCall => ALL_VOLATILE,
        Op::Jump => 0,
        Op::JumpIfFalse | Op::JumpIfTrue => R_RAX | R_R10,
        Op::ArrayAccess | Op::ByteAccess => R_RAX | R_RCX | R_R10,
        Op::ArrayAssign | Op::ByteAssign => R_RAX | R_RCX | R_RDX | R_R10,
        Op::StrByte => ALL_VOLATILE | R_R15,
        Op::StrCat => ALL_VOLATILE | R_RBX | R_R15,
        Op::Lea => R_RAX | R_R10,
        Op::Malloc | Op::Free => ALL_VOLATILE,
        Op::StoreAt | Op::LoadAt => R_RAX | R_R10 | R_R11,
        Op::FStoreAt => R_R10,
        Op::FLoadAt => R_R10,
        Op::IntToFloat | Op::FloatToInt => R_RAX | R_R10,
        Op::Return(_) | Op::Label(_) => 0,
    };

    if let Some(Operand::ConstIdx(idx)) = &inst.src1 {
        if matches!(constants[*idx], IRConst::Array(_)) {
            return base | ALL_VOLATILE;
        }
    }
    base
}

pub fn clobbers_all_volatile(inst: &Instruction, constants: &[IRConst]) -> bool {
    op_clobbers(inst, constants) & ALL_VOLATILE == ALL_VOLATILE
}

fn compute_liveness(
    instructions: &[Instruction],
    label_map: &HashMap<String, usize>,
    constants: &[IRConst],
) -> (HashMap<String, usize>, HashMap<String, usize>) {
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

    (first_def_or_use, last_live)
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
            let (range_mask, mask_volatile) = span_masks(instructions, start, end, constants);
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
                mask_volatile,
            });
        }
    };

    for (op, ty) in params {
        let k = op.key();
        if seen.insert(k.clone()) {
            let start = 0;
            let end = last_live.get(&k).copied().unwrap_or(start);
            let (range_mask, mask_volatile) = span_masks(instructions, start, end, constants);
            intervals.push(Interval {
                vreg: k,
                is_float: matches!(ty, IRType::Float),
                start,
                end,
                range_mask,
                mask_volatile,
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
                        mask_volatile: 0,
                    });
                }
            }
        }
    }

    intervals.sort_by_key(|iv| iv.start);
    intervals
}

fn span_masks(
    instructions: &[Instruction],
    start: usize,
    end: usize,
    constants: &[IRConst],
) -> (u32, u32) {
    let mut range_mask = 0u32;
    let mut mask_volatile = 0u32;
    for j in (start + 1)..=end {
        if j >= instructions.len() {
            break;
        }
        let c = op_clobbers(&instructions[j], constants);
        range_mask |= c;
        if c & ALL_VOLATILE == ALL_VOLATILE {
            mask_volatile |= c & !ALL_VOLATILE;
        } else {
            mask_volatile |= c;
        }
    }
    (range_mask, mask_volatile)
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

        let spans_wrapped = iv.range_mask & ALL_VOLATILE != 0;
        let mut chosen: Option<Reg> = None;
        let try_pool = |pool: &[Reg], mask: u32, chosen: &mut Option<Reg>| {
            if chosen.is_some() {
                return;
            }
            for reg in pool {
                if !used.contains(reg) && (mask & gpr_bit(*reg)) == 0 {
                    *chosen = Some(*reg);
                    return;
                }
            }
        };
        if spans_wrapped {
            try_pool(callee, iv.range_mask, &mut chosen);
            try_pool(volatile, iv.mask_volatile, &mut chosen);
        } else {
            try_pool(volatile, iv.mask_volatile, &mut chosen);
            try_pool(callee, iv.range_mask, &mut chosen);
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

fn coalesce_intervals(
    intervals: Vec<Interval>,
    instructions: &[Instruction],
    constants: &[IRConst],
    excluded: &HashSet<String>,
) -> (Vec<Interval>, HashMap<String, String>) {
    fn find(parent: &mut Vec<usize>, mut x: usize) -> usize {
        while parent[x] != x {
            parent[x] = parent[parent[x]];
            x = parent[x];
        }
        x
    }

    let idx_of: HashMap<&str, usize> = intervals
        .iter()
        .enumerate()
        .map(|(i, iv)| (iv.vreg.as_str(), i))
        .collect();
    let n = intervals.len();
    let mut parent: Vec<usize> = (0..n).collect();
    let mut range: Vec<(usize, usize)> = intervals.iter().map(|iv| (iv.start, iv.end)).collect();

    for (i, inst) in instructions.iter().enumerate() {
        if !matches!(inst.op, Op::Move | Op::FMove) {
            continue;
        }
        let (Some(dst), Some(src)) = (&inst.dst, &inst.src1) else {
            continue;
        };
        if !matches!(dst, Operand::Temp(_, _)) {
            continue;
        }
        if !matches!(src, Operand::Temp(_, _) | Operand::Var(_)) {
            continue;
        }
        let dk = dst.key();
        let sk = src.key();
        if excluded.contains(&dk) || excluded.contains(&sk) {
            continue;
        }
        let (Some(&di), Some(&si)) = (idx_of.get(dk.as_str()), idx_of.get(sk.as_str())) else {
            continue;
        };
        let ra = find(&mut parent, di);
        let rb = find(&mut parent, si);
        if ra == rb || intervals[ra].is_float != intervals[rb].is_float {
            continue;
        }
        if range[rb].1 != i || range[ra].0 != i {
            continue;
        }
        parent[rb] = ra;
        range[ra] = (range[ra].0.min(range[rb].0), range[ra].1.max(range[rb].1));
    }

    let mut members_of: HashMap<usize, Vec<usize>> = HashMap::new();
    for i in 0..n {
        let r = find(&mut parent, i);
        members_of.entry(r).or_default().push(i);
    }

    let mut new_intervals: Vec<Interval> = Vec::new();
    let mut key_map: HashMap<String, String> = HashMap::new();
    for (_, members) in members_of {
        let rep = *members.iter().min_by_key(|&&i| intervals[i].start).unwrap();
        let start = members.iter().map(|&i| intervals[i].start).min().unwrap();
        let end = members.iter().map(|&i| intervals[i].end).max().unwrap();
        let (range_mask, mask_volatile) = span_masks(instructions, start, end, constants);
        for &m in &members {
            key_map.insert(intervals[m].vreg.clone(), intervals[rep].vreg.clone());
        }
        new_intervals.push(Interval {
            vreg: intervals[rep].vreg.clone(),
            is_float: intervals[rep].is_float,
            start,
            end,
            range_mask,
            mask_volatile,
        });
    }
    new_intervals.sort_by_key(|iv| iv.start);
    (new_intervals, key_map)
}

pub fn allocate_registers(func: &IRFunction, program_constants: &[IRConst]) -> Allocation {
    let is_leaf = !func.instructions.iter().any(|i| {
        matches!(
            i.op,
            Op::Call
                | Op::TailCall
                | Op::Malloc
                | Op::Free
                | Op::StrCat
                | Op::StrByte
                | Op::Range(..)
                | Op::StrEq
                | Op::StrNe
                | Op::StrLt
                | Op::StrLe
                | Op::StrGt
                | Op::StrGe
        )
    });
    let label_map = build_label_map(&func.instructions);
    let (first_def_or_use, last_live) =
        compute_liveness(&func.instructions, &label_map, program_constants);
    let intervals = compute_intervals(
        &func.instructions,
        &func.params,
        &first_def_or_use,
        &last_live,
        program_constants,
    );

    let must_spill: HashSet<String> = func
        .instructions
        .iter()
        .filter(|inst| matches!(inst.op, Op::Lea))
        .filter_map(|inst| inst.src1.as_ref())
        .filter(|op| matches!(op, Operand::Temp(_, _) | Operand::Var(_)))
        .map(Operand::key)
        .collect();
    let mut elem_temps: HashSet<String> = HashSet::new();
    for inst in &func.instructions {
        for op in [inst.dst.as_ref(), inst.src1.as_ref(), inst.src2.as_ref()]
            .into_iter()
            .flatten()
        {
            collect_array_element_temps(op, program_constants, &mut elem_temps);
        }
    }
    let mut coalesce_excluded = must_spill.clone();
    coalesce_excluded.extend(elem_temps.iter().cloned());
    let (intervals, coalesced_into) = coalesce_intervals(
        intervals,
        &func.instructions,
        program_constants,
        &coalesce_excluded,
    );

    const VOLATILE_GPR: [Reg; 9] = [
        Reg::R8,
        Reg::R9,
        Reg::R10,
        Reg::R11,
        Reg::Rax,
        Reg::Rcx,
        Reg::Rdx,
        Reg::Rsi,
        Reg::Rdi,
    ];

    let volatile_pool: Vec<Reg> = VOLATILE_GPR.to_vec();
    const CALLEE_GPR: [Reg; 5] = [Reg::R12, Reg::R13, Reg::R14, Reg::Rbx, Reg::R15];
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
    let (mut flt_registers, mut flt_spilled) = alloc_pool(&flt_intervals, &[], &FLOAT_POOL);

    for vreg in &must_spill {
        if int_registers.remove(vreg).is_some() {
            int_spilled.push(vreg.clone());
        }
    }

    let mut registers = int_registers;
    registers.extend(flt_registers.drain());

    for (member, root) in &coalesced_into {
        if let Some(r) = registers.get(root).copied() {
            registers.insert(member.clone(), r);
        }
    }
    for k in &elem_temps {
        if let Some(r) = registers.get(k).copied() {
            if r.is_caller_saved_gp() {
                registers.remove(k);
                let is_flt = intervals.iter().any(|iv| iv.vreg == *k && iv.is_float);
                if is_flt {
                    flt_spilled.push(k.clone());
                } else {
                    int_spilled.push(k.clone());
                }
            }
        }
    }

    let mut used_callee_saved: Vec<Reg> = registers
        .values()
        .copied()
        .filter(|r| !r.is_xmm() && !r.is_caller_saved_gp())
        .collect();

    let writes_rbx = func.instructions.iter().any(|i| match i.op {
        Op::Div | Op::Mod | Op::StrCat => true,
        Op::Eq | Op::Ne | Op::Gt | Op::Ge | Op::Lt | Op::Le => {
            src2_clobber(i.src2.as_ref(), program_constants) & R_RBX != 0
        }
        Op::Add | Op::Sub | Op::Mul | Op::LAnd | Op::LOr | Op::And | Op::Or | Op::Xor => {
            src2_clobber(i.src2.as_ref(), program_constants) & R_RBX != 0
        }
        _ => false,
    });
    if writes_rbx {
        used_callee_saved.push(Reg::Rbx);
    }

    if func
        .instructions
        .iter()
        .any(|i| matches!(i.op, Op::StrCat | Op::StrByte))
    {
        used_callee_saved.push(Reg::R15);
    }
    used_callee_saved.sort_by_key(|r| r.reg_id());
    used_callee_saved.dedup();

    let mut interval_pos: HashMap<&str, (usize, usize)> = intervals
        .iter()
        .map(|iv| (iv.vreg.as_str(), (iv.start, iv.end)))
        .collect();
    let mut iv_by_key: HashMap<&str, &Interval> =
        intervals.iter().map(|iv| (iv.vreg.as_str(), iv)).collect();

    let rep_interval: HashMap<&str, &Interval> =
        intervals.iter().map(|iv| (iv.vreg.as_str(), iv)).collect();
    for (member, root) in &coalesced_into {
        if let Some(iv) = rep_interval.get(root.as_str()) {
            iv_by_key.insert(member.as_str(), iv);
            interval_pos.insert(member.as_str(), (iv.start, iv.end));
        }
    }
    let mut def_pos: HashMap<String, Vec<usize>> = HashMap::new();
    for (i, inst) in func.instructions.iter().enumerate() {
        if let Some(d) = &inst.dst {
            let is_def = match d {
                Operand::Temp(_, _) => {
                    !matches!(inst.op, Op::ArrayAssign | Op::ByteAssign | Op::StoreAt)
                }
                Operand::Var(_) => matches!(
                    inst.op,
                    Op::Store
                        | Op::FStore
                        | Op::Move
                        | Op::FMove
                        | Op::Load
                        | Op::FLoad
                        | Op::GlobLoad
                        | Op::FGlobLoad
                ),
                _ => false,
            };
            if is_def {
                def_pos.entry(d.key()).or_default().push(i);
            }
        }
    }
    for inst in func.instructions.iter() {
        match &inst.op {
            Op::Move | Op::Load => {
                let (Some(dst), Some(src)) = (&inst.dst, &inst.src1) else {
                    continue;
                };
                if !matches!(dst, Operand::Temp(_, _)) {
                    continue;
                }
                if !matches!(src, Operand::Var(_) | Operand::Temp(_, _)) {
                    continue;
                }
                let Some(&home) = registers.get(&src.key()) else {
                    continue;
                };
                let Some(&(tstart, tend)) = interval_pos.get(dst.key().as_str()) else {
                    continue;
                };

                let home_ok = match iv_by_key.get(dst.key().as_str()) {
                    Some(iv) => {
                        let mask = if home.is_caller_saved_gp() {
                            iv.mask_volatile
                        } else {
                            iv.range_mask
                        };
                        if mask & gpr_bit(home) != 0 {
                            false
                        } else {
                            let (s1, e1) = (tstart, tend);
                            !registers.iter().any(|(k2, r2)| {
                                *r2 == home
                                    && k2.as_str() != dst.key().as_str()
                                    && k2.as_str() != src.key().as_str()
                                    && iv_by_key
                                        .get(k2.as_str())
                                        .map(|iv2| iv2.start <= e1 && s1 <= iv2.end)
                                        .unwrap_or(false)
                            })
                        }
                    }
                    None => false,
                };
                if !home_ok {
                    continue;
                }

                let redefined = def_pos
                    .get(&src.key())
                    .map(|defs| defs.iter().any(|&p| p > tstart && p <= tend))
                    .unwrap_or(false);
                if !redefined {
                    registers.insert(dst.key(), home);
                }
            }
            _ => {}
        }
    }

    let mut spill_offsets: HashMap<String, usize> = HashMap::new();
    let mut offset = 0usize;

    let mut assign_spills = |keys: &[String], offset: &mut usize| {
        let mut sorted: Vec<&String> = keys.iter().collect();
        sorted.sort_by_key(|k| iv_by_key.get(k.as_str()).map(|iv| iv.start).unwrap_or(0));
        let mut buckets: Vec<(usize, usize)> = Vec::new();
        for k in sorted {
            let (start, end) = match iv_by_key.get(k.as_str()) {
                Some(iv) => (iv.start, iv.end),
                None => (0, usize::MAX),
            };
            let off = match buckets.iter_mut().find(|(bend, _)| *bend < start) {
                Some((bend, off)) => {
                    *bend = (*bend).max(end);
                    *off
                }
                None => {
                    *offset += 8;
                    buckets.push((end, *offset));
                    *offset
                }
            };
            spill_offsets.insert(k.clone(), off);
        }
    };
    assign_spills(&int_spilled, &mut offset);
    assign_spills(&flt_spilled, &mut offset);

    for (member, root) in &coalesced_into {
        if let Some(off) = spill_offsets.get(root).copied() {
            spill_offsets.insert(member.clone(), off);
        }
    }

    let mut xmm_saved: Vec<(Reg, usize)> = Vec::new();
    for reg in &FLOAT_POOL {
        if registers.values().any(|r| *r == *reg) {
            offset += 8;
            xmm_saved.push((*reg, offset));
        }
    }

    let mut call_saves: HashMap<usize, (Vec<Reg>, bool)> = HashMap::new();
    let interval_of: HashMap<&str, (usize, usize)> = interval_pos;
    let home_of: HashMap<&str, Reg> = registers.iter().map(|(k, r)| (k.as_str(), *r)).collect();
    for (i, inst) in func.instructions.iter().enumerate() {
        if !clobbers_all_volatile(inst, program_constants) {
            continue;
        }
        let mut regs: Vec<Reg> = home_of
            .iter()
            .filter(|(k, r)| {
                r.is_caller_saved_gp()
                    && interval_of
                        .get(*k)
                        .is_some_and(|(start, end)| *start < i && i <= *end)
            })
            .map(|(_, r)| *r)
            .collect();
        regs.sort_by_key(|r| r.reg_id());
        regs.dedup();
        if !regs.is_empty() {
            let pad = regs.len() % 2 == 1;
            call_saves.insert(i, (regs, pad));
        }
    }

    let stack_size = ((offset + 15) & !15).max(if is_leaf { 0 } else { 16 });

    let debug_intervals = if std::env::var("ALC_DEBUG_ALLOC").is_ok() {
        registers
            .iter()
            .filter_map(|(k, r)| {
                interval_of
                    .get(k.as_str())
                    .map(|&(s, e)| (k.clone(), s, e, *r))
            })
            .collect()
    } else {
        Vec::new()
    };

    let allocation = Allocation {
        registers,
        spill_offsets,
        stack_size,
        used_callee_saved,
        xmm_saved,
        is_leaf,
        call_saves,
        debug_intervals,
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
