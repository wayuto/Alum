use super::ir::{IRConst, IRType, Instruction, Operand};
use crate::compiler::codegen::CodeGenError;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub(super) struct Symbol {
    #[allow(dead_code)]
    pub name: String,
    pub ir_type: IRType,
}

pub(super) type Scope = HashMap<String, Symbol>;

pub(super) struct Context {
    pub instructions: Vec<Instruction>,
    pub tmp_cnt: usize,
    pub scope: Vec<Scope>,
    pub label_cnt: usize,
    pub loop_end_labels: Vec<String>,
    pub loop_inc_labels: Vec<String>,
    pub var_types: HashMap<String, crate::compiler::parser::Type>,
    pub func_name: String,
}

impl Context {
    pub fn new(func_name: String) -> Self {
        Self {
            instructions: Vec::new(),
            tmp_cnt: 0,
            scope: Vec::new(),
            label_cnt: 0,
            loop_end_labels: Vec::new(),
            loop_inc_labels: Vec::new(),
            var_types: HashMap::new(),
            func_name,
        }
    }

    pub fn new_tmp(&mut self, tmp_type: IRType) -> Operand {
        self.tmp_cnt += 1;
        Operand::Temp(self.tmp_cnt - 1, tmp_type)
    }

    pub fn new_label(&mut self, name: &str) -> String {
        self.label_cnt += 1;
        format!(".{}_{}_{}", self.func_name, name, self.label_cnt - 1)
    }

    pub fn enter_scope(&mut self) {
        self.scope.push(Scope::new());
    }

    pub fn exit_scope(&mut self) -> Result<(), CodeGenError> {
        self.scope.pop().ok_or_else(|| CodeGenError::ScopeError {
            message: "Tried to pop the root scope.".to_string(),
        })?;
        Ok(())
    }

    pub fn get_var_type(&self, name: &str) -> Result<IRType, CodeGenError> {
        for scope in self.scope.iter().rev() {
            if let Some(symbol) = scope.get(name) {
                return Ok(symbol.ir_type.clone());
            }
        }
        Err(CodeGenError::UndefinedVariable {
            name: name.to_string(),
            span: crate::compiler::Span::new(0, 0),
        })
    }

    pub fn type2ir_type(typ: &crate::compiler::parser::Type) -> IRType {
        match typ {
            crate::compiler::parser::Type::Named(name) => match name.as_str() {
                "int" => IRType::Int,
                "float" => IRType::Float,
                "bool" => IRType::Bool,
                "string" => IRType::String,
                "void" => IRType::Void,
                _ => IRType::Int,
            },
            crate::compiler::parser::Type::Array(_, len) => IRType::Array(Some(*len)),
            crate::compiler::parser::Type::Pointer(_) => IRType::Int,
            _ => IRType::Int,
        }
    }

    pub fn get_operand_type(
        &self,
        operand: &Operand,
        constants: &[IRConst],
    ) -> Result<IRType, CodeGenError> {
        match operand {
            Operand::ConstIdx(c) => match &constants[*c] {
                IRConst::Int(_) => Ok(IRType::Int),
                IRConst::Float(_) => Ok(IRType::Float),
                IRConst::Str(_) => Ok(IRType::String),
                IRConst::Array(len, _) => Ok(IRType::Array(Some(len.to_owned()))),
            },
            Operand::Var(name) => self.get_var_type(name),
            Operand::Temp(_, t) => Ok(t.to_owned()),
            Operand::Label(_) => Ok(IRType::Void),
            Operand::Function(_) => Ok(IRType::Void),
        }
    }

    pub fn declare_var(&mut self, name: String, ir_type: IRType) -> Result<(), CodeGenError> {
        let current_scope = self
            .scope
            .last_mut()
            .ok_or_else(|| CodeGenError::ScopeError {
                message: "No scope available".to_string(),
            })?;
        if current_scope.contains_key(&name) {
            return Err(CodeGenError::NameError {
                message: format!("variable '{}' already declared in this scope.", name),
            });
        }
        current_scope.insert(
            name.clone(),
            Symbol {
                name: name.clone(),
                ir_type,
            },
        );
        Ok(())
    }

    pub fn declare_var_with_type(
        &mut self,
        name: String,
        ir_type: IRType,
        htype: crate::compiler::parser::Type,
    ) -> Result<(), CodeGenError> {
        self.declare_var(name.clone(), ir_type)?;
        self.var_types.insert(name, htype);
        Ok(())
    }

    pub fn get_var_high_type(&self, name: &str) -> Option<&crate::compiler::parser::Type> {
        self.var_types.get(name)
    }
}
