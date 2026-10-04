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
        let program = self.lambda2function(program);
        self.program_body = program.body.clone();

        self.warn_unverifiable_extern_pure();
        self.resolve_native_signatures();

        super::purity::check_pure_functions(&self.program_body)?;
        self.collect_decls(&program.body)?;

        let const_decls: Vec<Expr> = program
            .body
            .iter()
            .filter(|e| {
                matches!(e, Expr::ConstDecl { name: _, ty: _, value: init, .. }
                    if !matches!(init.as_ref(), Expr::FuncDecl { .. })
                        && !matches!(init.as_ref(), Expr::Var { name: v, .. } if v.starts_with("_lambda_")))
            })
            .cloned()
            .collect();
        self.store_global_consts(&const_decls)?;
        self.store_global_vars(&program.body)?;

        for (name, params, body) in take(&mut self.pending_fn_bodies) {
            self.compile_fn(name, params, body)?;
        }

        for expr in program.body {
            match expr {
                Expr::FuncDecl {
                    name,
                    attrs: _,
                    type_params,
                    params,
                    return_type: _,
                    body,
                    ..
                } => {
                    if type_params.is_empty() {
                        self.compile_fn(name, params, *body)?;
                    }
                }
                Expr::ConstDecl { .. } | Expr::GlobalVar { .. } => {}
                Expr::Int { .. }
                | Expr::Float { .. }
                | Expr::Bool { .. }
                | Expr::String { .. }
                | Expr::Nil(_)
                | Expr::Var { .. } => {
                    let mut ctx = Context::new("_global".to_string());
                    ctx.enter_scope();
                    self.compile_expr(
                        Expr::VarDecl {
                            name: "_global".to_string(),
                            ty: Type::Primitive(Primitive::Int),
                            value: Box::new(expr),
                            span: Span::new(0, 0),
                        },
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

    fn register_bound_fn(
        &mut self,
        bind_name: &str,
        fn_decl: Option<Expr>,
    ) -> Result<(), CodeGenError> {
        if let Some(Expr::FuncDecl {
            name,
            attrs,
            type_params,
            params,
            return_type: ret_type,
            body: lam_body,
            ..
        }) = fn_decl
        {
            if type_params.is_empty() {
                self.func_decl(name.clone(), attrs, params.clone(), ret_type.clone())?;
                let func = self.functions.last_mut().unwrap();
                func.aliases.push(bind_name.to_string());
                self.func_high_returns
                    .insert(bind_name.to_string(), ret_type);
                self.pending_fn_bodies.push((name, params, *lam_body));
            }
        }
        Ok(())
    }

    fn collect_decls(&mut self, body: &[Expr]) -> Result<(), CodeGenError> {
        for expr in body {
            match expr {
                Expr::FuncDecl {
                    name,
                    attrs,
                    type_params,
                    params,
                    return_type: ret_type,
                    body,
                    ..
                } => {
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
                Expr::GlobalVar {
                    name: bind_name,
                    is_pub: _,
                    ty: _,
                    value: init,
                    ..
                } => {
                    let fn_decl = resolve_fn_init(body, init.as_deref());
                    self.register_bound_fn(bind_name, fn_decl)?;
                }
                Expr::ConstDecl {
                    name: bind_name,
                    ty: _,
                    value: init,
                    ..
                } => {
                    let fn_decl = resolve_fn_init(body, Some(init));
                    self.register_bound_fn(bind_name, fn_decl)?;
                }
                Expr::ExternVar { name, ty, .. } => {
                    self.extern_vars.insert(name.clone(), ty.clone());
                }
                Expr::Struct {
                    name,
                    type_params,
                    fields,
                    ..
                } => {
                    self.structs
                        .insert(name.clone(), (type_params.clone(), fields.clone()));
                }
                Expr::Union {
                    name,
                    type_params,
                    fields,
                    ..
                } => {
                    self.unions
                        .insert(name.clone(), (type_params.clone(), fields.clone()));
                }
                Expr::Enum { name, members, .. } => {
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
            if let Expr::FuncDecl { name, attrs, .. } = expr {
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
                if let Expr::FuncDecl {
                    name,
                    attrs,
                    type_params: _,
                    params,
                    return_type: ret_type,
                    ..
                } = expr
                {
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

fn resolve_fn_init(body: &[Expr], init: Option<&Expr>) -> Option<Expr> {
    match init? {
        f @ Expr::FuncDecl { .. } => Some(f.clone()),
        Expr::Var { name: vname, .. } if vname.starts_with("_lambda_") => body
            .iter()
            .find(|e| matches!(e, Expr::FuncDecl { name: n, .. } if n == vname))
            .cloned(),
        _ => None,
    }
}
