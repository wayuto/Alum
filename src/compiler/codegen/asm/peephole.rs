use super::types::*;
use std::collections::HashSet;

pub fn optimize(asms: &mut Vec<Asm>) {
    loop {
        let mut changed = false;

        changed |= pass_redundant_jmp(asms);
        changed |= pass_mov_imm_to_mem_merge(asms);
        changed |= pass_redundant_mov(asms);
        changed |= pass_add_sub_xor_zero(asms);
        changed |= pass_push_pop(asms);
        changed |= pass_dead_labels(asms);

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
                asms.splice(
                    i..i + 2,
                    std::iter::once(Asm::Mov(Operand::Mem(m), Operand::Imm(v))),
                );
                changed = true;
                continue;
            }
            _ => {}
        }
        i += 1;
    }
    changed
}

fn register_is_used_after(asms: &[Asm], start: usize, reg: Reg) -> bool {
    asms.iter().skip(start).any(|asm| match asm {
        Asm::Mov(dst, src) => {
            matches!(dst, Operand::Reg(r) if r == &reg)
                || matches!(src, Operand::Reg(r) if r == &reg)
        }
        Asm::Add(Operand::Reg(r), _)
        | Asm::Sub(Operand::Reg(r), _)
        | Asm::Imul(Operand::Reg(r), _)
        | Asm::Xor(Operand::Reg(r), _)
        | Asm::Or(Operand::Reg(r), _)
        | Asm::And(Operand::Reg(r), _)
        | Asm::Cmp(Operand::Reg(r), _)
        | Asm::Lea(Operand::Reg(r), _)
        | Asm::Movsd(Operand::Reg(r), _)
        | Asm::Addsd(Operand::Reg(r), _)
        | Asm::Subsd(Operand::Reg(r), _)
        | Asm::Mulsd(Operand::Reg(r), _)
        | Asm::Divsd(Operand::Reg(r), _)
        | Asm::Xorpd(r, _) => *r == reg,
        Asm::Push(r) | Asm::Pop(r) | Asm::Neg(r) | Asm::Inc(r) | Asm::Dec(r) => *r == reg,
        Asm::Call(Operand::Reg(r)) => *r == reg,
        _ => false,
    })
}

fn pass_redundant_load_store(asms: &mut Vec<Asm>) -> bool {
    let _ = asms;
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
                let is_safe_swap = !matches!(
                    b1,
                    Reg::Rax
                        | Reg::Rbx
                        | Reg::Rcx
                        | Reg::Rdx
                        | Reg::Rsi
                        | Reg::Rdi
                        | Reg::R8
                        | Reg::R9
                        | Reg::R10
                        | Reg::R11
                        | Reg::R15
                );
                if is_safe_swap {
                    asms.remove(i);
                    changed = true;
                    continue;
                }
            }
        }
        i += 1;
    }
    changed
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
                if lbl == "vec_new" {}
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
        match asm {
            Asm::Jmp(l) | Asm::Je(l) | Asm::Jge(l) => {
                refs.insert(l.clone());
            }
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
            size: None,
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
                    size: None,
                }),
                Operand::Reg(Reg::Rax),
            ),
            Asm::Mov(Operand::Reg(Reg::Rbx), Operand::Reg(Reg::Rax)),
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
    }
}
