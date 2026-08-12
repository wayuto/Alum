mod elf;
mod encoder;
mod peephole;
mod types;

pub use encoder::Assembler;
pub use types::*;

pub fn assemble2obj(asms: &[Asm]) -> Result<Vec<u8>, String> {
    let mut asms = asms.to_vec();
    if std::env::var("ALC_DUMP_ASM").is_ok() {
        eprintln!("=== RAW ASM ===");
        for a in &asms {
            eprintln!("{:?}", a);
        }
    }
    peephole::optimize(&mut asms);
    if std::env::var("ALC_DUMP_ASM").is_ok() {
        eprintln!("=== OPT ASM ===");
        for a in &asms {
            eprintln!("{:?}", a);
        }
    }
    let mut asm = Assembler::new();
    asm.assemble(&asms)?;
    Ok(asm.write_elf())
}
