use super::types::*;
use std::{collections::HashSet, iter::once};

pub fn optimize(asms: &mut Vec<Asm>) {
    loop {
        let mut changed = false;

        changed |= pass_dead_labels(asms);
        changed |= pass_redundant_jmp(asms);
        changed |= pass_jmp_chain(asms);
        changed |= pass_mov_imm_to_mem_merge(asms);
        changed |= pass_redundant_mov(asms);
        changed |= pass_mov_chain(asms);
        changed |= pass_dead_mov(asms);
        changed |= pass_cmp_zero_to_test(asms);
        changed |= pass_const_test_jcc(asms);
        changed |= pass_add_sub_xor_zero(asms);
        changed |= pass_push_pop(asms);
        changed |= pass_mov_mov_swap(asms);
        changed |= pass_self_mov(asms);
        changed |= pass_redundant_ret(asms);

        if !changed {
            break;
        }
    }
}

fn pass_mov_imm_to_mem_merge(asms: &mut Vec<Asm>) -> bool {
    let mut changed = false;
    let mut i = 0;
    while i + 1 < asms.len() {
        let a = asms[i].clone();
        let b = asms[i + 1].clone();
        match (a, b) {
            (
                Asm::Mov(Operand::Reg(r1), Operand::Imm(v)),
                Asm::Mov(Operand::Mem(m), Operand::Reg(r2)),
            ) if r1 == r2 && !register_is_used_after(asms, i + 2, r1) => {
                asms.splice(i..i + 2, once(Asm::Mov(Operand::Mem(m), Operand::Imm(v))));
                changed = true;
                continue;
            }
            _ => {}
        }
        i += 1;
    }
    changed
}

fn register_in_operand(op: &Operand, reg: Reg) -> bool {
    match op {
        Operand::Reg(r) => *r == reg,
        Operand::Mem(m) => m.base == Some(reg) || m.index == Some(reg),
        _ => false,
    }
}

fn operand_reads(asm: &Asm, reg: Reg) -> bool {
    match asm {
        Asm::Mov(dst, src) => {
            register_in_operand(src, reg)
                || matches!(dst, Operand::Mem(m) if m.base == Some(reg) || m.index == Some(reg))
        }
        Asm::Add(a, b)
        | Asm::Sub(a, b)
        | Asm::Imul(a, b)
        | Asm::Xor(a, b)
        | Asm::Or(a, b)
        | Asm::And(a, b)
        | Asm::Cmp(a, b)
        | Asm::Lea(a, b)
        | Asm::Movsd(a, b)
        | Asm::Addsd(a, b)
        | Asm::Subsd(a, b)
        | Asm::Mulsd(a, b)
        | Asm::Divsd(a, b) => register_in_operand(a, reg) || register_in_operand(b, reg),
        Asm::Movzx(_, src) => register_in_operand(src, reg),
        Asm::Movb(dst, src) => {
            *src == reg
                || matches!(dst, Operand::Mem(m) if m.base == Some(reg) || m.index == Some(reg))
        }
        Asm::Xorpd(a, b) => *a == reg || register_in_operand(b, reg),
        Asm::Ucomisd(a, b) => *a == reg || *b == reg,
        Asm::Push(r) | Asm::Neg(r) | Asm::Inc(r) | Asm::Dec(r) | Asm::Idiv(r) => *r == reg,
        Asm::Call(Operand::Reg(r)) => *r == reg,
        Asm::Cqo | Asm::Cdqe => reg == Reg::Rax,
        Asm::Ret => reg == Reg::Rax,
        _ => false,
    }
}

fn operand_writes(asm: &Asm, reg: Reg) -> bool {
    match asm {
        Asm::Mov(dst, _) | Asm::Movsd(dst, _) => {
            matches!(dst, Operand::Reg(r) if *r == reg)
        }
        Asm::Movzx(dst, _) => *dst == reg,
        Asm::Lea(dst, _) => matches!(dst, Operand::Reg(r) if *r == reg),
        Asm::Pop(r)
        | Asm::Sete(r)
        | Asm::Setne(r)
        | Asm::Setg(r)
        | Asm::Setge(r)
        | Asm::Setl(r)
        | Asm::Setle(r)
        | Asm::Seta(r)
        | Asm::Setae(r)
        | Asm::Setb(r)
        | Asm::Setbe(r) => *r == reg,
        _ => false,
    }
}

fn register_is_used_after(asms: &[Asm], start: usize, reg: Reg) -> bool {
    for asm in asms.iter().skip(start) {
        match asm {
            Asm::Ret => return reg == Reg::Rax,
            Asm::Call(_) => return true,
            _ => {}
        }
        if asm.jump_target().is_some() {
            return true;
        }
        if operand_reads(asm, reg) {
            return true;
        }
        if operand_writes(asm, reg) {
            return false;
        }
    }
    false
}

fn pass_redundant_mov(asms: &mut Vec<Asm>) -> bool {
    let mut changed = false;
    let mut i = 0;
    while i < asms.len() {
        if let Asm::Mov(Operand::Reg(a), Operand::Reg(b)) = &asms[i] {
            if a == b {
                asms.remove(i);
                changed = true;
                continue;
            }
        }
        i += 1;
    }
    changed
}

fn pass_mov_mov_swap(asms: &mut Vec<Asm>) -> bool {
    let mut changed = false;
    let mut i = 1;
    while i < asms.len() {
        if let (
            Asm::Mov(Operand::Reg(a1), Operand::Reg(b1)),
            Asm::Mov(Operand::Reg(a2), Operand::Reg(b2)),
        ) = (&asms[i - 1], &asms[i])
        {
            if a1 == b2 && b1 == a2 && a1 != b1 {
                asms.remove(i);
                changed = true;
                continue;
            }
        }
        i += 1;
    }
    changed
}

fn pass_mov_chain(asms: &mut Vec<Asm>) -> bool {
    let mut changed = false;
    let mut i = 0;
    while i + 1 < asms.len() {
        let a = asms[i].clone();
        let b = asms[i + 1].clone();
        match (a, b) {
            (Asm::Mov(Operand::Reg(r1), src), Asm::Mov(Operand::Reg(r2), Operand::Reg(r3)))
                if r1 == r3 && r2 != r1 && r1 != Reg::Rsp && r2 != Reg::Rsp =>
            {
                let fwd = match &src {
                    Operand::Imm(_) => true,
                    Operand::Reg(r) => *r != r2 && *r != Reg::Rsp,
                    _ => false,
                };
                if fwd {
                    let src = src.clone();
                    let r1_dead = !register_is_used_after(asms, i + 2, r1);
                    asms[i + 1] = Asm::Mov(Operand::Reg(r2), src);
                    changed = true;
                    if r1_dead {
                        asms.remove(i);
                        continue;
                    }
                }
            }
            _ => {}
        }
        i += 1;
    }
    changed
}

fn pass_dead_mov(asms: &mut Vec<Asm>) -> bool {
    let mut changed = false;
    let mut i = 0;
    while i < asms.len() {
        if let Asm::Mov(Operand::Reg(r), _) = &asms[i] {
            if *r != Reg::Rsp && !register_is_used_after(asms, i + 1, *r) {
                asms.remove(i);
                changed = true;
                continue;
            }
        }
        i += 1;
    }
    changed
}

fn pass_cmp_zero_to_test(asms: &mut Vec<Asm>) -> bool {
    let mut changed = false;
    for asm in asms.iter_mut() {
        if let Asm::Cmp(Operand::Reg(r), Operand::Imm(0)) = asm {
            *asm = Asm::Test(*r);
            changed = true;
        }
    }
    changed
}

fn pass_const_test_jcc(asms: &mut Vec<Asm>) -> bool {
    let mut changed = false;
    let mut i = 0;
    while i < asms.len() {
        let (reg, val) = match &asms[i] {
            Asm::Mov(Operand::Reg(r), Operand::Imm(v)) if *r != Reg::Rsp && *r != Reg::Rbp => {
                (*r, *v)
            }
            _ => {
                i += 1;
                continue;
            }
        };
        let mut j = i + 1;
        let mut test_at = None;
        while j < asms.len() {
            let is_test = match &asms[j] {
                Asm::Test(tr) => *tr == reg,
                Asm::Cmp(Operand::Reg(cr), Operand::Imm(0)) => *cr == reg,
                _ => false,
            };
            if is_test {
                test_at = Some(j);
                break;
            }
            if operand_writes(&asms[j], reg)
                || matches!(&asms[j], Asm::Ret | Asm::Call(_))
                || asms[j].jump_target().is_some()
            {
                break;
            }
            j += 1;
        }
        if let Some(j) = test_at {
            if let Asm::Je(l) = &asms[j + 1] {
                let l = l.clone();
                if val == 0 {
                    asms[j + 1] = Asm::Jmp(l);
                } else {
                    asms.remove(j + 1);
                }
                changed = true;
                continue;
            }
        }
        i += 1;
    }
    changed
}

fn pass_jmp_chain(asms: &mut Vec<Asm>) -> bool {
    let mut changed = false;
    loop {
        let mut forward: Vec<(String, String)> = Vec::new();
        for w in asms.windows(2) {
            if let (Asm::Label(l), Asm::Jmp(t)) = (&w[0], &w[1]) {
                forward.push((l.clone(), t.clone()));
            }
        }
        if forward.is_empty() {
            break;
        }
        let mut any = false;
        for asm in asms.iter_mut() {
            let target = match &*asm {
                Asm::Call(Operand::Label(l)) => Some(l.clone()),
                other => other.jump_target().cloned(),
            };
            if let Some(l) = target {
                let mut chain = l.clone();
                let mut seen = HashSet::new();
                while let Some((_, t)) = forward.iter().find(|(s, _)| *s == chain) {
                    if !seen.insert(chain.clone()) {
                        break;
                    }
                    chain = t.clone();
                }
                if l != chain {
                    set_jump_target(asm, &chain);
                    any = true;
                }
            }
        }
        let referenced = collect_label_refs(asms);
        let globals: HashSet<String> = asms
            .iter()
            .filter_map(|a| {
                if let Asm::Global(name) = a {
                    Some(name.clone())
                } else {
                    None
                }
            })
            .collect();
        let mut i = 0;
        while i < asms.len() {
            if let Asm::Label(l) = &asms[i] {
                let unused = !referenced.contains(l) && !globals.contains(l);
                let fallen_into = i == 0 || matches!(asms[i - 1], Asm::Ret | Asm::Jmp(_));
                if unused && fallen_into {
                    asms.remove(i);
                    while i < asms.len() && !matches!(asms[i], Asm::Label(_)) {
                        if matches!(
                            asms[i],
                            Asm::Global(_) | Asm::Extern(_) | Asm::Section(_) | Asm::Align(_)
                        ) {
                            break;
                        }
                        asms.remove(i);
                    }
                    any = true;
                    continue;
                }
            }
            i += 1;
        }
        if !any {
            break;
        }
        changed = true;
    }
    changed
}

fn pass_self_mov(asms: &mut Vec<Asm>) -> bool {
    let mut changed = false;
    let mut i = 0;
    while i < asms.len() {
        if let Asm::Mov(Operand::Reg(r1), Operand::Reg(r2)) = &asms[i] {
            if r1 == r2 {
                asms.remove(i);
                changed = true;
                continue;
            }
        }
        i += 1;
    }
    changed
}

fn pass_redundant_ret(asms: &mut Vec<Asm>) -> bool {
    let mut changed = false;
    let mut i = 1;
    while i < asms.len() {
        if matches!(asms[i - 1], Asm::Ret) && matches!(asms[i], Asm::Ret) {
            asms.remove(i);
            changed = true;
            continue;
        }
        i += 1;
    }
    changed
}

fn set_jump_target(asm: &mut Asm, label: &str) {
    match asm {
        Asm::Jmp(l)
        | Asm::Je(l)
        | Asm::Jne(l)
        | Asm::Jl(l)
        | Asm::Jle(l)
        | Asm::Jg(l)
        | Asm::Jge(l)
        | Asm::Ja(l)
        | Asm::Jae(l)
        | Asm::Jb(l)
        | Asm::Jbe(l) => *l = label.to_string(),
        Asm::Call(Operand::Label(l)) => *l = label.to_string(),
        _ => {}
    }
}

fn pass_redundant_jmp(asms: &mut Vec<Asm>) -> bool {
    let mut changed = false;
    let mut i = 1;
    while i < asms.len() {
        if let Asm::Jmp(lbl) = &asms[i - 1] {
            if let Asm::Label(next) = &asms[i] {
                if lbl == next {
                    asms.remove(i - 1);
                    changed = true;
                    continue;
                }
            }
        }
        i += 1;
    }
    changed
}

fn pass_add_sub_xor_zero(asms: &mut Vec<Asm>) -> bool {
    let mut changed = false;
    let mut i = 0;
    while i < asms.len() {
        let remove = match &asms[i] {
            Asm::Add(Operand::Reg(_), Operand::Imm(0))
            | Asm::Sub(Operand::Reg(_), Operand::Imm(0))
            | Asm::Xor(Operand::Reg(_), Operand::Imm(0)) => true,
            _ => false,
        };
        if remove {
            asms.remove(i);
            changed = true;
            continue;
        }
        i += 1;
    }
    changed
}

fn pass_push_pop(asms: &mut Vec<Asm>) -> bool {
    let mut changed = false;
    let mut i = 1;
    while i < asms.len() {
        if let (Asm::Push(a), Asm::Pop(b)) = (&asms[i - 1], &asms[i]) {
            if a == b {
                asms.remove(i - 1);
                asms.remove(i - 1);
                changed = true;
                continue;
            }
        }
        i += 1;
    }
    changed
}

fn pass_dead_labels(asms: &mut Vec<Asm>) -> bool {
    let referenced: HashSet<String> = collect_label_refs(asms);
    let globals: HashSet<String> = asms
        .iter()
        .filter_map(|a| {
            if let Asm::Global(name) = a {
                Some(name.clone())
            } else {
                None
            }
        })
        .collect();

    let mut changed = false;
    let mut i = 0;
    while i < asms.len() {
        if let Asm::Label(lbl) = &asms[i] {
            if !referenced.contains(lbl) && !globals.contains(lbl) {
                asms.remove(i);
                changed = true;
                continue;
            }
        }
        i += 1;
    }
    changed
}

fn collect_label_refs(asms: &[Asm]) -> HashSet<String> {
    let mut refs = HashSet::new();
    for asm in asms {
        if let Some(l) = asm.jump_target() {
            refs.insert(l.clone());
        }
        match asm {
            Asm::Call(op) => {
                if let Operand::Label(l) = op {
                    refs.insert(l.clone());
                }
            }
            _ => {}
        }
        collect_operand_labels(asm, &mut refs);
    }
    refs
}

fn collect_operand_labels(asm: &Asm, refs: &mut HashSet<String>) {
    let mut check = |op: &Operand| {
        if let Operand::Label(l) = op {
            refs.insert(l.clone());
        }
        if let Operand::DataLabel(l) = op {
            refs.insert(l.clone());
        }
    };
    match asm {
        Asm::Mov(a, b)
        | Asm::Add(a, b)
        | Asm::Sub(a, b)
        | Asm::Imul(a, b)
        | Asm::Xor(a, b)
        | Asm::Or(a, b)
        | Asm::And(a, b)
        | Asm::Cmp(a, b)
        | Asm::Lea(a, b)
        | Asm::Movsd(a, b)
        | Asm::Addsd(a, b)
        | Asm::Subsd(a, b)
        | Asm::Mulsd(a, b)
        | Asm::Divsd(a, b) => {
            check(a);
            check(b);
        }
        Asm::Xorpd(_a, b) => {
            check(b);
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mem(off: i32) -> Operand {
        Operand::Mem(Mem {
            base: Some(Reg::Rbp),
            index: None,
            scale: 0,
            disp: off,
        })
    }

    #[test]
    fn preserves_load_store_sequences() {
        let mut asms = vec![
            Asm::Mov(Operand::Reg(Reg::Rax), mem(-8)),
            Asm::Mov(mem(-8), Operand::Reg(Reg::Rax)),
        ];

        optimize(&mut asms);

        assert_eq!(asms.len(), 2);
        assert!(matches!(
            asms[0],
            Asm::Mov(Operand::Reg(Reg::Rax), Operand::Mem(_))
        ));
        assert!(matches!(
            asms[1],
            Asm::Mov(Operand::Mem(_), Operand::Reg(Reg::Rax))
        ));
    }

    #[test]
    fn preserves_imm_to_mem_sequences_when_register_is_used_later() {
        let mut asms = vec![
            Asm::Mov(Operand::Reg(Reg::Rax), Operand::Imm(1)),
            Asm::Mov(
                Operand::Mem(Mem {
                    base: Some(Reg::Rbp),
                    index: None,
                    scale: 0,
                    disp: -8,
                }),
                Operand::Reg(Reg::Rax),
            ),
            Asm::Add(Operand::Reg(Reg::Rbx), Operand::Reg(Reg::Rax)),
        ];

        optimize(&mut asms);

        assert!(matches!(
            asms[0],
            Asm::Mov(Operand::Reg(Reg::Rax), Operand::Imm(1))
        ));
        assert!(matches!(
            asms[1],
            Asm::Mov(Operand::Mem(_), Operand::Reg(Reg::Rax))
        ));
        assert!(matches!(
            asms[2],
            Asm::Add(Operand::Reg(Reg::Rbx), Operand::Reg(Reg::Rax))
        ));
    }

    #[test]
    fn removes_dead_mov_and_merges_imm_to_mem() {
        let mut asms = vec![
            Asm::Mov(Operand::Reg(Reg::Rax), Operand::Imm(1)),
            Asm::Mov(
                Operand::Mem(Mem {
                    base: Some(Reg::Rbp),
                    index: None,
                    scale: 0,
                    disp: -8,
                }),
                Operand::Reg(Reg::Rax),
            ),
            Asm::Mov(Operand::Reg(Reg::Rbx), Operand::Reg(Reg::Rax)),
        ];

        optimize(&mut asms);

        assert_eq!(asms.len(), 1);
        assert!(matches!(
            asms[0],
            Asm::Mov(Operand::Mem(_), Operand::Imm(1))
        ));
    }

    #[test]
    fn propagates_mov_chains() {
        let mut asms = vec![
            Asm::Mov(Operand::Reg(Reg::R11), Operand::Imm(5)),
            Asm::Mov(Operand::Reg(Reg::R8), Operand::Reg(Reg::R11)),
            Asm::Mov(Operand::Reg(Reg::R9), Operand::Reg(Reg::R8)),
            Asm::Mov(Operand::Reg(Reg::Rax), Operand::Reg(Reg::R9)),
            Asm::Ret,
        ];
        optimize(&mut asms);
        assert_eq!(asms.len(), 2);
        assert!(matches!(
            asms[0],
            Asm::Mov(Operand::Reg(Reg::Rax), Operand::Imm(5))
        ));
        assert!(matches!(asms[1], Asm::Ret));
    }

    #[test]
    fn forwards_chain_but_keeps_live_src() {
        let mut asms = vec![
            Asm::Mov(Operand::Reg(Reg::Rax), Operand::Reg(Reg::R8)),
            Asm::Mov(Operand::Reg(Reg::R9), Operand::Reg(Reg::Rax)),
            Asm::Push(Reg::Rax),
            Asm::Ret,
        ];
        optimize(&mut asms);
        assert_eq!(asms.len(), 3);
        assert!(matches!(
            asms[0],
            Asm::Mov(Operand::Reg(Reg::Rax), Operand::Reg(Reg::R8))
        ));
        assert!(matches!(asms[1], Asm::Push(Reg::Rax)));
    }

    #[test]
    fn converts_cmp_imm0_to_test() {
        let mut asms = vec![
            Asm::Mov(Operand::Reg(Reg::R8), Operand::Reg(Reg::Rdi)),
            Asm::Cmp(Operand::Reg(Reg::R8), Operand::Imm(0)),
            Asm::Je("L".to_string()),
        ];
        optimize(&mut asms);
        assert!(matches!(asms[1], Asm::Test(Reg::R8)));
    }

    #[test]
    fn keeps_cmp_with_nonzero_imm() {
        let mut asms = vec![
            Asm::Cmp(Operand::Reg(Reg::R8), Operand::Imm(1)),
            Asm::Je("L".to_string()),
        ];
        optimize(&mut asms);
        assert!(matches!(
            asms[0],
            Asm::Cmp(Operand::Reg(Reg::R8), Operand::Imm(1))
        ));
    }

    #[test]
    fn removes_jmp_to_following_label_without_other_refs() {
        let mut asms = vec![
            Asm::Jmp(".L_exit".to_string()),
            Asm::Label(".L_exit".to_string()),
            Asm::Mov(Operand::Reg(Reg::Rsp), Operand::Reg(Reg::Rbp)),
            Asm::Ret,
        ];
        optimize(&mut asms);
        assert_eq!(asms.len(), 2);
        assert!(matches!(asms[0], Asm::Mov(Operand::Reg(_), _)));
    }

    #[test]
    fn swap_with_volatile_reg_is_removable() {
        let mut asms = vec![
            Asm::Mov(Operand::Reg(Reg::Rax), Operand::Reg(Reg::R8)),
            Asm::Mov(Operand::Reg(Reg::R8), Operand::Reg(Reg::Rax)),
            Asm::Ret,
        ];
        optimize(&mut asms);
        assert_eq!(asms.len(), 2);
        assert!(matches!(
            asms[0],
            Asm::Mov(Operand::Reg(Reg::Rax), Operand::Reg(Reg::R8))
        ));
    }

    #[test]
    fn preserves_imm_to_reg_for_return_value() {
        let mut asms = vec![
            Asm::Mov(Operand::Reg(Reg::Rax), Operand::Imm(0)),
            Asm::Mov(
                Operand::Mem(Mem {
                    base: Some(Reg::Rbp),
                    index: None,
                    scale: 0,
                    disp: -24,
                }),
                Operand::Reg(Reg::Rax),
            ),
            Asm::Ret,
        ];

        optimize(&mut asms);

        assert_eq!(asms.len(), 3);
        assert!(matches!(
            asms[0],
            Asm::Mov(Operand::Reg(Reg::Rax), Operand::Imm(0))
        ));
        assert!(matches!(
            asms[1],
            Asm::Mov(Operand::Mem(_), Operand::Reg(Reg::Rax))
        ));
        assert!(matches!(asms[2], Asm::Ret));
    }

    #[test]
    fn folds_const_zero_jcc_to_jmp() {
        let mut asms = vec![
            Asm::Mov(Operand::Reg(Reg::Rax), Operand::Imm(0)),
            Asm::Test(Reg::Rax),
            Asm::Je("L".to_string()),
            Asm::Label("L".to_string()),
            Asm::Ret,
        ];

        optimize(&mut asms);

        assert_eq!(asms.len(), 3);
        assert!(matches!(
            asms[0],
            Asm::Mov(Operand::Reg(Reg::Rax), Operand::Imm(0))
        ));
        assert!(matches!(asms[1], Asm::Test(_)));
        assert!(matches!(asms[2], Asm::Ret));
        assert!(!asms.iter().any(|a| matches!(a, Asm::Je(_) | Asm::Jmp(_))));
    }

    #[test]
    fn removes_const_nonzero_jcc() {
        let mut asms = vec![
            Asm::Mov(Operand::Reg(Reg::Rax), Operand::Imm(5)),
            Asm::Test(Reg::Rax),
            Asm::Je("L".to_string()),
            Asm::Mov(Operand::Reg(Reg::Rax), Operand::Imm(1)),
            Asm::Ret,
        ];

        optimize(&mut asms);

        assert_eq!(asms.len(), 3);
        assert!(!asms.iter().any(|a| matches!(a, Asm::Je(_))));
    }

    #[test]
    fn keeps_jcc_when_reg_rewritten_between() {
        let mut asms = vec![
            Asm::Mov(Operand::Reg(Reg::Rax), Operand::Imm(5)),
            Asm::Mov(Operand::Reg(Reg::Rax), Operand::Reg(Reg::R8)),
            Asm::Test(Reg::Rax),
            Asm::Je("L".to_string()),
            Asm::Label("L".to_string()),
            Asm::Ret,
        ];

        optimize(&mut asms);

        assert!(asms.iter().any(|a| matches!(a, Asm::Je(_))));
    }

    #[test]
    fn threads_jmp_chains_and_removes_dead_block() {
        let mut asms = vec![
            Asm::Jmp("A".to_string()),
            Asm::Label("A".to_string()),
            Asm::Jmp("B".to_string()),
            Asm::Label("B".to_string()),
            Asm::Mov(Operand::Reg(Reg::Rax), Operand::Imm(7)),
            Asm::Ret,
        ];

        optimize(&mut asms);

        assert_eq!(asms.len(), 2);
        assert!(matches!(
            asms[0],
            Asm::Mov(Operand::Reg(Reg::Rax), Operand::Imm(7))
        ));
        assert!(matches!(asms[1], Asm::Ret));
        assert!(
            !asms
                .iter()
                .any(|a| matches!(a, Asm::Label(_) | Asm::Jmp(_)))
        );
    }

    #[test]
    fn collapses_repeated_ret() {
        let mut asms = vec![Asm::Ret, Asm::Ret, Asm::Ret];

        optimize(&mut asms);

        assert_eq!(asms.len(), 1);
        assert!(matches!(asms[0], Asm::Ret));
    }

    #[test]
    fn removes_self_mov() {
        let mut asms = vec![
            Asm::Mov(Operand::Reg(Reg::Rax), Operand::Reg(Reg::Rax)),
            Asm::Ret,
        ];

        optimize(&mut asms);

        assert_eq!(asms.len(), 1);
        assert!(matches!(asms[0], Asm::Ret));
    }
}
