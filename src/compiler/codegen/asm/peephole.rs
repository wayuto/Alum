use super::types::*;
use std::collections::HashSet;

pub fn optimize(asms: &mut Vec<Asm>) {
    loop {
        let mut changed = false;

        changed |= pass_redundant_jmp(asms);
        changed |= pass_mov_mov_swap(asms);
        changed |= pass_redundant_mov(asms);
        changed |= pass_add_sub_xor_zero(asms);
        changed |= pass_push_pop(asms);
        changed |= pass_dead_labels(asms);

        if !changed {
            break;
        }
    }
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
