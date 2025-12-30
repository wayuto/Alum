use std::{collections::HashMap, iter::zip, mem::take};

use ordered_float::OrderedFloat;

use crate::{
    ast::{Expr, Extern, FuncDecl, Program, Var},
    ir::{IRConst, IRFunction, IRProgram, IRType, Instruction, Op, Operand},
    token::{Literal, TokenType, VarType},
};

#[derive(Debug, Clone)]
pub enum IRGenError {
    NameError { message: String },
    TypeError { message: String },
    ScopeError { message: String },
    SyntaxError { message: String },
}

impl std::error::Error for IRGenError {}

impl std::fmt::Display for IRGenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IRGenError::NameError { message } => write!(f, "Name error: {}", message),
            IRGenError::TypeError { message } => write!(f, "Type error: {}", message),
            IRGenError::ScopeError { message } => write!(f, "Scope error: {}", message),
            IRGenError::SyntaxError { message } => write!(f, "Syntax error: {}", message),
        }
    }
}

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
    pub label_cnt: usize,
}

impl Context {
    pub fn new() -> Self {
        Self {
            instructions: Vec::new(),
            tmp_cnt: 0,
            scope: Vec::new(),
            label_cnt: 0,
        }
    }

    pub fn new_tmp(&mut self, tmp_type: IRType) -> Operand {
        self.tmp_cnt += 1;
        Operand::Temp(self.tmp_cnt - 1, tmp_type)
    }

    pub fn new_label(&mut self, name: &str) -> String {
        self.label_cnt += 1;
        format!(".{}_{:X}", name, self.label_cnt - 1)
    }

    pub fn enter_scope(&mut self) {
        self.scope.push(Scope::new());
    }

    pub fn exit_scope(&mut self) -> Result<(), IRGenError> {
        self.scope.pop().ok_or_else(|| IRGenError::ScopeError {
            message: "Tried to pop the root scope.".to_string(),
        })?;
        Ok(())
    }

    fn get_var_type(&self, name: &str) -> Result<IRType, IRGenError> {
        for scope in self.scope.iter().rev() {
            if let Some(symbol) = scope.get(name) {
                return Ok(symbol.ir_type.clone());
            }
        }
        Err(IRGenError::NameError {
            message: format!("undefined variable '{}' in current scope.", name),
        })
    }

    pub fn from_var_type(&self, var_type: &VarType) -> IRType {
        match var_type {
            VarType::Int => IRType::Int,
            VarType::Float => IRType::Float,
            VarType::Bool => IRType::Bool,
            VarType::Str => IRType::String,
            VarType::Array(len) => IRType::Array(len.to_owned()),
            VarType::Void => IRType::Void,
        }
    }

    pub fn get_operand_type(&self, operand: &Operand) -> Result<IRType, IRGenError> {
        match operand {
            Operand::Const(c) => match c {
                IRConst::Int(_) => Ok(IRType::Int),
                IRConst::Float(_) => Ok(IRType::Float),
                IRConst::Bool(_) => Ok(IRType::Bool),
                IRConst::Str(_) => Ok(IRType::String),
                IRConst::Array(len, _) => Ok(IRType::Array(Some(len.to_owned()))),
                IRConst::Void => Ok(IRType::Void),
            },
            Operand::Var(name) => self.get_var_type(&name),
            Operand::Temp(_, t) => Ok(t.to_owned()),
            Operand::Label(_) => Ok(IRType::Void),
            Operand::Function(_) => Ok(IRType::Void),
            Operand::ConstIdx(_) => Ok(IRType::Void),
        }
    }

    pub fn declare_var(&mut self, name: String, ir_type: IRType) -> Result<(), IRGenError> {
        let current_scope = self
            .scope
            .last_mut()
            .ok_or_else(|| IRGenError::ScopeError {
                message: "No scope available".to_string(),
            })?;
        if current_scope.contains_key(&name) {
            return Err(IRGenError::NameError {
                message: format!("variable '{}' already declared in this scope.", name),
            });
        }
        current_scope.insert(name.clone(), Symbol { name, ir_type });
        Ok(())
    }
}

pub struct IRGen {
    functions: Vec<IRFunction>,
    constants: Vec<IRConst>,
    constant_pool: HashMap<IRConst, usize>,
}

impl IRGen {
    pub fn new() -> Self {
        Self {
            functions: Vec::new(),
            constants: Vec::new(),
            constant_pool: HashMap::new(),
        }
    }

    pub fn compile(&mut self, program: Program) -> Result<IRProgram, IRGenError> {
        for expr in &program.body {
            match expr {
                Expr::FuncDecl(decl) => {
                    self.func_decl(decl.clone())?;
                }
                Expr::Extern(ext) => {
                    self.extern_decl(ext.clone())?;
                }
                _ => {}
            }
        }

        for expr in program.body {
            match expr {
                Expr::FuncDecl(decl) => {
                    self.compile_fn(decl)?;
                }
                Expr::Val(val) => {
                    self.global_constant(val.value)?;
                }
                _ => {}
            }
        }

        Ok(IRProgram {
            functions: take(&mut self.functions),
            constants: take(&mut self.constants),
        })
    }

    fn get_const_index(&mut self, constant: IRConst) -> usize {
        if let Some(&index) = self.constant_pool.get(&constant) {
            return index;
        }

        let index = self.constants.len();
        self.constants.push(constant.clone());
        self.constant_pool.insert(constant, index);
        index
    }

    fn compile_expr(&mut self, expr: Expr, ctx: &mut Context) -> Result<Operand, IRGenError> {
        match expr {
            Expr::Val(val) => {
                let (ir_const, ir_type) = match val.value {
                    Literal::Int(n) => (IRConst::Int(n), IRType::Int),
                    Literal::Float(f) => (IRConst::Float(f), IRType::Float),
                    Literal::Bool(b) => (IRConst::Int(if b { 1 } else { 0 }), IRType::Int),
                    Literal::Str(s) => (IRConst::Str(s), IRType::String),
                    Literal::Void => return Ok(ctx.new_tmp(IRType::Void)),
                    Literal::Array(len, arr) => {
                        let is_fill_syntax = len > 1 && arr.len() == 1;
                        if is_fill_syntax {
                            let fill_element = self.compile_expr(arr[0].clone(), ctx)?;
                            let mut elements = Vec::new();
                            for _ in 0..len {
                                elements.push(fill_element.clone());
                            }
                            (
                                IRConst::Array(elements.len(), elements.clone()),
                                IRType::Array(Some(elements.len())),
                            )
                        } else {
                            let mut elements = Vec::new();
                            for e in arr.iter() {
                                elements.push(self.compile_expr(e.to_owned(), ctx)?);
                            }

                            if len != 0 && len != elements.len() {
                                return Err(IRGenError::TypeError {
                                    message: format!(
                                        "Array literal length mismatch: declared {}, actual {}",
                                        len,
                                        elements.len()
                                    ),
                                });
                            }

                            (
                                IRConst::Array(elements.len(), elements.clone()),
                                IRType::Array(Some(elements.len())),
                            )
                        }
                    }
                };

                let res_tmp = ctx.new_tmp(ir_type.clone());
                let const_idx = self.get_const_index(ir_const);
                match ir_type {
                    IRType::Float => ctx.instructions.push(Instruction {
                        op: Op::FMove,
                        dst: Some(res_tmp.clone()),
                        src1: Some(Operand::ConstIdx(const_idx)),
                        src2: None,
                    }),
                    _ => ctx.instructions.push(Instruction {
                        op: Op::Move,
                        dst: Some(res_tmp.clone()),
                        src1: Some(Operand::ConstIdx(const_idx)),
                        src2: None,
                    }),
                }
                Ok(res_tmp)
            }

            Expr::VarDecl(decl) => {
                let mut value = self.compile_expr(*decl.value.clone(), ctx)?;
                let value_type = ctx.get_operand_type(&value)?;

                let var_ir_type = match &decl.typ {
                    VarType::Array(Some(declared_len)) => {
                        if let IRType::Array(Some(actual_len)) = &value_type {
                            if *declared_len > *actual_len && *actual_len == 1 {
                                if let Operand::Temp(_, _) = value {
                                    if let Some(last_inst) = ctx.instructions.last() {
                                        if let Some(Operand::ConstIdx(idx)) = &last_inst.src1 {
                                            if let IRConst::Array(_, elems) = &self.constants[*idx]
                                            {
                                                let fill_elem = elems[0].clone();
                                                let new_elems = vec![fill_elem; *declared_len];

                                                let new_const =
                                                    IRConst::Array(*declared_len, new_elems);
                                                let new_idx = self.get_const_index(new_const);

                                                if let Some(last_inst) = ctx.instructions.last_mut()
                                                {
                                                    last_inst.src1 =
                                                        Some(Operand::ConstIdx(new_idx));
                                                }

                                                value = Operand::Temp(
                                                    ctx.tmp_cnt - 1,
                                                    IRType::Array(Some(*declared_len)),
                                                );
                                            }
                                        }
                                    }
                                }
                            } else if *declared_len != *actual_len {
                                return Err(IRGenError::TypeError {
                                    message: "array length mismatch".to_string(),
                                });
                            }
                            IRType::Array(Some(*declared_len))
                        } else {
                            return Err(IRGenError::TypeError {
                                message: "expected array".to_string(),
                            });
                        }
                    }
                    _ => ctx.from_var_type(&decl.typ),
                };

                ctx.declare_var(decl.name.clone(), var_ir_type.clone())?;
                match var_ir_type {
                    IRType::Float => ctx.instructions.push(Instruction {
                        op: Op::FStore,
                        dst: Some(Operand::Var(decl.name)),
                        src1: Some(value),
                        src2: None,
                    }),
                    _ => ctx.instructions.push(Instruction {
                        op: Op::Store,
                        dst: Some(Operand::Var(decl.name)),
                        src1: Some(value),
                        src2: None,
                    }),
                }
                Ok(ctx.new_tmp(IRType::Void))
            }
            Expr::VarMod(modi) => {
                let value = self.compile_expr(*modi.value, ctx)?;
                let typ = ctx.get_operand_type(&value)?;
                let var_typ = ctx.get_var_type(&modi.name)?;
                if typ != var_typ {
                    return Err(IRGenError::TypeError {
                        message: format!("unexpected type: {:?}", typ),
                    });
                }
                match typ {
                    IRType::Float => ctx.instructions.push(Instruction {
                        op: Op::FStore,
                        dst: Some(Operand::Var(modi.name)),
                        src1: Some(value),
                        src2: None,
                    }),
                    _ => ctx.instructions.push(Instruction {
                        op: Op::Store,
                        dst: Some(Operand::Var(modi.name)),
                        src1: Some(value),
                        src2: None,
                    }),
                }
                Ok(ctx.new_tmp(IRType::Void))
            }
            Expr::Var(var) => {
                let var_type = ctx.get_var_type(&var.name)?;
                let res_tmp = ctx.new_tmp(var_type.clone());
                match var_type {
                    IRType::Float => ctx.instructions.push(Instruction {
                        op: Op::FLoad,
                        dst: Some(res_tmp.clone()),
                        src1: Some(Operand::Var(var.name)),
                        src2: None,
                    }),
                    _ => ctx.instructions.push(Instruction {
                        op: Op::Load,
                        dst: Some(res_tmp.clone()),
                        src1: Some(Operand::Var(var.name)),
                        src2: None,
                    }),
                }
                Ok(res_tmp)
            }
            Expr::BinOp(bin) => {
                let left = self.compile_expr(*bin.left, ctx)?;
                let right = self.compile_expr(*bin.right, ctx)?;
                let typ = ctx.get_operand_type(&left)?;
                let res_tmp: Operand;
                if bin.operator == TokenType::RANGE {
                    res_tmp = ctx.new_tmp(IRType::Array(None));
                } else {
                    res_tmp = ctx.new_tmp(typ.clone());
                }

                ctx.instructions.push(Instruction {
                    op: match bin.operator {
                        TokenType::ADD
                        | TokenType::SUB
                        | TokenType::MUL
                        | TokenType::DIV
                        | TokenType::COMPEQ
                        | TokenType::COMPNE
                        | TokenType::COMPGT
                        | TokenType::COMPGE
                        | TokenType::COMPLT
                        | TokenType::COMPLE
                        | TokenType::COMPAND
                        | TokenType::COMPOR => match typ {
                            IRType::Float => match bin.operator {
                                TokenType::ADD => Op::FAdd,
                                TokenType::SUB => Op::FSub,
                                TokenType::MUL => Op::FMul,
                                TokenType::DIV => Op::FDiv,
                                TokenType::COMPEQ => Op::FEq,
                                TokenType::COMPNE => Op::FNe,
                                TokenType::COMPGT => Op::FGt,
                                TokenType::COMPGE => Op::FGe,
                                TokenType::COMPLT => Op::FLt,
                                TokenType::COMPLE => Op::FLe,
                                _ => {
                                    return Err(IRGenError::TypeError {
                                        message: format!(
                                            "unsupported float operation: {:?}",
                                            bin.operator
                                        ),
                                    });
                                }
                            },
                            _ => match bin.operator {
                                TokenType::ADD => Op::Add,
                                TokenType::SUB => Op::Sub,
                                TokenType::MUL => Op::Mul,
                                TokenType::DIV => Op::Div,
                                TokenType::COMPEQ => Op::Eq,
                                TokenType::COMPNE => Op::Ne,
                                TokenType::COMPGT => Op::Gt,
                                TokenType::COMPGE => Op::Ge,
                                TokenType::COMPLT => Op::Lt,
                                TokenType::COMPLE => Op::Le,
                                TokenType::COMPAND => Op::And,
                                TokenType::COMPOR => Op::Or,
                                _ => {
                                    return Err(IRGenError::TypeError {
                                        message: format!(
                                            "unsupported operation: {:?}",
                                            bin.operator
                                        ),
                                    });
                                }
                            },
                        },
                        TokenType::LOGAND => Op::LAnd,
                        TokenType::LOGOR => Op::LOr,
                        TokenType::LOGXOR => Op::Xor,
                        TokenType::RANGE => Op::Range,
                        _ => {
                            return Err(IRGenError::TypeError {
                                message: format!("unsupported operation: {:?}", bin.operator),
                            });
                        }
                    },
                    dst: Some(res_tmp.clone()),
                    src1: Some(left),
                    src2: Some(right),
                });
                Ok(res_tmp)
            }
            Expr::UnaryOp(unary) => {
                let argument = self.compile_expr(*unary.argument, ctx)?;
                let typ = ctx.get_operand_type(&argument)?;
                let res_tmp = ctx.new_tmp(typ.clone());
                match typ {
                    IRType::Float => match unary.operator {
                        TokenType::NEG => ctx.instructions.push(Instruction {
                            op: Op::FNeg,
                            dst: Some(res_tmp.clone()),
                            src1: Some(argument.clone()),
                            src2: None,
                        }),
                        TokenType::INC => ctx.instructions.push(Instruction {
                            op: Op::FAdd,
                            dst: Some(res_tmp.clone()),
                            src1: Some(argument.clone()),
                            src2: Some(Operand::Const(IRConst::Float(OrderedFloat(1.0)))),
                        }),
                        TokenType::DEC => ctx.instructions.push(Instruction {
                            op: Op::FSub,
                            dst: Some(res_tmp.clone()),
                            src1: Some(argument.clone()),
                            src2: Some(Operand::Const(IRConst::Float(OrderedFloat(1.0)))),
                        }),
                        _ => {
                            return Err(IRGenError::TypeError {
                                message: "unsupported float unary operation".to_string(),
                            });
                        }
                    },
                    _ => ctx.instructions.push(Instruction {
                        op: match unary.operator {
                            TokenType::NEG => Op::Neg,
                            TokenType::INC => Op::Inc,
                            TokenType::DEC => Op::Dec,
                            TokenType::SIZEOF => Op::SizeOf,
                            _ => {
                                return Err(IRGenError::TypeError {
                                    message: format!(
                                        "unsupported unary operation: {:?}",
                                        unary.operator
                                    ),
                                });
                            }
                        },
                        dst: Some(res_tmp.clone()),
                        src1: Some(argument),
                        src2: None,
                    }),
                }
                Ok(res_tmp)
            }
            Expr::Stmt(stmt) => {
                ctx.enter_scope();

                let body_len = stmt.body.len();

                for i in 0..body_len.saturating_sub(1) {
                    self.compile_expr(stmt.body[i].clone(), ctx)?;
                }

                let result_operand = if let Some(last_expr) = stmt.body.last() {
                    self.compile_expr(last_expr.clone(), ctx)?
                } else {
                    ctx.new_tmp(IRType::Void)
                };
                ctx.exit_scope()?;
                Ok(result_operand)
            }
            Expr::Return(ret_expr) => {
                if let Some(val) = ret_expr.value {
                    let res_op = self.compile_expr(*val, ctx)?;
                    match ctx.get_operand_type(&res_op)? {
                        IRType::Float => ctx.instructions.push(Instruction {
                            op: Op::Return(String::from("xmm0")),
                            dst: None,
                            src1: Some(res_op),
                            src2: None,
                        }),
                        _ => ctx.instructions.push(Instruction {
                            op: Op::Return(String::from("rax")),
                            dst: None,
                            src1: Some(res_op),
                            src2: None,
                        }),
                    }
                }
                Ok(ctx.new_tmp(IRType::Void))
            }
            Expr::If(i) => {
                let label_else = ctx.new_label("else");
                let label_end = ctx.new_label("endif");

                let cond = self.compile_expr(*i.condition, ctx)?;

                ctx.instructions.push(Instruction {
                    op: Op::JumpIfFalse,
                    dst: None,
                    src1: Some(cond),
                    src2: Some(Operand::Label(label_else.clone())),
                });

                let res_tmp = ctx.new_tmp(IRType::Void);

                if !matches!(*i.then_branch, Expr::Stmt(_)) {
                    ctx.enter_scope();
                }
                let then_op = self.compile_expr(*i.then_branch.clone(), ctx)?;

                ctx.instructions.push(Instruction {
                    op: Op::Move,
                    dst: Some(res_tmp.clone()),
                    src1: Some(then_op),
                    src2: None,
                });
                if !matches!(*i.then_branch, Expr::Stmt(_)) {
                    ctx.exit_scope()?;
                }

                ctx.instructions.push(Instruction {
                    op: Op::Jump,
                    dst: None,
                    src1: Some(Operand::Label(label_end.clone())),
                    src2: None,
                });

                ctx.instructions.push(Instruction {
                    op: Op::Label(label_else),
                    dst: None,
                    src1: None,
                    src2: None,
                });

                if let Some(else_expr) = i.else_branch {
                    if !matches!(*else_expr, Expr::Stmt(_)) {
                        ctx.enter_scope();
                    }
                    let else_op = self.compile_expr(*else_expr.to_owned(), ctx)?;

                    ctx.instructions.push(Instruction {
                        op: Op::Move,
                        dst: Some(res_tmp.clone()),
                        src1: Some(else_op),
                        src2: None,
                    });
                    if !matches!(*else_expr.clone(), Expr::Stmt(_)) {
                        ctx.exit_scope()?;
                    }
                }

                ctx.instructions.push(Instruction {
                    op: Op::Label(label_end),
                    dst: None,
                    src1: None,
                    src2: None,
                });

                Ok(res_tmp)
            }
            Expr::While(w) => {
                let label_start = ctx.new_label("while_start");
                let label_end = ctx.new_label("while_end");
                let cond = self.compile_expr(*w.condition, ctx)?;
                ctx.instructions.push(Instruction {
                    op: Op::JumpIfFalse,
                    dst: None,
                    src1: Some(cond),
                    src2: Some(Operand::Label(label_end.clone())),
                });
                ctx.instructions.push(Instruction {
                    op: Op::Label(label_start.clone()),
                    dst: None,
                    src1: None,
                    src2: None,
                });
                if !matches!(*w.body, Expr::Stmt(_)) {
                    ctx.enter_scope();
                }
                self.compile_expr(*w.body.clone(), ctx)?;
                if !matches!(*w.body, Expr::Stmt(_)) {
                    ctx.exit_scope()?;
                }
                ctx.instructions.push(Instruction {
                    op: Op::Jump,
                    dst: None,
                    src1: Some(Operand::Label(label_start)),
                    src2: None,
                });
                ctx.instructions.push(Instruction {
                    op: Op::Label(label_end.clone()),
                    dst: None,
                    src1: None,
                    src2: None,
                });
                Ok(ctx.new_tmp(IRType::Void))
            }
            Expr::For(f) => {
                let array_operand = self.compile_expr(*f.iter, ctx)?;
                let array_type = ctx.get_operand_type(&array_operand)?;

                let array_len_operand = match array_type {
                    IRType::Array(Some(l)) => {
                        let idx = self.get_const_index(IRConst::Int(l as i64));
                        Operand::ConstIdx(idx)
                    }
                    IRType::Array(None) => {
                        let len_tmp = ctx.new_tmp(IRType::Int);
                        ctx.instructions.push(Instruction {
                            op: Op::SizeOf,
                            dst: Some(len_tmp.clone()),
                            src1: Some(array_operand.clone()),
                            src2: None,
                        });
                        len_tmp
                    }
                    _ => {
                        return Err(IRGenError::TypeError {
                            message: format!(
                                "can only iterate over arrays, found {:?}",
                                array_type
                            ),
                        });
                    }
                };

                ctx.enter_scope();
                let idx_name = ctx.new_label("idx");
                let idx_var = Operand::Var(idx_name.clone());
                ctx.declare_var(idx_name.clone(), IRType::Int)?;

                let zero_idx = self.get_const_index(IRConst::Int(0));
                ctx.instructions.push(Instruction {
                    op: Op::Store,
                    dst: Some(idx_var.clone()),
                    src1: Some(Operand::ConstIdx(zero_idx)),
                    src2: None,
                });

                let label_cond = ctx.new_label("for_cond");
                let label_end = ctx.new_label("for_end");
                ctx.instructions.push(Instruction {
                    op: Op::Label(label_cond.clone()),
                    dst: None,
                    src1: None,
                    src2: None,
                });

                let curr_idx = ctx.new_tmp(IRType::Int);
                ctx.instructions.push(Instruction {
                    op: Op::Load,
                    dst: Some(curr_idx.clone()),
                    src1: Some(idx_var.clone()),
                    src2: None,
                });

                let cond_tmp = ctx.new_tmp(IRType::Bool);
                ctx.instructions.push(Instruction {
                    op: Op::Lt,
                    dst: Some(cond_tmp.clone()),
                    src1: Some(curr_idx.clone()),
                    src2: Some(array_len_operand),
                });

                ctx.instructions.push(Instruction {
                    op: Op::JumpIfFalse,
                    dst: None,
                    src1: Some(cond_tmp),
                    src2: Some(Operand::Label(label_end.clone())),
                });

                ctx.declare_var(f.init.clone(), IRType::Int)?;
                let element_tmp = ctx.new_tmp(IRType::Int);

                ctx.instructions.push(Instruction {
                    op: Op::ArrayAccess,
                    dst: Some(element_tmp.clone()),
                    src1: Some(array_operand),
                    src2: Some(curr_idx.clone()),
                });

                ctx.instructions.push(Instruction {
                    op: Op::Store,
                    dst: Some(Operand::Var(f.init)),
                    src1: Some(element_tmp),
                    src2: None,
                });

                self.compile_expr(*f.body, ctx)?;

                let one_idx = self.get_const_index(IRConst::Int(1));
                let next_idx = ctx.new_tmp(IRType::Int);

                ctx.instructions.push(Instruction {
                    op: Op::Add,
                    dst: Some(next_idx.clone()),
                    src1: Some(curr_idx),
                    src2: Some(Operand::ConstIdx(one_idx)),
                });
                ctx.instructions.push(Instruction {
                    op: Op::Store,
                    dst: Some(idx_var),
                    src1: Some(next_idx),
                    src2: None,
                });
                ctx.instructions.push(Instruction {
                    op: Op::Jump,
                    dst: None,
                    src1: Some(Operand::Label(label_cond)),
                    src2: None,
                });
                ctx.instructions.push(Instruction {
                    op: Op::Label(label_end),
                    dst: None,
                    src1: None,
                    src2: None,
                });

                ctx.exit_scope()?;
                Ok(ctx.new_tmp(IRType::Void))
            }
            Expr::FuncDecl(_) => {
                return Err(IRGenError::SyntaxError {
                    message: "cannot declare a function in a function".to_string(),
                });
            }
            Expr::FuncCall(call) => {
                let func = self.find_func(&call.name)?;
                if call.args.len() != func.params.len() {
                    return Err(IRGenError::TypeError {
                        message: format!(
                            "expected {} arguments, got {}",
                            func.params.len(),
                            call.args.len()
                        ),
                    });
                }
                let res_tmp = ctx.new_tmp(ctx.from_var_type(&call.ret_type));
                let mut n = 0;
                for (arg, param) in zip(call.args.iter(), func.params.iter()) {
                    let operand = self.compile_expr(arg.clone(), ctx)?;
                    let operand_type = ctx.get_operand_type(&operand)?;
                    if operand_type != param.1 {
                        return Err(IRGenError::TypeError {
                            message: format!(
                                "unexpected type {:?}, expected {:?}",
                                operand_type, param.1
                            ),
                        });
                    }
                    match param.1 {
                        IRType::Float => ctx.instructions.push(Instruction {
                            op: Op::FArg(n),
                            dst: None,
                            src1: Some(operand),
                            src2: None,
                        }),
                        _ => ctx.instructions.push(Instruction {
                            op: Op::Arg(n),
                            dst: None,
                            src1: Some(operand),
                            src2: None,
                        }),
                    }
                    n += 1;
                }
                ctx.instructions.push(Instruction {
                    op: Op::Call,
                    dst: Some(res_tmp.clone()),
                    src1: Some(Operand::Function(call.name)),
                    src2: None,
                });
                Ok(res_tmp)
            }
            Expr::ArrayAccess(aa) => {
                let arr = Operand::Var(aa.array.clone());
                let arr_type = ctx.get_operand_type(&arr)?;
                if let IRType::Array(_) = arr_type {
                    let offset = self.compile_expr(*aa.offset, ctx)?;
                    let res_tmp = ctx.new_tmp(IRType::Int);
                    ctx.instructions.push(Instruction {
                        op: Op::ArrayAccess,
                        dst: Some(res_tmp.clone()),
                        src1: Some(arr),
                        src2: Some(offset),
                    });
                    Ok(res_tmp)
                } else {
                    Err(IRGenError::TypeError {
                        message: format!("{} is not an array", aa.array),
                    })
                }
            }
            Expr::ArrayAssign(aa) => {
                let arr = Operand::Var(aa.array.clone());
                let arr_type = ctx.get_operand_type(&arr)?;
                if let IRType::Array(_) = arr_type {
                    let offset = self.compile_expr(*aa.offset, ctx)?;
                    let val = self.compile_expr(*aa.value, ctx)?;
                    let res_tmp = ctx.new_tmp(IRType::Void);
                    ctx.instructions.push(Instruction {
                        op: Op::ArrayAssign,
                        dst: Some(arr),
                        src1: Some(offset),
                        src2: Some(val),
                    });
                    Ok(res_tmp)
                } else {
                    Err(IRGenError::TypeError {
                        message: format!("{} is not an array", aa.array),
                    })
                }
            }
            Expr::Extern(_) => {
                return Err(IRGenError::SyntaxError {
                    message: "cannot extern a function in a function".to_string(),
                });
            }
            Expr::Goto(goto) => {
                ctx.instructions.push(Instruction {
                    op: Op::Jump,
                    dst: None,
                    src1: Some(Operand::Label(goto.label)),
                    src2: None,
                });
                Ok(ctx.new_tmp(IRType::Void))
            }
            Expr::Label(label) => {
                ctx.instructions.push(Instruction {
                    op: Op::Label(label.name),
                    dst: None,
                    src1: None,
                    src2: None,
                });
                Ok(ctx.new_tmp(IRType::Void))
            }
        }
    }

    fn global_constant(&mut self, literal: Literal) -> Result<(), IRGenError> {
        match literal {
            Literal::Int(n) => {
                self.constants.push(IRConst::Int(n));
                Ok(())
            }
            Literal::Bool(b) => {
                self.constants.push(IRConst::Bool(b));
                Ok(())
            }
            Literal::Str(s) => {
                self.constants.push(IRConst::Str(s));
                Ok(())
            }
            _ => Err(IRGenError::TypeError {
                message: "Invalid global constant type.".to_string(),
            }),
        }
    }

    fn func_decl(&mut self, decl: FuncDecl) -> Result<(), IRGenError> {
        let mut temp_ctx = Context::new();
        let params: Vec<(Operand, IRType)> = decl
            .params
            .iter()
            .enumerate()
            .map(|(i, (name, typ))| (Operand::Var(name.clone()), temp_ctx.from_var_type(typ)))
            .collect();

        let ret_type = temp_ctx.from_var_type(&decl.ret_type);

        self.functions.push(IRFunction {
            name: decl.name.clone(),
            params,
            ret_type,
            instructions: Vec::new(),
            is_pub: decl.is_pub,
            is_external: false,
        });
        Ok(())
    }

    fn compile_fn(&mut self, decl: FuncDecl) -> Result<(), IRGenError> {
        let name = decl.name.clone();
        let func = self.find_func(&name)?;

        let mut ctx = Context::new();
        ctx.enter_scope();

        for (i, (param, ty)) in func.params.iter().enumerate() {
            if let Operand::Var(name) = param {
                if let Some(scope) = ctx.scope.last_mut() {
                    scope.insert(
                        name.clone(),
                        Symbol {
                            name: name.clone(),
                            ir_type: ty.clone(),
                        },
                    );
                }
            }
        }

        let body = *decl.body;
        let last_op = self.compile_expr(body, &mut ctx)?;
        ctx.exit_scope()?;

        let last_inst_op = ctx.instructions.last().map(|i| i.op.clone());

        let last_is_return = matches!(last_inst_op, Some(Op::Return(_)));

        if !last_is_return {
            let reg = if func.ret_type == IRType::Float {
                "xmm0".to_string()
            } else {
                "rax".to_string()
            };

            ctx.instructions.push(Instruction {
                op: Op::Return(reg),
                dst: None,
                src1: Some(last_op),
                src2: None,
            });
        }

        if let Some(f) = self.functions.iter_mut().find(|f| f.name == name) {
            f.instructions = take(&mut ctx.instructions);
        }
        Ok(())
    }

    fn extern_decl(&mut self, ext: Extern) -> Result<(), IRGenError> {
        let name = ext.name;
        let params: Vec<(Operand, IRType)> = ext
            .params
            .into_iter()
            .enumerate()
            .map(|(i, typ)| {
                let temp_ctx = Context::new();
                let param_name = format!("a{}", i);
                (Operand::Var(param_name), temp_ctx.from_var_type(&typ))
            })
            .collect();

        let ret_type = Context::new().from_var_type(&ext.ret_type);

        let signature = IRFunction {
            name: name.clone(),
            params,
            ret_type,
            instructions: Vec::new(),
            is_pub: false,
            is_external: true,
        };
        self.functions.push(signature);
        Ok(())
    }

    fn find_func(&self, name: &String) -> Result<IRFunction, IRGenError> {
        for func in self.functions.iter().rev() {
            if func.name == *name {
                return Ok(func.to_owned());
            }
        }
        Err(IRGenError::NameError {
            message: format!("undefined function '{}' in current scope", name),
        })
    }
}
