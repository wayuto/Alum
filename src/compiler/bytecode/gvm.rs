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

    fn read_u32(&mut self) -> u32 {
        let mut buf = [0u8; 4];
        for b in &mut buf {
            *b = self.read();
        }
        u32::from_le_bytes(buf)
    }

    fn enter_frame(&mut self, target: usize, args: Vec<Value>, operand_base: usize) {
        let args_count = args.len();
        let key = (target, args.clone());
        if let Some(cached) = self.memo.get(&key).cloned() {
            self.stack.push(cached);
            return;
        }

        if self.call_stack.len() >= 1_000_000 {
            panic!("GVM: recursion depth exceeded 1000000 frames during constant evaluation");
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
        let mut steps: u64 = 0;
        let start = std::time::Instant::now();
        loop {
            let code = self.read();
            let op = Op::try_from(code).expect("Bytecode: unknown opcode");
            steps += 1;
            if steps > 100_000_000 {
                panic!(
                    "GVM: execution step limit exceeded (ip={}, op={:?})",
                    self.ip, op
                );
            }

            if self.stack.len() > 20_000_000 {
                panic!(
                    "GVM: operand stack limit exceeded ({} values)",
                    self.stack.len()
                );
            }

            if steps % 1_000_000 == 0 && start.elapsed() > std::time::Duration::from_secs(60) {
                panic!("GVM: constant evaluation exceeded 60s time budget");
            }
            match op {
                Op::LOADCONST => {
                    let idx = self.read_u32() as usize;
                    self.stack.push(self.bytecode.chunk.constants[idx].clone());
                }
                Op::LOADVAR => {
                    let slot = self.read_u32() as usize;
                    let index = self.curr_base_slot + slot;
                    if index >= self.slots.len() {
                        self.slots.resize(index + 1, Value::Void);
                    }
                    self.stack.push(self.slots[index].clone());
                }
                Op::STOREVAR => {
                    let slot = self.read_u32() as usize;
                    let index = self.curr_base_slot + slot;
                    if index >= self.slots.len() {
                        self.slots.resize(index + 1, Value::Void);
                    }

                    self.slots[index] = self.stack.last().cloned().unwrap_or(Value::Void);
                }
                Op::ADD => self.binop("ADD", |left, right| match (&left, &right) {
                    (Value::Int(a), Value::Int(b)) => Some(Value::Int(a.wrapping_add(*b))),
                    (Value::Float(a), Value::Float(b)) => Some(Value::Float(a + b)),
                    (Value::Str(a), Value::Str(b)) => Some(Value::Str(a.clone() + b)),
                    (Value::Bool(a), Value::Bool(b)) => Some(Value::Bool(*a & *b)),
                    _ => None,
                }),
                Op::SUB => self.binop("SUB", |left, right| match (&left, &right) {
                    (Value::Int(a), Value::Int(b)) => Some(Value::Int(a.wrapping_sub(*b))),
                    (Value::Float(a), Value::Float(b)) => Some(Value::Float(a - b)),
                    _ => None,
                }),
                Op::MUL => self.binop("MUL", |left, right| match (&left, &right) {
                    (Value::Int(a), Value::Int(b)) => Some(Value::Int(a.wrapping_mul(*b))),
                    (Value::Float(a), Value::Float(b)) => Some(Value::Float(a * b)),
                    _ => None,
                }),
                Op::DIV => self.binop("DIV", |left, right| match (&left, &right) {
                    (Value::Int(_), Value::Int(0)) => {
                        panic!("DivisionError: division by zero during constant evaluation")
                    }
                    (Value::Int(a), Value::Int(b)) => Some(Value::Int(a.wrapping_div(*b))),
                    (Value::Float(a), Value::Float(b)) => Some(Value::Float(a / b)),
                    _ => None,
                }),
                Op::MOD => self.binop("MOD", |left, right| match (&left, &right) {
                    (Value::Int(_), Value::Int(0)) => {
                        panic!("DivisionError: modulo by zero during constant evaluation")
                    }
                    (Value::Int(a), Value::Int(b)) => Some(Value::Int(a.wrapping_rem(*b))),
                    (Value::Float(a), Value::Float(b)) => Some(Value::Float(a % b)),
                    _ => None,
                }),
                Op::FADD => self.binop("F_ADD", |left, right| match (&left, &right) {
                    (Value::Float(l), Value::Float(r)) => Some(Value::Float(l + r)),
                    _ => None,
                }),
                Op::FSUB => self.binop("F_SUB", |left, right| match (&left, &right) {
                    (Value::Float(l), Value::Float(r)) => Some(Value::Float(l - r)),
                    _ => None,
                }),
                Op::FMUL => self.binop("F_MUL", |left, right| match (&left, &right) {
                    (Value::Float(l), Value::Float(r)) => Some(Value::Float(l * r)),
                    _ => None,
                }),
                Op::FDIV => self.binop("F_DIV", |left, right| match (&left, &right) {
                    (Value::Float(l), Value::Float(r)) => Some(Value::Float(l / r)),
                    _ => None,
                }),
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
                Op::GT | Op::FGT => {
                    let right = self.pop();
                    let left = self.pop();
                    self.stack.push(Value::Bool(num_cmp(
                        &left,
                        &right,
                        |a, b| a > b,
                        |a, b| a > b,
                    )));
                }
                Op::GE | Op::FGE => {
                    let right = self.pop();
                    let left = self.pop();
                    self.stack.push(Value::Bool(num_cmp(
                        &left,
                        &right,
                        |a, b| a >= b,
                        |a, b| a >= b,
                    )));
                }
                Op::LT | Op::FLT => {
                    let right = self.pop();
                    let left = self.pop();
                    self.stack.push(Value::Bool(num_cmp(
                        &left,
                        &right,
                        |a, b| a < b,
                        |a, b| a < b,
                    )));
                }
                Op::LE | Op::FLE => {
                    let right = self.pop();
                    let left = self.pop();
                    self.stack.push(Value::Bool(num_cmp(
                        &left,
                        &right,
                        |a, b| a <= b,
                        |a, b| a <= b,
                    )));
                }
                Op::FEQ => {
                    let right = self.pop();
                    let left = self.pop();
                    self.stack.push(Value::Bool(num_cmp(
                        &left,
                        &right,
                        |a, b| a == b,
                        |a, b| a == b,
                    )));
                }
                Op::FNE => {
                    let right = self.pop();
                    let left = self.pop();
                    self.stack.push(Value::Bool(num_cmp(
                        &left,
                        &right,
                        |a, b| a != b,
                        |a, b| a != b,
                    )));
                }
                Op::POP => {
                    if self.stack.len() > self.curr_operand_base {
                        self.stack.pop();
                    }
                }
                Op::NEG => self.unop("NEG", |v| match v {
                    Value::Int(v) => Some(Value::Int(v.wrapping_neg())),
                    _ => None,
                }),
                Op::FNEG => self.unop("F_NEG", |v| match v {
                    Value::Float(v) => Some(Value::Float(-*v)),
                    _ => None,
                }),
                Op::I2F => self.unop("I2F", |v| match v {
                    Value::Int(v) => Some(Value::Float(*v as f64)),
                    Value::Float(_) => Some(v.clone()),
                    _ => None,
                }),
                Op::F2I => self.unop("F2I", |v| match v {
                    Value::Float(v) => Some(Value::Int(*v as i64)),
                    Value::Int(_) => Some(v.clone()),
                    _ => None,
                }),
                Op::INC => self.unop("INC", |v| match v {
                    Value::Int(v) => Some(Value::Int(v.wrapping_add(1))),
                    _ => None,
                }),
                Op::DEC => self.unop("DEC", |v| match v {
                    Value::Int(v) => Some(Value::Int(v.wrapping_sub(1))),
                    _ => None,
                }),
                Op::LOGNOT => self.unop("LOG_NOT", |v| match v {
                    Value::Bool(v) => Some(Value::Bool(!*v)),
                    _ => None,
                }),
                Op::LOGAND => self.binop("LOG_AND", |left, right| match (&left, &right) {
                    (Value::Int(l), Value::Int(r)) => Some(Value::Int(l & r)),
                    (Value::Bool(l), Value::Bool(r)) => Some(Value::Bool(l & r)),
                    _ => None,
                }),
                Op::LOGOR => self.binop("LOG_OR", |left, right| match (&left, &right) {
                    (Value::Int(l), Value::Int(r)) => Some(Value::Int(l | r)),
                    (Value::Bool(l), Value::Bool(r)) => Some(Value::Bool(l | r)),
                    _ => None,
                }),
                Op::LOGXOR => self.binop("LOG_XOR", |left, right| match (&left, &right) {
                    (Value::Int(l), Value::Int(r)) => Some(Value::Int(l ^ r)),
                    (Value::Bool(l), Value::Bool(r)) => Some(Value::Bool(l ^ r)),
                    _ => None,
                }),
                Op::SHL => self.binop("SHL", |left, right| match (&left, &right) {
                    (Value::Int(l), Value::Int(r)) => Some(Value::Int(l.wrapping_shl(*r as u32))),
                    _ => None,
                }),
                Op::SHR => self.binop("SHR", |left, right| match (&left, &right) {
                    (Value::Int(l), Value::Int(r)) => Some(Value::Int(l.wrapping_shr(*r as u32))),
                    _ => None,
                }),
                Op::BNOT => self.unop("BNOT", |v| match v {
                    Value::Int(i) => Some(Value::Int(!*i)),
                    _ => None,
                }),
                Op::JUMP => {
                    self.ip = self.read_u32() as usize;
                }
                Op::JUMPIFFALSE => {
                    let target = self.read_u32() as usize;
                    match self.pop() {
                        Value::Bool(false) => self.ip = target,
                        Value::Bool(true) => {}

                        Value::Void => panic!("TypeError: Void condition for JUMP_IF_FALSE"),
                        _ => panic!("TypeError: Wrong type for JUMP_IF_FALSE operation"),
                    }
                }
                Op::CALL => {
                    let target = self.read_u32() as usize;
                    let args_count = self.read_u32() as usize;

                    let operand_base = self.stack.len() - args_count;
                    let args: Vec<Value> = (0..args_count).map(|_| self.pop()).collect();
                    self.stack.truncate(operand_base);
                    self.enter_frame(target, args, operand_base);
                }
                Op::CALLVALUE => {
                    let args_count = self.read_u32() as usize;
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
                        _ => panic!("TypeError: value is not callable (CALL_VALUE operation)"),
                    }
                }
                Op::CALLNATIVE => {
                    let idx = self.read_u32() as usize;
                    let argc = self.read_u32() as usize;
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
                    let addr = self.read_u32();
                    let arity = self.read_u32();
                    self.stack.push(Value::Fn(addr, arity));
                }
                Op::TAILCALL => {
                    let target = self.read_u32() as usize;
                    let args_count = self.read_u32() as usize;

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

                    if let Some(frame) = self.call_stack.last_mut() {
                        frame.cache_key = None;
                    }
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

                    self.stack.push(val);
                }
                Op::NEWARRAY => {
                    let n = self.read_u32() as usize;
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
                        (Value::Str(s), Value::Int(i)) => {
                            if *i < 0 || *i as usize >= s.len() {
                                panic!("IndexError: String index out of bounds: {i}");
                            }
                            let byte = s.as_bytes()[*i as usize];
                            self.stack.push(Value::Int(byte as i64));
                        }
                        (Value::Void, _) | (_, Value::Void) => self.stack.push(Value::Void),
                        _ => panic!("TypeError: Wrong types for ARRAY_GET operation"),
                    }
                }
                Op::ARRAYSET => {
                    let slot = self.read_u32() as usize;
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
                        (Value::Str(s), Value::Int(i)) => {
                            if *i < 0 || *i as usize >= s.len() {
                                panic!("IndexError: String index out of bounds: {i}");
                            }
                            let byte: u8 = match &value {
                                Value::Int(v) => *v as u8,
                                Value::Str(sv) => sv.as_bytes().first().copied().unwrap_or(0),
                                _ => panic!("TypeError: Wrong types for ARRAY_SET on string"),
                            };
                            let mut s = s.clone();
                            s.replace_range(*i as usize..*i as usize + 1, &(byte as char).to_string());
                            self.slots[index] = Value::Str(s);
                        }
                        (Value::Void, _) | (_, Value::Void) => {}
                        _ => panic!("TypeError: Wrong types for ARRAY_SET operation"),
                    }
                }
                Op::ARRAYLEN => {
                    let arr = self.pop();
                    match arr {
                        Value::Array(a) => self.stack.push(Value::Int(a.len() as i64)),
                        Value::Void => self.stack.push(Value::Int(0)),
                        _ => panic!("TypeError: ARRAYLEN on non-array"),
                    }
                }
                Op::HALT => return,
            }
        }
    }
    fn binop(&mut self, name: &str, f: impl FnOnce(&Value, &Value) -> Option<Value>) {
        let right = self.pop();
        let left = self.pop();
        let value = if matches!(left, Value::Void) || matches!(right, Value::Void) {
            Value::Void
        } else {
            f(&left, &right)
                .unwrap_or_else(|| panic!("TypeError: Wrong types for {} operation", name))
        };
        self.stack.push(value);
    }
    fn unop(&mut self, name: &str, f: impl FnOnce(&Value) -> Option<Value>) {
        let v = self.pop();
        let value = if matches!(v, Value::Void) {
            Value::Void
        } else {
            f(&v).unwrap_or_else(|| panic!("TypeError: Wrong type for {} operation", name))
        };
        self.stack.push(value);
    }
}

fn num_cmp(
    left: &Value,
    right: &Value,
    int_cmp: impl Fn(i64, i64) -> bool,
    float_cmp: impl Fn(f64, f64) -> bool,
) -> bool {
    match (left, right) {
        (Value::Int(a), Value::Int(b)) => int_cmp(*a, *b),
        (Value::Float(a), Value::Float(b)) => float_cmp(*a, *b),
        _ => panic!("TypeError: Wrong types for comparison operation"),
    }
}
