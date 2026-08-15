mod asm;
mod codegen;
mod compile_code;
mod compile_fn;
mod error;
mod operand;
mod regalloc;

pub use error::CodeGenError;

use crate::compiler::{
    irgen::{IRGen, optimizer},
    parser::Program,
};

pub struct CodeGen {
    ast: Program,
    cte_libs: Vec<String>,
}

impl CodeGen {
    pub fn new(ast: Program, cte_libs: Vec<String>) -> Self {
        Self { ast, cte_libs }
    }

    pub fn generate(self) -> Result<Vec<u8>, CodeGenError> {
        let mut ir_gen = IRGen::new(&self.cte_libs);
        let mut ir_program = ir_gen.compile(self.ast)?;

        if std::env::var("ALC_NO_OPT").is_err() {
            optimizer::optimize(&mut ir_program);
        }

        let mut asm_gen = codegen::AsmCodeGen::new(ir_program);
        let asm_items = asm_gen.compile()?;

        asm::assemble2obj(&asm_items).map_err(|e| CodeGenError::AssemblyError(e))
    }
}
