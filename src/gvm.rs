use std::{io::stdin, process::exit};

use crate::{bytecode::Op, compiler::Bytecode, token::Literal};

struct CallStack {
    return_ip: usize,
    base_slot: usize,
}

pub struct Gvm {
    ip: usize,
    stack: Vec<Literal>,
    slots: Vec<Literal>,
    call_stack: Vec<CallStack>,
    curr_base_slot: usize,
    bytecode: Bytecode,
}

impl Gvm {
    pub fn new(bytecode: Bytecode) -> Self {
        Gvm {
            ip: 0,
            stack: Vec::new(),
            slots: vec![Literal::Void; bytecode.max_slot as usize],
            call_stack: Vec::new(),
            curr_base_slot: 0,
            bytecode,
        }
    }

    pub fn run(&mut self) -> () {
        loop {
            let op = self.bytecode.chunk.code[self.ip as usize];
            self.ip += 1;
            match Op::from_u8(op).unwrap() {
                Op::LOADCONST => {
                    let idx = self.bytecode.chunk.code[self.ip] as usize;
                    self.ip += 1;
                    self.stack.push(self.bytecode.chunk.constants[idx].clone());
                }
                Op::LOADVAR => {
                    let slot = self.bytecode.chunk.code[self.ip] as usize;
                    self.ip += 1;
                    self.stack
                        .push(self.slots[self.curr_base_slot + slot].clone());
                }
                Op::STOREVAR => {
                    let slot = self.bytecode.chunk.code[self.ip] as usize;
                    self.ip += 1;
                    self.slots[self.curr_base_slot + slot] = self.stack.pop().unwrap();
                }
                Op::ADD => {
                    let right = self.stack.pop().unwrap();
                    let left = self.stack.pop().unwrap();
                    match (left, right) {
                        (Literal::Number(l), Literal::Number(r)) => {
                            self.stack.push(Literal::Number(l + r));
                        }
                        (Literal::Str(l), Literal::Str(r)) => {
                            self.stack.push(Literal::Str(l + &r));
                        }
                        (Literal::Void, _) => {
                            self.stack.push(Literal::Void);
                        }
                        _ => {
                            panic!("TypeError: Wrong types for ADD operation");
                        }
                    }
                }
                Op::SUB => {
                    let right = self.stack.pop().unwrap();
                    let left = self.stack.pop().unwrap();
                    match (left, right) {
                        (Literal::Number(l), Literal::Number(r)) => {
                            self.stack.push(Literal::Number(l - r));
                        }
                        (Literal::Void, _) => {
                            self.stack.push(Literal::Void);
                        }
                        _ => {
                            panic!("TypeError: Wrong types for SUB operation");
                        }
                    }
                }
                Op::MUL => {
                    let right = self.stack.pop().unwrap();
                    let left = self.stack.pop().unwrap();
                    match (left, right) {
                        (Literal::Number(l), Literal::Number(r)) => {
                            self.stack.push(Literal::Number(l * r));
                        }
                        (Literal::Void, _) => {
                            self.stack.push(Literal::Void);
                        }
                        _ => {
                            panic!("TypeError: Wrong types for MUL operation");
                        }
                    }
                }
                Op::DIV => {
                    let right = self.stack.pop().unwrap();
                    let left = self.stack.pop().unwrap();
                    match (left, right) {
                        (Literal::Number(l), Literal::Number(r)) => {
                            self.stack.push(Literal::Number(l / r));
                        }
                        (Literal::Void, _) => {
                            self.stack.push(Literal::Void);
                        }
                        _ => {
                            panic!("TypeError: Wrong types for DIV operation");
                        }
                    }
                }
                Op::EQ => {
                    let right = self.stack.pop().unwrap();
                    let left = self.stack.pop().unwrap();
                    self.stack.push(Literal::Bool(left == right));
                }
                Op::NE => {
                    let right = self.stack.pop().unwrap();
                    let left = self.stack.pop().unwrap();
                    self.stack.push(Literal::Bool(left != right));
                }
                Op::GT => {
                    let right = self.stack.pop().unwrap();
                    let left = self.stack.pop().unwrap();
                    match (left, right) {
                        (Literal::Number(l), Literal::Number(r)) => {
                            self.stack.push(Literal::Bool(l > r));
                        }
                        _ => {
                            panic!("TypeError: Wrong types for GT operation");
                        }
                    }
                }
                Op::GE => {
                    let right = self.stack.pop().unwrap();
                    let left = self.stack.pop().unwrap();
                    match (left, right) {
                        (Literal::Number(l), Literal::Number(r)) => {
                            self.stack.push(Literal::Bool(l >= r));
                        }
                        _ => {
                            panic!("TypeError: Wrong types for GE operation");
                        }
                    }
                }
                Op::LT => {
                    let right = self.stack.pop().unwrap();
                    let left = self.stack.pop().unwrap();
                    match (left, right) {
                        (Literal::Number(l), Literal::Number(r)) => {
                            self.stack.push(Literal::Bool(l < r));
                        }
                        _ => {
                            panic!("TypeError: Wrong types for LT operation");
                        }
                    }
                }
                Op::LE => {
                    let right = self.stack.pop().unwrap();
                    let left = self.stack.pop().unwrap();
                    match (left, right) {
                        (Literal::Number(l), Literal::Number(r)) => {
                            self.stack.push(Literal::Bool(l <= r));
                        }
                        _ => {
                            panic!("TypeError: Wrong types for LE operation");
                        }
                    }
                }
                Op::AND => {
                    let right = self.stack.pop().unwrap();
                    let left = self.stack.pop().unwrap();
                    match (left, right) {
                        (Literal::Bool(l), Literal::Bool(r)) => {
                            self.stack.push(Literal::Bool(l && r));
                        }
                        _ => {
                            panic!("TypeError: Wrong types for AND operation");
                        }
                    }
                }
                Op::OR => {
                    let right = self.stack.pop().unwrap();
                    let left = self.stack.pop().unwrap();
                    match (left, right) {
                        (Literal::Bool(l), Literal::Bool(r)) => {
                            self.stack.push(Literal::Bool(l || r));
                        }
                        _ => {
                            panic!("TypeError: Wrong types for OR operation");
                        }
                    }
                }
                Op::OUT => match self.stack.pop().unwrap() {
                    Literal::Bool(b) => {
                        println!("{}", b);
                    }
                    Literal::Number(n) => {
                        println!("{}", n);
                    }
                    Literal::Str(s) => {
                        println!("{}", s);
                    }
                    Literal::Void => {
                        println!("void");
                    }
                },
                Op::IN => {
                    let slot = self.bytecode.chunk.code[self.ip] as usize;
                    self.ip += 1;

                    let mut val = String::new();
                    stdin()
                        .read_line(&mut val)
                        .expect("GVM: Failed to read line");
                    self.slots[self.curr_base_slot + slot] = Literal::Str(val);
                }
                Op::POP => {
                    self.stack.pop();
                }
                Op::NEG => {
                    let value = self.stack.pop().unwrap();
                    match value {
                        Literal::Number(v) => {
                            self.stack.push(Literal::Number(-v));
                        }
                        Literal::Void => {
                            self.stack.push(Literal::Void);
                        }
                        _ => {
                            panic!("TypeError: Wrong type for NEG operation");
                        }
                    }
                }
                Op::INC => {
                    let value = self.stack.pop().unwrap();
                    match value {
                        Literal::Number(v) => {
                            self.stack.push(Literal::Number(v + 1.0));
                        }
                        Literal::Void => {
                            self.stack.push(Literal::Void);
                        }
                        _ => {
                            panic!("TypeError: Wrong type for INC operation");
                        }
                    }
                }
                Op::DEC => {
                    let value = self.stack.pop().unwrap();
                    match value {
                        Literal::Number(v) => {
                            self.stack.push(Literal::Number(v - 1.0));
                        }
                        Literal::Void => {
                            self.stack.push(Literal::Void);
                        }
                        _ => {
                            panic!("TypeError: Wrong type for DEC operation");
                        }
                    }
                }
                Op::LOGNOT => {
                    let value = self.stack.pop().unwrap();
                    match value {
                        Literal::Bool(v) => {
                            self.stack.push(Literal::Bool(!v));
                        }
                        Literal::Void => {
                            self.stack.push(Literal::Void);
                        }
                        _ => {
                            panic!("TypeError: Wrong type for LOGNOT operation");
                        }
                    }
                }
                Op::LOGAND => {
                    let right = self.stack.pop().unwrap();
                    let left = self.stack.pop().unwrap();
                    match (left, right) {
                        (Literal::Number(l), Literal::Number(r)) => {
                            self.stack
                                .push(Literal::Number((l as isize & r as isize) as f64));
                        }
                        (Literal::Bool(l), Literal::Bool(r)) => {
                            self.stack.push(Literal::Bool(l & r));
                        }
                        (Literal::Void, _) => {
                            self.stack.push(Literal::Void);
                        }
                        _ => {
                            panic!("TypeError: Wrong types for LOGAND operation");
                        }
                    }
                }
                Op::LOGOR => {
                    let right = self.stack.pop().unwrap();
                    let left = self.stack.pop().unwrap();
                    match (left, right) {
                        (Literal::Number(l), Literal::Number(r)) => {
                            self.stack
                                .push(Literal::Number((l as isize | r as isize) as f64));
                        }
                        (Literal::Bool(l), Literal::Bool(r)) => {
                            self.stack.push(Literal::Bool(l | r));
                        }
                        (Literal::Void, _) => {
                            self.stack.push(Literal::Void);
                        }
                        _ => {
                            panic!("TypeError: Wrong types for LOGOR operation");
                        }
                    }
                }
                Op::LOGXOR => {
                    let right = self.stack.pop().unwrap();
                    let left = self.stack.pop().unwrap();
                    match (left, right) {
                        (Literal::Number(l), Literal::Number(r)) => {
                            self.stack
                                .push(Literal::Number((l as isize ^ r as isize) as f64));
                        }
                        (Literal::Bool(l), Literal::Bool(r)) => {
                            self.stack.push(Literal::Bool(l ^ r));
                        }
                        (Literal::Void, _) => {
                            self.stack.push(Literal::Void);
                        }
                        _ => {
                            panic!("TypeError: Wrong types for LOGXOR operation");
                        }
                    }
                }
                Op::JUMP => {
                    let high = self.bytecode.chunk.code[self.ip] as usize;
                    self.ip += 1;
                    let low = self.bytecode.chunk.code[self.ip] as usize;
                    self.ip += 1;
                    let target = (high << 8) | low;
                    self.ip = target;
                }
                Op::JUMPIFFALSE => {
                    let high = self.bytecode.chunk.code[self.ip] as usize;
                    self.ip += 1;
                    let low = self.bytecode.chunk.code[self.ip] as usize;
                    self.ip += 1;
                    let target = (high << 8) | low;
                    let condition = self.stack.pop().unwrap();
                    match condition {
                        Literal::Bool(false) => {
                            self.ip = target;
                        }
                        Literal::Bool(true) => {}
                        Literal::Void => {}
                        _ => {
                            panic!("TypeError: Wrong type for JUMP_IF_FALSE operation");
                        }
                    }
                }
                Op::CALL => {
                    let high = self.bytecode.chunk.code[self.ip] as usize;
                    self.ip += 1;
                    let low = self.bytecode.chunk.code[self.ip] as usize;
                    self.ip += 1;
                    let args_count = self.bytecode.chunk.code[self.ip] as usize;
                    self.ip += 1;

                    let target = (high << 8) | low;

                    self.call_stack.push(CallStack {
                        return_ip: self.ip,
                        base_slot: self.curr_base_slot,
                    });

                    let new_base_slot = self.slots.len() as usize;

                    let args: Vec<Literal> =
                        (0..args_count).map(|_| self.stack.pop().unwrap()).collect();

                    for i in 0..args_count {
                        self.slots.push(args[args_count - i - 1].clone());
                    }

                    self.curr_base_slot = new_base_slot;
                    self.ip = target;
                }
                Op::RET => {
                    let val = self.stack.pop();

                    if self.call_stack.is_empty() {
                        panic!("RuntimeError: Call stack underflow on RET");
                    }

                    let frame = self.call_stack.pop().unwrap();

                    let curr_frame_size = self.slots.len() - self.curr_base_slot;
                    self.slots
                        .drain(self.curr_base_slot..self.curr_base_slot + curr_frame_size);

                    self.ip = frame.return_ip;
                    self.curr_base_slot = frame.base_slot;

                    if let Some(val) = val {
                        self.stack.push(val);
                    }
                }
                Op::EXIT => {
                    let status = self.stack.pop().unwrap();
                    match status {
                        Literal::Number(s) => {
                            exit(s as i32);
                        }
                        _ => {
                            exit(0);
                        }
                    }
                }
                Op::HALT => {
                    return;
                }
                _ => {}
            }
        }
    }
}
