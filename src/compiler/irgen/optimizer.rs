use super::ir::{IRConst, IRProgram, Instruction, Op, Operand};
#[cfg(test)]
use super::ir::{IRFunction, IRType};
use ordered_float::OrderedFloat;
use std::collections::{HashMap, HashSet};

pub(crate) fn optimize(program: &mut IRProgram) {
    let mut pool = ConstPool::new(&program.constants);
    for func in &mut program.functions {
        if func.is_external || func.instructions.is_empty() {
            continue;
        }
        optimize_fn(&mut func.instructions, &mut program.constants, &mut pool);
    }
    if std::env::var("ALC_DEBUG_OPT").is_ok() {
        for func in &program.functions {
            eprintln!("=== OPT {} ===", func.name);
            for (i, inst) in func.instructions.iter().enumerate() {
                eprintln!("{:3} {:?}", i, inst);
            }
        }
    }
}

fn optimize_fn(insts: &mut Vec<Instruction>, constants: &mut Vec<IRConst>, pool: &mut ConstPool) {
    loop {
        let mut changed = false;
        changed |= pass_const_fold(insts, constants, pool);
        changed |= pass_algebraic(insts, constants, pool);
        changed |= pass_branch_const(insts, constants);
        changed |= pass_copy_prop(insts);
        changed |= pass_dead_code(insts, constants);
        changed |= pass_unreachable(insts);
        changed |= pass_jump_to_next(insts);
        changed |= pass_dead_labels(insts);
        changed |= pass_licm(insts);
        if !changed {
            break;
        }
    }
}

struct ConstPool {
    map: HashMap<IRConst, usize>,
}

impl ConstPool {
    fn new(constants: &[IRConst]) -> Self {
        let mut map = HashMap::with_capacity(constants.len());
        for (i, c) in constants.iter().enumerate() {
            map.insert(c.clone(), i);
        }
        Self { map }
    }

    fn intern(&mut self, constants: &mut Vec<IRConst>, value: IRConst) -> usize {
        if let Some(&idx) = self.map.get(&value) {
            return idx;
        }
        let idx = constants.len();
        constants.push(value.clone());
        self.map.insert(value, idx);
        idx
    }
}

fn const_int(constants: &[IRConst], op: &Operand) -> Option<i64> {
    match op {
        Operand::ConstIdx(i) => match &constants[*i] {
            IRConst::Int(v) => Some(*v),
            _ => None,
        },
        _ => None,
    }
}

fn const_float(constants: &[IRConst], op: &Operand) -> Option<f64> {
    match op {
        Operand::ConstIdx(i) => match &constants[*i] {
            IRConst::Float(v) => Some(v.into_inner()),
            _ => None,
        },
        _ => None,
    }
}

fn const_str<'a>(constants: &'a [IRConst], op: &'a Operand) -> Option<&'a str> {
    match op {
        Operand::ConstIdx(i) => match &constants[*i] {
            IRConst::Str(s) => Some(s),
            _ => None,
        },
        _ => None,
    }
}

fn fold_binop(op: &Op, constants: &[IRConst], a: &Operand, b: &Operand) -> Option<IRConst> {
    use Op::*;
    match op {
        Add | Sub | Mul | Div | Mod | Xor | LAnd | LOr => {
            let a = const_int(constants, a)?;
            let b = const_int(constants, b)?;
            match op {
                Add => Some(IRConst::Int(a.wrapping_add(b))),
                Sub => Some(IRConst::Int(a.wrapping_sub(b))),
                Mul => Some(IRConst::Int(a.wrapping_mul(b))),
                Div if b != 0 => Some(IRConst::Int(a.wrapping_div(b))),
                Mod if b != 0 => Some(IRConst::Int(a.wrapping_rem(b))),
                Xor => Some(IRConst::Int(a ^ b)),
                LAnd => Some(IRConst::Int(a & b)),
                LOr => Some(IRConst::Int(a | b)),
                _ => None,
            }
        }
        FAdd | FSub | FMul | FDiv => {
            let a = const_float(constants, a)?;
            let b = const_float(constants, b)?;
            match op {
                FAdd => Some(IRConst::Float(OrderedFloat(a + b))),
                FSub => Some(IRConst::Float(OrderedFloat(a - b))),
                FMul => Some(IRConst::Float(OrderedFloat(a * b))),
                FDiv if b != 0.0 => Some(IRConst::Float(OrderedFloat(a / b))),
                _ => None,
            }
        }
        Eq | Ne | Gt | Ge | Lt | Le => {
            let a = const_int(constants, a)?;
            let b = const_int(constants, b)?;
            let r = match op {
                Eq => a == b,
                Ne => a != b,
                Gt => a > b,
                Ge => a >= b,
                Lt => a < b,
                Le => a <= b,
                _ => unreachable!(),
            };
            Some(IRConst::Int(r as i64))
        }
        FEq | FNe | FGt | FGe | FLt | FLe => {
            let a = const_float(constants, a)?;
            let b = const_float(constants, b)?;
            let r = match op {
                FEq => a == b,
                FNe => a != b,
                FGt => a > b,
                FGe => a >= b,
                FLt => a < b,
                FLe => a <= b,
                _ => unreachable!(),
            };
            Some(IRConst::Int(r as i64))
        }
        StrEq | StrNe | StrLt | StrLe | StrGt | StrGe => {
            let a = const_str(constants, a)?;
            let b = const_str(constants, b)?;
            let r = match op {
                StrEq => a == b,
                StrNe => a != b,
                StrLt => a < b,
                StrLe => a <= b,
                StrGt => a > b,
                StrGe => a >= b,
                _ => unreachable!(),
            };
            Some(IRConst::Int(r as i64))
        }
        _ => None,
    }
}

fn fold_unop(op: &Op, constants: &[IRConst], a: &Operand) -> Option<IRConst> {
    use Op::*;
    match op {
        Neg => Some(IRConst::Int(const_int(constants, a)?.wrapping_neg())),
        FNeg => Some(IRConst::Float(OrderedFloat(-const_float(constants, a)?))),
        Not => Some(IRConst::Int(const_int(constants, a)? ^ 1)),
        Inc => Some(IRConst::Int(const_int(constants, a)?.wrapping_add(1))),
        Dec => Some(IRConst::Int(const_int(constants, a)?.wrapping_sub(1))),
        IntToFloat => Some(IRConst::Float(
            OrderedFloat(const_int(constants, a)? as f64),
        )),
        FloatToInt => {
            let v = const_float(constants, a)?;

            if v.is_finite() && v >= -9_223_372_036_854_775_808.0 && v < 9_223_372_036_854_775_808.0
            {
                Some(IRConst::Int(v as i64))
            } else {
                None
            }
        }
        _ => None,
    }
}

fn pass_const_fold(
    insts: &mut [Instruction],
    constants: &mut Vec<IRConst>,
    pool: &mut ConstPool,
) -> bool {
    let mut changed = false;
    for inst in insts.iter_mut() {
        let Some(src1) = inst.src1.clone() else {
            continue;
        };
        let result = match &inst.src2 {
            Some(src2) => fold_binop(&inst.op, constants, &src1, src2),
            None => fold_unop(&inst.op, constants, &src1),
        };
        let Some(result) = result else { continue };
        let idx = pool.intern(constants, result);
        let is_float = matches!(&constants[idx], IRConst::Float(_));
        inst.op = if is_float { Op::FMove } else { Op::Move };
        inst.src1 = Some(Operand::ConstIdx(idx));
        inst.src2 = None;
        changed = true;
    }
    changed
}

fn pass_algebraic(
    insts: &mut [Instruction],
    constants: &mut Vec<IRConst>,
    pool: &mut ConstPool,
) -> bool {
    let zero = pool.intern(constants, IRConst::Int(0));
    let one = pool.intern(constants, IRConst::Int(1));
    let mut changed = false;
    for inst in insts.iter_mut() {
        if inst.dst.is_none() {
            continue;
        }
        let (Some(src1), Some(src2)) = (inst.src1.clone(), inst.src2.clone()) else {
            continue;
        };
        let c1 = const_int(constants, &src1);
        let c2 = const_int(constants, &src2);
        let z = || Operand::ConstIdx(zero);
        let o = || Operand::ConstIdx(one);
        let rep = match inst.op {
            Op::Add if c2 == Some(0) => Some((Op::Move, src1)),
            Op::Add if c1 == Some(0) => Some((Op::Move, src2)),
            Op::Sub if c2 == Some(0) => Some((Op::Move, src1)),
            Op::Sub if c1 == Some(0) => Some((Op::Neg, src2)),
            Op::Mul if c2 == Some(1) => Some((Op::Move, src1)),
            Op::Mul if c1 == Some(1) => Some((Op::Move, src2)),
            Op::Mul if c2 == Some(0) => Some((Op::Move, z())),
            Op::Mul if c1 == Some(0) => Some((Op::Move, z())),
            Op::Div if c2 == Some(1) => Some((Op::Move, src1)),
            Op::Xor if c2 == Some(0) => Some((Op::Move, src1)),
            Op::Xor if c1 == Some(0) => Some((Op::Move, src2)),
            Op::LAnd if c2 == Some(0) => Some((Op::Move, z())),
            Op::LAnd if c1 == Some(0) => Some((Op::Move, z())),
            Op::LAnd if c2 == Some(1) => Some((Op::Move, src1)),
            Op::LAnd if c1 == Some(1) => Some((Op::Move, src2)),
            Op::LOr if c2 == Some(0) => Some((Op::Move, src1)),
            Op::LOr if c1 == Some(0) => Some((Op::Move, src2)),
            Op::LOr if c2 == Some(1) => Some((Op::Move, o())),
            Op::LOr if c1 == Some(1) => Some((Op::Move, o())),
            _ => None,
        };
        if let Some((op, src)) = rep {
            inst.op = op;
            inst.src1 = Some(src);
            inst.src2 = None;
            changed = true;
        }
    }
    changed
}

fn pass_branch_const(insts: &mut Vec<Instruction>, constants: &[IRConst]) -> bool {
    let mut changed = false;
    let mut i = 0;
    while i < insts.len() {
        let (is_false, is_true) = match insts[i].op {
            Op::JumpIfFalse => (true, false),
            Op::JumpIfTrue => (false, true),
            _ => (false, false),
        };
        if is_false || is_true {
            let cond = match &insts[i].src1 {
                Some(Operand::ConstIdx(idx)) => match &constants[*idx] {
                    IRConst::Int(v) => Some(*v),
                    _ => None,
                },
                _ => None,
            };
            if let Some(v) = cond {
                let label = insts[i].src2.clone().unwrap();
                let always_jump = if is_false { v == 0 } else { v != 0 };
                if always_jump {
                    insts[i].op = Op::Jump;
                    insts[i].src1 = Some(label);
                    insts[i].src2 = None;
                    changed = true;
                } else {
                    insts.remove(i);
                    changed = true;
                    continue;
                }
            }
        }
        i += 1;
    }
    changed
}

fn replace_operand(slot: &mut Option<Operand>, from: &Operand, to: &Operand) -> bool {
    match slot {
        Some(op) if op == from => {
            *op = to.clone();
            true
        }
        _ => false,
    }
}

fn pass_copy_prop(insts: &mut [Instruction]) -> bool {
    let mut changed = false;
    let mut i = 0;
    while i < insts.len() {
        let (src, dst) = match (
            insts[i].op.clone(),
            insts[i].src1.clone(),
            insts[i].dst.clone(),
        ) {
            (Op::Move | Op::FMove, Some(src), Some(dst @ Operand::Temp(..))) => (src, dst),
            _ => {
                i += 1;
                continue;
            }
        };
        if !matches!(src, Operand::Temp(_, _) | Operand::ConstIdx(_)) || src == dst {
            i += 1;
            continue;
        }
        let mut replaced = false;
        let mut j = i + 1;
        while j < insts.len() {
            let inst = &mut insts[j];
            if matches!(inst.op, Op::Label(_) | Op::Jump | Op::Return(_)) {
                break;
            }
            if inst.dst.as_ref() == Some(&src) {
                break;
            }
            if matches!(inst.op, Op::JumpIfFalse | Op::JumpIfTrue) {
                replaced |= replace_operand(&mut inst.src1, &dst, &src);
                break;
            }

            if matches!(inst.op, Op::StrCat | Op::Lea) {
                j += 1;
                continue;
            }
            replaced |= replace_operand(&mut inst.src1, &dst, &src);
            replaced |= replace_operand(&mut inst.src2, &dst, &src);
            j += 1;
        }
        if replaced {
            changed = true;
        }
        i += 1;
    }
    changed
}

fn is_pure(op: &Op) -> bool {
    matches!(
        op,
        Op::Move
            | Op::FMove
            | Op::Load
            | Op::FLoad
            | Op::GlobLoad
            | Op::FGlobLoad
            | Op::LoadAt
            | Op::Add
            | Op::FAdd
            | Op::Sub
            | Op::FSub
            | Op::Mul
            | Op::FMul
            | Op::Div
            | Op::FDiv
            | Op::Mod
            | Op::Eq
            | Op::FEq
            | Op::Ne
            | Op::FNe
            | Op::Gt
            | Op::FGt
            | Op::Ge
            | Op::FGe
            | Op::Lt
            | Op::FLt
            | Op::Le
            | Op::FLe
            | Op::StrEq
            | Op::StrNe
            | Op::StrLt
            | Op::StrLe
            | Op::StrGt
            | Op::StrGe
            | Op::LAnd
            | Op::LOr
            | Op::Xor
            | Op::Neg
            | Op::FNeg
            | Op::Not
            | Op::Inc
            | Op::Dec
            | Op::SizeOf
            | Op::IntToFloat
            | Op::FloatToInt
            | Op::ArrayAccess
            | Op::ByteAccess
            | Op::Lea
    )
}

fn collect_used_temps(op: &Operand, constants: &[IRConst], out: &mut HashSet<usize>) {
    match op {
        Operand::Temp(id, _) => {
            out.insert(*id);
        }
        Operand::ConstIdx(idx) => {
            if let IRConst::Array(elems) = &constants[*idx] {
                for e in elems {
                    collect_used_temps(e, constants, out);
                }
            }
        }
        _ => {}
    }
}

fn pass_dead_code(insts: &mut Vec<Instruction>, constants: &[IRConst]) -> bool {
    let mut changed = false;
    let mut i = 0;
    while i < insts.len() {
        if matches!(insts[i].op, Op::Move | Op::FMove)
            && insts[i].dst.is_some()
            && insts[i].dst == insts[i].src1
        {
            insts.remove(i);
            changed = true;
            continue;
        }
        i += 1;
    }

    let mut used: HashSet<usize> = HashSet::new();
    for inst in insts.iter() {
        let dst_reads = matches!(inst.op, Op::ArrayAssign | Op::ByteAssign | Op::StoreAt);
        let mut ops: Vec<Option<&Operand>> = vec![inst.src1.as_ref(), inst.src2.as_ref()];
        if dst_reads {
            ops.push(inst.dst.as_ref());
        }
        for op in ops.into_iter().flatten() {
            collect_used_temps(op, constants, &mut used);
        }
    }

    let mut i = 0;
    while i < insts.len() {
        let removable = is_pure(&insts[i].op);
        let dead = match &insts[i].dst {
            Some(Operand::Temp(id, _)) => !used.contains(id),
            _ => false,
        };
        if removable && dead {
            insts.remove(i);
            changed = true;
            continue;
        }
        i += 1;
    }
    changed
}

fn pass_unreachable(insts: &mut Vec<Instruction>) -> bool {
    let mut changed = false;
    let mut i = 0;
    while i < insts.len() {
        if matches!(insts[i].op, Op::Jump | Op::Return(_)) {
            let mut j = i + 1;
            while j < insts.len() && !matches!(insts[j].op, Op::Label(_)) {
                j += 1;
            }
            if j > i + 1 {
                insts.drain(i + 1..j);
                changed = true;
            }
            i += 1;
        } else {
            i += 1;
        }
    }
    changed
}

fn rewrite_jump_target(inst: &mut Instruction, from: &str, to: &str) -> bool {
    match inst.op.clone() {
        Op::Jump => {
            if let Some(Operand::Label(l)) = &inst.src1 {
                if l == from {
                    inst.src1 = Some(Operand::Label(to.to_string()));
                    return true;
                }
            }
        }
        Op::JumpIfFalse | Op::JumpIfTrue => {
            if let Some(Operand::Label(l)) = &inst.src2 {
                if l == from {
                    inst.src2 = Some(Operand::Label(to.to_string()));
                    return true;
                }
            }
        }
        _ => {}
    }
    false
}

fn pass_jump_to_next(insts: &mut Vec<Instruction>) -> bool {
    let mut changed = false;

    let mut i = 0;
    while i + 1 < insts.len() {
        let redundant = if let (Op::Jump, Op::Label(l2)) = (&insts[i].op, &insts[i + 1].op) {
            matches!(&insts[i].src1, Some(Operand::Label(l1)) if l1 == l2)
        } else {
            false
        };
        if redundant {
            insts.remove(i);
            changed = true;
            continue;
        }
        i += 1;
    }

    let mut i = 0;
    while i + 1 < insts.len() {
        let thread = match (&insts[i].op, &insts[i + 1].op) {
            (Op::Label(l1), Op::Jump) => match &insts[i + 1].src1 {
                Some(Operand::Label(l2)) if l1 != l2 => Some((l1.clone(), l2.clone())),
                _ => None,
            },
            _ => None,
        };
        if let Some((l1, l2)) = thread {
            let mut any = false;
            for inst in insts.iter_mut() {
                if rewrite_jump_target(inst, &l1, &l2) {
                    any = true;
                }
            }
            if any {
                changed = true;
            }
        }
        i += 1;
    }
    changed
}

fn collect_label_refs(insts: &[Instruction]) -> HashSet<String> {
    let mut refs = HashSet::new();
    for inst in insts {
        match inst.op {
            Op::Jump => {
                if let Some(Operand::Label(l)) = &inst.src1 {
                    refs.insert(l.clone());
                }
            }
            Op::JumpIfFalse | Op::JumpIfTrue => {
                if let Some(Operand::Label(l)) = &inst.src2 {
                    refs.insert(l.clone());
                }
            }
            _ => {}
        }
    }
    refs
}

fn pass_dead_labels(insts: &mut Vec<Instruction>) -> bool {
    let referenced = collect_label_refs(insts);
    let mut changed = false;
    let mut i = 0;
    while i < insts.len() {
        if let Op::Label(l) = &insts[i].op {
            if !referenced.contains(l) {
                insts.remove(i);
                changed = true;
                continue;
            }
        }
        i += 1;
    }
    changed
}

fn is_hoistable(op: &Op) -> bool {
    matches!(
        op,
        Op::Move
            | Op::FMove
            | Op::Add
            | Op::FAdd
            | Op::Sub
            | Op::FSub
            | Op::Mul
            | Op::FMul
            | Op::Xor
            | Op::LAnd
            | Op::LOr
            | Op::Eq
            | Op::Ne
            | Op::Gt
            | Op::Ge
            | Op::Lt
            | Op::Le
            | Op::FEq
            | Op::FNe
            | Op::FGt
            | Op::FGe
            | Op::FLt
            | Op::FLe
            | Op::Neg
            | Op::FNeg
            | Op::Not
            | Op::IntToFloat
            | Op::FloatToInt
    )
}

fn operands_available(inst: &Instruction, available: &HashSet<usize>) -> bool {
    let ok = |op: &Option<Operand>| -> bool {
        match op {
            None => true,

            Some(Operand::Temp(id, _)) => available.contains(id),

            Some(Operand::ConstIdx(_))
            | Some(Operand::Label(_))
            | Some(Operand::Function(_))
            | Some(Operand::Global(_)) => true,

            Some(Operand::Var(_)) => false,
        }
    };
    ok(&inst.src1) && ok(&inst.src2)
}

fn pass_licm(insts: &mut Vec<Instruction>) -> bool {
    let label_pos: HashMap<String, usize> = insts
        .iter()
        .enumerate()
        .filter_map(|(i, inst)| match &inst.op {
            Op::Label(l) => Some((l.clone(), i)),
            _ => None,
        })
        .collect();
    for j in 0..insts.len() {
        if let Op::Jump = &insts[j].op {
            if let Some(Operand::Label(l)) = &insts[j].src1 {
                if let Some(&i) = label_pos.get(l) {
                    if i < j {
                        return hoist_loop(insts, i, j);
                    }
                }
            }
        }
    }
    false
}

fn hoist_loop(insts: &mut Vec<Instruction>, header: usize, back: usize) -> bool {
    let mut available: HashSet<usize> = HashSet::new();
    for inst in insts.iter().take(header) {
        if let Some(Operand::Temp(id, _)) = &inst.dst {
            available.insert(*id);
        }
    }

    let mut to_hoist: Vec<(usize, Instruction)> = Vec::new();
    let mut back = back;
    let mut progress = true;
    while progress {
        progress = false;
        let mut k = header + 1;
        while k < back {
            let inst = &insts[k];
            if is_hoistable(&inst.op) && operands_available(inst, &available) {
                let cloned = inst.clone();
                if let Some(Operand::Temp(id, _)) = &cloned.dst {
                    available.insert(*id);
                }
                to_hoist.push((k, cloned));
                insts.remove(k);
                back -= 1;
                progress = true;
            } else {
                k += 1;
            }
        }
    }

    if to_hoist.is_empty() {
        return false;
    }
    to_hoist.sort_by_key(|(k, _)| *k);
    for (_, inst) in to_hoist.into_iter().rev() {
        insts.insert(header, inst);
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn inst(
        op: Op,
        dst: Option<Operand>,
        src1: Option<Operand>,
        src2: Option<Operand>,
    ) -> Instruction {
        Instruction {
            op,
            dst,
            src1,
            src2,
        }
    }

    fn temp(id: usize, ty: IRType) -> Operand {
        Operand::Temp(id, ty)
    }

    fn run(insts: Vec<Instruction>) -> (Vec<Instruction>, Vec<IRConst>) {
        let mut program = IRProgram {
            functions: vec![IRFunction {
                name: "f".into(),
                params: vec![],
                ret_type: IRType::Int,
                is_pub: false,
                is_external: false,
                instructions: insts,
            }],
            constants: vec![
                IRConst::Int(0),
                IRConst::Int(1),
                IRConst::Int(2),
                IRConst::Int(3),
                IRConst::Int(5),
            ],
            extern_vars: vec![],
            global_vars: vec![],
        };
        optimize(&mut program);
        let func = program.functions.pop().unwrap();
        (func.instructions, program.constants)
    }

    fn const_int_at(constants: &[IRConst], idx: usize) -> i64 {
        match &constants[idx] {
            IRConst::Int(v) => *v,
            other => panic!("expected int constant, got {:?}", other),
        }
    }

    fn cidx(value: i64) -> usize {
        match value {
            0 => 0,
            1 => 1,
            2 => 2,
            3 => 3,
            5 => 4,
            _ => panic!("seed only contains 0,1,2,3,5"),
        }
    }

    fn is_move_of(inst: &Instruction, constants: &[IRConst], value: i64) -> bool {
        matches!(
            inst,
            Instruction {
                op: Op::Move,
                src1: Some(Operand::ConstIdx(idx)),
                src2: None,
                ..
            } if const_int_at(constants, *idx) == value
        )
    }

    #[test]
    fn folds_int_arith() {
        let (insts, constants) = run(vec![
            inst(
                Op::Add,
                Some(temp(0, IRType::Int)),
                Some(Operand::ConstIdx(cidx(2))),
                Some(Operand::ConstIdx(cidx(3))),
            ),
            inst(
                Op::Return("rax".into()),
                None,
                Some(temp(0, IRType::Int)),
                None,
            ),
        ]);
        assert_eq!(insts.len(), 2);
        assert!(is_move_of(&insts[0], &constants, 5), "got {:?}", insts[0]);
    }

    #[test]
    fn folds_comparison() {
        let (insts, constants) = run(vec![
            inst(
                Op::Gt,
                Some(temp(0, IRType::Bool)),
                Some(Operand::ConstIdx(cidx(5))),
                Some(Operand::ConstIdx(cidx(2))),
            ),
            inst(
                Op::Return("rax".into()),
                None,
                Some(temp(0, IRType::Int)),
                None,
            ),
        ]);
        assert_eq!(insts.len(), 2);
        assert!(is_move_of(&insts[0], &constants, 1), "got {:?}", insts[0]);
    }

    #[test]
    fn does_not_fold_div_by_zero() {
        let (insts, _) = run(vec![
            inst(
                Op::Div,
                Some(temp(0, IRType::Int)),
                Some(Operand::ConstIdx(cidx(5))),
                Some(Operand::ConstIdx(cidx(0))),
            ),
            inst(
                Op::Return("rax".into()),
                None,
                Some(temp(0, IRType::Int)),
                None,
            ),
        ]);
        assert_eq!(insts.len(), 2);
        assert!(matches!(insts[0].op, Op::Div));
    }

    #[test]
    fn simplifies_x_plus_zero() {
        let (insts, _) = run(vec![
            inst(
                Op::Add,
                Some(temp(0, IRType::Int)),
                Some(temp(1, IRType::Int)),
                Some(Operand::ConstIdx(cidx(0))),
            ),
            inst(
                Op::Return("rax".into()),
                None,
                Some(temp(0, IRType::Int)),
                None,
            ),
        ]);
        assert_eq!(insts.len(), 2);
        assert!(matches!(
            &insts[0],
            Instruction {
                op: Op::Move,
                src1: Some(Operand::Temp(1, _)),
                src2: None,
                ..
            }
        ));
    }

    #[test]
    fn simplifies_sub_zero_to_neg() {
        let (insts, _) = run(vec![
            inst(
                Op::Sub,
                Some(temp(0, IRType::Int)),
                Some(Operand::ConstIdx(cidx(0))),
                Some(temp(1, IRType::Int)),
            ),
            inst(
                Op::Return("rax".into()),
                None,
                Some(temp(0, IRType::Int)),
                None,
            ),
        ]);
        assert_eq!(insts.len(), 2);
        assert!(matches!(
            &insts[0],
            Instruction {
                op: Op::Neg,
                src1: Some(Operand::Temp(1, _)),
                ..
            }
        ));
    }

    #[test]
    fn folds_constant_branch() {
        let (insts, _) = run(vec![
            inst(
                Op::JumpIfFalse,
                None,
                Some(Operand::ConstIdx(cidx(1))),
                Some(Operand::Label("L".into())),
            ),
            inst(Op::Label("L".into()), None, None, None),
        ]);
        assert_eq!(insts.len(), 0);

        let (insts, _) = run(vec![
            inst(
                Op::JumpIfFalse,
                None,
                Some(Operand::ConstIdx(cidx(0))),
                Some(Operand::Label("L".into())),
            ),
            inst(Op::Label("L".into()), None, None, None),
        ]);
        assert_eq!(insts.len(), 0);
    }

    #[test]
    fn removes_dead_pure_instruction() {
        let (insts, _) = run(vec![
            inst(
                Op::Move,
                Some(temp(0, IRType::Int)),
                Some(Operand::ConstIdx(cidx(5))),
                None,
            ),
            inst(
                Op::Add,
                Some(temp(1, IRType::Int)),
                Some(temp(2, IRType::Int)),
                Some(Operand::ConstIdx(cidx(1))),
            ),
            inst(
                Op::Return("rax".into()),
                None,
                Some(temp(1, IRType::Int)),
                None,
            ),
        ]);
        assert_eq!(insts.len(), 2);
        assert!(matches!(insts[0].op, Op::Add));
        assert!(matches!(insts[1].op, Op::Return(_)));
    }

    #[test]
    fn copy_propagates_constant() {
        let (insts, constants) = run(vec![
            inst(
                Op::Move,
                Some(temp(0, IRType::Int)),
                Some(Operand::ConstIdx(cidx(5))),
                None,
            ),
            inst(
                Op::Add,
                Some(temp(1, IRType::Int)),
                Some(temp(0, IRType::Int)),
                Some(temp(2, IRType::Int)),
            ),
            inst(
                Op::Return("rax".into()),
                None,
                Some(temp(1, IRType::Int)),
                None,
            ),
        ]);
        assert_eq!(insts.len(), 2);
        assert!(matches!(
            &insts[0],
            Instruction {
                op: Op::Add,
                src1: Some(Operand::ConstIdx(idx)),
                ..
            } if const_int_at(&constants, *idx) == 5
        ));
        assert!(matches!(insts[1].op, Op::Return(_)));
    }

    #[test]
    fn removes_unreachable_after_return() {
        let (insts, _) = run(vec![
            inst(
                Op::Return("rax".into()),
                None,
                Some(temp(0, IRType::Int)),
                None,
            ),
            inst(
                Op::Move,
                Some(temp(1, IRType::Int)),
                Some(Operand::ConstIdx(cidx(1))),
                None,
            ),
            inst(Op::Jump, None, Some(Operand::Label("L".into())), None),
            inst(Op::Label("L".into()), None, None, None),
        ]);
        assert_eq!(insts.len(), 1);
        assert!(matches!(insts[0].op, Op::Return(_)));
    }

    #[test]
    fn removes_jump_to_next_label() {
        let (insts, _) = run(vec![
            inst(Op::Jump, None, Some(Operand::Label("L".into())), None),
            inst(Op::Label("L".into()), None, None, None),
            inst(
                Op::Return("rax".into()),
                None,
                Some(temp(0, IRType::Int)),
                None,
            ),
        ]);
        assert_eq!(insts.len(), 1);
        assert!(matches!(insts[0].op, Op::Return(_)));
    }

    #[test]
    fn threads_jump_chains() {
        let (insts, _) = run(vec![
            inst(Op::Jump, None, Some(Operand::Label("A".into())), None),
            inst(Op::Label("A".into()), None, None, None),
            inst(Op::Jump, None, Some(Operand::Label("B".into())), None),
            inst(Op::Label("B".into()), None, None, None),
            inst(
                Op::Return("rax".into()),
                None,
                Some(temp(0, IRType::Int)),
                None,
            ),
        ]);
        assert_eq!(insts.len(), 1);
        assert!(matches!(insts[0].op, Op::Return(_)));
    }

    #[test]
    fn hoists_loop_invariants() {
        let (insts, _) = run(vec![
            inst(
                Op::Load,
                Some(temp(0, IRType::Int)),
                Some(Operand::Var("v0".into())),
                None,
            ),
            inst(Op::Label("L".into()), None, None, None),
            inst(
                Op::Add,
                Some(temp(1, IRType::Int)),
                Some(temp(0, IRType::Int)),
                Some(Operand::ConstIdx(cidx(1))),
            ),
            inst(
                Op::Store,
                Some(Operand::Var("v".into())),
                Some(temp(1, IRType::Int)),
                None,
            ),
            inst(Op::Jump, None, Some(Operand::Label("L".into())), None),
        ]);
        let label_idx = insts
            .iter()
            .position(|i| matches!(i.op, Op::Label(_)))
            .unwrap();
        assert!(
            label_idx >= 2,
            "expected the invariant add hoisted before the loop label (label at {}): {:?}",
            label_idx,
            insts
        );
        assert!(matches!(insts[label_idx - 1].op, Op::Add));
    }

    #[test]
    fn does_not_hoist_loop_varying_operand() {
        let (insts, _) = run(vec![
            inst(Op::Label("L".into()), None, None, None),
            inst(
                Op::Add,
                Some(temp(1, IRType::Int)),
                Some(temp(0, IRType::Int)),
                Some(Operand::ConstIdx(cidx(1))),
            ),
            inst(Op::Jump, None, Some(Operand::Label("L".into())), None),
        ]);

        let label_idx = insts
            .iter()
            .position(|i| matches!(i.op, Op::Label(_)))
            .unwrap();
        assert_eq!(label_idx, 0, "label should stay first: {:?}", insts);
    }
}
