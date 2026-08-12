use std::collections::HashMap;

use crate::compiler::{
    Span,
    bytecode::{Op, Value},
    parser::{Expr, Primitive, Program, Type},
};

#[derive(Debug, Clone)]
pub struct Bytecode {
    pub chunk: Chunk,
    pub max_slot: u32,
}

#[derive(Debug, Clone)]
pub struct Chunk {
    pub code: Vec<u8>,
    pub constants: Vec<Value>,
    pub native_names: Vec<String>,
}

struct Scope {
    vars: HashMap<String, u32>,
    slot_count: u32,
}

struct Func {
    addr: u32,
    param_count: u32,
}

pub struct Compiler {
    constants: Vec<Value>,
    code: Vec<u8>,
    scopes: Vec<Scope>,
    next_slot: u32,
    funcs: Vec<HashMap<String, Func>>,
    current_func: Option<(String, u32)>,
    lambda_counter: u32,
    native_names: Vec<String>,
    native_idx: HashMap<String, u16>,
    loop_targets: Vec<(u32, Vec<u32>)>,
}

impl Compiler {
    pub fn new() -> Self {
        Self {
            constants: Vec::new(),
            code: Vec::new(),
            scopes: Vec::new(),
            next_slot: 0,
            funcs: Vec::new(),
            current_func: None,
            lambda_counter: 0,
            native_names: Vec::new(),
            native_idx: HashMap::new(),
            loop_targets: Vec::new(),
        }
    }

    fn emit(&mut self, op: Op, args: &[u32]) -> () {
        self.code.push(op as u8);
        for arg in args {
            self.code.push(*arg as u8);
        }
    }

    fn enter_scope(&mut self) {
        self.scopes.push(Scope {
            vars: HashMap::new(),
            slot_count: 0,
        });
        self.funcs.push(HashMap::new());
    }

    fn exit_scope(&mut self) {
        if let Some(scope) = self.scopes.pop() {
            self.next_slot -= scope.slot_count;
        }
        self.funcs.pop();
    }

    fn load_var(&self, name: &str) -> Option<u32> {
        self.scopes
            .iter()
            .rev()
            .find_map(|scope| scope.vars.get(name).copied())
    }

    fn load_func(&self, name: &str) -> Option<Func> {
        self.funcs
            .iter()
            .rev()
            .find_map(|map| map.get(name))
            .map(|f| Func {
                addr: f.addr,
                param_count: f.param_count,
            })
    }

    fn decl_var(&mut self, name: &str) -> u32 {
        let curr_scope = self.scopes.last_mut().unwrap();
        let slot = self.next_slot;
        self.next_slot += 1;
        curr_scope.vars.insert(name.to_string(), slot);
        curr_scope.slot_count += 1;
        slot
    }

    fn mod_var(&mut self, name: &str) -> u32 {
        self.load_var(name)
            .unwrap_or_else(|| panic!("Compiler: Variable {} not found", name))
    }

    fn add_const(&mut self, value: Value) -> u32 {
        self.constants.push(value);
        (self.constants.len() - 1) as u32
    }

    pub fn compile(&mut self, program: Program) -> Bytecode {
        for expr in &program.body {
            if let Expr::FuncDecl(name, attrs, ..) = expr {
                if attrs.is_external {
                    let idx = self.native_names.len();
                    self.native_names.push(name.clone());
                    self.native_idx.insert(name.clone(), idx as u16);
                }
            }
        }

        self.enter_scope();

        for expr in program.body {
            self.compile_expr(&expr);
        }
        self.emit(Op::HALT, &[]);

        Bytecode {
            chunk: Chunk {
                code: self.code.clone(),
                constants: self.constants.clone(),
                native_names: self.native_names.clone(),
            },
            max_slot: self.next_slot,
        }
    }

    pub fn compile_expr(&mut self, expr: &Expr) {
        match expr {
            Expr::Int(n, _) => {
                let idx = self.add_const(Value::Int(*n as i64));
                self.emit(Op::LOADCONST, &[idx]);
            }
            Expr::Float(f, _) => {
                let idx = self.add_const(Value::Float(*f));
                self.emit(Op::LOADCONST, &[idx]);
            }
            Expr::Bool(b, _) => {
                let idx = self.add_const(Value::Bool(*b));
                self.emit(Op::LOADCONST, &[idx]);
            }
            Expr::String(s, _) => {
                let idx = self.add_const(Value::Str(s.clone()));
                self.emit(Op::LOADCONST, &[idx]);
            }
            Expr::Nil(_) => {
                let idx = self.add_const(Value::Void);
                self.emit(Op::LOADCONST, &[idx]);
            }
            Expr::Var(name, _) => {
                let slot = self.load_var(name);
                match slot {
                    Some(s) => self.emit(Op::LOADVAR, &[s]),
                    None => {
                        let func = self
                            .load_func(name)
                            .unwrap_or_else(|| panic!("Compiler: Variable {} not found", name));
                        self.emit(
                            Op::MAKEFUNC,
                            &[(func.addr >> 8) & 0xFF, func.addr & 0xFF, func.param_count],
                        );
                    }
                }
            }

            Expr::Add(l, r, _) | Expr::Sub(l, r, _) | Expr::Mul(l, r, _) | Expr::Div(l, r, _) => {
                self.compile_expr(l);
                self.compile_expr(r);
                let op = match expr {
                    Expr::Add(..) => Op::ADD,
                    Expr::Sub(..) => Op::SUB,
                    Expr::Mul(..) => Op::MUL,
                    Expr::Div(..) => Op::DIV,
                    _ => unreachable!(),
                };
                self.emit(op, &[]);
            }
            Expr::Mod(l, r, _) => {
                self.compile_expr(l);
                self.compile_expr(r);
                self.emit(Op::MOD, &[]);
            }
            Expr::FAdd(l, r, _)
            | Expr::FSub(l, r, _)
            | Expr::FMul(l, r, _)
            | Expr::FDiv(l, r, _) => {
                self.compile_expr(l);
                self.compile_expr(r);
                let op = match expr {
                    Expr::FAdd(..) => Op::FADD,
                    Expr::FSub(..) => Op::FSUB,
                    Expr::FMul(..) => Op::FMUL,
                    Expr::FDiv(..) => Op::FDIV,
                    _ => unreachable!(),
                };
                self.emit(op, &[]);
            }
            Expr::Eq(l, r, _)
            | Expr::Ne(l, r, _)
            | Expr::Lt(l, r, _)
            | Expr::Le(l, r, _)
            | Expr::Gt(l, r, _)
            | Expr::Ge(l, r, _) => {
                self.compile_expr(l);
                self.compile_expr(r);
                let op = match expr {
                    Expr::Eq(..) => Op::EQ,
                    Expr::Ne(..) => Op::NE,
                    Expr::Lt(..) => Op::LT,
                    Expr::Le(..) => Op::LE,
                    Expr::Gt(..) => Op::GT,
                    Expr::Ge(..) => Op::GE,
                    _ => unreachable!(),
                };
                self.emit(op, &[]);
            }
            Expr::FEq(l, r, _)
            | Expr::FNe(l, r, _)
            | Expr::FLt(l, r, _)
            | Expr::FLe(l, r, _)
            | Expr::FGt(l, r, _)
            | Expr::FGe(l, r, _) => {
                self.compile_expr(l);
                self.compile_expr(r);
                let op = match expr {
                    Expr::FEq(..) => Op::FEQ,
                    Expr::FNe(..) => Op::FNE,
                    Expr::FLt(..) => Op::FLT,
                    Expr::FLe(..) => Op::FLE,
                    Expr::FGt(..) => Op::FGT,
                    Expr::FGe(..) => Op::FGE,
                    _ => unreachable!(),
                };
                self.emit(op, &[]);
            }
            Expr::Not(e, _) => {
                self.compile_expr(e);
                self.emit(Op::LOGNOT, &[]);
            }
            Expr::Neg(e, _) => {
                self.compile_expr(e);
                self.emit(Op::NEG, &[]);
            }
            Expr::FNeg(e, _) => {
                self.compile_expr(e);
                self.emit(Op::FNEG, &[]);
            }
            Expr::Xor(l, r, _) => {
                self.compile_expr(l);
                self.compile_expr(r);
                self.emit(Op::LOGXOR, &[]);
            }
            Expr::LAnd(l, r, _) => {
                self.compile_expr(l);
                self.compile_expr(r);
                self.emit(Op::LOGAND, &[]);
            }
            Expr::LOr(l, r, _) => {
                self.compile_expr(l);
                self.compile_expr(r);
                self.emit(Op::LOGOR, &[]);
            }
            Expr::StrCat(l, r, _) => {
                self.compile_expr(l);
                self.compile_expr(r);
                self.emit(Op::ADD, &[]);
            }
            Expr::Inc(name, _) => self.compile_inc_dec(name, true),
            Expr::Dec(name, _) => self.compile_inc_dec(name, false),

            Expr::ArrayLiteral(elements, _) => {
                for e in elements {
                    self.compile_expr(e);
                }
                self.emit(Op::NEWARRAY, &[elements.len() as u32]);
            }
            Expr::ArrayFill(ty, len, _) => {
                self.compile_expr(len);
                let zero_idx = self.add_const(array_fill_zero(ty));
                self.emit(Op::LOADCONST, &[zero_idx]);
                self.emit(Op::ARRAYFILL, &[]);
            }
            Expr::Index(arr, idx, _) => {
                self.compile_expr(arr);
                self.compile_expr(idx);
                self.emit(Op::ARRAYGET, &[]);
            }
            Expr::IndexAssign(arr_idx, value, _) => {
                let (slot, idx) = match arr_idx.as_ref() {
                    Expr::Index(arr, idx, _) => match arr.as_ref() {
                        Expr::Var(name, _) => (self.mod_var(name), idx.clone()),
                        other => {
                            panic!("Compiler: unsupported indexed assignment target {other:?}")
                        }
                    },
                    other => panic!("Compiler: unsupported indexed assignment target {other:?}"),
                };
                self.compile_expr(&idx);
                self.compile_expr(value);
                self.emit(Op::ARRAYSET, &[slot]);
            }

            Expr::VarDecl(name, _, value, _) => {
                self.compile_expr(value);
                let slot = self.decl_var(name);
                self.emit(Op::STOREVAR, &[slot]);
                self.emit(Op::POP, &[]);
            }
            Expr::ConstDecl(name, _, value, _, _) => {
                self.compile_expr(value);
                let slot = self.decl_var(name);
                self.emit(Op::STOREVAR, &[slot]);
                self.emit(Op::POP, &[]);
            }
            Expr::GlobalVar(name, _, _, init, _) => {
                match init {
                    Some(init) => self.compile_expr(init),
                    None => {
                        let idx = self.add_const(Value::Void);
                        self.emit(Op::LOADCONST, &[idx]);
                    }
                }
                let slot = self.decl_var(name);
                self.emit(Op::STOREVAR, &[slot]);
                self.emit(Op::POP, &[]);
            }
            Expr::VarAssign(name, value, _) => {
                self.compile_expr(value);
                let slot = self.mod_var(name);
                self.emit(Op::STOREVAR, &[slot]);
                self.emit(Op::POP, &[]);
            }
            Expr::AddAssign(name, value, _) | Expr::SubAssign(name, value, _) => {
                let slot = self.mod_var(name);
                self.compile_expr(&Expr::Var(name.clone(), Span::new(0, 0)));
                self.compile_expr(value);
                match expr {
                    Expr::AddAssign(..) => self.emit(Op::ADD, &[]),
                    Expr::SubAssign(..) => self.emit(Op::SUB, &[]),
                    _ => unreachable!(),
                }
                self.emit(Op::STOREVAR, &[slot]);
                self.emit(Op::POP, &[]);
            }

            Expr::Return(value, _) => {
                if let Some((target, param_count)) = self.tail_self_call(value) {
                    if let Expr::Call(_, _, args, _) = value.as_ref() {
                        for arg in args {
                            self.compile_expr(arg);
                        }
                    }
                    self.emit(
                        Op::TAILCALL,
                        &[((target >> 8) & 0xFF), (target & 0xFF), param_count as u32],
                    );
                } else {
                    self.compile_expr(value);
                    self.emit(Op::RET, &[]);
                }
            }
            Expr::Block(body, _) => {
                self.enter_scope();
                for i in 0..body.len() {
                    let is_last = i == body.len() - 1;
                    self.compile_expr(&body[i]);
                    if !is_last {
                        self.emit(Op::POP, &[]);
                    }
                }
                self.exit_scope();
            }
            Expr::If(cond, then_branch, else_branch, _) => {
                self.compile_expr(cond);
                let then_addr = self.code.len() as u32;
                self.emit(Op::JUMPIFFALSE, &[0, 0]);

                self.compile_expr(then_branch);

                let mut else_addr: u32 = 1;
                if else_branch.is_some() {
                    else_addr = self.code.len() as u32;
                    self.emit(Op::JUMP, &[0, 0]);
                }
                let then_end = self.code.len() as u32;
                self.patch_jump_addr(then_addr + 1, then_end);

                if let Some(else_branch) = else_branch {
                    self.compile_expr(else_branch);
                    let else_end = self.code.len() as u32;
                    self.patch_jump_addr(else_addr + 1, else_end);
                }
            }
            Expr::While(cond, body, _) => {
                self.enter_scope();
                let loop_pos = self.code.len() as u32;
                self.compile_expr(cond);
                let iff = self.code.len() as u32;
                self.emit(Op::JUMPIFFALSE, &[0, 0]);
                self.loop_targets.push((loop_pos, Vec::new()));
                self.compile_expr(body);
                let break_pos = self.code.len() as u32;
                let (_, break_jumps) = self.loop_targets.pop().unwrap();
                for j in break_jumps {
                    self.patch_jump_addr(j + 1, break_pos);
                }
                self.emit(Op::JUMP, &[((loop_pos >> 8) & 0xff), loop_pos & 0xFF]);
                self.patch_jump_addr(iff + 1, break_pos);
                self.exit_scope();
            }
            Expr::Break(_) => {
                if let Some((_, break_jumps)) = self.loop_targets.last_mut() {
                    let j = self.code.len() as u32;
                    break_jumps.push(j);
                    self.emit(Op::JUMP, &[0, 0]);
                } else {
                    panic!("Compiler: break outside loop for bytecode");
                }
            }
            Expr::Continue(_) => {
                if let Some(&(continue_target, _)) = self.loop_targets.last() {
                    self.emit(
                        Op::JUMP,
                        &[((continue_target >> 8) & 0xff), continue_target & 0xFF],
                    );
                } else {
                    panic!("Compiler: continue outside loop for bytecode");
                }
            }
            Expr::FuncDecl(name, attrs, _, params, _, body, _) => {
                if attrs.is_external {
                    return;
                }
                self.compile_func(name, params.len(), params, body);
            }
            Expr::Call(f, _, args, _) => {
                let direct_func: Option<Func> = match f.as_ref() {
                    Expr::Var(name, _) => {
                        if self.load_var(name).is_none() {
                            self.load_func(name)
                        } else {
                            None
                        }
                    }
                    _ => None,
                };
                match direct_func {
                    Some(func) => {
                        if func.param_count != args.len() as u32 {
                            panic!(
                                "Compiler: Function call expects {} arguments, got {}",
                                func.param_count,
                                args.len()
                            );
                        }
                        for arg in args {
                            self.compile_expr(arg);
                        }
                        self.emit(
                            Op::CALL,
                            &[
                                ((func.addr >> 8) & 0xFF),
                                func.addr & 0xFF,
                                func.param_count,
                            ],
                        );
                    }
                    None => {
                        if let Expr::Var(name, _) = f.as_ref() {
                            if let Some(&idx) = self.native_idx.get(name) {
                                if args.len() > 255 {
                                    panic!("Compiler: too many arguments for native call");
                                }
                                for arg in args {
                                    self.compile_expr(arg);
                                }
                                self.emit(
                                    Op::CALLNATIVE,
                                    &[
                                        ((idx >> 8) & 0xFF) as u32,
                                        (idx & 0xFF) as u32,
                                        args.len() as u32,
                                    ],
                                );
                                return;
                            }
                        }
                        self.compile_expr(f);
                        for arg in args {
                            self.compile_expr(arg);
                        }
                        self.emit(Op::CALLVALUE, &[args.len() as u32]);
                    }
                }
            }

            Expr::Lambda(params, body, _, _) => {
                let name = format!("_lambda_{}", self.lambda_counter);
                self.lambda_counter += 1;

                let jump_addr = self.code.len() as u32;
                self.emit(Op::JUMP, &[0, 0]);
                let func_addr = self.code.len() as u32;

                self.funcs.last_mut().unwrap().insert(
                    name.clone(),
                    Func {
                        addr: func_addr,
                        param_count: params.len() as u32,
                    },
                );

                self.enter_scope();
                for (param, _) in params {
                    self.decl_var(param);
                }
                self.current_func = Some((name, func_addr));
                self.compile_expr(body);
                self.current_func = None;

                match &**body {
                    Expr::Return(..) => {}
                    _ => self.emit(Op::RET, &[]),
                }
                self.exit_scope();

                self.patch_jump_addr(jump_addr + 1, self.code.len() as u32);
                self.emit(
                    Op::MAKEFUNC,
                    &[
                        (func_addr >> 8) & 0xFF,
                        func_addr & 0xFF,
                        params.len() as u32,
                    ],
                );
            }

            _ => {
                panic!("Compiler: unsupported expression for bytecode: {expr:?}");
            }
        }
    }

    fn compile_inc_dec(&mut self, name: &str, inc: bool) {
        let slot = self.mod_var(name);
        self.emit(Op::LOADVAR, &[slot]);
        self.emit(if inc { Op::INC } else { Op::DEC }, &[]);
        self.emit(Op::STOREVAR, &[slot]);
        self.emit(Op::POP, &[]);
    }

    fn compile_func(
        &mut self,
        name: &str,
        num_params: usize,
        params: &[(String, Type)],
        body: &Expr,
    ) {
        let jump_addr = self.code.len() as u32;
        self.emit(Op::JUMP, &[0, 0]);
        let func_addr = self.code.len() as u32;

        let curr_func = self.funcs.last_mut().unwrap();
        if curr_func.contains_key(name) {
            panic!("Compiler: Function {} already declared", name);
        }
        curr_func.insert(
            name.to_string(),
            Func {
                addr: func_addr,
                param_count: num_params as u32,
            },
        );

        self.current_func = Some((name.to_string(), func_addr));
        self.enter_scope();
        for (param, _) in params {
            self.decl_var(param);
        }
        self.compile_expr(body);

        match body {
            Expr::Return(..) => {}
            _ => self.emit(Op::RET, &[]),
        }
        self.exit_scope();
        self.current_func = None;

        self.patch_jump_addr(jump_addr + 1, self.code.len() as u32);
    }

    fn patch_jump_addr(&mut self, pos: u32, addr: u32) {
        self.code[pos as usize] = ((addr >> 8) & 0xff) as u8;
        self.code[(pos + 1) as usize] = (addr & 0xff) as u8;
    }

    fn tail_self_call(&self, value: &Expr) -> Option<(u32, usize)> {
        let Expr::Call(callee, _, args, _) = value else {
            return None;
        };
        let Expr::Var(name, _) = callee.as_ref() else {
            return None;
        };
        let Some((fname, faddr)) = &self.current_func else {
            return None;
        };
        if name != fname {
            return None;
        }
        Some((*faddr, args.len()))
    }
}

fn array_fill_zero(ty: &Type) -> Value {
    match ty {
        Type::Primitive(Primitive::Float) => Value::Float(0.0),
        Type::Primitive(Primitive::Boolean) => Value::Bool(false),
        Type::Primitive(Primitive::String) => Value::Str(String::new()),
        Type::Array(_) => Value::Array(Vec::new()),
        _ => Value::Int(0),
    }
}
