use std::{collections::HashMap, mem::take};

use clap::builder::Str;

use crate::native::{IRConst, IRFunction, IRProgram, Instruction, Op, Operand};

macro_rules! assemble {
    ($buf:expr, $fmt:literal $(, $arg:expr)* $(,)?) => {
        $buf.push_str(&format!(concat!($fmt, "\n") $(, $arg)*))
    };
}

pub struct CodeGen {
    program: IRProgram,
    text: String,
    data: String,
    vars: HashMap<String, usize>,
    str_cnt: usize,
    stack_ptr: usize,
    arg_reg: Vec<String>,
    ret_label: String,
    regs: HashMap<String, Option<Operand>>,
    curr_fn: String,
    loop_label: String,
}

impl CodeGen {
    pub fn new(program: IRProgram) -> Self {
        Self {
            program,
            text: String::new(),
            data: String::new(),
            vars: HashMap::new(),
            str_cnt: 0,
            stack_ptr: 0,
            arg_reg: vec![
                "rdi".to_string(),
                "rsi".to_string(),
                "rdx".to_string(),
                "rcx".to_string(),
                "r8".to_string(),
                "r9".to_string(),
            ],
            ret_label: String::new(),
            regs: HashMap::new(),
            curr_fn: String::new(),
            loop_label: String::new(),
        }
    }

    pub fn compile(&mut self) -> String {
        assemble!(self.text, "section .text");
        assemble!(self.data, "section .data");
        for func in take(&mut self.program.functions) {
            self.compile_fn(func);
        }
        take(&mut self.data) + &self.text
    }

    fn compile_code(&mut self, code: Instruction) {
        match code.op {
            Op::Move => {
                let src = code.src1.as_ref().unwrap();
                let dst = code.dst.as_ref().unwrap();
                self.load(src, "rax");
                assemble!(self.text, "mov [rbp - {}], rax", self.get_offset(dst));

                self.regs.insert("rax".to_string(), Some(dst.clone()));
            }
            Op::Load | Op::Store => {
                let src = code.src1.as_ref().unwrap();
                let dst = code.dst.as_ref().unwrap();
                self.load(src, "rax");
                assemble!(self.text, "mov [rbp - {}], rax", self.get_offset(dst));
            }
            Op::Add | Op::Sub | Op::Mul | Op::Div | Op::LAnd | Op::LOr | Op::Xor => {
                let dst = code.dst.as_ref().unwrap();
                let src1 = code.src1.as_ref().unwrap();
                let src2 = code.src2.as_ref().unwrap();

                self.load(src1, "rax");
                self.load(src2, "rbx");
                match code.op {
                    Op::Add => assemble!(self.text, "add rax, rbx"),
                    Op::Sub => assemble!(self.text, "sub rax, rbx"),
                    Op::Mul => assemble!(self.text, "imul rax, rbx"),
                    Op::Div => {
                        assemble!(self.text, "cqo");
                        assemble!(self.text, "idiv rbx");
                    }
                    Op::LAnd => assemble!(self.text, "and rax, rbx"),
                    Op::LOr => assemble!(self.text, "or rax, rbx"),
                    Op::Xor => assemble!(self.text, "xor rax, rbx"),
                    _ => unreachable!(),
                }
                assemble!(self.text, "mov [rbp - {}], rax", self.get_offset(dst));
                self.regs.clear();
                self.regs.insert("rax".to_string(), Some(dst.clone()));
            }
            Op::Eq | Op::Ne | Op::Gt | Op::Ge | Op::Lt | Op::Le => {
                let dst = code.dst.as_ref().unwrap();
                let src1 = code.src1.as_ref().unwrap();
                let src2 = code.src2.as_ref().unwrap();
                self.load(src1, "rax");
                self.load(src2, "rbx");
                assemble!(self.text, "cmp rax, rbx");
                let set_op = match code.op {
                    Op::Eq => "sete",
                    Op::Ne => "setne",
                    Op::Gt => "setg",
                    Op::Ge => "setge",
                    Op::Lt => "setl",
                    Op::Le => "setle",
                    _ => unreachable!(),
                };
                assemble!(self.text, "{} al", set_op);
                assemble!(self.text, "movzx eax, al");
                assemble!(self.text, "mov [rbp - {}], rax", self.get_offset(dst));
                self.regs.clear();
                self.regs.insert("rax".to_string(), Some(dst.clone()));
            }
            Op::Neg | Op::Inc | Op::Dec | Op::SizeOf => {
                let dst = code.dst.as_ref().unwrap();
                let src1 = code.src1.as_ref().unwrap();
                self.load(src1, "rax");
                match code.op {
                    Op::Neg => assemble!(self.text, "neg rax"),
                    Op::Inc => assemble!(self.text, "inc rax"),
                    Op::Dec => assemble!(self.text, "dec rax"),
                    Op::SizeOf => assemble!(self.text, "mov rax, [rax]"),
                    _ => unreachable!(),
                }
                assemble!(self.text, "mov [rbp - {}], rax", self.get_offset(dst));
                self.regs.clear();
                self.regs.insert("rax".to_string(), Some(dst.clone()));
            }
            Op::Range => {
                let dst = code.dst.as_ref().unwrap();
                self.load(code.src1.as_ref().unwrap(), "rdi");
                self.load(code.src2.as_ref().unwrap(), "rsi");
                assemble!(self.text, "call range");
                assemble!(self.text, "mov [rbp - {}], rax", self.get_offset(dst));
                self.regs.clear();
                self.regs.insert("rax".to_string(), Some(dst.clone()));
            }
            Op::Arg(n) => {
                let op = code.src1.as_ref().unwrap();
                if n < 6 {
                    let reg = self.arg_reg[n].clone();
                    self.load(op, &reg);
                } else {
                    self.load(op, "rax");
                    assemble!(self.text, "push rax");
                }
            }
            Op::Call => {
                let dst = code.dst.as_ref().unwrap();
                if let Operand::Function(name) = code.src1.as_ref().unwrap() {
                    assemble!(self.text, "call {}", name);
                    self.regs.clear();
                    self.regs.insert("rax".to_string(), Some(dst.clone()));
                    assemble!(self.text, "mov [rbp - {}], rax", self.get_offset(dst));
                }
            }
            Op::Label(lbl) => {
                assemble!(self.text, "{}:", lbl);
                self.regs.clear();
            }
            Op::Jump => {
                if let Operand::Label(lbl) = code.src1.as_ref().unwrap() {
                    assemble!(self.text, "jmp {}", lbl);
                }
            }
            Op::JumpIfFalse => {
                let src1 = code.src1.as_ref().unwrap();
                let lbl = match code.src2.as_ref().unwrap() {
                    Operand::Label(s) => s,
                    _ => panic!("TypeError"),
                };
                self.load(src1, "rax");
                assemble!(self.text, "cmp rax, 0");
                assemble!(self.text, "je {}", lbl);
            }
            Op::ArrayAccess => {
                let dst = code.dst.as_ref().unwrap();
                self.load(code.src1.as_ref().unwrap(), "r10");
                self.load(code.src2.as_ref().unwrap(), "rcx");
                assemble!(self.text, "lea rax, [r10 + rcx * 8 + 8]");
                assemble!(self.text, "mov rax, [rax]");
                assemble!(self.text, "mov [rbp - {}], rax", self.get_offset(dst));
                self.regs.clear();
                self.regs.insert("rax".to_string(), Some(dst.clone()));
            }
            Op::ArrayAssign => {
                self.load(code.dst.as_ref().unwrap(), "r10");
                self.load(code.src1.as_ref().unwrap(), "rcx");
                self.load(code.src2.as_ref().unwrap(), "rax");
                assemble!(self.text, "lea rdx, [r10 + rcx * 8 + 8]");
                assemble!(self.text, "mov [rdx], rax");
            }
            Op::Return => {
                if let Some(ref val) = code.src1 {
                    self.load(val, "rax");
                }
                assemble!(self.text, "jmp {}", self.ret_label);
            }
            _ => panic!("Unknown TAC: {:?}", code.op),
        }
    }

    fn compile_fn(&mut self, func: IRFunction) {
        if func.is_external {
            assemble!(self.text, "extern {}", func.name);
            return;
        }

        self.vars.clear();
        self.regs.clear();
        let mut offset = 0;

        for (param, _) in &func.params {
            if let Operand::Var(name) = param {
                if !self.vars.contains_key(name) {
                    offset += 8;
                    self.vars.insert(name.clone(), offset);
                }
            }
        }

        for inst in &func.instructions {
            let mut register_op = |op_opt: &Option<Operand>| {
                if let Some(op) = op_opt {
                    match op {
                        Operand::Var(name) => {
                            if !self.vars.contains_key(name) {
                                offset += 8;
                                self.vars.insert(name.clone(), offset);
                            }
                        }
                        Operand::Temp(id, _) => {
                            let temp_key = format!("_tmp_{}", id);
                            if !self.vars.contains_key(&temp_key) {
                                offset += 8;
                                self.vars.insert(temp_key, offset);
                            }
                        }
                        _ => {}
                    }
                }
            };
            register_op(&inst.dst);
            register_op(&inst.src1);
            register_op(&inst.src2);
        }

        let stack_size = (offset + 15) & !15;

        if func.is_pub {
            assemble!(self.text, "global {}", func.name);
        }

        assemble!(self.text, "{}:", func.name);
        assemble!(self.text, "push rbp");
        assemble!(self.text, "mov rbp, rsp");
        if stack_size > 0 {
            assemble!(self.text, "sub rsp, {}", stack_size);
        }

        let loop_label = format!(".L_{}_loop", func.name);
        assemble!(self.text, "{}:", loop_label);

        self.curr_fn = func.name.clone();
        self.ret_label = format!(".L_{}_exit", func.name);

        for (i, (param, _)) in func.params.iter().enumerate() {
            if i < 6 {
                let off = self.get_offset(param);
                assemble!(self.text, "mov [rbp - {}], {}", off, self.arg_reg[i]);
            }
        }

        let insts = &func.instructions;
        let mut i = 0;
        while i < insts.len() {
            let code = &insts[i];

            if code.op == Op::Call {
                if let Some(Operand::Function(ref name)) = code.src1 {
                    let is_tail = name == &self.curr_fn
                        && (i + 1 < insts.len() && insts[i + 1].op == Op::Return);

                    if is_tail {
                        self.regs.clear();
                        assemble!(self.text, "jmp {}", loop_label);
                        i += 2;
                        continue;
                    }
                }
            }

            match code.op {
                Op::Return => {
                    if let Some(ref val) = code.src1 {
                        self.load(val, "rax");
                    }
                    assemble!(self.text, "jmp {}", self.ret_label);
                }
                Op::Label(ref name) => {
                    assemble!(self.text, "{}:", name);
                    self.regs.clear();
                }
                _ => {
                    self.compile_code(code.clone());
                }
            }
            i += 1;
        }

        assemble!(self.text, "{}:", self.ret_label);
        assemble!(self.text, "leave");
        assemble!(self.text, "ret");
    }
    fn load(&mut self, op: &Operand, reg: &str) {
        if self.regs.get(reg).and_then(|i| i.as_ref()) == Some(op) {
            return;
        }

        let mut found_reg = None;
        for (r_name, r_op) in &self.regs {
            if r_op.as_ref() == Some(op) {
                found_reg = Some(r_name.clone());
                break;
            }
        }

        if let Some(src_reg) = found_reg {
            if src_reg != reg {
                assemble!(self.text, "mov {}, {}", reg, src_reg);
            }
        } else {
            match op {
                Operand::ConstIdx(idx) => match &self.program.constants[*idx] {
                    IRConst::Number(n) => assemble!(self.text, "mov {}, {}", reg, n),
                    IRConst::Bool(b) => {
                        assemble!(self.text, "mov {}, {}", reg, if *b { 1 } else { 0 })
                    }
                    IRConst::Void => assemble!(self.text, "mov {}, 0", reg),
                    IRConst::Str(s) => {
                        let lbl = self.alloc_str(s.clone());
                        assemble!(self.text, "mov {}, {}", reg, lbl);
                    }
                    IRConst::Array(_, arr) => self.alloc_arr(arr.len(), arr.clone(), reg),
                },
                Operand::Var(_) | Operand::Temp(_, _) => {
                    assemble!(self.text, "mov {}, [rbp - {}]", reg, self.get_offset(op));
                }
                _ => unreachable!(),
            }
        }
        self.regs.insert(reg.to_string(), Some(op.clone()));
    }

    fn alloc_str(&mut self, s: String) -> String {
        let lbl = format!(".S.{}", self.str_cnt);
        self.str_cnt += 1;
        assemble!(self.data, "{} db {}, 0", lbl, s);
        lbl
    }

    fn get_offset(&self, op: &Operand) -> usize {
        match op {
            Operand::Var(name) => *self.vars.get(name).unwrap(),
            Operand::Temp(id, _) => *self.vars.get(&format!("_tmp_{}", id)).unwrap(),
            _ => panic!("Not a stack operand"),
        }
    }

    fn alloc_arr(&mut self, len: usize, arr: Vec<Operand>, reg: &str) {
        let size = (len * 8 + 8 + 15) & !15;
        assemble!(self.text, "sub rsp, {}", size);
        assemble!(self.text, "mov r10, rsp");
        assemble!(self.text, "mov rax, {}", len);
        assemble!(self.text, "mov [r10], rax");
        for (i, op) in arr.iter().enumerate() {
            self.load(op, "rax");
            assemble!(self.text, "mov [r10 + {}], rax", 8 + i * 8);
        }
        assemble!(self.text, "mov {}, r10", reg);
        self.regs.clear();
    }
}
