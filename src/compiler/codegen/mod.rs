mod codegen;
mod ir;
mod irgen;
mod types;

use crate::compiler::parser::Program;
use std::io::Write;
use std::process::Command;
pub use types::CodeGenError;

pub struct CodeGen {
    ast: Program,
}

impl CodeGen {
    pub fn new(ast: Program) -> Self {
        Self { ast }
    }

    pub fn generate(self) -> Result<Vec<u8>, CodeGenError> {
        let mut ir_gen = irgen::IRGen::new();
        let ir_program = ir_gen.compile(self.ast)?;

        let mut asm_gen = codegen::AsmCodeGen::new(ir_program);
        let assembly = asm_gen.compile()?;

        let asm_file = format!("/tmp/alum_{}.asm", std::process::id());
        let obj_file = asm_file.replace(".asm", ".o");

        let mut file = std::fs::File::create(&asm_file)
            .map_err(|e| CodeGenError::NasmError(format!("Failed to create temp file: {}", e)))?;
        file.write_all(assembly.as_bytes())
            .map_err(|e| CodeGenError::NasmError(format!("Failed to write assembly: {}", e)))?;
        drop(file);

        let output = Command::new("nasm")
            .args(["-f", "elf64", "-o", &obj_file, &asm_file])
            .output()
            .map_err(|e| {
                CodeGenError::NasmError(format!("Failed to execute nasm (is it installed?): {}", e))
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(CodeGenError::NasmError(format!(
                "NASM assembly failed: {}",
                stderr.trim()
            )));
        }

        let object_code = std::fs::read(&obj_file)
            .map_err(|e| CodeGenError::NasmError(format!("Failed to read object file: {}", e)))?;

        let _ = std::fs::remove_file(&asm_file);
        let _ = std::fs::remove_file(&obj_file);

        Ok(object_code)
    }
}
