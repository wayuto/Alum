use super::context::Context;
use super::ir::{IRConst, IRFunction, IRProgram, Op};
use crate::compiler::codegen::CodeGenError;
use crate::compiler::parser::{Expr, Program, Type};
use std::{collections::HashMap, mem::take};

pub struct IRGen {
    pub(super) functions: Vec<IRFunction>,
    pub(super) constants: Vec<IRConst>,
    constant_pool: HashMap<IRConst, usize>,
    pub(super) structs: HashMap<String, Vec<(String, Type)>>,
    pub(super) lambda_counter: u32,
}

impl IRGen {
    pub fn new() -> Self {
        Self {
            functions: Vec::new(),
            constants: Vec::new(),
            constant_pool: HashMap::new(),
            structs: HashMap::new(),
            lambda_counter: 0,
        }
    }

    pub fn compile(&mut self, program: Program) -> Result<IRProgram, CodeGenError> {
        let program = self.lambda2function(program);

        for expr in &program.body {
            match expr {
                Expr::FuncDecl(name, params, ret_type, _, _) => {
                    self.func_decl(name.clone(), params.clone(), ret_type.clone())?;
                }
                Expr::Extern(name, params, ret_type, _) => {
                    self.extern_decl(name.clone(), params.clone(), ret_type.clone())?;
                }
                Expr::Struct(name, fields, _) => {
                    self.structs.insert(name.clone(), fields.clone());
                }
                _ => {}
            }
        }

        for expr in program.body {
            match expr {
                Expr::FuncDecl(name, params, _, body, _) => {
                    self.compile_fn(name, params, *body)?;
                }
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
                            Type::Named("int".to_string()),
                            Box::new(expr),
                            crate::compiler::Span::new(0, 0),
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
        })
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

    pub(super) fn get_binop_parts(expr: Expr) -> Result<(Op, Box<Expr>, Box<Expr>), CodeGenError> {
        let _span = crate::compiler::Span::new(0, 0);
        match expr {
            Expr::Add(l, r, _) => Ok((Op::Add, l, r)),
            Expr::Sub(l, r, _) => Ok((Op::Sub, l, r)),
            Expr::Mul(l, r, _) => Ok((Op::Mul, l, r)),
            Expr::Div(l, r, _) => Ok((Op::Div, l, r)),
            Expr::Mod(l, r, _) => Ok((Op::Mod, l, r)),
            Expr::FAdd(l, r, _) => Ok((Op::FAdd, l, r)),
            Expr::FSub(l, r, _) => Ok((Op::FSub, l, r)),
            Expr::FMul(l, r, _) => Ok((Op::FMul, l, r)),
            Expr::FDiv(l, r, _) => Ok((Op::FDiv, l, r)),
            Expr::Eq(l, r, _) => Ok((Op::Eq, l, r)),
            Expr::Ne(l, r, _) => Ok((Op::Ne, l, r)),
            Expr::Lt(l, r, _) => Ok((Op::Lt, l, r)),
            Expr::Le(l, r, _) => Ok((Op::Le, l, r)),
            Expr::Gt(l, r, _) => Ok((Op::Gt, l, r)),
            Expr::Ge(l, r, _) => Ok((Op::Ge, l, r)),
            Expr::FEq(l, r, _) => Ok((Op::FEq, l, r)),
            Expr::FNe(l, r, _) => Ok((Op::FNe, l, r)),
            Expr::FLt(l, r, _) => Ok((Op::FLt, l, r)),
            Expr::FLe(l, r, _) => Ok((Op::FLe, l, r)),
            Expr::FGt(l, r, _) => Ok((Op::FGt, l, r)),
            Expr::FGe(l, r, _) => Ok((Op::FGe, l, r)),
            Expr::Xor(l, r, _) => Ok((Op::Xor, l, r)),
            Expr::LAnd(l, r, _) => Ok((Op::LAnd, l, r)),
            Expr::LOr(l, r, _) => Ok((Op::LOr, l, r)),
            Expr::StrCat(l, r, _) => Ok((Op::StrCat, l, r)),
            _ => Err(CodeGenError::UnsupportedOperation {
                message: "not a binary operation".to_string(),
            }),
        }
    }
}
