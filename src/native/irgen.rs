use std::{collections::HashMap, mem::take};

use crate::{
    ast::{Expr, FuncDecl, Program},
    native::{IRConst, IRFunction, IRProgram, IRType, Instruction, Op, Operand},
    token::{Literal, VarType},
};

#[derive(Debug, Clone)]
struct Symbol {
    pub name: String,
    pub ir_type: IRType,
}

type Scope = HashMap<String, Symbol>;

struct Context {
    pub instructions: Vec<Instruction>,
    pub tmp_cnt: usize,
    pub scope: Vec<Scope>,
}

impl Context {
    pub fn new() -> Self {
        Self {
            instructions: Vec::new(),
            tmp_cnt: 0,
            scope: Vec::new(),
        }
    }

    pub fn new_tmp(&mut self, tmp_type: IRType) -> Operand {
        self.tmp_cnt += 1;
        Operand::Temp(self.tmp_cnt - 1, tmp_type)
    }

    pub fn enter_scope(&mut self) {
        self.scope.push(Scope::new());
    }

    pub fn exit_scope(&mut self) {
        self.scope.pop().expect("Tried to pop the root scope.");
    }

    fn get_var_type(&self, name: &str) -> IRType {
        for scope in self.scope.iter().rev() {
            if let Some(symbol) = scope.get(name) {
                return symbol.ir_type.clone();
            }
        }
        panic!("Error: Undefined variable '{}' in current scope.", name);
    }

    pub fn declare_var(&mut self, name: String, ir_type: IRType) {
        let current_scope = self.scope.last_mut().unwrap();
        if current_scope.contains_key(&name) {
            panic!("Error: Variable '{}' already declared in this scope.", name);
        }
        current_scope.insert(name.clone(), Symbol { name, ir_type });
    }
}

pub struct IRGen {
    functions: Vec<IRFunction>,
    constants: Vec<IRConst>,
}

impl IRGen {
    pub fn new() -> Self {
        Self {
            functions: Vec::new(),
            constants: Vec::new(),
        }
    }
    pub fn compile(&mut self, program: Program) -> IRProgram {
        for expr in program.body {
            match expr {
                Expr::FuncDecl(decl) => {
                    self.func_decl(decl);
                }
                Expr::Val(val) => {
                    self.global_constant(val.value);
                }
                _ => {}
            }
        }
        IRProgram {
            functions: take(&mut self.functions),
            constants: take(&mut self.constants),
        }
    }

    fn compile_expr(&mut self, expr: Expr, ctx: &mut Context) -> Operand {
        match expr {
            Expr::Val(val) => {
                let (ir_const, ir_type) = match val.value {
                    Literal::Number(n) => (IRConst::I64(n), IRType::Number),
                    Literal::Bool(b) => (IRConst::I64(if b { 1 } else { 0 }), IRType::Number),
                    Literal::Str(s) => (IRConst::Str(s), IRType::String),
                    Literal::Void => return ctx.new_tmp(IRType::Void),
                    Literal::Array(_, _) => unimplemented!(),
                };

                let res_tmp = ctx.new_tmp(ir_type);
                self.constants.push(ir_const.clone());

                ctx.instructions.push(Instruction {
                    op: Op::Move,
                    dst: Some(res_tmp.clone()),
                    src1: Some(Operand::Const(ir_const)),
                    src2: None,
                });
                res_tmp
            }
            Expr::VarDecl(decl) => {
                let value = self.compile_expr(*decl.value, ctx);
                ctx.declare_var(
                    decl.name.clone(),
                    match decl.typ {
                        VarType::Number => IRType::Number,
                        VarType::Bool => IRType::Bool,
                        VarType::Str => IRType::String,
                        VarType::Void => IRType::Void,
                        _ => unimplemented!(),
                    },
                );
                ctx.instructions.push(Instruction {
                    op: Op::Store,
                    dst: Some(Operand::Var(decl.name)),
                    src1: Some(value),
                    src2: None,
                });
                ctx.new_tmp(IRType::Void)
            }
            Expr::Var(var) => {
                let var_type = ctx.get_var_type(&var.name);
                let res_tmp = ctx.new_tmp(var_type);
                ctx.instructions.push(Instruction {
                    op: Op::Load,
                    dst: Some(res_tmp.clone()),
                    src1: Some(Operand::Var(var.name)),
                    src2: None,
                });
                res_tmp
            }
            Expr::Stmt(stmt) => {
                ctx.enter_scope();

                let body_len = stmt.body.len();

                for i in 0..body_len.saturating_sub(1) {
                    self.compile_expr(stmt.body[i].clone(), ctx);
                }

                let result_operand = if let Some(last_expr) = stmt.body.last() {
                    self.compile_expr(last_expr.clone(), ctx)
                } else {
                    ctx.new_tmp(IRType::Void)
                };
                ctx.exit_scope();
                result_operand
            }
            _ => unimplemented!("{:?}", expr),
        }
    }

    fn global_constant(&mut self, literal: Literal) {
        match literal {
            Literal::Number(n) => self.constants.push(IRConst::I64(n)),
            Literal::Bool(b) => self.constants.push(IRConst::Bool(b)),
            Literal::Str(s) => self.constants.push(IRConst::Str(s)),
            _ => panic!("Invalid global constant type."),
        }
    }

    fn func_decl(&mut self, decl: FuncDecl) -> () {
        let name = decl.name.clone();
        let mut ctx = Context::new();
        ctx.enter_scope();
        let params: Vec<_> = decl
            .params
            .into_iter()
            .map(|(param, typ)| {
                (
                    Operand::Var(param),
                    match typ {
                        VarType::Number => IRType::Number,
                        VarType::Bool => IRType::Bool,
                        VarType::Str => IRType::String,
                        VarType::Void => IRType::Void,
                        _ => unimplemented!(),
                    },
                )
            })
            .collect();

        let body = *decl.body;
        let last_op = self.compile_expr(body, &mut ctx);
        ctx.exit_scope();

        ctx.instructions.push(Instruction {
            op: Op::Return,
            dst: None,
            src1: Some(last_op),
            src2: None,
        });

        self.functions.push(IRFunction {
            name: name,
            params,
            ret_type: IRType::Number,
            instructions: ctx.instructions,
            is_pub: decl.is_pub,
        });
    }
}
