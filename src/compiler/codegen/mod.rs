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

#[derive(Clone, Copy, Default)]
pub struct DumpOptions {
    pub ir: bool,

    pub asm: bool,
}

pub struct CodeGen {
    ast: Program,
    cte_libs: Vec<String>,
    dumps: DumpOptions,
}

impl CodeGen {
    pub fn new(ast: Program, cte_libs: Vec<String>) -> Self {
        Self {
            ast,
            cte_libs,
            dumps: DumpOptions::default(),
        }
    }

    pub fn with_dumps(mut self, dumps: DumpOptions) -> Self {
        self.dumps = dumps;
        self
    }

    pub fn generate(self) -> Result<Vec<u8>, CodeGenError> {
        let mut ir_gen = IRGen::new(&self.cte_libs);
        let mut ir_program = ir_gen.compile(self.ast)?;

        if std::env::var("ALC_NO_OPT").is_err() {
            optimizer::optimize(&mut ir_program);
        }

        if self.dumps.ir {
            for func in &ir_program.functions {
                eprintln!("=== IR {} ===", func.name);
                for (i, inst) in func.instructions.iter().enumerate() {
                    eprintln!("{:4}: {:?}", i, inst);
                }
            }
        }

        let mut asm_gen = codegen::AsmCodeGen::new(ir_program);
        let asm_items = asm_gen.compile()?;

        if self.dumps.asm {
            for asm in &asm_items {
                eprintln!("{:?}", asm);
            }
        }

        asm::assemble2obj(&asm_items).map_err(|e| CodeGenError::AssemblyError(e))
    }
}
