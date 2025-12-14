use crate::native::IRProgram;

pub struct Optimizer {
    program: IRProgram,
}

impl Optimizer {
    pub fn new(program: IRProgram) -> Self {
        Self { program }
    }
}
