use std::collections::HashMap;

use crate::{
    ast::{Expr, Program},
    token::{Literal, TokenType, VarType},
};

macro_rules! assemble {
    ($buf:expr, $fmt:literal $(, $arg:expr)* $(,)?) => {
        $buf.push_str(&format!(concat!("\n", $fmt) $(, $arg)*))
    };
}

struct Scope {
    vars: HashMap<String, u32>,
    next_slot: u32,
    saved_base: u32,
}

pub struct Compiler {
    text: String,
    data: String,
    scope_stack: Vec<Scope>,
    base_offset: u32,
    in_function: bool,
    str_cache: HashMap<String, String>,
    arr_lens: HashMap<String, usize>,
    strs: usize,
}

impl Compiler {
    pub fn new() -> Self {
        Self {
            text: String::new(),
            data: String::new(),
            scope_stack: Vec::new(),
            base_offset: 1,
            in_function: false,
            str_cache: HashMap::new(),
            arr_lens: HashMap::new(),
            strs: 0,
        }
    }

    fn enter_scope(&mut self, is_function: bool) {
        let saved_base = self.base_offset;
        self.scope_stack.push(Scope {
            vars: HashMap::new(),
            next_slot: 0,
            saved_base,
        });

        if is_function {
            self.in_function = true;
            self.base_offset = 1;
        }
    }

    fn exit_scope(&mut self) {
        if let Some(top) = self.scope_stack.pop() {
            self.base_offset = top.saved_base;

            if self.in_function {
                self.in_function = false;
            }
        }
    }

    fn store_var(&mut self, name: String) -> u32 {
        let outer_slots: u32 = self
            .scope_stack
            .iter()
            .take(self.scope_stack.len().saturating_sub(1))
            .map(|s| s.next_slot)
            .sum();

        let new_slot = self.scope_stack.last().unwrap().next_slot;
        let byte_offset = (self.base_offset + outer_slots + new_slot) * 8;
        let scope = self.scope_stack.last_mut().unwrap();

        scope.vars.insert(name.clone(), new_slot);
        scope.next_slot += 1;

        byte_offset
    }

    fn find_var(&self, name: &str) -> Option<u32> {
        if let Some(i) = self
            .scope_stack
            .iter()
            .rev()
            .position(|s| s.vars.contains_key(name))
        {
            let scope_index = self.scope_stack.len() - 1 - i;

            let outer_slots: u32 = self
                .scope_stack
                .iter()
                .take(scope_index)
                .map(|s| s.next_slot)
                .sum();

            let slot = self.scope_stack[scope_index].vars.get(name).unwrap();
            let byte_offset = (self.base_offset + outer_slots + *slot) * 8;
            return Some(byte_offset);
        }

        None
    }

    pub fn compile(&mut self, program: Program) -> String {
        self.enter_scope(true);
        assemble!(self.text, "section .text");

        for expr in program.body.iter() {
            self.compile_expr(expr.clone());
        }

        let mut result = String::new();
        if !self.data.is_empty() {
            result.push_str(&self.data);
        }
        result.push_str(&self.text);
        self.optim(result.trim().to_string())
    }

    fn optim(&mut self, src: String) -> String {
        let lines: Vec<String> = src.lines().map(|s| s.to_string()).collect();
        let mut result = Vec::new();
        let mut i = 0;

        while i < lines.len() {
            let current = lines[i].trim();

            if let Some(push_reg) = current.strip_prefix("push ") {
                if i + 1 < lines.len() {
                    let next = lines[i + 1].trim();

                    if let Some(pop_reg) = next.strip_prefix("pop ") {
                        let push_reg = push_reg.trim();
                        let pop_reg = pop_reg.trim();

                        if push_reg == pop_reg {
                            i += 2;
                            continue;
                        } else {
                            result.push(format!("mov {}, {}", pop_reg, push_reg));
                            i += 2;
                            continue;
                        }
                    }
                }
            }

            result.push(lines[i].clone());
            i += 1;
        }

        let opt = result.join("\n");

        if opt == src { opt } else { self.optim(opt) }
    }
    fn compile_expr(&mut self, expr: Expr) -> () {
        match expr {
            Expr::Val(val) => match val.value {
                Literal::Number(n) => {
                    assemble!(self.text, "mov rax, {}", n);
                    assemble!(self.text, "push rax");
                }
                Literal::Str(s) => {
                    if self.data.is_empty() {
                        assemble!(self.data, "section .data");
                    }
                    if self.str_cache.contains_key(&s) {
                        assemble!(
                            self.text,
                            "lea rax, [rel {}]",
                            self.str_cache.get(&s).unwrap()
                        );
                        assemble!(self.text, "push rax");
                        return;
                    }
                    let label = format!("L.str_{}", self.strs);
                    self.strs += 1;
                    let raw = format!("{:?}", &s);
                    assemble!(self.data, "{} db `{}`, 0", label, &raw[1..&raw.len() - 1]);
                    assemble!(self.text, "lea rax, [rel {}]", label);
                    assemble!(self.text, "push rax");
                    self.str_cache.insert(s, label);
                }
                Literal::Bool(b) => {
                    let val = if b { 1 } else { 0 };
                    assemble!(self.text, "mov rax, {}", val);
                    assemble!(self.text, "push rax");
                }
                Literal::Array(len, arr) => self.alloc_arr(len, arr),
                Literal::Void => {
                    assemble!(self.text, "xor rax, rax");
                    assemble!(self.text, "push rax");
                }
            },
            Expr::Var(var) => {
                let offset = self
                    .find_var(&var.name)
                    .unwrap_or_else(|| panic!("Variable '{}' not found", var.name));
                assemble!(self.text, "mov rax, [rbp - {}]", offset);
                assemble!(self.text, "push rax");
            }
            Expr::BinOp(bin) => {
                self.compile_expr(*bin.left);
                self.compile_expr(*bin.right);
                assemble!(self.text, "pop rbx");
                assemble!(self.text, "pop rax");

                match bin.operator {
                    TokenType::ADD => assemble!(self.text, "add rax, rbx"),
                    TokenType::SUB => assemble!(self.text, "sub rax, rbx"),
                    TokenType::MUL => assemble!(self.text, "imul rax, rbx"),
                    TokenType::DIV => {
                        assemble!(self.text, "cqo");
                        assemble!(self.text, "idiv rbx");
                    }

                    TokenType::EQ | TokenType::COMPEQ => {
                        assemble!(self.text, "cmp rax, rbx");
                        assemble!(self.text, "sete al");
                        assemble!(self.text, "movzx rax, al");
                    }
                    TokenType::COMPNE => {
                        assemble!(self.text, "cmp rax, rbx");
                        assemble!(self.text, "setne al");
                        assemble!(self.text, "movzx rax, al");
                    }
                    TokenType::COMPGT => {
                        assemble!(self.text, "cmp rax, rbx");
                        assemble!(self.text, "setg al");
                        assemble!(self.text, "movzx rax, al");
                    }
                    TokenType::COMPGE => {
                        assemble!(self.text, "cmp rax, rbx");
                        assemble!(self.text, "setge al");
                        assemble!(self.text, "movzx rax, al");
                    }
                    TokenType::COMPLT => {
                        assemble!(self.text, "cmp rax, rbx");
                        assemble!(self.text, "setl al");
                        assemble!(self.text, "movzx rax, al");
                    }
                    TokenType::COMPLE => {
                        assemble!(self.text, "cmp rax, rbx");
                        assemble!(self.text, "setle al");
                        assemble!(self.text, "movzx rax, al");
                    }
                    TokenType::COMPAND => {
                        assemble!(self.text, "and rax, rbx");
                    }
                    TokenType::COMPOR => {
                        assemble!(self.text, "or rax, rbx");
                    }
                    TokenType::LOGAND => {
                        assemble!(self.text, "test rax, rax");
                        assemble!(self.text, "setnz al");
                        assemble!(self.text, "movzx rax, al");
                        assemble!(self.text, "test rbx, rbx");
                        assemble!(self.text, "setnz bl");
                        assemble!(self.text, "movzx rbx, bl");
                        assemble!(self.text, "and rax, rbx");
                    }
                    TokenType::LOGOR => {
                        assemble!(self.text, "test rax, rax");
                        assemble!(self.text, "setnz al");
                        assemble!(self.text, "movzx rax, al");
                        assemble!(self.text, "test rbx, rbx");
                        assemble!(self.text, "setnz bl");
                        assemble!(self.text, "movzx rbx, bl");
                        assemble!(self.text, "or rax, rbx");
                    }
                    TokenType::LOGXOR => {
                        assemble!(self.text, "test rax, rax");
                        assemble!(self.text, "setnz al");
                        assemble!(self.text, "movzx rax, al");
                        assemble!(self.text, "test rbx, rbx");
                        assemble!(self.text, "setnz bl");
                        assemble!(self.text, "movzx rbx, bl");
                        assemble!(self.text, "xor rax, rbx");
                    }
                    TokenType::LOGNOT => {
                        assemble!(self.text, "not rax")
                    }
                    _ => {}
                }
                assemble!(self.text, "push rax");
            }

            Expr::UnaryOp(unary) => {
                match *unary.argument.clone() {
                    Expr::Var(var) => {
                        let name = var.name;
                        if unary.operator == TokenType::INC {
                            let offset = self.find_var(&name).unwrap();
                            assemble!(self.text, "inc qword [rbp - {}]", offset);
                            return;
                        } else if unary.operator == TokenType::DEC {
                            let offset = self.find_var(&name).unwrap();
                            assemble!(self.text, "dec qword [rbp - {}]", offset);
                            return;
                        } else if unary.operator == TokenType::SIZEOF {
                            if let Some(&len) = self.arr_lens.get(&name) {
                                assemble!(self.text, "mov rax, {}", len);
                                assemble!(self.text, "push rax");
                                return;
                            } else {
                                panic!("Variable '{}' not found or is not an array", name);
                            }
                        }
                    }
                    _ => {}
                }

                self.compile_expr(*unary.argument.clone());
                assemble!(self.text, "pop rax");

                if unary.operator != TokenType::SIZEOF {
                    assemble!(self.text, "push rax");
                }
            }
            Expr::VarDecl(decl) => {
                if let VarType::Array(len) = decl.typ {
                    self.arr_lens.insert(decl.name.clone(), len);
                }

                self.compile_expr(*decl.value);

                let offset = self.store_var(decl.name);
                assemble!(self.text, "pop rax");
                assemble!(self.text, "mov [rbp - {}], rax", offset);
            }
            Expr::VarMod(m) => {
                self.compile_expr(*m.value);
                let offset = self
                    .find_var(&m.name)
                    .unwrap_or_else(|| panic!("Variable '{}' not found for modification", m.name));
                assemble!(self.text, "pop rax");
                assemble!(self.text, "mov [rbp - {}], rax", offset);
            }
            Expr::Stmt(stmt) => {
                self.enter_scope(false);
                let body_len = stmt.body.len();
                for (i, expr) in stmt.body.into_iter().enumerate() {
                    self.compile_expr(expr.clone());
                    let pushes_value = match expr {
                        Expr::Val(_)
                        | Expr::Var(_)
                        | Expr::BinOp(_)
                        | Expr::UnaryOp(_)
                        | Expr::ArrayAccess(_)
                        | Expr::FuncCall(_) => true,

                        Expr::VarDecl(_)
                        | Expr::VarMod(_)
                        | Expr::Stmt(_)
                        | Expr::FuncDecl(_)
                        | Expr::Return(_)
                        | Expr::While(_)
                        | Expr::If(_)
                        | Expr::Label(_)
                        | Expr::Goto(_)
                        | Expr::Extern(_)
                        | Expr::ArrayAssign(_) => false,
                    };

                    if pushes_value && i < body_len - 1 {
                        assemble!(self.text, "pop rax");
                    }
                }
                self.exit_scope();
            }
            Expr::FuncDecl(decl) => {
                if decl.is_pub {
                    assemble!(self.text, "global {}", decl.name);
                }
                assemble!(self.text, "{}:", decl.name);

                assemble!(self.text, "push rbp");
                assemble!(self.text, "mov rbp, rsp");

                self.enter_scope(true);

                let mut local_slots = decl.params.len() as u32;

                local_slots += 8;
                if local_slots > 0 {
                    let mut alloc_size = local_slots * 8 + 32;
                    if (alloc_size + 8) % 16 != 0 {
                        alloc_size += 8;
                    }
                    assemble!(self.text, "sub rsp, {}", alloc_size);
                }

                let regs = ["rdi", "rsi", "rdx", "rcx", "r8", "r9"];
                for (i, param) in decl.params.iter().enumerate() {
                    let offset = self.store_var(param.clone());
                    if i < 6 {
                        assemble!(self.text, "mov [rbp - {}], {}", offset, regs[i]);
                    } else {
                        let stack_offset = 16 + (i - 6) * 8;
                        assemble!(self.text, "mov rax, [rbp + {}]", stack_offset);
                        assemble!(self.text, "mov [rbp - {}], rax", offset);
                    }
                }

                self.compile_expr(*decl.body.clone());

                match *decl.body.clone() {
                    Expr::Stmt(stmt) => {
                        let last = stmt.body.last().unwrap();
                        if !matches!(last, &Expr::Return(_)) {
                            assemble!(self.text, "xor rax, rax");
                            assemble!(self.text, "leave");
                            assemble!(self.text, "ret");
                        }
                    }
                    _ => {
                        assemble!(self.text, "xor rax, rax");
                        assemble!(self.text, "leave");
                        assemble!(self.text, "ret");
                    }
                }

                self.exit_scope();
            }
            Expr::Return(ret) => {
                if let Some(val) = ret.value {
                    self.compile_expr(*val);
                    assemble!(self.text, "pop rax");
                } else {
                    assemble!(self.text, "xor rax, rax");
                }
                assemble!(self.text, "leave");
                assemble!(self.text, "ret");
            }
            Expr::FuncCall(call) => {
                let arg_cnt = call.args.len();

                for arg in call.args.iter().rev() {
                    self.compile_expr(arg.clone());
                }

                let regs = ["rdi", "rsi", "rdx", "rcx", "r8", "r9"];

                let stack_args_cnt = arg_cnt.saturating_sub(6);
                if stack_args_cnt > 0 {
                    assemble!(self.text, "sub rsp, {}", stack_args_cnt * 8);

                    for i in 0..stack_args_cnt {
                        assemble!(self.text, "pop qword [rsp + {}]", i * 8);
                    }
                }

                for i in 0..arg_cnt.min(6) {
                    assemble!(self.text, "pop {}", regs[i]);
                }
                assemble!(self.text, "xor al, al");
                assemble!(self.text, "call {}", call.name);

                if stack_args_cnt > 0 {
                    assemble!(self.text, "add rsp, {}", stack_args_cnt * 8);
                }

                assemble!(self.text, "push rax");
            }
            Expr::While(wh) => {
                let loop_id = self.text.len();
                let loop_start_label = format!("while_start_{:x}", loop_id);
                let loop_end_label = format!("while_end_{:x}", loop_id);

                assemble!(self.text, ".{}:", loop_start_label);

                self.compile_expr(*wh.condition.clone());

                assemble!(self.text, "pop rax");
                assemble!(self.text, "test rax, rax");
                assemble!(self.text, "jz .{}", loop_end_label);

                self.enter_scope(false);

                self.compile_expr(*wh.body.clone());

                self.exit_scope();
                assemble!(self.text, "jmp .{}", loop_start_label);
                assemble!(self.text, ".{}:", loop_end_label);
            }
            Expr::If(if_expr) => {
                let id = self.text.len();
                let else_label = format!("if_else_{:x}", id);
                let end_label = format!("if_end_{:x}", id);

                self.compile_expr(*if_expr.condition.clone());

                assemble!(self.text, "pop rax");
                assemble!(self.text, "test rax, rax");

                let has_else = if_expr.else_branch.is_some();

                if has_else {
                    assemble!(self.text, "jz .{}", else_label);
                    self.compile_expr(*if_expr.then.clone());
                    assemble!(self.text, "jmp .{}", end_label);
                    assemble!(self.text, ".{}:", else_label);
                    if let Some(else_expr) = if_expr.else_branch {
                        self.compile_expr(*else_expr);
                    }
                } else {
                    assemble!(self.text, "jz .{}", end_label);
                    self.compile_expr(*if_expr.then.clone());
                }

                assemble!(self.text, ".{}:", end_label);
            }
            Expr::Label(label) => {
                assemble!(self.text, "{}:", label.name);
            }
            Expr::Goto(goto) => {
                assemble!(self.text, "jmp {}", goto.label);
            }
            Expr::ArrayAccess(aa) => {
                self.compile_expr(*aa.offset);
                assemble!(self.text, "pop rbx");
                let var_offset = self
                    .find_var(&aa.array)
                    .unwrap_or_else(|| panic!("Array '{}' not found", aa.array));

                assemble!(self.text, "mov rax, [rbp - {}]", var_offset);
                assemble!(self.text, "mov rax, [rax + rbx * 8]");
                assemble!(self.text, "push rax");
            }
            Expr::ArrayAssign(aa) => {
                let var_offset = self
                    .find_var(&aa.array)
                    .unwrap_or_else(|| panic!("Array '{}' not found", aa.array));

                assemble!(self.text, "mov r10, [rbp - {}]", var_offset);
                self.compile_expr(*aa.value.clone());
                assemble!(self.text, "pop rcx");
                self.compile_expr(*aa.offset.clone());
                assemble!(self.text, "pop rbx");
                assemble!(self.text, "mov [r10 + rbx * 8], rcx");
            }
            Expr::Extern(ext) => {
                assemble!(self.text, "extern {}", ext.func);
            }
        }
    }

    fn alloc_arr(&mut self, len: usize, arr: Vec<Expr>) {
        let block_size = len * 8;
        let padding = (16 - (block_size % 16)) % 16;
        let padded_block_size = block_size + padding;

        assemble!(self.text, "sub rsp, {}", padded_block_size);
        assemble!(self.text, "mov r10, rsp");

        for (i, elem) in arr.iter().enumerate() {
            self.compile_expr(elem.clone());
            assemble!(self.text, "pop rbx");

            assemble!(self.text, "mov [r10 + {}], rbx", i * 8);
        }

        assemble!(self.text, "push r10");
    }
}
