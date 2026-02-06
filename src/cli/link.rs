use object::{
    BinaryFormat, Object, ObjectSection, ObjectSymbol, SectionKind, SymbolKind, SymbolScope,
    read::File as ObjectFile, write,
};
use std::fs;

pub fn link_objects(
    obj_files: Vec<String>,
    std_lib_path: &str,
    exe_path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let obj_files_to_delete = obj_files.clone();

    let mut exe = write::Object::new(
        BinaryFormat::Elf,
        object::Architecture::X86_64,
        object::Endianness::Little,
    );

    exe.add_symbol(write::Symbol {
        name: "_start".into(),
        value: 0,
        size: 0,
        kind: SymbolKind::Text,
        scope: SymbolScope::Linkage,
        weak: false,
        section: write::SymbolSection::Undefined,
        flags: object::SymbolFlags::None,
    });

    for obj_path in &obj_files {
        let obj_data = fs::read(obj_path)?;
        let obj_file = ObjectFile::parse(&*obj_data)?;

        for section in obj_file.sections() {
            let name: &str = section.name()?;
            if name == ".text" {
                let section_id = exe.add_section(Vec::new(), b".text".to_vec(), SectionKind::Text);
                let data = section.data()?;
                exe.append_section_data(section_id, data, 1);
            } else if name == ".data" || name == ".rodata" || name == ".bss" {
                let section_id = exe.add_section(
                    Vec::new(),
                    name.as_bytes().to_vec(),
                    if name == ".bss" {
                        SectionKind::UninitializedData
                    } else {
                        SectionKind::Data
                    },
                );
                let data = section.data()?;
                exe.append_section_data(section_id, data, 1);
            }
        }

        for symbol in obj_file.symbols() {
            let name: &str = symbol.name()?;
            if !name.is_empty() {
                exe.add_symbol(write::Symbol {
                    name: name.into(),
                    value: symbol.address(),
                    size: symbol.size(),
                    kind: SymbolKind::Text,
                    scope: SymbolScope::Dynamic,
                    weak: false,
                    section: write::SymbolSection::Undefined,
                    flags: object::SymbolFlags::None,
                });
            }
        }
    }

    let std_lib_data = fs::read(std_lib_path)?;
    let std_lib_file = ObjectFile::parse(&*std_lib_data)?;

    for section in std_lib_file.sections() {
        let name: &str = section.name()?;
        if name == ".text" || name == ".data" || name == ".rodata" || name == ".bss" {
            let section_id = exe.add_section(
                Vec::new(),
                name.as_bytes().to_vec(),
                if name == ".text" {
                    SectionKind::Text
                } else if name == ".bss" {
                    SectionKind::UninitializedData
                } else {
                    SectionKind::Data
                },
            );
            let data = section.data()?;
            exe.append_section_data(section_id, data, 1);
        }
    }

    for symbol in std_lib_file.symbols() {
        let name: &str = symbol.name()?;
        if !name.is_empty() {
            exe.add_symbol(write::Symbol {
                name: name.into(),
                value: symbol.address(),
                size: symbol.size(),
                kind: SymbolKind::Text,
                scope: SymbolScope::Dynamic,
                weak: false,
                section: write::SymbolSection::Undefined,
                flags: object::SymbolFlags::None,
            });
        }
    }

    let exe_bytes = exe.write()?;
    fs::write(exe_path, exe_bytes)?;

    for obj_file in &obj_files_to_delete {
        fs::remove_file(obj_file)?;
    }

    Ok(())
}
