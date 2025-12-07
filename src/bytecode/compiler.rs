use core::panic;
use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::{
    ast::{Expr, Program},
    bytecode::Op,
    token::{Literal, TokenType},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bytecode {
    pub chunk: Chunk,
    pub max_slot: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chunk {
    pub code: Vec<u8>,
    pub constants: Vec<Literal>,
}

struct Scope {
    vars: HashMap<String, u32>,
    slot_count: u32,
}

struct Func {
    addr: u32,
    param_count: u32,
}

struct Label {
    high: u32,
    low: u32,
}

pub struct Compiler {
    constants: Vec<Literal>,
    code: Vec<u8>,
    scopes: Vec<Scope>,
    next_slot: u32,
    funcs: Vec<HashMap<String, Func>>,
    labels: HashMap<String, Label>,
}

impl Compiler {
    pub fn new() -> Self {
        Self {
            constants: Vec::new(),
            code: Vec::new(),
            scopes: Vec::new(),
            next_slot: 0,
            funcs: Vec::new(),
            labels: HashMap::new(),
        }
    }

    fn emit(&mut self, op: Op, args: &[u32]) -> () {
        self.code.push(op as u8);
        for arg in args {
            self.code.push(*arg as u8);
        }
    }

    fn enter_scope(&mut self) -> () {
        self.scopes.push(Scope {
            vars: HashMap::new(),
            slot_count: 0,
        });
        self.funcs.push(HashMap::new())
    }

    fn exit_scope(&mut self) -> () {
        if let Some(scope) = self.scopes.pop() {
            self.next_slot -= scope.slot_count;
        }
        self.funcs.pop();
    }

    fn load_var(&self, name: String) -> Option<&u32> {
        self.scopes
            .iter()
            .rev()
            .find_map(|scope| scope.vars.get(&name))
    }

    fn load_func(&self, name: String) -> Option<Func> {
        for func_map in self.funcs.iter().rev() {
            if let Some(func) = func_map.get(&name) {
                return Some(Func {
                    addr: func.addr,
                    param_count: func.param_count,
                });
            }
        }
        None
    }

    fn decl_var(&mut self, name: String) -> u32 {
        let curr_scope = self.scopes.last_mut().unwrap();
        let slot = self.next_slot;
        self.next_slot += 1;
        curr_scope.vars.insert(name, slot);
        curr_scope.slot_count += 1;
        slot
    }

    fn mod_var(&mut self, name: String) -> u32 {
        let slot = self.load_var(name.clone());
        match slot {
            Some(s) => *s,
            None => panic!("Compiler: Variable {} not found", name),
        }
    }

    pub fn compile(&mut self, program: Program) -> Bytecode {
        self.enter_scope();

        for expr in program.body {
            self.compile_expr(expr)
        }
        self.emit(Op::HALT, &[]);

        return Bytecode {
            chunk: Chunk {
                code: self.code.clone(),
                constants: self.constants.clone(),
            },
            max_slot: self.next_slot,
        };
    }

    fn compile_expr(&mut self, expr: Expr) -> () {
        match expr {
            Expr::Val(val) => {
                let value = val.value;
                self.constants.push(value);
                self.emit(Op::LOADCONST, &[(self.constants.len() - 1) as u32]);
            }
            Expr::Var(var) => {
                let slot = self.load_var(var.name.clone());
                match slot {
                    Some(s) => {
                        self.emit(Op::LOADVAR, &[*s]);
                    }
                    None => {
                        panic!("Compiler: Variable {} not found", var.name);
                    }
                }
            }
            Expr::VarDecl(decl) => {
                self.compile_expr(*decl.value);
                let slot = self.decl_var(decl.name);
                self.emit(Op::STOREVAR, &[slot]);
                self.emit(Op::POP, &[]);
            }
            Expr::VarMod(decl) => {
                self.compile_expr(*decl.value);
                let slot = self.mod_var(decl.name);
                self.emit(Op::STOREVAR, &[slot]);
                self.emit(Op::POP, &[]);
            }
            Expr::BinOp(bin) => {
                self.compile_expr(*bin.left);
                self.compile_expr(*bin.right);
                match bin.operator {
                    TokenType::ADD => self.emit(Op::ADD, &[]),
                    TokenType::SUB => self.emit(Op::SUB, &[]),
                    TokenType::MUL => self.emit(Op::MUL, &[]),
                    TokenType::DIV => self.emit(Op::DIV, &[]),
                    TokenType::LOGAND => self.emit(Op::LOGAND, &[]),
                    TokenType::LOGOR => self.emit(Op::LOGOR, &[]),
                    TokenType::LOGXOR => self.emit(Op::LOGXOR, &[]),
                    TokenType::COMPEQ => self.emit(Op::EQ, &[]),
                    TokenType::COMPNE => self.emit(Op::NE, &[]),
                    TokenType::COMPLT => self.emit(Op::LT, &[]),
                    TokenType::COMPGT => self.emit(Op::GT, &[]),
                    TokenType::COMPLE => self.emit(Op::LE, &[]),
                    TokenType::COMPGE => self.emit(Op::GE, &[]),
                    TokenType::COMPAND => self.emit(Op::AND, &[]),
                    TokenType::COMPOR => self.emit(Op::OR, &[]),
                    _ => {
                        panic!("Compiler: Unimplemented binary operator {:?}", bin.operator);
                    }
                }
            }
            Expr::UnaryOp(unary) => {
                self.compile_expr(*unary.argument.clone());
                match unary.operator {
                    TokenType::NEG => self.emit(Op::NEG, &[]),
                    TokenType::LOGNOT => self.emit(Op::LOGNOT, &[]),
                    TokenType::INC => match *unary.argument.clone() {
                        Expr::Var(var) => {
                            let name = var.name.clone();
                            let slot = self.mod_var(name.clone());
                            self.emit(Op::LOADVAR, &[slot]);
                            self.emit(Op::INC, &[]);
                            self.emit(Op::STOREVAR, &[slot]);
                        }
                        _ => panic!("Compiler: Unary INC operator only supports variables"),
                    },
                    TokenType::DEC => match *unary.argument.clone() {
                        Expr::Var(var) => {
                            let name = var.name.clone();
                            let slot = self.mod_var(name.clone());
                            self.emit(Op::LOADVAR, &[slot]);
                            self.emit(Op::DEC, &[]);
                            self.emit(Op::STOREVAR, &[slot]);
                        }
                        _ => panic!("Compiler: Unary DEC operator only supports variables"),
                    },
                    _ => {
                        panic!(
                            "Compiler: Unimplemented unary operator {:?}",
                            unary.operator
                        );
                    }
                }
            }
            Expr::Out(out) => {
                self.compile_expr(*out.value);
                self.emit(Op::OUT, &[]);
            }
            Expr::In(i) => {
                let slot = self.decl_var(i.name);
                self.emit(Op::IN, &[slot]);
            }
            Expr::Stmt(stmt) => {
                self.enter_scope();
                let body = stmt.body;

                for i in 0..body.len() - 1 {
                    self.compile_expr(body[i].clone());
                    self.emit(Op::POP, &[]);
                }

                if body.len() > 0 {
                    self.compile_expr(body.last().unwrap().clone());
                } else {
                    self.constants.push(Literal::Void);
                    self.emit(Op::LOADCONST, &[self.constants.len() as u32 - 1]);
                }

                self.exit_scope();
            }
            Expr::If(i) => {
                self.compile_expr(*i.condition);

                let then_branch_addr = self.code.len() as u32;
                self.emit(Op::JUMPIFFALSE, &[0, 0]);

                self.enter_scope();
                self.compile_expr(*i.then);
                self.exit_scope();

                let mut else_branch_addr: u32 = 1;
                if let Some(_) = i.else_branch.clone() {
                    else_branch_addr = self.code.len() as u32;
                    self.emit(Op::JUMP, &[0, 0]);
                }

                let then_end_addr = self.code.len() as u32;
                self.patch_jump_addr(then_branch_addr + 1, then_end_addr);

                if let Some(else_branch) = i.else_branch {
                    self.enter_scope();
                    self.compile_expr(*else_branch);
                    self.exit_scope();

                    let else_end_addr = self.code.len() as u32;
                    self.patch_jump_addr(else_branch_addr + 1, else_end_addr);
                }
            }
            Expr::While(w) => {
                self.enter_scope();

                let loop_pos = self.code.len() as u32;
                self.compile_expr(*w.condition.clone());

                let jump_if_false = self.code.len() as u32;
                self.emit(Op::JUMPIFFALSE, &[0, 0]);

                self.compile_expr(*w.body.clone());
                self.emit(
                    Op::JUMP,
                    &[((loop_pos >> 8) & 0xff) as u32, loop_pos & 0xFF],
                );

                let break_pos = self.code.len() as u32;
                self.patch_jump_addr(jump_if_false + 1, break_pos);

                self.exit_scope();
            }
            Expr::FuncDecl(decl) => {
                let jump_addr = self.code.len() as u32;
                self.emit(Op::JUMP, &[0, 0]);
                let func_addr = self.code.len() as u32;
                let curr_func = self.funcs.last_mut().unwrap();

                if curr_func.contains_key(&decl.name) {
                    panic!("Compiler: Function {} already declared", decl.name);
                }

                curr_func.insert(
                    decl.name.clone(),
                    Func {
                        addr: func_addr,
                        param_count: decl.params.len() as u32,
                    },
                );

                self.enter_scope();

                for param in decl.params {
                    self.decl_var(param);
                }

                self.compile_expr(*decl.body);
                self.emit(Op::RET, &[]);

                self.exit_scope();
                self.patch_jump_addr(jump_addr + 1, self.code.len() as u32);
            }
            Expr::FuncCall(call) => {
                for arg in call.args.clone() {
                    self.compile_expr(arg);
                }

                let func = self.load_func(call.name.clone());

                match func {
                    Some(f) => {
                        if f.param_count != call.args.len() as u32 {
                            panic!(
                                "Compiler: Function {} expects {} arguments, got {}",
                                call.name,
                                f.param_count,
                                call.args.len()
                            );
                        }

                        let target = f.addr;

                        self.emit(
                            Op::CALL,
                            &[
                                ((target >> 8) & 0xFF) as u32,
                                (target & 0xFF) as u32,
                                f.param_count,
                            ],
                        );
                    }
                    _ => {
                        panic!("Compiler: Function {} not found", call.name);
                    }
                }
            }
            Expr::Exit(exit) => {
                self.compile_expr(*exit.code);
                self.emit(Op::EXIT, &[]);
            }
            Expr::Return(ret) => {
                if let Some(value) = ret.value {
                    self.compile_expr(*value);
                } else {
                    self.constants.push(Literal::Void);
                    self.emit(Op::LOADCONST, &[self.constants.len() as u32 - 1]);
                }
                self.emit(Op::RET, &[]);
            }
            Expr::Label(label) => {
                let addr = self.code.len() as u32;
                self.labels.insert(
                    label.name.clone(),
                    Label {
                        high: (addr >> 8) & 0xff,
                        low: (addr & 0xff),
                    },
                );
            }
            Expr::Goto(goto) => {
                let label = self.labels.get(&goto.label);
                match label {
                    Some(l) => {
                        self.emit(Op::JUMP, &[l.high, l.low]);
                    }
                    None => {
                        panic!("Compiler: Label {} not found", goto.label);
                    }
                }
            }
        }
    }

    fn patch_jump_addr(&mut self, pos: u32, addr: u32) -> () {
        self.code[pos as usize] = ((addr >> 8) & 0xff) as u8;
        self.code[(pos + 1) as usize] = (addr & 0xff) as u8;
    }
}

impl Bytecode {
    pub fn print(&self) -> () {
        println!("=== Constants ===");
        for (i, constant) in self.chunk.constants.iter().enumerate() {
            println!("  [{}] {:?}", i, constant);
        }
        println!("\n=== Bytecode ===");
        println!("Max Slot: {}", self.max_slot);

        let mut i = 0;
        while i < self.chunk.code.len() {
            let op_byte = self.chunk.code[i];
            if let Some(opcode) = Op::from_u8(op_byte) {
                let args_count = opcode.operand_count();
                print!("{:04x}: {:12}", i, Op::to_str(opcode.clone()));

                match opcode {
                    Op::LOADCONST => {
                        if i + 1 < self.chunk.code.len() {
                            let const_index = self.chunk.code[i + 1] as usize;
                            if const_index < self.chunk.constants.len() {
                                print!(
                                    " [{}] ; {:?}",
                                    const_index, self.chunk.constants[const_index]
                                );
                            } else {
                                print!(" [{}] ; INVALID", const_index);
                            }
                        }
                    }
                    Op::LOADVAR | Op::STOREVAR | Op::IN => {
                        if i + 1 < self.chunk.code.len() {
                            let slot = self.chunk.code[i + 1];
                            print!(" [slot {}]", slot);
                        }
                    }
                    Op::JUMP | Op::JUMPIFFALSE => {
                        if i + 2 < self.chunk.code.len() {
                            let addr = ((self.chunk.code[i + 1] as u16) << 8)
                                | (self.chunk.code[i + 2] as u16);
                            print!(" [addr {:04x}]", addr);
                        }
                    }
                    Op::CALL => {
                        if i + 3 < self.chunk.code.len() {
                            let addr = ((self.chunk.code[i + 1] as u16) << 8)
                                | (self.chunk.code[i + 2] as u16);
                            let arg_count = self.chunk.code[i + 3];
                            print!(" [addr {:04x}, {} args]", addr, arg_count);
                        }
                    }
                    _ => {
                        for j in 1..=args_count {
                            if i + j < self.chunk.code.len() {
                                print!(" {:02x}", self.chunk.code[i + j]);
                            }
                        }
                    }
                }
                println!();

                i += 1 + args_count;
            } else {
                println!("{:04x}: [UNKNOWN: {:02x}]", i, op_byte);
                i += 1;
            }
        }
    }
}
