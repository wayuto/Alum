use super::encoder::Assembler;
use std::collections::HashMap;

pub fn write_elf(asm: &Assembler) -> Vec<u8> {
    let text_data = asm.text_bytes();
    let data_data = asm.data_bytes();
    let relocs = asm.relocs();
    let externs = asm.externs();
    let globals = asm.globals();

    let mut syms: Vec<(String, bool, u64, u16)> = Vec::new();
    syms.push((String::new(), false, 0, 0));

    let mut sym_map: HashMap<String, u32> = HashMap::new();

    for e in externs {
        let idx = syms.len() as u32;
        sym_map.insert(e.clone(), idx);
        syms.push((e.clone(), true, 0, 0));
    }
    for g in globals {
        if let Some((section, offset)) = asm.labels().get(g) {
            let shndx: u16 = if section == "text" {
                1
            } else if section == "data" {
                2
            } else {
                0
            };
            let idx = syms.len() as u32;
            sym_map.insert(g.clone(), idx);
            syms.push((g.clone(), false, *offset, shndx));
        }
    }

    for r in relocs {
        if sym_map.contains_key(&r.target) {
            continue;
        }
        if let Some((section, offset)) = asm.labels().get(&r.target) {
            let shndx: u16 = if section == "text" {
                1
            } else if section == "data" {
                2
            } else {
                0
            };
            let idx = syms.len() as u32;
            sym_map.insert(r.target.clone(), idx);
            syms.push((r.target.clone(), false, *offset, shndx));
        } else {
            let idx = syms.len() as u32;
            sym_map.insert(r.target.clone(), idx);
            syms.push((r.target.clone(), true, 0, 0));
        }
    }

    let sec_names = &[
        ".text",
        ".data",
        ".symtab",
        ".strtab",
        ".rela.text",
        ".shstrtab",
    ];
    let mut shstrtab_data = Vec::<u8>::new();
    shstrtab_data.push(0);
    let mut shname_map = HashMap::new();
    for name in sec_names {
        shname_map.insert(name.to_string(), shstrtab_data.len() as u32);
        shstrtab_data.extend_from_slice(name.as_bytes());
        shstrtab_data.push(0);
    }

    let mut strtab_data = Vec::<u8>::new();
    let mut strtab_ofs = Vec::<u32>::new();
    strtab_data.push(0);
    strtab_ofs.push(0);
    for (i, (sym_name, ..)) in syms.iter().enumerate() {
        if i == 0 {
            continue;
        }
        strtab_ofs.push(strtab_data.len() as u32);
        if !sym_name.is_empty() {
            strtab_data.extend_from_slice(sym_name.as_bytes());
            strtab_data.push(0);
        }
    }

    let mut symtab_data = Vec::<u8>::new();
    for (i, (_sym_name, is_undef, sym_value, st_shndx)) in syms.iter().enumerate() {
        let name_off = strtab_ofs[i];
        let st_info: u8 = if i == 0 {
            0
        } else if *is_undef {
            0x10
        } else {
            0x12
        };
        symtab_data.extend_from_slice(&name_off.to_le_bytes());
        symtab_data.push(st_info);
        symtab_data.push(0);
        symtab_data.extend_from_slice(&st_shndx.to_le_bytes());
        symtab_data.extend_from_slice(&sym_value.to_le_bytes());
        symtab_data.extend_from_slice(&0u64.to_le_bytes());
    }

    let mut rela_data = Vec::<u8>::new();
    for r in relocs {
        let sym_idx = sym_map.get(&r.target).copied().unwrap_or(0);
        let r_type: u32 = match r.kind {
            super::encoder::RelocKind::Pc32 => 2,
            super::encoder::RelocKind::Plt32 => 4,
        };
        let r_info = ((sym_idx as u64) << 32) | (r_type as u64);
        rela_data.extend_from_slice(&r.offset.to_le_bytes());
        rela_data.extend_from_slice(&r_info.to_le_bytes());
        rela_data.extend_from_slice(&r.addend.to_le_bytes());
    }

    let ehdr_size = 64u64;
    let shdr_size = 64u64;
    let num_shdrs = 7u64;
    let shdrs_off = ehdr_size;
    let shdrs_end = shdrs_off + num_shdrs * shdr_size;

    let text_off = align_up(shdrs_end, 16);
    let data_off = align_up(text_off + text_data.len() as u64, 16);
    let symtab_off = align_up(data_off + data_data.len() as u64, 8);
    let strtab_off = align_up(symtab_off + symtab_data.len() as u64, 8);
    let rela_off = align_up(strtab_off + strtab_data.len() as u64, 8);
    let shstrtab_off = align_up(rela_off + rela_data.len() as u64, 1);
    let file_end = align_up(shstrtab_off + shstrtab_data.len() as u64, 16);

    let mut buf = Vec::with_capacity(4096);

    buf.extend_from_slice(b"\x7fELF\x02\x01\x01\x00\x00\x00\x00\x00\x00\x00\x00\x00");
    buf.extend_from_slice(&1u16.to_le_bytes());
    buf.extend_from_slice(&0x3eu16.to_le_bytes());
    buf.extend_from_slice(&1u32.to_le_bytes());
    buf.extend_from_slice(&0u64.to_le_bytes());
    buf.extend_from_slice(&0u64.to_le_bytes());
    buf.extend_from_slice(&shdrs_off.to_le_bytes());
    buf.extend_from_slice(&0u32.to_le_bytes());
    buf.extend_from_slice(&64u16.to_le_bytes());
    buf.extend_from_slice(&0u16.to_le_bytes());
    buf.extend_from_slice(&0u16.to_le_bytes());
    buf.extend_from_slice(&64u16.to_le_bytes());
    buf.extend_from_slice(&(num_shdrs as u16).to_le_bytes());
    buf.extend_from_slice(&6u16.to_le_bytes());

    write_shdr(&mut buf, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0);
    write_shdr(
        &mut buf,
        shname_map[".text"],
        1,
        0x6,
        0,
        text_off,
        text_data.len() as u64,
        0,
        0,
        16,
        0,
    );
    write_shdr(
        &mut buf,
        shname_map[".data"],
        1,
        0x3,
        0,
        data_off,
        data_data.len() as u64,
        0,
        0,
        16,
        0,
    );
    write_shdr(
        &mut buf,
        shname_map[".symtab"],
        2,
        0,
        0,
        symtab_off,
        symtab_data.len() as u64,
        4,
        1,
        8,
        24,
    );
    write_shdr(
        &mut buf,
        shname_map[".strtab"],
        3,
        0,
        0,
        strtab_off,
        strtab_data.len() as u64,
        0,
        0,
        1,
        0,
    );
    write_shdr(
        &mut buf,
        shname_map[".rela.text"],
        4,
        0,
        0,
        rela_off,
        rela_data.len() as u64,
        3,
        1,
        8,
        24,
    );
    write_shdr(
        &mut buf,
        shname_map[".shstrtab"],
        3,
        0,
        0,
        shstrtab_off,
        shstrtab_data.len() as u64,
        0,
        0,
        1,
        0,
    );

    fill_to(&mut buf, text_off);
    buf.extend_from_slice(text_data);
    fill_to(&mut buf, data_off);
    buf.extend_from_slice(data_data);
    fill_to(&mut buf, symtab_off);
    buf.extend_from_slice(&symtab_data);
    fill_to(&mut buf, strtab_off);
    buf.extend_from_slice(&strtab_data);
    fill_to(&mut buf, rela_off);
    buf.extend_from_slice(&rela_data);
    fill_to(&mut buf, shstrtab_off);
    buf.extend_from_slice(&shstrtab_data);
    fill_to(&mut buf, file_end);

    buf
}

fn write_shdr(
    buf: &mut Vec<u8>,
    sh_name: u32,
    sh_type: u32,
    sh_flags: u64,
    sh_addr: u64,
    sh_offset: u64,
    sh_size: u64,
    sh_link: u32,
    sh_info: u32,
    sh_addralign: u64,
    sh_entsize: u64,
) {
    buf.extend_from_slice(&sh_name.to_le_bytes());
    buf.extend_from_slice(&sh_type.to_le_bytes());
    buf.extend_from_slice(&sh_flags.to_le_bytes());
    buf.extend_from_slice(&sh_addr.to_le_bytes());
    buf.extend_from_slice(&sh_offset.to_le_bytes());
    buf.extend_from_slice(&sh_size.to_le_bytes());
    buf.extend_from_slice(&sh_link.to_le_bytes());
    buf.extend_from_slice(&sh_info.to_le_bytes());
    buf.extend_from_slice(&sh_addralign.to_le_bytes());
    buf.extend_from_slice(&sh_entsize.to_le_bytes());
}

fn fill_to(buf: &mut Vec<u8>, target: u64) {
    while (buf.len() as u64) < target {
        buf.push(0);
    }
}

fn align_up(x: u64, align: u64) -> u64 {
    (x + align - 1) & !(align - 1)
}
