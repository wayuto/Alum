use super::ir::{IRConst, IRType, Instruction, Operand};
use crate::compiler::{
    Span,
    codegen::CodeGenError,
    parser::{Primitive, Type, Type as HighType},
};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone)]
pub(super) struct Symbol {
    pub ir_type: IRType,
    pub slot: String,
}

pub(super) type Scope = HashMap<String, Symbol>;

pub(super) struct Context {
    pub instructions: Vec<Instruction>,
    pub tmp_cnt: usize,
    pub scope: Vec<Scope>,
    pub label_cnt: usize,
    pub loop_end_labels: Vec<String>,
    pub loop_inc_labels: Vec<String>,
    pub loop_scope_depths: Vec<usize>,
    pub var_types: HashMap<String, Type>,
    pub array_lengths: HashMap<String, usize>,
    pub func_name: String,
    pub borrowed: HashSet<String>,
    pub(super) var_slots: HashMap<String, Vec<String>>,

    pub(super) var_type_history: HashMap<String, Vec<Option<Type>>>,
    pub(super) array_len_history: HashMap<String, Vec<Option<usize>>>,
    pub(super) borrow_history: HashMap<String, Vec<bool>>,
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
            loop_scope_depths: Vec::new(),
            var_types: HashMap::new(),
            array_lengths: HashMap::new(),
            func_name,
            borrowed: HashSet::new(),
            var_slots: HashMap::new(),
            var_type_history: HashMap::new(),
            array_len_history: HashMap::new(),
            borrow_history: HashMap::new(),
        }
    }

    pub fn slot(&self, name: &str) -> String {
        for scope in self.scope.iter().rev() {
            if let Some(symbol) = scope.get(name) {
                return symbol.slot.clone();
            }
        }
        name.to_string()
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
        let scope = self.scope.pop().ok_or_else(|| CodeGenError::ScopeError {
            message: "Tried to pop the root scope.".to_string(),
        })?;
        for name in scope.keys() {
            if let Some(slots) = self.var_slots.get_mut(name) {
                slots.pop();
                if slots.is_empty() {
                    self.var_slots.remove(name);
                }
            }
            if let Some(history) = self.var_type_history.get_mut(name) {
                if let Some(prev) = history.pop() {
                    match prev {
                        Some(t) => {
                            self.var_types.insert(name.clone(), t);
                        }
                        None => {
                            self.var_types.remove(name);
                        }
                    }
                }
                if history.is_empty() {
                    self.var_type_history.remove(name);
                }
            }
            if let Some(history) = self.array_len_history.get_mut(name) {
                if let Some(prev) = history.pop() {
                    match prev {
                        Some(l) => {
                            self.array_lengths.insert(name.clone(), l);
                        }
                        None => {
                            self.array_lengths.remove(name);
                        }
                    }
                }
                if history.is_empty() {
                    self.array_len_history.remove(name);
                }
            }
            if let Some(history) = self.borrow_history.get_mut(name) {
                if let Some(was_borrowed) = history.pop() {
                    if was_borrowed {
                        self.borrowed.insert(name.clone());
                    } else {
                        self.borrowed.remove(name);
                    }
                }
                if history.is_empty() {
                    self.borrow_history.remove(name);
                }
            }
        }
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
            span: Span::new(0, 0),
        })
    }

    pub fn type_to_ir_type(typ: &Type) -> IRType {
        match typ {
            HighType::Primitive(p) => match p {
                Primitive::Int => IRType::Int,
                Primitive::Float => IRType::Float,
                Primitive::String => IRType::String,
                Primitive::Boolean => IRType::Bool,
                Primitive::Void => IRType::Void,
            },
            HighType::Array(_) => IRType::Array,
            HighType::Pointer(_) => IRType::Int,
            HighType::Function(_, _) => IRType::Int,
            HighType::Struct(_, _) => IRType::Int,
            HighType::Union(_, _) => IRType::Int,
            HighType::Param(_) | HighType::TypeVar(_) | HighType::Unknown => IRType::Int,
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
                IRConst::Array(_) => Ok(IRType::Array),
            },
            Operand::Var(name) => self.get_var_type(name),
            Operand::Temp(_, t) => Ok(t.to_owned()),
            Operand::Label(_) => Ok(IRType::Void),
            Operand::Function(_) => Ok(IRType::Void),
            Operand::Global(_) => Ok(IRType::Void),
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
        let slot = if self.var_slots.contains_key(&name) {
            let depth = self.var_slots.get(&name).map(|v| v.len()).unwrap_or(0);
            format!("{}${}", name, depth)
        } else {
            name.clone()
        };
        self.var_slots
            .entry(name.clone())
            .or_insert_with(Vec::new)
            .push(slot.clone());

        self.var_type_history
            .entry(name.clone())
            .or_default()
            .push(self.var_types.get(&name).cloned());
        self.array_len_history
            .entry(name.clone())
            .or_default()
            .push(self.array_lengths.get(&name).copied());
        self.borrow_history
            .entry(name.clone())
            .or_default()
            .push(self.borrowed.contains(&name));
        current_scope.insert(name.clone(), Symbol { ir_type, slot });
        Ok(())
    }

    pub fn declare_var_with_type(
        &mut self,
        name: String,
        ir_type: IRType,
        htype: Type,
    ) -> Result<(), CodeGenError> {
        self.declare_var(name.clone(), ir_type)?;
        self.var_types.insert(name, htype);
        Ok(())
    }

    pub fn get_var_high_type(&self, name: &str) -> Option<&Type> {
        self.var_types.get(name)
    }
}
