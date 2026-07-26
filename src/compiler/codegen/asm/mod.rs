mod elf;
mod encoder;
mod peephole;
mod types;

pub use encoder::Assembler;
pub use types::*;

pub fn assemble_to_obj(asms: &[Asm]) -> Result<Vec<u8>, String> {
    let mut asms = asms.to_vec();
    peephole::optimize(&mut asms);
    let mut asm = Assembler::new();
    asm.assemble(&asms)?;
    Ok(asm.write_elf())
}
