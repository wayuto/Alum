use ordered_float::OrderedFloat;

use crate::gir::{IRConst, IRFunction, IRProgram, IRType, Instruction, Op, Operand};
use std::{collections::HashMap, mem::take};

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
    lbl_cnt: usize,
    stack_ptr: usize,
    arg_reg: Vec<String>,
    flt_arg_reg: Vec<String>,
    ret_label: String,
    regs: HashMap<String, Option<Operand>>,
    curr_fn: String,
    loop_label: String,
    curr_flt_reg: usize,
}

impl CodeGen {
    pub fn new(program: IRProgram) -> Self {
        Self {
            program,
            text: String::new(),
            data: String::new(),
            vars: HashMap::new(),
            lbl_cnt: 0,
            stack_ptr: 0,
            arg_reg: vec![
                "rdi".to_string(),
                "rsi".to_string(),
                "rdx".to_string(),
                "rcx".to_string(),
                "r8".to_string(),
                "r9".to_string(),
            ],
            flt_arg_reg: vec![
                "xmm0".to_string(),
                "xmm1".to_string(),
                "xmm2".to_string(),
                "xmm3".to_string(),
                "xmm4".to_string(),
                "xmm5".to_string(),
                "xmm6".to_string(),
                "xmm7".to_string(),
            ],
            ret_label: String::new(),
            regs: HashMap::new(),
            curr_fn: String::new(),
            loop_label: String::new(),
            curr_flt_reg: 0,
        }
    }

    pub fn compile(&mut self) -> String {
        assemble!(self.text, "section .text");
        assemble!(self.data, "section .data");
        assemble!(self.data, "align 16");
        assemble!(self.data, "neg_mask: dq 0x8000000000000000, 0");
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
            Op::FMove => {
                let src = code.src1.as_ref().unwrap();
                let dst = code.dst.as_ref().unwrap();
                self.load(src, "xmm0");
                assemble!(self.text, "movsd [rbp - {}], xmm0", self.get_offset(dst));
                self.regs.insert("xmm0".to_string(), Some(dst.clone()));
            }
            Op::Load | Op::Store => {
                let src = code.src1.as_ref().unwrap();
                let dst = code.dst.as_ref().unwrap();
                self.load(src, "rax");
                assemble!(self.text, "mov [rbp - {}], rax", self.get_offset(dst));
            }
            Op::FLoad | Op::FStore => {
                let src = code.src1.as_ref().unwrap();
                let dst = code.dst.as_ref().unwrap();
                self.load(src, "xmm0");
                assemble!(self.text, "movsd [rbp - {}], xmm0", self.get_offset(dst));
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
            Op::FAdd | Op::FSub | Op::FMul | Op::FDiv => {
                let dst = code.dst.as_ref().unwrap();
                let src1 = code.src1.as_ref().unwrap();
                let src2 = code.src2.as_ref().unwrap();

                self.load(src1, "xmm0");
                self.load(src2, "xmm1");
                match code.op {
                    Op::FAdd => assemble!(self.text, "addsd xmm0, xmm1"),
                    Op::FSub => assemble!(self.text, "subsd xmm0, xmm1"),
                    Op::FMul => assemble!(self.text, "mulsd xmm0, xmm1"),
                    Op::FDiv => assemble!(self.text, "divsd xmm0, xmm1"),
                    _ => unreachable!(),
                }
                assemble!(self.text, "movsd [rbp - {}], xmm0", self.get_offset(dst));
                self.regs.clear();
                self.regs.insert("xmm0".to_string(), Some(dst.clone()));
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
                    Op::And => "setne",
                    Op::Or => "setne",
                    _ => unreachable!(),
                };
                assemble!(self.text, "{} al", set_op);
                assemble!(self.text, "movzx eax, al");
                assemble!(self.text, "mov [rbp - {}], rax", self.get_offset(dst));
                self.regs.clear();
                self.regs.insert("rax".to_string(), Some(dst.clone()));
            }
            Op::FEq | Op::FNe | Op::FGt | Op::FGe | Op::FLt | Op::FLe => {
                let dst = code.dst.as_ref().unwrap();
                let src1 = code.src1.as_ref().unwrap();
                let src2 = code.src2.as_ref().unwrap();
                self.load(src1, "xmm0");
                self.load(src2, "xmm1");
                assemble!(self.text, "ucomisd xmm0, xmm1");
                let set_op = match code.op {
                    Op::FEq => "sete",
                    Op::FNe => "setne",
                    Op::FGt => "setg",
                    Op::FGe => "setge",
                    Op::FLt => "setl",
                    Op::FLe => "setle",
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
            Op::FNeg => {
                let dst = code.dst.as_ref().unwrap();
                let src1 = code.src1.as_ref().unwrap();
                self.load(src1, "xmm0");
                assemble!(self.text, "xorpd xmm0, oword [rel neg_mask]");
                assemble!(self.text, "movsd [rbp - {}], xmm0", self.get_offset(dst));
                self.regs.clear();
                self.regs.insert("xmm0".to_string(), Some(dst.clone()));
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
            Op::FArg(n) => {
                let op = code.src1.as_ref().unwrap();
                if n < 8 {
                    self.curr_flt_reg = n + 1;
                    let reg = self.flt_arg_reg[n].clone();
                    self.load(op, &reg);
                } else {
                    self.curr_flt_reg = 8;
                    self.load(op, "xmm0");
                    assemble!(self.text, "sub rsp, 8");
                    assemble!(self.text, "movsd [rsp], xmm0");
                }
            }
            Op::Call => {
                let dst = code.dst.as_ref().unwrap();
                if let Operand::Function(name) = code.src1.as_ref().unwrap() {
                    if self.curr_flt_reg > 0 {
                        assemble!(self.text, "mov al, {}", self.curr_flt_reg);
                    } else {
                        assemble!(self.text, "xor al, al");
                    }
                    self.curr_flt_reg = 0;

                    assemble!(self.text, "call {}", name);

                    self.regs.clear();

                    let is_float = match dst {
                        Operand::Temp(_, IRType::Float) => true,
                        _ => false,
                    };

                    if is_float {
                        assemble!(self.text, "movsd [rbp - {}], xmm0", self.get_offset(dst));
                        self.regs.insert("xmm0".to_string(), Some(dst.clone()));
                    } else {
                        assemble!(self.text, "mov [rbp - {}], rax", self.get_offset(dst));
                        self.regs.insert("rax".to_string(), Some(dst.clone()));
                    }
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
            Op::Return(reg) => {
                if let Some(ref val) = code.src1 {
                    self.load(val, reg.as_str());
                }
                assemble!(self.text, "jmp {}", self.ret_label);
            }
            _ => panic!("CodeGenError: unsupported operation {:?}", code.op),
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

        let mut int_idx = 0;
        let mut flt_idx = 0;
        for (param, ty) in &func.params {
            let off = self.get_offset(param);
            if *ty == IRType::Float {
                if flt_idx < 8 {
                    assemble!(self.text, "movsd [rbp - {}], xmm{}", off, flt_idx);
                    flt_idx += 1;
                }
            } else {
                if int_idx < 6 {
                    assemble!(self.text, "mov [rbp - {}], {}", off, self.arg_reg[int_idx]);
                    int_idx += 1;
                }
            }
        }

        let insts = &func.instructions;
        let mut i = 0;
        while i < insts.len() {
            let code = &insts[i];

            if code.op == Op::Call {
                if let Some(Operand::Function(ref name)) = code.src1 {
                    let is_tail = name == &self.curr_fn
                        && (i + 1 < insts.len() && matches!(insts[i + 1].op, Op::Return(_)));

                    if is_tail {
                        self.regs.clear();
                        assemble!(self.text, "jmp {}", loop_label);
                        i += 2;
                        continue;
                    }
                }
            }

            match code.op.clone() {
                Op::Return(reg_name) => {
                    if let Some(ref val) = code.src1 {
                        self.load(val, reg_name.as_str());
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

        let is_xmm = reg.starts_with("xmm");
        let mov_inst = if is_xmm { "movsd" } else { "mov" };

        let mut found_reg = None;
        for (r_name, r_op) in &self.regs {
            if r_op.as_ref() == Some(op) {
                found_reg = Some(r_name.clone());
                break;
            }
        }

        if let Some(src_reg) = found_reg {
            if src_reg != reg {
                let inst = if is_xmm && src_reg.starts_with("xmm") {
                    "movsd"
                } else {
                    "mov"
                };
                assemble!(self.text, "{} {}, {}", inst, reg, src_reg);
            }
        } else {
            match op {
                Operand::ConstIdx(idx) => match &self.program.constants[*idx] {
                    IRConst::Int(n) => assemble!(self.text, "mov {}, {}", reg, n),
                    IRConst::Bool(b) => {
                        assemble!(self.text, "mov {}, {}", reg, if *b { 1 } else { 0 })
                    }
                    IRConst::Void => assemble!(self.text, "mov {}, 0", reg),
                    IRConst::Str(s) => {
                        let lbl = self.alloc_str(s.to_owned());
                        assemble!(self.text, "mov {}, {}", reg, lbl);
                    }
                    IRConst::Array(_, arr) => self.alloc_arr(arr.len(), arr.clone(), reg),
                    IRConst::Float(f) => {
                        let lbl = self.alloc_flt(*f);
                        assemble!(self.text, "movsd {}, [rel {}]", reg, lbl);
                    }
                },
                Operand::Var(_) | Operand::Temp(_, _) => {
                    if is_xmm {
                        assemble!(
                            self.text,
                            "movsd {}, qword [rbp - {}]",
                            reg,
                            self.get_offset(op)
                        );
                    } else {
                        assemble!(self.text, "mov {}, [rbp - {}]", reg, self.get_offset(op));
                    }
                }
                _ => unreachable!(),
            }
        }
        self.regs.insert(reg.to_string(), Some(op.clone()));
    }

    fn alloc_str(&mut self, s: String) -> String {
        let lbl = format!("L.S.{}", self.lbl_cnt);
        self.lbl_cnt += 1;
        assemble!(self.data, "{} db {}, 0", lbl, s);
        lbl
    }

    fn alloc_flt(&mut self, f: OrderedFloat<f64>) -> String {
        let lbl = format!("L.F.{}", self.lbl_cnt);
        self.lbl_cnt += 1;
        assemble!(self.data, "{} dq 0x{:x}", lbl, f.0.to_bits());
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
