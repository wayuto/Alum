use super::types::*;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelocKind {
    Pc32,
    Plt32,
}

#[derive(Debug, Clone)]
pub struct Reloc {
    pub offset: u64,
    pub kind: RelocKind,
    pub target: String,
    pub addend: i64,
}

pub struct Assembler {
    sections: Vec<String>,
    text_bytes: Vec<u8>,
    data_bytes: Vec<u8>,
    labels: HashMap<String, (String, u64)>,
    relocs: Vec<Reloc>,
    externs: Vec<String>,
    globals: Vec<String>,
}

fn mem_rex_bits(m: &Mem) -> (bool, bool) {
    let rex_x = m.index.map(|i| i.rex_b()).unwrap_or(false);
    let rex_b = m.base.map(|b| b.rex_b()).unwrap_or(false);
    (rex_x, rex_b)
}

fn m_rex_x(op: &Operand) -> bool {
    match op {
        Operand::Mem(m) => m.index.map(|i| i.rex_b()).unwrap_or(false),
        _ => false,
    }
}

fn m_rex_b(op: &Operand) -> bool {
    match op {
        Operand::Mem(m) => m.base.map(|b| b.rex_b()).unwrap_or(false),
        Operand::Reg(r) => r.rex_b(),
        _ => false,
    }
}

impl Assembler {
    pub fn new() -> Self {
        Assembler {
            sections: vec!["text".to_string()],
            text_bytes: Vec::new(),
            data_bytes: Vec::new(),
            labels: HashMap::new(),
            relocs: Vec::new(),
            externs: Vec::new(),
            globals: Vec::new(),
        }
    }

    pub fn assemble(&mut self, asms: &[Asm]) -> Result<(), String> {
        self.pass1(asms)
    }

    fn cur_section(&self) -> String {
        self.sections
            .last()
            .cloned()
            .unwrap_or_else(|| "text".to_string())
    }

    fn cur_offset(&self, section: &str) -> u64 {
        match section {
            "text" => self.text_bytes.len() as u64,
            "data" => self.data_bytes.len() as u64,
            _ => 0,
        }
    }

    fn emit_bytes(&mut self, section: &str, bytes: Vec<u8>) {
        match section {
            "text" => self.text_bytes.extend(bytes),
            "data" => self.data_bytes.extend(bytes),
            _ => {}
        }
    }

    fn emit_slice(&mut self, section: &str, bytes: &[u8]) {
        match section {
            "text" => self.text_bytes.extend_from_slice(bytes),
            "data" => self.data_bytes.extend_from_slice(bytes),
            _ => {}
        }
    }

    fn pass1(&mut self, asms: &[Asm]) -> Result<(), String> {
        for item in asms {
            match item {
                Asm::Section(s) => {
                    let name = match s {
                        Section::Text => "text",
                        Section::Data => "data",
                    };
                    self.sections.push(name.to_string());
                }
                Asm::Extern(name) => {
                    if !self.externs.contains(name) {
                        self.externs.push(name.clone());
                    }
                }
                Asm::Global(name) => {
                    if !self.globals.contains(name) {
                        self.globals.push(name.clone());
                    }
                }
                Asm::Label(name) => {
                    let sec = self.cur_section();
                    let off = self.cur_offset(&sec);
                    self.labels.insert(name.clone(), (sec.clone(), off));
                }
                Asm::Align(n) => {
                    let sec = self.cur_section();
                    let off = self.cur_offset(&sec);
                    let padding = ((off + *n as u64 - 1) & !(*n as u64 - 1)).wrapping_sub(off);
                    self.emit_bytes(&sec, vec![0u8; padding as usize]);
                }
                Asm::Dq(vals) => {
                    let sec = self.cur_section();
                    let mut buf = Vec::with_capacity(vals.len() * 8);
                    for v in vals {
                        buf.extend_from_slice(&v.to_le_bytes());
                    }
                    self.emit_bytes(&sec, buf);
                }
                Asm::Db(bytes) => {
                    let sec = self.cur_section();
                    self.emit_bytes(&sec, bytes.clone());
                }
                other => {
                    let sec = self.cur_section();
                    let off = self.cur_offset(&sec);
                    self.emit_sized(other, &sec, off)?;
                }
            }
        }
        Ok(())
    }

    fn emit_sized(&mut self, asm: &Asm, section: &str, offset: u64) -> Result<(), String> {
        use Asm::*;
        match asm {
            Mov(Operand::Reg(r), Operand::Imm(v)) => {
                if r.rex_b() {
                    self.emit_bytes(section, vec![0x48 | 0x01]);
                } else {
                    self.emit_bytes(section, vec![0x48]);
                }
                self.emit_bytes(section, vec![0xb8 | (r.reg_id() & 7)]);
                self.emit_slice(section, &v.to_le_bytes()[..8]);
            }
            Mov(Operand::Reg(r), Operand::DataLabel(l))
            | Mov(Operand::Reg(r), Operand::Label(l)) => {
                self.emit_mov_label(*r, l, section, offset);
            }
            Mov(Operand::Reg(r), op) if !r.is_xmm() => {
                self.emit_r64_rm64(0x8b, *r, op, section, offset)?;
            }
            Mov(op, Operand::Reg(r)) if !r.is_xmm() => {
                self.emit_rm64_r64(0x89, op, *r, section, offset)?;
            }
            Mov(Operand::Mem(m), Operand::Imm(v)) => {
                let (rex_x, rex_b) = mem_rex_bits(m);
                self.emit_rex(section, true, false, rex_x, rex_b);
                self.emit_bytes(section, vec![0xc7]);
                self.emit_modrm_sib(section, 0, m, offset)?;
                self.emit_slice(section, &(*v as i32).to_le_bytes());
            }
            Movzx(d, s) => {
                self.emit_rex(section, false, d.rex_b(), m_rex_x(&s), m_rex_b(&s));
                self.emit_slice(section, &[0x0f, 0xb6]);
                match s {
                    Operand::Reg(s) => {
                        self.emit_bytes(
                            section,
                            vec![self.modrm(3, d.reg_id() & 7, s.reg_id() & 7)],
                        );
                    }
                    Operand::Mem(m) => {
                        self.emit_modrm_sib(section, d.reg_id() & 7, m, offset)?;
                    }
                    _ => return Err(format!("unsupported movzx source {:?}", s)),
                }
            }
            Movb(dst, src) => {
                self.emit_rex(section, false, src.rex_b(), m_rex_x(&dst), m_rex_b(&dst));
                self.emit_slice(section, &[0x88]);
                match dst {
                    Operand::Mem(m) => {
                        self.emit_modrm_sib(section, src.reg_id() & 7, m, offset)?;
                    }
                    _ => return Err(format!("unsupported movb destination {:?}", dst)),
                }
            }
            Movsd(dst, src) => self.emit_movsd(dst, src, section, offset)?,
            Add(dst, src) => self.emit_binop(0x03, 0x83, 0x00, dst, src, section, offset)?,
            Sub(dst, src) => self.emit_binop(0x2b, 0x83, 0x05, dst, src, section, offset)?,
            Imul(Operand::Reg(r), op) => {
                self.emit_binop_imul(*r, op, section, offset)?;
            }
            Idiv(reg) => {
                self.emit_rex(section, true, false, false, reg.rex_b());
                self.emit_slice(section, &[0xf7, self.modrm(3, 7, reg.reg_id() & 7)]);
            }
            Cqo => {
                self.emit_slice(section, &[0x48, 0x99]);
            }
            Cdqe => {
                self.emit_slice(section, &[0x48, 0x98]);
            }
            Neg(reg) => {
                self.emit_rex(section, true, false, false, reg.rex_b());
                self.emit_slice(section, &[0xf7, self.modrm(3, 3, reg.reg_id() & 7)]);
            }
            Not(reg) => {
                self.emit_rex(section, true, false, false, reg.rex_b());
                self.emit_slice(section, &[0xf7, self.modrm(3, 2, reg.reg_id() & 7)]);
            }
            Shl(reg) => {
                self.emit_rex(section, true, false, false, reg.rex_b());
                self.emit_slice(section, &[0xd3, self.modrm(3, 4, reg.reg_id() & 7)]);
            }
            Sar(reg) => {
                self.emit_rex(section, true, false, false, reg.rex_b());
                self.emit_slice(section, &[0xd3, self.modrm(3, 7, reg.reg_id() & 7)]);
            }
            Inc(reg) => {
                self.emit_rex(section, true, false, false, reg.rex_b());
                self.emit_slice(section, &[0xff, self.modrm(3, 0, reg.reg_id() & 7)]);
            }
            Dec(reg) => {
                self.emit_rex(section, true, false, false, reg.rex_b());
                self.emit_slice(section, &[0xff, self.modrm(3, 1, reg.reg_id() & 7)]);
            }
            Xor(dst, src) => self.emit_binop(0x33, 0x83, 0x06, dst, src, section, offset)?,
            Or(dst, src) => self.emit_binop(0x0b, 0x83, 0x01, dst, src, section, offset)?,
            And(dst, src) => self.emit_binop(0x23, 0x83, 0x04, dst, src, section, offset)?,
            Cmp(Operand::Reg(r), Operand::Imm(0)) => {
                self.emit_rex(section, true, false, false, r.rex_b());
                self.emit_slice(section, &[0x83, self.modrm(3, 7, r.reg_id() & 7), 0x00]);
            }
            Cmp(Operand::Reg(r), op) => {
                self.emit_binop(0x3b, 0x83, 0x07, &Operand::Reg(*r), op, section, offset)?;
            }
            Test(reg) => {
                self.emit_rex(section, true, false, false, reg.rex_b());
                self.emit_slice(section, &[0x85, self.modrm(3, 0, reg.reg_id() & 7)]);
            }
            Push(reg) => {
                if reg.rex_b() {
                    self.emit_slice(section, &[0x41]);
                }
                self.emit_slice(section, &[0x50 + (reg.reg_id() & 7)]);
            }
            Pop(reg) => {
                if reg.rex_b() {
                    self.emit_slice(section, &[0x41]);
                }
                self.emit_slice(section, &[0x58 + (reg.reg_id() & 7)]);
            }
            Call(Operand::Reg(r)) => {
                self.emit_slice(section, &[0xff, self.modrm(3, 2, r.reg_id() & 7)]);
            }
            Call(Operand::Label(l)) => {
                self.emit_slice(section, &[0xe8]);
                self.emit_reloc(section, RelocKind::Pc32, l.clone(), -4);
            }
            Call(Operand::PLT(l)) => {
                self.emit_slice(section, &[0xe8]);
                self.emit_reloc(section, RelocKind::Plt32, l.clone(), -4);
            }
            Ret => {
                self.emit_slice(section, &[0xc3]);
            }
            Jmp(lbl) => {
                self.emit_slice(section, &[0xe9]);
                self.emit_reloc(section, RelocKind::Pc32, lbl.clone(), -4);
            }
            Je(lbl) => {
                self.emit_cond_jmp(0x0f84, lbl, section);
            }
            Jne(lbl) => {
                self.emit_cond_jmp(0x0f85, lbl, section);
            }
            Jl(lbl) => {
                self.emit_cond_jmp(0x0f8c, lbl, section);
            }
            Jle(lbl) => {
                self.emit_cond_jmp(0x0f8e, lbl, section);
            }
            Jg(lbl) => {
                self.emit_cond_jmp(0x0f8f, lbl, section);
            }
            Jge(lbl) => {
                self.emit_cond_jmp(0x0f8d, lbl, section);
            }
            Ja(lbl) => {
                self.emit_cond_jmp(0x0f87, lbl, section);
            }
            Jae(lbl) => {
                self.emit_cond_jmp(0x0f83, lbl, section);
            }
            Jb(lbl) => {
                self.emit_cond_jmp(0x0f82, lbl, section);
            }
            Jbe(lbl) => {
                self.emit_cond_jmp(0x0f86, lbl, section);
            }
            Sete(reg) => {
                self.emit_setcc(0x0f94, *reg, section);
            }
            Setne(reg) => {
                self.emit_setcc(0x0f95, *reg, section);
            }
            Setg(reg) => {
                self.emit_setcc(0x0f9f, *reg, section);
            }
            Setge(reg) => {
                self.emit_setcc(0x0f9d, *reg, section);
            }
            Setl(reg) => {
                self.emit_setcc(0x0f9c, *reg, section);
            }
            Setle(reg) => {
                self.emit_setcc(0x0f9e, *reg, section);
            }
            Seta(reg) => {
                self.emit_setcc(0x0f97, *reg, section);
            }
            Setae(reg) => {
                self.emit_setcc(0x0f93, *reg, section);
            }
            Setb(reg) => {
                self.emit_setcc(0x0f92, *reg, section);
            }
            Setbe(reg) => {
                self.emit_setcc(0x0f96, *reg, section);
            }
            Addsd(dst, src) => self.emit_fp_binop(0x58, dst, src, section, offset)?,
            Subsd(dst, src) => self.emit_fp_binop(0x5c, dst, src, section, offset)?,
            Mulsd(dst, src) => self.emit_fp_binop(0x59, dst, src, section, offset)?,
            Divsd(dst, src) => self.emit_fp_binop(0x5e, dst, src, section, offset)?,
            Ucomisd(a, b) => {
                self.emit_slice(section, &[0x66]);
                self.emit_rex(section, false, b.rex_b(), false, a.rex_b());
                self.emit_slice(
                    section,
                    &[0x0f, 0x2e, self.modrm(3, b.reg_id() & 7, a.reg_id() & 7)],
                );
            }
            Xorpd(d, src) => {
                let prefix = 0x66u8;
                self.emit_slice(section, &[prefix]);
                match src {
                    Operand::Mem(m) => {
                        let (rex_x, rex_b) = mem_rex_bits(m);
                        self.emit_rex(section, false, d.rex_b(), rex_x, rex_b);
                        self.emit_slice(section, &[0x0f, 0x57]);
                        self.emit_modrm_sib(section, d.reg_id() & 7, m, offset)?;
                    }
                    Operand::DataLabel(l) => {
                        self.emit_rex(section, false, false, false, d.rex_b());
                        self.emit_slice(section, &[0x0f, 0x57]);
                        self.emit_rm_disp32(section, d.reg_id() & 7);
                        self.emit_reloc(section, RelocKind::Pc32, l.clone(), -4);
                    }
                    _ => return Err(format!("unsupported xorpd src: {:?}", src)),
                }
            }
            Cvtsi2sd(dst, src) => {
                let f2 = 0xf2u8;
                match src {
                    Operand::Reg(s) => {
                        self.emit_slice(section, &[f2]);
                        self.emit_rex(section, true, dst.rex_b(), false, s.rex_b());
                        self.emit_slice(
                            section,
                            &[0x0f, 0x2a, self.modrm(3, dst.reg_id() & 7, s.reg_id() & 7)],
                        );
                    }
                    Operand::Mem(m) => {
                        let (rex_x, rex_b) = mem_rex_bits(m);
                        self.emit_slice(section, &[f2]);
                        self.emit_rex(section, true, dst.rex_b(), rex_x, rex_b);
                        self.emit_slice(section, &[0x0f, 0x2a]);
                        self.emit_modrm_sib(section, dst.reg_id() & 7, m, offset)?;
                    }
                    _ => return Err(format!("unsupported cvtsi2sd src: {:?}", src)),
                }
            }
            Cvttsd2si(dst, src) => {
                let f2 = 0xf2u8;
                match src {
                    Operand::Reg(s) if s.is_xmm() => {
                        self.emit_slice(section, &[f2]);
                        self.emit_rex(section, true, dst.rex_b(), false, s.rex_b());
                        self.emit_slice(
                            section,
                            &[0x0f, 0x2c, self.modrm(3, dst.reg_id() & 7, s.reg_id() & 7)],
                        );
                    }
                    Operand::Mem(m) => {
                        let (rex_x, rex_b) = mem_rex_bits(m);
                        self.emit_slice(section, &[f2]);
                        self.emit_rex(section, true, dst.rex_b(), rex_x, rex_b);
                        self.emit_slice(section, &[0x0f, 0x2c]);
                        self.emit_modrm_sib(section, dst.reg_id() & 7, m, offset)?;
                    }
                    _ => return Err(format!("unsupported cvttsd2si src: {:?}", src)),
                }
            }
            Lea(Operand::Reg(r), src) => match src {
                Operand::Mem(m) => {
                    let (rex_x, rex_b) = mem_rex_bits(m);
                    self.emit_rex(section, true, r.rex_b(), rex_x, rex_b);
                    self.emit_slice(section, &[0x8d]);
                    self.emit_modrm_sib(section, r.reg_id() & 7, m, offset)?;
                }
                Operand::DataLabel(l) => {
                    self.emit_rex(section, true, r.rex_b(), false, false);
                    self.emit_slice(section, &[0x8d]);
                    self.emit_rm_disp32(section, r.reg_id() & 7);
                    self.emit_reloc(section, RelocKind::Pc32, l.clone(), -4);
                }
                _ => return Err(format!("unsupported lea src: {:?}", src)),
            },
            _ => return Err(format!("unsupported instruction: {:?}", asm)),
        }
        Ok(())
    }

    fn emit_rex(&mut self, section: &str, w: bool, r: bool, x: bool, b: bool) {
        if w || r || x || b {
            self.emit_slice(
                section,
                &[0x40 | (w as u8) << 3 | (r as u8) << 2 | (x as u8) << 1 | b as u8],
            );
        }
    }

    fn modrm(&self, mod_: u8, reg: u8, rm: u8) -> u8 {
        (mod_ << 6) | ((reg & 7) << 3) | (rm & 7)
    }

    fn emit_rm_disp32(&mut self, section: &str, reg: u8) {
        self.emit_slice(section, &[self.modrm(0, reg & 7, 5)]);
    }
    fn emit_mov_label(&mut self, r: Reg, lbl: &str, section: &str, _offset: u64) {
        self.emit_rex(section, true, r.rex_b(), false, false);
        self.emit_slice(section, &[0x8d]);
        self.emit_rm_disp32(section, r.reg_id() & 7);
        self.emit_reloc(section, RelocKind::Pc32, lbl.to_string(), -4);
    }
    fn emit_r64_rm64(
        &mut self,
        opcode: u8,
        r: Reg,
        src: &Operand,
        section: &str,
        offset: u64,
    ) -> Result<(), String> {
        match src {
            Operand::Reg(s) => {
                self.emit_rex(section, true, r.rex_b(), false, s.rex_b());
                self.emit_slice(
                    section,
                    &[opcode, self.modrm(3, r.reg_id() & 7, s.reg_id() & 7)],
                );
            }
            Operand::Mem(m) => {
                let (rex_x, rex_b) = mem_rex_bits(m);
                self.emit_rex(section, true, r.rex_b(), rex_x, rex_b);
                self.emit_slice(section, &[opcode]);
                self.emit_modrm_sib(section, r.reg_id() & 7, m, offset)?;
            }
            _ => return Err(format!("unsupported r64 rm64 src: {:?}", src)),
        }
        Ok(())
    }

    fn emit_rm64_r64(
        &mut self,
        opcode: u8,
        dst: &Operand,
        r: Reg,
        section: &str,
        offset: u64,
    ) -> Result<(), String> {
        match dst {
            Operand::Reg(d) => {
                self.emit_rex(section, true, r.rex_b(), false, d.rex_b());
                self.emit_slice(
                    section,
                    &[opcode, self.modrm(3, r.reg_id() & 7, d.reg_id() & 7)],
                );
            }
            Operand::Mem(m) => {
                let (rex_x, rex_b) = mem_rex_bits(m);
                self.emit_rex(section, true, r.rex_b(), rex_x, rex_b);
                self.emit_slice(section, &[opcode]);
                self.emit_modrm_sib(section, r.reg_id() & 7, m, offset)?;
            }
            _ => return Err(format!("unsupported rm64 r64 dst: {:?}", dst)),
        }
        Ok(())
    }

    fn emit_binop(
        &mut self,
        op_rm: u8,
        op_imm8: u8,
        op_imm_ext: u8,
        dst: &Operand,
        src: &Operand,
        section: &str,
        offset: u64,
    ) -> Result<(), String> {
        match (dst, src) {
            (Operand::Reg(r), Operand::Reg(s)) => {
                self.emit_rex(section, true, r.rex_b(), false, s.rex_b());
                self.emit_slice(
                    section,
                    &[op_rm, self.modrm(3, r.reg_id() & 7, s.reg_id() & 7)],
                );
            }
            (Operand::Reg(r), Operand::Mem(m)) => {
                let (rex_x, rex_b) = mem_rex_bits(m);
                self.emit_rex(section, true, r.rex_b(), rex_x, rex_b);
                self.emit_slice(section, &[op_rm]);
                self.emit_modrm_sib(section, r.reg_id() & 7, m, offset)?;
            }
            (Operand::Reg(r), Operand::Imm(v)) if *v >= -128 && *v <= 127 => {
                self.emit_rex(section, true, false, false, r.rex_b());
                self.emit_slice(
                    section,
                    &[op_imm8, self.modrm(3, op_imm_ext, r.reg_id() & 7)],
                );
                self.emit_slice(section, &[(*v & 0xff) as u8]);
            }
            (Operand::Reg(r), Operand::Imm(v)) => {
                self.emit_rex(section, true, false, false, r.rex_b());
                self.emit_slice(section, &[0x81, self.modrm(3, op_imm_ext, r.reg_id() & 7)]);
                self.emit_slice(section, &(*v as i32).to_le_bytes());
            }
            _ => return Err(format!("unsupported binop: {:?} {:?}", dst, src)),
        }
        Ok(())
    }

    fn emit_binop_imul(
        &mut self,
        r: Reg,
        src: &Operand,
        section: &str,
        offset: u64,
    ) -> Result<(), String> {
        match src {
            Operand::Reg(s) => {
                self.emit_rex(section, true, r.rex_b(), false, s.rex_b());
                self.emit_slice(
                    section,
                    &[0x0f, 0xaf, self.modrm(3, r.reg_id() & 7, s.reg_id() & 7)],
                );
            }
            Operand::Mem(m) => {
                let (rex_x, rex_b) = mem_rex_bits(m);
                self.emit_rex(section, true, r.rex_b(), rex_x, rex_b);
                self.emit_slice(section, &[0x0f, 0xaf]);
                self.emit_modrm_sib(section, r.reg_id() & 7, m, offset)?;
            }
            Operand::Imm(v) if *v >= -128 && *v <= 127 => {
                self.emit_rex(section, true, false, false, r.rex_b());
                self.emit_slice(
                    section,
                    &[0x6b, self.modrm(3, r.reg_id() & 7, r.reg_id() & 7)],
                );
                self.emit_slice(section, &[(*v & 0xff) as u8]);
            }
            Operand::Imm(v) => {
                self.emit_rex(section, true, false, false, r.rex_b());
                self.emit_slice(
                    section,
                    &[0x69, self.modrm(3, r.reg_id() & 7, r.reg_id() & 7)],
                );
                self.emit_slice(section, &(*v as i32).to_le_bytes());
            }
            _ => return Err(format!("unsupported imul: {:?}", src)),
        }
        Ok(())
    }

    fn emit_movsd(
        &mut self,
        dst: &Operand,
        src: &Operand,
        section: &str,
        offset: u64,
    ) -> Result<(), String> {
        let f2 = 0xf2u8;
        match (dst, src) {
            (Operand::Reg(r), Operand::Mem(m)) if r.is_xmm() => {
                let (rex_x, rex_b) = mem_rex_bits(m);
                self.emit_slice(section, &[f2]);
                self.emit_rex(section, false, r.rex_b(), rex_x, rex_b);
                self.emit_slice(section, &[0x0f, 0x10]);
                self.emit_modrm_sib(section, r.reg_id() & 7, m, offset)?;
            }
            (Operand::Mem(m), Operand::Reg(r)) if r.is_xmm() => {
                let (rex_x, rex_b) = mem_rex_bits(m);
                self.emit_slice(section, &[f2]);
                self.emit_rex(section, false, r.rex_b(), rex_x, rex_b);
                self.emit_slice(section, &[0x0f, 0x11]);
                self.emit_modrm_sib(section, r.reg_id() & 7, m, offset)?;
            }
            (Operand::Reg(d), Operand::Reg(s)) if d.is_xmm() && s.is_xmm() => {
                self.emit_slice(section, &[f2]);
                self.emit_rex(section, false, d.rex_b(), false, s.rex_b());
                self.emit_slice(
                    section,
                    &[0x0f, 0x10, self.modrm(3, d.reg_id() & 7, s.reg_id() & 7)],
                );
            }
            (Operand::Reg(r), Operand::DataLabel(l)) => {
                self.emit_slice(section, &[f2]);
                self.emit_rex(section, false, false, false, r.rex_b());
                self.emit_slice(section, &[0x0f, 0x10]);
                self.emit_rm_disp32(section, r.reg_id() & 7);
                self.emit_reloc(section, RelocKind::Pc32, l.clone(), -4);
            }
            _ => return Err(format!("unsupported movsd: {:?} {:?}", dst, src)),
        }
        Ok(())
    }

    fn emit_fp_binop(
        &mut self,
        opcode: u8,
        dst: &Operand,
        src: &Operand,
        section: &str,
        offset: u64,
    ) -> Result<(), String> {
        let f2 = 0xf2u8;
        match (dst, src) {
            (Operand::Reg(r), Operand::Mem(m)) if r.is_xmm() => {
                let (rex_x, rex_b) = mem_rex_bits(m);
                self.emit_slice(section, &[f2]);
                self.emit_rex(section, false, r.rex_b(), rex_x, rex_b);
                self.emit_slice(section, &[0x0f, opcode]);
                self.emit_modrm_sib(section, r.reg_id() & 7, m, offset)?;
            }
            (Operand::Reg(d), Operand::Reg(s)) if d.is_xmm() && s.is_xmm() => {
                self.emit_slice(section, &[f2]);
                self.emit_rex(section, false, d.rex_b(), false, s.rex_b());
                self.emit_slice(
                    section,
                    &[0x0f, opcode, self.modrm(3, d.reg_id() & 7, s.reg_id() & 7)],
                );
            }
            (Operand::Reg(r), Operand::DataLabel(l)) => {
                self.emit_slice(section, &[f2]);
                self.emit_rex(section, false, false, false, r.rex_b());
                self.emit_slice(section, &[0x0f, opcode]);
                self.emit_rm_disp32(section, r.reg_id() & 7);
                self.emit_reloc(section, RelocKind::Pc32, l.clone(), -4);
            }
            _ => return Err(format!("unsupported fp binop: {:?} {:?}", dst, src)),
        }
        Ok(())
    }

    fn emit_setcc(&mut self, opcode: u16, reg: Reg, section: &str) {
        self.emit_rex(section, false, false, false, reg.rex_b());
        self.emit_slice(section, &opcode.to_be_bytes());
        self.emit_slice(section, &[self.modrm(3, 0, reg.reg_id() & 7)]);
    }

    fn emit_cond_jmp(&mut self, opcode: u16, lbl: &str, section: &str) {
        self.emit_slice(section, &opcode.to_be_bytes());
        self.emit_reloc(section, RelocKind::Pc32, lbl.to_string(), -4);
    }

    fn emit_modrm_sib(
        &mut self,
        section: &str,
        reg: u8,
        m: &Mem,
        _offset: u64,
    ) -> Result<(), String> {
        let reg7 = reg & 7;
        match (m.base, m.index) {
            (Some(base), Some(idx)) => {
                let scale_bits = match m.scale {
                    1 => 0u8,
                    2 => 1,
                    4 => 2,
                    8 => 3,
                    _ => return Err("invalid scale".to_string()),
                };
                let mod_ = if m.disp == 0 {
                    0u8
                } else if m.disp >= -128 && m.disp <= 127 {
                    1
                } else {
                    2
                };
                self.emit_slice(section, &[self.modrm(mod_, reg7, 0x04)]);
                self.emit_slice(
                    section,
                    &[(scale_bits << 6) | ((idx.reg_id() & 7) << 3) | (base.reg_id() & 7)],
                );
                match mod_ {
                    1 => self.emit_slice(section, &(m.disp as i8 as u8).to_le_bytes()),
                    2 => self.emit_slice(section, &(m.disp as i32).to_le_bytes()),
                    _ => {}
                }
            }
            (Some(base), None) => {
                if m.disp == 0 && base != Reg::Rbp {
                    let rm = if base == Reg::Rsp {
                        0x04
                    } else {
                        base.reg_id() & 7
                    };
                    self.emit_slice(section, &[self.modrm(0, reg7, rm)]);
                    if base == Reg::Rsp {
                        self.emit_slice(section, &[0x24]);
                    }
                } else {
                    let mod_ = if m.disp >= -128 && m.disp <= 127 {
                        1u8
                    } else {
                        2u8
                    };
                    let rm = if base == Reg::Rsp {
                        0x04
                    } else {
                        base.reg_id() & 7
                    };
                    self.emit_slice(section, &[self.modrm(mod_, reg7, rm)]);
                    if base == Reg::Rsp {
                        self.emit_slice(section, &[0x24]);
                    }
                    match mod_ {
                        1 => self.emit_slice(section, &(m.disp as i8 as u8).to_le_bytes()),
                        2 => self.emit_slice(section, &(m.disp as i32).to_le_bytes()),
                        _ => {}
                    }
                }
            }
            (None, Some(idx)) => {
                let scale_bits = match m.scale {
                    1 => 0u8,
                    2 => 1,
                    4 => 2,
                    8 => 3,
                    _ => return Err("invalid scale".to_string()),
                };
                self.emit_slice(section, &[self.modrm(0, reg7, 0x04)]);
                self.emit_slice(
                    section,
                    &[(scale_bits << 6) | ((idx.reg_id() & 7) << 3) | 0x05],
                );
                self.emit_slice(section, &(m.disp as i32).to_le_bytes());
            }
            (None, None) => {
                return Err("direct rip-relative must use DataLabel".to_string());
            }
        }
        Ok(())
    }

    fn emit_reloc(&mut self, section: &str, kind: RelocKind, target: String, addend: i64) {
        let offset = self.cur_offset(section) as u64;

        self.emit_slice(section, &[0u8; 4]);
        self.relocs.push(Reloc {
            offset,
            kind,
            target,
            addend,
        });
    }

    pub fn text_bytes(&self) -> &[u8] {
        &self.text_bytes
    }
    pub fn data_bytes(&self) -> &[u8] {
        &self.data_bytes
    }
    pub fn relocs(&self) -> &[Reloc] {
        &self.relocs
    }
    pub fn externs(&self) -> &[String] {
        &self.externs
    }
    pub fn globals(&self) -> &[String] {
        &self.globals
    }
    pub fn labels(&self) -> &HashMap<String, (String, u64)> {
        &self.labels
    }

    pub fn write_elf(&self) -> Vec<u8> {
        super::elf::write_elf(self)
    }
}
