use std::process::exit;

use crate::compiler::bytecode::{Bytecode, NativeEntry, Op, Value, call_native};
use std::collections::HashMap;

struct CallStack {
    return_ip: usize,
    base_slot: usize,
    operand_base: usize,
    cache_key: Option<(usize, Vec<Value>)>,
}

pub struct GVM {
    ip: usize,
    stack: Vec<Value>,
    slots: Vec<Value>,
    call_stack: Vec<CallStack>,
    curr_base_slot: usize,
    curr_operand_base: usize,
    bytecode: Bytecode,
    natives: HashMap<String, NativeEntry>,
    memo: HashMap<(usize, Vec<Value>), Value>,
}

impl GVM {
    pub fn new(bytecode: Bytecode, natives: HashMap<String, NativeEntry>) -> Self {
        GVM {
            ip: 0,
            stack: Vec::new(),
            slots: vec![Value::Void; bytecode.max_slot as usize],
            call_stack: Vec::new(),
            curr_base_slot: 0,
            curr_operand_base: 0,
            bytecode,
            natives,
            memo: HashMap::new(),
        }
    }

    fn read(&mut self) -> u8 {
        let b = self.bytecode.chunk.code[self.ip];
        self.ip += 1;
        b
    }

    fn enter_frame(&mut self, target: usize, args: Vec<Value>, operand_base: usize) {
        let args_count = args.len();
        let key = (target, args.clone());
        if let Some(cached) = self.memo.get(&key).cloned() {
            self.stack.push(cached);
            return;
        }

        self.call_stack.push(CallStack {
            return_ip: self.ip,
            base_slot: self.curr_base_slot,
            operand_base,
            cache_key: Some(key),
        });

        let new_base_slot = self.slots.len();
        for i in 0..args_count {
            self.slots.push(args[args_count - i - 1].clone());
        }
        self.curr_base_slot = new_base_slot;
        self.curr_operand_base = operand_base;
        self.ip = target;
    }

    fn pop(&mut self) -> Value {
        self.stack.pop().unwrap_or(Value::Void)
    }

    pub fn result(&self) -> Option<Value> {
        self.stack.last().cloned()
    }

    pub fn run(&mut self) {
        loop {
            let code = self.read();
            let op = Op::from_u8(code).expect("Bytecode: unknown opcode");
            match op {
                Op::LOADCONST => {
                    let idx = self.read() as usize;
                    self.stack.push(self.bytecode.chunk.constants[idx].clone());
                }
                Op::LOADVAR => {
                    let slot = self.read() as usize;
                    let index = self.curr_base_slot + slot;
                    if index >= self.slots.len() {
                        self.slots.resize(index + 1, Value::Void);
                    }
                    self.stack.push(self.slots[index].clone());
                }
                Op::STOREVAR => {
                    let slot = self.read() as usize;
                    let index = self.curr_base_slot + slot;
                    if index >= self.slots.len() {
                        self.slots.resize(index + 1, Value::Void);
                    }
                    self.slots[index] = self.pop();
                }
                Op::ADD => {
                    let right = self.pop();
                    let left = self.pop();
                    self.stack.push(match (&left, &right) {
                        (Value::Int(a), Value::Int(b)) => Value::Int(a.wrapping_add(*b)),
                        (Value::Float(a), Value::Float(b)) => Value::Float(a + b),
                        (Value::Str(a), Value::Str(b)) => Value::Str(a.clone() + b),
                        (Value::Bool(a), Value::Bool(b)) => Value::Bool(*a & *b),
                        (Value::Void, _) | (_, Value::Void) => Value::Void,
                        _ => panic!("TypeError: Wrong types for ADD operation"),
                    });
                }
                Op::SUB => {
                    let right = self.pop();
                    let left = self.pop();
                    self.stack.push(match (&left, &right) {
                        (Value::Int(a), Value::Int(b)) => Value::Int(a.wrapping_sub(*b)),
                        (Value::Float(a), Value::Float(b)) => Value::Float(a - b),
                        (Value::Void, _) | (_, Value::Void) => Value::Void,
                        _ => panic!("TypeError: Wrong types for SUB operation"),
                    });
                }
                Op::MUL => {
                    let right = self.pop();
                    let left = self.pop();
                    self.stack.push(match (&left, &right) {
                        (Value::Int(a), Value::Int(b)) => Value::Int(a.wrapping_mul(*b)),
                        (Value::Float(a), Value::Float(b)) => Value::Float(a * b),
                        (Value::Void, _) | (_, Value::Void) => Value::Void,
                        _ => panic!("TypeError: Wrong types for MUL operation"),
                    });
                }
                Op::DIV => {
                    let right = self.pop();
                    let left = self.pop();
                    self.stack.push(match (&left, &right) {
                        (Value::Int(a), Value::Int(b)) => Value::Int(a.wrapping_div(*b)),
                        (Value::Float(a), Value::Float(b)) => Value::Float(a / b),
                        (Value::Void, _) | (_, Value::Void) => Value::Void,
                        _ => panic!("TypeError: Wrong types for DIV operation"),
                    });
                }
                Op::MOD => {
                    let right = self.pop();
                    let left = self.pop();
                    self.stack.push(match (&left, &right) {
                        (Value::Int(a), Value::Int(b)) => Value::Int(a.wrapping_rem(*b)),
                        (Value::Float(a), Value::Float(b)) => Value::Float(a % b),
                        (Value::Void, _) | (_, Value::Void) => Value::Void,
                        _ => panic!("TypeError: Wrong types for MOD operation"),
                    });
                }
                Op::FADD => {
                    let right = self.pop();
                    let left = self.pop();
                    self.stack.push(match (left, right) {
                        (Value::Float(l), Value::Float(r)) => Value::Float(l + r),
                        (Value::Void, _) | (_, Value::Void) => Value::Void,
                        _ => panic!("TypeError: Wrong types for F_ADD operation"),
                    });
                }
                Op::FSUB => {
                    let right = self.pop();
                    let left = self.pop();
                    self.stack.push(match (left, right) {
                        (Value::Float(l), Value::Float(r)) => Value::Float(l - r),
                        (Value::Void, _) | (_, Value::Void) => Value::Void,
                        _ => panic!("TypeError: Wrong types for F_SUB operation"),
                    });
                }
                Op::FMUL => {
                    let right = self.pop();
                    let left = self.pop();
                    self.stack.push(match (left, right) {
                        (Value::Float(l), Value::Float(r)) => Value::Float(l * r),
                        (Value::Void, _) | (_, Value::Void) => Value::Void,
                        _ => panic!("TypeError: Wrong types for F_MUL operation"),
                    });
                }
                Op::FDIV => {
                    let right = self.pop();
                    let left = self.pop();
                    self.stack.push(match (left, right) {
                        (Value::Float(l), Value::Float(r)) => Value::Float(l / r),
                        (Value::Void, _) | (_, Value::Void) => Value::Void,
                        _ => panic!("TypeError: Wrong types for F_DIV operation"),
                    });
                }
                Op::EQ => {
                    let right = self.pop();
                    let left = self.pop();
                    self.stack.push(Value::Bool(left == right));
                }
                Op::NE => {
                    let right = self.pop();
                    let left = self.pop();
                    self.stack.push(Value::Bool(left != right));
                }
                Op::GT => {
                    let right = self.pop();
                    let left = self.pop();
                    self.stack
                        .push(Value::Bool(num_cmp(&left, &right, |a, b| a > b)));
                }
                Op::GE => {
                    let right = self.pop();
                    let left = self.pop();
                    self.stack
                        .push(Value::Bool(num_cmp(&left, &right, |a, b| a >= b)));
                }
                Op::LT => {
                    let right = self.pop();
                    let left = self.pop();
                    self.stack
                        .push(Value::Bool(num_cmp(&left, &right, |a, b| a < b)));
                }
                Op::LE => {
                    let right = self.pop();
                    let left = self.pop();
                    self.stack
                        .push(Value::Bool(num_cmp(&left, &right, |a, b| a <= b)));
                }
                Op::FEQ => {
                    let right = self.pop();
                    let left = self.pop();
                    self.stack
                        .push(Value::Bool(num_cmp(&left, &right, |a, b| a == b)));
                }
                Op::FNE => {
                    let right = self.pop();
                    let left = self.pop();
                    self.stack
                        .push(Value::Bool(num_cmp(&left, &right, |a, b| a != b)));
                }
                Op::FGT => {
                    let right = self.pop();
                    let left = self.pop();
                    self.stack
                        .push(Value::Bool(num_cmp(&left, &right, |a, b| a > b)));
                }
                Op::FGE => {
                    let right = self.pop();
                    let left = self.pop();
                    self.stack
                        .push(Value::Bool(num_cmp(&left, &right, |a, b| a >= b)));
                }
                Op::FLT => {
                    let right = self.pop();
                    let left = self.pop();
                    self.stack
                        .push(Value::Bool(num_cmp(&left, &right, |a, b| a < b)));
                }
                Op::FLE => {
                    let right = self.pop();
                    let left = self.pop();
                    self.stack
                        .push(Value::Bool(num_cmp(&left, &right, |a, b| a <= b)));
                }
                Op::AND => {
                    let right = self.pop();
                    let left = self.pop();
                    self.stack.push(match (left, right) {
                        (Value::Bool(l), Value::Bool(r)) => Value::Bool(l && r),
                        _ => panic!("TypeError: Wrong types for AND operation"),
                    });
                }
                Op::OR => {
                    let right = self.pop();
                    let left = self.pop();
                    self.stack.push(match (left, right) {
                        (Value::Bool(l), Value::Bool(r)) => Value::Bool(l || r),
                        _ => panic!("TypeError: Wrong types for OR operation"),
                    });
                }
                Op::POP => {
                    if self.stack.len() > self.curr_operand_base {
                        self.stack.pop();
                    }
                }
                Op::NEG => {
                    let v = self.pop();
                    self.stack.push(match v {
                        Value::Int(v) => Value::Int(v.wrapping_neg()),
                        Value::Void => Value::Void,
                        _ => panic!("TypeError: Wrong type for NEG operation"),
                    });
                }
                Op::FNEG => {
                    let v = self.pop();
                    self.stack.push(match v {
                        Value::Float(v) => Value::Float(-v),
                        Value::Void => Value::Void,
                        _ => panic!("TypeError: Wrong type for F_NEG operation"),
                    });
                }
                Op::POS => {}
                Op::INC => {
                    let v = self.pop();
                    self.stack.push(match v {
                        Value::Int(v) => Value::Int(v.wrapping_add(1)),
                        Value::Void => Value::Void,
                        _ => panic!("TypeError: Wrong type for INC operation"),
                    });
                }
                Op::DEC => {
                    let v = self.pop();
                    self.stack.push(match v {
                        Value::Int(v) => Value::Int(v.wrapping_sub(1)),
                        Value::Void => Value::Void,
                        _ => panic!("TypeError: Wrong type for DEC operation"),
                    });
                }
                Op::LOGNOT => {
                    let v = self.pop();
                    self.stack.push(match v {
                        Value::Bool(v) => Value::Bool(!v),
                        Value::Void => Value::Void,
                        _ => panic!("TypeError: Wrong type for LOG_NOT operation"),
                    });
                }
                Op::LOGAND => {
                    let right = self.pop();
                    let left = self.pop();
                    self.stack.push(match (left, right) {
                        (Value::Int(l), Value::Int(r)) => Value::Int(l & r),
                        (Value::Bool(l), Value::Bool(r)) => Value::Bool(l & r),
                        (Value::Void, _) | (_, Value::Void) => Value::Void,
                        _ => panic!("TypeError: Wrong types for LOG_AND operation"),
                    });
                }
                Op::LOGOR => {
                    let right = self.pop();
                    let left = self.pop();
                    self.stack.push(match (left, right) {
                        (Value::Int(l), Value::Int(r)) => Value::Int(l | r),
                        (Value::Bool(l), Value::Bool(r)) => Value::Bool(l | r),
                        (Value::Void, _) | (_, Value::Void) => Value::Void,
                        _ => panic!("TypeError: Wrong types for LOG_OR operation"),
                    });
                }
                Op::LOGXOR => {
                    let right = self.pop();
                    let left = self.pop();
                    self.stack.push(match (left, right) {
                        (Value::Int(l), Value::Int(r)) => Value::Int(l ^ r),
                        (Value::Bool(l), Value::Bool(r)) => Value::Bool(l ^ r),
                        (Value::Void, _) | (_, Value::Void) => Value::Void,
                        _ => panic!("TypeError: Wrong types for LOG_XOR operation"),
                    });
                }
                Op::JUMP => {
                    let high = self.read() as usize;
                    let low = self.read() as usize;
                    self.ip = (high << 8) | low;
                }
                Op::JUMPIFFALSE => {
                    let high = self.read() as usize;
                    let low = self.read() as usize;
                    let target = (high << 8) | low;
                    match self.pop() {
                        Value::Bool(false) => self.ip = target,
                        Value::Bool(true) | Value::Void => {}
                        _ => panic!("TypeError: Wrong type for JUMP_IF_FALSE operation"),
                    }
                }
                Op::CALL => {
                    let high = self.read() as usize;
                    let low = self.read() as usize;
                    let args_count = self.read() as usize;
                    let target = (high << 8) | low;

                    let operand_base = self.stack.len() - args_count;
                    let args: Vec<Value> = (0..args_count).map(|_| self.pop()).collect();
                    self.stack.truncate(operand_base);
                    self.enter_frame(target, args, operand_base);
                }
                Op::CALLVALUE => {
                    let args_count = self.read() as usize;
                    let operand_base = self.stack.len() - (args_count + 1);
                    let args: Vec<Value> = (0..args_count).map(|_| self.pop()).collect();
                    match self.pop() {
                        Value::Fn(target, arity) => {
                            if arity as usize != args_count {
                                panic!(
                                    "ArgumentError: lambda expects {arity} arguments, got {args_count}"
                                );
                            }
                            self.stack.truncate(operand_base);
                            self.enter_frame(target as usize, args, operand_base);
                        }
                        Value::Void => {}
                        _ => panic!("TypeError: value is not callable (CALL_VALUE operation)"),
                    }
                }
                Op::CALLNATIVE => {
                    let hi = self.read() as usize;
                    let lo = self.read() as usize;
                    let argc = self.read() as usize;
                    let idx = (hi << 8) | lo;
                    let entry = self
                        .bytecode
                        .chunk
                        .native_names
                        .get(idx)
                        .and_then(|name| self.natives.get(name).copied());
                    let entry = match entry {
                        Some(e) => e,
                        None => {
                            panic!(
                                "NativeError: unresolved native function (idx {idx}) in bytecode"
                            );
                        }
                    };
let operand_base = self.stack.len() - argc;
let mut args: Vec<Value> = (0..argc).map(|_| self.pop()).collect();
self.stack.truncate(operand_base);
args.reverse();
let result = call_native(&entry, &args)
    .unwrap_or_else(|| panic!("NativeError: call to native function failed"));
self.stack.push(result);
                }
                Op::MAKEFUNC => {
                    let high = self.read() as usize;
                    let low = self.read() as usize;
                    let arity = self.read() as usize;
                    self.stack
                        .push(Value::Fn(((high << 8) | low) as u32, arity as u32));
                }
                Op::TAILCALL => {
                    let high = self.read() as usize;
                    let low = self.read() as usize;
                    let args_count = self.read() as usize;
                    let target = (high << 8) | low;

                    let operand_base = self.stack.len() - args_count;
                    let args: Vec<Value> = (0..args_count).map(|_| self.pop()).collect();
                    self.stack.truncate(operand_base);

                    for i in 0..args_count {
                        let index = self.curr_base_slot + i;
                        if index >= self.slots.len() {
                            self.slots.resize(index + 1, Value::Void);
                        }
                        self.slots[index] = args[args_count - i - 1].clone();
                    }
                    self.slots.truncate(self.curr_base_slot + args_count);
                    self.ip = target;
                }
                Op::RET => {
                    let val = self.pop();
                    if self.call_stack.is_empty() {
                        panic!("RuntimeError: Call stack underflow on RET");
                    }
                    let frame = self.call_stack.pop().unwrap();
                    if let Some(key) = frame.cache_key {
                        self.memo.insert(key, val.clone());
                    }
                    let frame_size = self.slots.len() - self.curr_base_slot;
                    self.slots
                        .drain(self.curr_base_slot..self.curr_base_slot + frame_size);
                    self.ip = frame.return_ip;
                    self.curr_base_slot = frame.base_slot;
                    self.stack.truncate(frame.operand_base);
                    self.curr_operand_base = frame.operand_base;
                    if !matches!(val, Value::Void) || self.stack.is_empty() {
                        self.stack.push(val);
                    }
                }
                Op::NEWARRAY => {
                    let n = self.read() as usize;
                    let mut elems = Vec::with_capacity(n);
                    for _ in 0..n {
                        elems.push(self.pop());
                    }
                    elems.reverse();
                    self.stack.push(Value::Array(elems));
                }
                Op::ARRAYFILL => {
                    let elem = self.pop();
                    let len = match self.pop() {
                        Value::Int(n) if n >= 0 => n as usize,
                        Value::Int(_) => 0,
                        _ => panic!("TypeError: Wrong types for ARRAY_FILL operation"),
                    };
                    self.stack.push(Value::Array(vec![elem; len]));
                }
                Op::ARRAYGET => {
                    let idx = self.pop();
                    let arr = self.pop();
                    match (&arr, &idx) {
                        (Value::Array(a), Value::Int(i)) => {
                            if *i < 0 || *i as usize >= a.len() {
                                panic!("IndexError: Array index out of bounds: {i}");
                            }
                            self.stack.push(a[*i as usize].clone());
                        }
                        (Value::Void, _) | (_, Value::Void) => self.stack.push(Value::Void),
                        _ => panic!("TypeError: Wrong types for ARRAY_GET operation"),
                    }
                }
                Op::ARRAYSET => {
                    let slot = self.read() as usize;
                    let index = self.curr_base_slot + slot;
                    let value = self.pop();
                    let idx = self.pop();
                    let arr = self.slots.get(index).cloned().unwrap_or(Value::Void);
                    match (&arr, &idx) {
                        (Value::Array(a), Value::Int(i)) => {
                            if *i < 0 || *i as usize >= a.len() {
                                panic!("IndexError: Array index out of bounds: {i}");
                            }
                            let mut a = a.clone();
                            a[*i as usize] = value;
                            if index >= self.slots.len() {
                                self.slots.resize(index + 1, Value::Void);
                            }
                            self.slots[index] = Value::Array(a);
                        }
                        (Value::Void, _) | (_, Value::Void) => {}
                        _ => panic!("TypeError: Wrong types for ARRAY_SET operation"),
                    }
                }
                Op::EXIT => match self.pop() {
                    Value::Int(s) => exit(s as i32),
                    _ => exit(0),
                },
                Op::HALT => return,
            }
        }
    }
}

fn num_cmp(left: &Value, right: &Value, f: impl Fn(f64, f64) -> bool) -> bool {
    match (left, right) {
        (Value::Int(a), Value::Int(b)) => f(*a as f64, *b as f64),
        (Value::Float(a), Value::Float(b)) => f(*a, *b),
        _ => panic!("TypeError: Wrong types for comparison operation"),
    }
}
