mod asm;
mod codegen;
mod compile_code;
mod compile_fn;
mod error;
mod operand;
mod regalloc;

pub use error::CodeGenError;

use crate::compiler::{irgen::IRGen, parser::Program};

pub struct CodeGen {
    ast: Program,
}

impl CodeGen {
    pub fn new(ast: Program) -> Self {
        Self { ast }
    }

    pub fn generate(self) -> Result<Vec<u8>, CodeGenError> {
        let mut ir_gen = IRGen::new();
        let ir_program = ir_gen.compile(self.ast)?;

        let mut asm_gen = codegen::AsmCodeGen::new(ir_program);
        let asm_items = asm_gen.compile()?;

        asm::assemble2obj(&asm_items).map_err(|e| CodeGenError::AssemblyError(e))
    }
}
