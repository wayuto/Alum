use super::context::Context;
use super::ir::{IRConst, IRGlobalVar, IRType};
use crate::compiler::{
    codegen::CodeGenError,
    irgen::IRGen,
    parser::{Expr, Type},
};

impl IRGen {
    pub(super) fn store_global_vars(&mut self, body: &[Expr]) -> Result<(), CodeGenError> {
        for expr in body {
            match expr {
                Expr::GlobalVar(name, is_pub, ty, init, _) => {
                    if matches!(init.as_deref(), Some(Expr::FuncDecl(..))) {
                        continue;
                    }
                    let value = match init {
                        Some(init) => match self.eval_const(init, None) {
                            Some((cv, _)) => Some(cv),
                            None => {
                                return Err(CodeGenError::TypeError {
                                    message: format!(
                                        "initializer of global variable '{}' is not a compile-time constant",
                                        name
                                    ),
                                });
                            }
                        },
                        None => None,
                    };
                    let ir_type = if matches!(ty, Type::Unknown) {
                        match value.as_ref() {
                            Some(IRConst::Float(_)) => IRType::Float,

                            Some(IRConst::Str(_)) | Some(IRConst::Array(_)) => {
                                return Err(CodeGenError::TypeError {
                                    message: format!(
                                        "unsupported type for global variable '{}' (only int/float/bool)",
                                        name
                                    ),
                                });
                            }
                            _ => IRType::Int,
                        }
                    } else {
                        Context::type_to_ir_type(ty)
                    };
                    if !matches!(ir_type, IRType::Int | IRType::Float | IRType::Bool) {
                        return Err(CodeGenError::TypeError {
                            message: format!(
                                "unsupported type '{}' for global variable '{}' (only int/float/bool)",
                                ty, name
                            ),
                        });
                    }
                    self.global_storage
                        .insert(name.clone(), (ir_type.clone(), value.clone(), *is_pub));
                    self.global_emits.push(IRGlobalVar {
                        name: name.clone(),
                        value,
                        is_pub: *is_pub,
                    });
                }
                Expr::ConstDecl(name, _, _, is_pub, _) => {
                    if *is_pub {
                        if let Some((cv, _)) = self.globals.get(name).cloned() {
                            if matches!(&cv, IRConst::Str(_) | IRConst::Array(_)) {
                                return Err(CodeGenError::TypeError {
                                    message: format!(
                                        "unsupported value for global constant '{}' (only int/float/bool can be exported)",
                                        name
                                    ),
                                });
                            }
                            self.global_emits.push(IRGlobalVar {
                                name: name.clone(),
                                value: Some(cv),
                                is_pub: true,
                            });
                        }
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }

    pub(super) fn store_global_consts(&mut self, exprs: &[Expr]) -> Result<(), CodeGenError> {
        let mut pending: Vec<(String, Expr)> = exprs
            .iter()
            .filter_map(|e| match e {
                Expr::ConstDecl(name, _, value, _, _) => {
                    Some((name.clone(), value.as_ref().clone()))
                }
                _ => None,
            })
            .collect();
        let mut progressed = true;
        while !pending.is_empty() && progressed {
            progressed = false;
            let mut next = Vec::new();
            for (name, value) in pending {
                if let Some(cv) = self.eval_const(&value, None) {
                    self.globals.insert(name, cv);
                    progressed = true;
                } else {
                    next.push((name, value));
                }
            }
            pending = next;
        }
        for (name, _) in pending {
            return Err(CodeGenError::TypeError {
                message: format!(
                    "initializer of constant '{}' is not a compile-time constant",
                    name
                ),
            });
        }
        Ok(())
    }
}
