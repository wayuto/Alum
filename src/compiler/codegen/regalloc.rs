use std::collections::{HashMap, HashSet};

use super::asm::Reg;
use crate::compiler::irgen::ir::{IRFunction, IRType, Op, Operand};

fn key(op: &Operand) -> String {
    match op {
        Operand::Var(name) => name.clone(),
        Operand::Temp(id, _) => format!("_tmp_{}", id),
        _ => panic!("unsupported operand for allocation: {:?}", op),
    }
}

fn is_float_op(op: &Operand, _params: &[(Operand, IRType)]) -> bool {
    matches!(op, Operand::Temp(_, ty) if *ty == IRType::Float)
}

#[derive(Debug, Clone)]
struct Interval {
    vreg: String,
    is_float: bool,
    start: usize,
    end: usize,
}

#[derive(Debug, Clone)]
pub struct Allocation {
    pub registers: HashMap<String, Reg>,
    pub spill_offsets: HashMap<String, usize>,
    pub stack_size: usize,
    pub used_callee_saved: Vec<Reg>,
}

fn build_label_map(
    instructions: &[crate::compiler::irgen::ir::Instruction],
) -> HashMap<String, usize> {
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
    inst: &crate::compiler::irgen::ir::Instruction,
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

fn collect_array_element_temps(
    op: &Operand,
    constants: &[crate::compiler::irgen::ir::IRConst],
    out: &mut HashSet<String>,
) {
    if let Operand::ConstIdx(idx) = op {
        if let crate::compiler::irgen::ir::IRConst::Array(elems) = &constants[*idx] {
            for elem in elems {
                if let Operand::Temp(_, _) = elem {
                    out.insert(key(elem));
                }
            }
        }
    }
}

fn compute_liveness(
    instructions: &[crate::compiler::irgen::ir::Instruction],
    label_map: &HashMap<String, usize>,
    constants: &[crate::compiler::irgen::ir::IRConst],
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
        if let Some(ref dst) = inst.dst {
            if matches!(dst, Operand::Temp(_, _)) {
                def[i].insert(key(dst));
            }
        }
        for src in [inst.src1.as_ref(), inst.src2.as_ref()].iter().flatten() {
            if matches!(src, Operand::Temp(_, _)) {
                use_[i].insert(key(src));
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
            if matches!(op, Operand::Temp(_, _)) {
                let k = key(op);
                first_def_or_use.entry(k).or_insert(i);
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
            if matches!(op, Operand::Temp(_, _)) {
                let k = key(op);
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
    instructions: &[crate::compiler::irgen::ir::Instruction],
    params: &[(Operand, IRType)],
    first_def_or_use: &HashMap<String, usize>,
    last_live: &HashMap<String, usize>,
    constants: &[crate::compiler::irgen::ir::IRConst],
) -> Vec<Interval> {
    let mut seen = HashSet::new();
    let mut intervals = Vec::new();

    for (op, _) in params {
        if matches!(op, Operand::Temp(_, _)) {
            let k = key(op);
            if seen.insert(k.clone()) {
                let start = first_def_or_use.get(&k).copied().unwrap_or(0);
                let end = last_live.get(&k).copied().unwrap_or(start);
                intervals.push(Interval {
                    vreg: k,
                    is_float: is_float_op(op, params),
                    start,
                    end,
                });
            }
        }
    }

    for inst in instructions {
        for op in [inst.dst.as_ref(), inst.src1.as_ref(), inst.src2.as_ref()]
            .iter()
            .flatten()
        {
            if matches!(op, Operand::Temp(_, _)) {
                let k = key(op);
                if seen.insert(k.clone()) {
                    let start = first_def_or_use.get(&k).copied().unwrap_or(0);
                    let end = last_live.get(&k).copied().unwrap_or(start);
                    intervals.push(Interval {
                        vreg: k,
                        is_float: is_float_op(op, params),
                        start,
                        end,
                    });
                }
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
                    });
                }
            }
        }
    }

    intervals.sort_by_key(|iv| iv.start);
    intervals
}

fn alloc_pool(intervals: Vec<Interval>, pool: &[Reg]) -> (HashMap<String, Reg>, Vec<String>) {
    let mut allocation: HashMap<String, Reg> = HashMap::new();
    let mut spilled: Vec<String> = Vec::new();
    let mut active: Vec<(usize, String, Reg)> = Vec::new();

    for iv in &intervals {
        let mut i = 0;
        while i < active.len() {
            if active[i].0 <= iv.start {
                active.swap_remove(i);
            } else {
                i += 1;
            }
        }

        if active.len() < pool.len() {
            let used: HashSet<Reg> = active.iter().map(|(_, _, r)| *r).collect();
            if let Some(reg) = pool.iter().find(|r| !used.contains(r)) {
                allocation.insert(iv.vreg.clone(), *reg);
                active.push((iv.end, iv.vreg.clone(), *reg));
            } else {
                spilled.push(iv.vreg.clone());
            }
        } else {
            let max_idx = active
                .iter()
                .enumerate()
                .max_by_key(|(_, (end, _, _))| *end)
                .map(|(i, _)| i)
                .unwrap();
            let (max_end, max_vreg, max_reg) = active[max_idx].clone();
            if max_end > iv.end {
                active.remove(max_idx);
                active.push((iv.end, iv.vreg.clone(), max_reg));
                allocation.insert(iv.vreg.clone(), max_reg);
                allocation.remove(&max_vreg);
                spilled.push(max_vreg);
            } else {
                spilled.push(iv.vreg.clone());
            }
        }

        active.sort_by_key(|(end, _, _)| *end);
    }

    (allocation, spilled)
}

pub fn allocate_registers(
    func: &IRFunction,
    program_constants: &[crate::compiler::irgen::ir::IRConst],
) -> Allocation {
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

    let int_pool: Vec<Reg> = vec![Reg::R12, Reg::R13, Reg::R14, Reg::R15];

    let int_intervals: Vec<Interval> = intervals
        .iter()
        .filter(|iv| !iv.is_float)
        .cloned()
        .collect();
    let flt_intervals: Vec<Interval> = intervals.iter().filter(|iv| iv.is_float).cloned().collect();

    let (mut int_registers, mut int_spilled) = alloc_pool(int_intervals, &int_pool);
    let flt_spilled: Vec<String> = flt_intervals.iter().map(|iv| iv.vreg.clone()).collect();

    let must_spill: HashSet<String> = func
        .instructions
        .iter()
        .filter(|inst| matches!(inst.op, Op::Lea))
        .filter_map(|inst| inst.src1.as_ref())
        .filter(|op| matches!(op, Operand::Temp(_, _)))
        .map(key)
        .collect();

    for vreg in &must_spill {
        if int_registers.remove(vreg).is_some() {
            int_spilled.push(vreg.clone());
        }
    }

    let registers = int_registers;

    let mut used_callee_saved: Vec<Reg> = registers
        .values()
        .copied()
        .filter(|r| r.reg_id() >= 12)
        .collect();
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

    let stack_size = ((offset + 15) & !15).max(16);

    let allocation = Allocation {
        registers,
        spill_offsets,
        stack_size,
        used_callee_saved,
    };
    if std::env::var("ALC_DEBUG_ALLOC").is_ok() {
        eprintln!("=== ALLOC {} ===", func.name);
        let mut v: Vec<_> = allocation.registers.iter().collect();
        v.sort_by_key(|(k, _)| k.clone());
        for (k, r) in v {
            eprintln!("{} -> {}", k, r.reg_id());
        }
        let mut v: Vec<_> = allocation.spill_offsets.iter().collect();
        v.sort_by_key(|(k, _)| k.clone());
        for (k, o) in v {
            eprintln!("SPILL {} -> {}", k, o);
        }
    }
    allocation
}
