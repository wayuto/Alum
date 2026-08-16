use super::context::Context;
use super::ir::{IRConst, IRProgram};
use crate::compiler::{
    Span,
    codegen::CodeGenError,
    irgen::IRGen,
    parser::{Expr, Primitive, Program, Type},
};
use std::mem::take;

impl IRGen {
    pub(super) fn native_resolved(&self, name: &str) -> bool {
        self.natives
            .as_ref()
            .map(|t| t.entries.contains_key(name))
            .unwrap_or(false)
    }

    pub fn compile(&mut self, program: Program) -> Result<IRProgram, CodeGenError> {
        super::purity::check_lambda_params(&program.body)?;
        let program = self.lambda2function(program);
        self.program_body = program.body.clone();

        self.warn_unverifiable_extern_pure();
        self.resolve_native_signatures();

        super::purity::check_pure_functions(&self.program_body)?;
        self.collect_decls(&program.body)?;

        let const_decls: Vec<Expr> = program
            .body
            .iter()
            .filter(|e| matches!(e, Expr::ConstDecl(..)))
            .cloned()
            .collect();
        self.store_global_consts(&const_decls)?;
        self.store_global_vars(&program.body)?;

        for expr in program.body {
            match expr {
                Expr::FuncDecl(name, _, type_params, params, _, body, _) => {
                    if type_params.is_empty() {
                        self.compile_fn(name, params, *body)?;
                    }
                }
                Expr::ConstDecl(_, _, _, _, _) | Expr::GlobalVar(_, _, _, _, _) => {}
                Expr::Int(_, _)
                | Expr::Float(_, _)
                | Expr::Bool(_, _)
                | Expr::String(_, _)
                | Expr::Nil(_)
                | Expr::Var(_, _) => {
                    let mut ctx = Context::new("_global".to_string());
                    ctx.enter_scope();
                    self.compile_expr(
                        Expr::VarDecl(
                            "_global".to_string(),
                            Type::Primitive(Primitive::Int),
                            Box::new(expr),
                            Span::new(0, 0),
                        ),
                        &mut ctx,
                    )?;
                }
                _ => {}
            }
        }

        Ok(IRProgram {
            functions: take(&mut self.functions),
            constants: take(&mut self.constants),
            extern_vars: take(&mut self.extern_vars).into_keys().collect(),
            global_vars: take(&mut self.global_emits),
        })
    }

    fn collect_decls(&mut self, body: &[Expr]) -> Result<(), CodeGenError> {
        for expr in body {
            match expr {
                Expr::FuncDecl(name, attrs, type_params, params, ret_type, body, _) => {
                    if type_params.is_empty() {
                        self.func_decl(
                            name.clone(),
                            attrs.clone(),
                            params.clone(),
                            ret_type.clone(),
                        )?;
                        self.func_high_returns
                            .insert(name.clone(), ret_type.clone());
                    } else {
                        self.generic_funcs.insert(
                            name.clone(),
                            (
                                type_params.clone(),
                                params.clone(),
                                ret_type.clone(),
                                body.clone(),
                            ),
                        );
                    }
                }
                Expr::ExternVar(name, ty, _) => {
                    self.extern_vars.insert(name.clone(), ty.clone());
                }
                Expr::Struct(name, type_params, fields, _) => {
                    self.structs
                        .insert(name.clone(), (type_params.clone(), fields.clone()));
                }
                Expr::Union(name, type_params, fields, _) => {
                    self.unions
                        .insert(name.clone(), (type_params.clone(), fields.clone()));
                }
                Expr::Enum(name, members, _) => {
                    self.enums.insert(name.clone(), members.clone());
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn warn_unverifiable_extern_pure(&self) {
        let mut warned: std::collections::HashSet<String> = std::collections::HashSet::new();
        for expr in &self.program_body {
            if let Expr::FuncDecl(name, attrs, _, _, _, _, _) = expr {
                let shown = attrs.link_name.as_deref().unwrap_or(name);
                if attrs.is_external && attrs.is_pure && !warned.contains(shown) {
                    warned.insert(shown.to_string());
                    eprintln!("warning: purity of external function '{shown}' cannot be verified");
                }
            }
        }
    }

    fn resolve_native_signatures(&mut self) {
        if let Some(natives) = self.natives.as_mut() {
            for expr in &self.program_body {
                if let Expr::FuncDecl(name, attrs, _, params, ret_type, _, _) = expr {
                    if attrs.is_external && attrs.is_pure {
                        if let Some(sig) = super::const_eval::native_sig(params, ret_type) {
                            match &attrs.link_name {
                                Some(l) => {
                                    if let Some(entry) = natives.resolve(l, sig) {
                                        natives.entries.insert(name.clone(), entry);
                                    }
                                }
                                None => {
                                    natives.resolve(name, sig);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    pub(super) fn get_const_index(&mut self, constant: IRConst) -> usize {
        if let Some(&index) = self.constant_pool.get(&constant) {
            return index;
        }
        let index = self.constants.len();
        self.constants.push(constant.clone());
        self.constant_pool.insert(constant, index);
        index
    }
}
