use crate::native::IRProgram;

pub struct CodeGen {
    program: IRProgram,
}

impl CodeGen {
    pub fn new(program: IRProgram) -> Self {
        Self { program }
    }
}
