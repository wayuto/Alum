use crate::compiler::{
    Span,
    bytecode::{Op, Value},
    parser::{Expr, Primitive, Program, Type},
};
use std::collections::HashMap;

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

#[derive(Default)]
struct LoopTargets {
    break_jumps: Vec<u32>,
    continue_jumps: Vec<u32>,
    break_slot: Option<u32>,
}

fn has_break_value(expr: &Expr) -> bool {
    match expr {
        Expr::Break { value: Some(_), .. } => true,
        Expr::Break { value: None, .. }
        | Expr::Continue(_)
        | Expr::While { .. }
        | Expr::For { .. }
        | Expr::Lambda { .. }
        | Expr::FuncDecl { .. } => false,
        Expr::Block { stmts: body, .. } => body.iter().any(has_break_value),
        Expr::If {
            cond: c,
            then_branch: t,
            else_branch: e,
            ..
        } => {
            has_break_value(c)
                || has_break_value(t)
                || e.as_ref().map(|e| has_break_value(e)).unwrap_or(false)
        }
        Expr::Match {
            target: t,
            branches: arms,
            default: d,
            ..
        } => {
            has_break_value(t)
                || arms.iter().any(|(p, g, b)| {
                    has_break_value(p)
                        || g.as_ref().map(|e| has_break_value(e)).unwrap_or(false)
                        || has_break_value(b)
                })
                || d.as_ref().map(|e| has_break_value(e)).unwrap_or(false)
        }
        Expr::Add {
            left: l, right: r, ..
        }
        | Expr::Sub {
            left: l, right: r, ..
        }
        | Expr::Mul {
            left: l, right: r, ..
        }
        | Expr::Div {
            left: l, right: r, ..
        }
        | Expr::Mod {
            left: l, right: r, ..
        }
        | Expr::FAdd {
            left: l, right: r, ..
        }
        | Expr::FSub {
            left: l, right: r, ..
        }
        | Expr::FMul {
            left: l, right: r, ..
        }
        | Expr::FDiv {
            left: l, right: r, ..
        }
        | Expr::Eq {
            left: l, right: r, ..
        }
        | Expr::Ne {
            left: l, right: r, ..
        }
        | Expr::Lt {
            left: l, right: r, ..
        }
        | Expr::Le {
            left: l, right: r, ..
        }
        | Expr::Gt {
            left: l, right: r, ..
        }
        | Expr::Ge {
            left: l, right: r, ..
        }
        | Expr::FEq {
            left: l, right: r, ..
        }
        | Expr::FNe {
            left: l, right: r, ..
        }
        | Expr::FLt {
            left: l, right: r, ..
        }
        | Expr::FLe {
            left: l, right: r, ..
        }
        | Expr::FGt {
            left: l, right: r, ..
        }
        | Expr::FGe {
            left: l, right: r, ..
        }
        | Expr::Xor {
            left: l, right: r, ..
        }
        | Expr::BAnd {
            left: l, right: r, ..
        }
        | Expr::BOr {
            left: l, right: r, ..
        }
        | Expr::LAnd {
            left: l, right: r, ..
        }
        | Expr::LOr {
            left: l, right: r, ..
        }
        | Expr::Shl {
            left: l, right: r, ..
        }
        | Expr::Shr {
            left: l, right: r, ..
        }
        | Expr::StrCat {
            left: l, right: r, ..
        }
        | Expr::Range {
            start: l, end: r, ..
        }
        | Expr::Index {
            array: l, index: r, ..
        }
        | Expr::IndexAssign {
            target: l,
            value: r,
            ..
        }
        | Expr::DerefAssign {
            ptr: l, value: r, ..
        } => has_break_value(l) || has_break_value(r),
        Expr::Not { expr: e, .. }
        | Expr::Neg { expr: e, .. }
        | Expr::FNeg { expr: e, .. }
        | Expr::BNot { expr: e, .. }
        | Expr::Return { value: e, .. }
        | Expr::VarDecl {
            name: _,
            ty: _,
            value: e,
            ..
        }
        | Expr::ConstDecl {
            name: _,
            ty: _,
            value: e,
            ..
        }
        | Expr::VarAssign {
            name: _, value: e, ..
        }
        | Expr::Cast { expr: e, .. }
        | Expr::Deref { expr: e, .. }
        | Expr::AddressOf { expr: e, .. } => has_break_value(e),
        Expr::AddAssign {
            name: _, value: e, ..
        }
        | Expr::SubAssign {
            name: _, value: e, ..
        }
        | Expr::MulAssign {
            name: _, value: e, ..
        }
        | Expr::DivAssign {
            name: _, value: e, ..
        }
        | Expr::ModAssign {
            name: _, value: e, ..
        }
        | Expr::AndAssign {
            name: _, value: e, ..
        }
        | Expr::OrAssign {
            name: _, value: e, ..
        }
        | Expr::XorAssign {
            name: _, value: e, ..
        }
        | Expr::ShlAssign {
            name: _, value: e, ..
        }
        | Expr::ShrAssign {
            name: _, value: e, ..
        } => has_break_value(e),
        Expr::Inc { .. }
        | Expr::Dec { .. }
        | Expr::Char { .. }
        | Expr::Int { .. }
        | Expr::Float { .. }
        | Expr::Bool { .. }
        | Expr::String { .. }
        | Expr::Nil(_)
        | Expr::Var { .. }
        | Expr::TypeDef(_)
        | Expr::Struct { .. }
        | Expr::Union { .. }
        | Expr::Enum { .. }
        | Expr::ExternVar { .. }
        | Expr::GlobalVar { .. }
        | Expr::FString { .. } => false,
        Expr::Call {
            callee: f,
            type_args: _,
            args,
            ..
        } => has_break_value(f) || args.iter().any(has_break_value),
        Expr::ArrayLiteral { elements: es, .. } => es.iter().any(has_break_value),
        Expr::ArrayFill {
            elem_type: _, len, ..
        } => has_break_value(len),
        Expr::MemberAccess { obj: o, .. } => has_break_value(o),
        Expr::MemberAssign {
            obj: o,
            field: _,
            value: v,
            ..
        } => has_break_value(o) || has_break_value(v),
        Expr::StructLiteral {
            name: _,
            type_args: _,
            fields: fs,
            ..
        }
        | Expr::UnionLiteral {
            name: _,
            type_args: _,
            fields: fs,
            ..
        } => fs.iter().any(|(_, v)| has_break_value(v)),
    }
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
    native_idx: HashMap<String, u32>,
    loop_targets: Vec<LoopTargets>,
    structs: HashMap<String, Vec<(String, Type)>>,
    unions: HashMap<String, Vec<(String, Type)>>,
    var_types: Vec<HashMap<String, String>>,

    global_consts: HashMap<String, Value>,
}
impl Compiler {
    fn resolve_member_index(&self, obj: &Expr, field_name: &str) -> (String, usize) {
        let type_name = self
            .resolve_struct_type(obj)
            .unwrap_or_else(|| panic!("unsupported member access in CTE"));
        let idx = self.field_index(&type_name, field_name).unwrap_or_else(|| {
            panic!("Compiler: type {type_name} has no field {field_name} (CTE)")
        });
        (type_name, idx)
    }

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
            structs: HashMap::new(),
            unions: HashMap::new(),
            var_types: Vec::new(),
            global_consts: HashMap::new(),
        }
    }

    pub fn with_global_consts(map: HashMap<String, Value>) -> Self {
        let mut c = Self::new();
        c.global_consts = map;
        c
    }
    fn patch_loop(&mut self, targets: LoopTargets, break_pos: u32, continue_pos: u32) {
        for j in targets.break_jumps {
            self.patch_jump_addr(j + 1, break_pos);
        }
        for j in targets.continue_jumps {
            self.patch_jump_addr(j + 1, continue_pos);
        }
    }

    fn emit(&mut self, op: Op, args: &[u32]) -> () {
        self.code.push(op as u8);
        for arg in args {
            self.code.extend_from_slice(&arg.to_le_bytes());
        }
    }

    fn enter_scope(&mut self) {
        self.scopes.push(Scope {
            vars: HashMap::new(),
            slot_count: 0,
        });
        self.funcs.push(HashMap::new());
        self.var_types.push(HashMap::new());
    }

    fn exit_scope(&mut self) {
        if let Some(scope) = self.scopes.pop() {
            self.next_slot -= scope.slot_count;
        }
        self.funcs.pop();
        self.var_types.pop();
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

    fn lookup_struct_fields(&self, name: &str) -> Option<&Vec<(String, Type)>> {
        self.structs.get(name).or_else(|| self.unions.get(name))
    }

    fn field_index(&self, type_name: &str, field_name: &str) -> Option<usize> {
        if let Some(fields) = self.unions.get(type_name) {
            return fields.iter().position(|(n, _)| n == field_name).map(|_| 0);
        }
        self.structs
            .get(type_name)?
            .iter()
            .position(|(n, _)| n == field_name)
    }

    fn track_var_type(&mut self, var_name: &str, type_name: &str) {
        if let Some(scope) = self.var_types.last_mut() {
            scope.insert(var_name.to_string(), type_name.to_string());
        }
    }

    fn lookup_var_type(&self, var_name: &str) -> Option<&str> {
        self.var_types
            .iter()
            .rev()
            .find_map(|scope| scope.get(var_name).map(|s| s.as_str()))
    }

    fn resolve_struct_type(&self, obj: &Expr) -> Option<String> {
        match obj {
            Expr::StructLiteral { name, .. } => Some(name.clone()),
            Expr::UnionLiteral { name, .. } => Some(name.clone()),
            Expr::Var { name, .. } => self.lookup_var_type(name).map(|s| s.to_string()),
            _ => None,
        }
    }

    pub fn compile(&mut self, program: Program) -> Bytecode {
        for expr in &program.body {
            match expr {
                Expr::FuncDecl { name, attrs, .. } if attrs.is_external => {
                    let idx = self.native_names.len();
                    self.native_names.push(name.clone());
                    self.native_idx.insert(name.clone(), idx as u32);
                }
                Expr::Struct {
                    name,
                    type_params: _,
                    fields,
                    ..
                } => {
                    self.structs.insert(name.clone(), fields.clone());
                }
                Expr::Union {
                    name,
                    type_params: _,
                    fields,
                    ..
                } => {
                    self.unions.insert(name.clone(), fields.clone());
                }
                _ => {}
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
            Expr::Int { value: n, .. } => {
                let idx = self.add_const(Value::Int(*n as i64));
                self.emit(Op::LOADCONST, &[idx]);
            }
            Expr::Char { value: c, .. } => {
                let idx = self.add_const(Value::Int(*c as i64));
                self.emit(Op::LOADCONST, &[idx]);
            }
            Expr::Float { value: f, .. } => {
                let idx = self.add_const(Value::Float(*f));
                self.emit(Op::LOADCONST, &[idx]);
            }
            Expr::Bool { value: b, .. } => {
                let idx = self.add_const(Value::Bool(*b));
                self.emit(Op::LOADCONST, &[idx]);
            }
            Expr::String { value: s, .. } => {
                let idx = self.add_const(Value::Str(s.clone()));
                self.emit(Op::LOADCONST, &[idx]);
            }
            Expr::Nil(_) => {
                let idx = self.add_const(Value::Void);
                self.emit(Op::LOADCONST, &[idx]);
            }
            Expr::Var { name, .. } => {
                let slot = self.load_var(name);
                match slot {
                    Some(s) => self.emit(Op::LOADVAR, &[s]),
                    None => {
                        if let Some(value) = self.global_consts.get(name).cloned() {
                            let idx = self.add_const(value);
                            self.emit(Op::LOADCONST, &[idx]);
                        } else {
                            let func = self
                                .load_func(name)
                                .unwrap_or_else(|| panic!("Compiler: Variable {} not found", name));
                            self.emit(Op::MAKEFUNC, &[func.addr, func.param_count]);
                        }
                    }
                }
            }

            Expr::Add {
                left: l, right: r, ..
            }
            | Expr::Sub {
                left: l, right: r, ..
            }
            | Expr::Mul {
                left: l, right: r, ..
            }
            | Expr::Div {
                left: l, right: r, ..
            } => {
                self.compile_expr(l);
                self.compile_expr(r);
                let op = match expr {
                    Expr::Add { .. } => Op::ADD,
                    Expr::Sub { .. } => Op::SUB,
                    Expr::Mul { .. } => Op::MUL,
                    Expr::Div { .. } => Op::DIV,
                    _ => unreachable!(),
                };
                self.emit(op, &[]);
            }
            Expr::Mod {
                left: l, right: r, ..
            } => {
                self.compile_expr(l);
                self.compile_expr(r);
                self.emit(Op::MOD, &[]);
            }
            Expr::Shl {
                left: l, right: r, ..
            } => {
                self.compile_expr(l);
                self.compile_expr(r);
                self.emit(Op::SHL, &[]);
            }
            Expr::Shr {
                left: l, right: r, ..
            } => {
                self.compile_expr(l);
                self.compile_expr(r);
                self.emit(Op::SHR, &[]);
            }
            Expr::FAdd {
                left: l, right: r, ..
            }
            | Expr::FSub {
                left: l, right: r, ..
            }
            | Expr::FMul {
                left: l, right: r, ..
            }
            | Expr::FDiv {
                left: l, right: r, ..
            } => {
                self.compile_expr(l);
                self.compile_expr(r);
                let op = match expr {
                    Expr::FAdd { .. } => Op::FADD,
                    Expr::FSub { .. } => Op::FSUB,
                    Expr::FMul { .. } => Op::FMUL,
                    Expr::FDiv { .. } => Op::FDIV,
                    _ => unreachable!(),
                };
                self.emit(op, &[]);
            }
            Expr::Eq {
                left: l, right: r, ..
            }
            | Expr::Ne {
                left: l, right: r, ..
            }
            | Expr::Lt {
                left: l, right: r, ..
            }
            | Expr::Le {
                left: l, right: r, ..
            }
            | Expr::Gt {
                left: l, right: r, ..
            }
            | Expr::Ge {
                left: l, right: r, ..
            } => {
                self.compile_expr(l);
                self.compile_expr(r);
                let op = match expr {
                    Expr::Eq { .. } => Op::EQ,
                    Expr::Ne { .. } => Op::NE,
                    Expr::Lt { .. } => Op::LT,
                    Expr::Le { .. } => Op::LE,
                    Expr::Gt { .. } => Op::GT,
                    Expr::Ge { .. } => Op::GE,
                    _ => unreachable!(),
                };
                self.emit(op, &[]);
            }
            Expr::FEq {
                left: l, right: r, ..
            }
            | Expr::FNe {
                left: l, right: r, ..
            }
            | Expr::FLt {
                left: l, right: r, ..
            }
            | Expr::FLe {
                left: l, right: r, ..
            }
            | Expr::FGt {
                left: l, right: r, ..
            }
            | Expr::FGe {
                left: l, right: r, ..
            } => {
                self.compile_expr(l);
                self.compile_expr(r);
                let op = match expr {
                    Expr::FEq { .. } => Op::FEQ,
                    Expr::FNe { .. } => Op::FNE,
                    Expr::FLt { .. } => Op::FLT,
                    Expr::FLe { .. } => Op::FLE,
                    Expr::FGt { .. } => Op::FGT,
                    Expr::FGe { .. } => Op::FGE,
                    _ => unreachable!(),
                };
                self.emit(op, &[]);
            }
            Expr::Not { expr: e, .. } => {
                self.compile_expr(e);
                self.emit(Op::LOGNOT, &[]);
            }
            Expr::Neg { expr: e, .. } => {
                self.compile_expr(e);
                self.emit(Op::NEG, &[]);
            }
            Expr::FNeg { expr: e, .. } => {
                self.compile_expr(e);
                self.emit(Op::FNEG, &[]);
            }
            Expr::Cast {
                expr: inner,
                ty: target_ty,
                ..
            } => {
                self.compile_expr(inner);
                if matches!(target_ty, Type::Primitive(Primitive::Void)) {
                    self.emit(Op::POP, &[]);
                    let vidx = self.add_const(Value::Void);
                    self.emit(Op::LOADCONST, &[vidx]);
                    return;
                }
                match target_ty {
                    Type::Primitive(Primitive::Float) => self.emit(Op::I2F, &[]),
                    Type::Primitive(Primitive::Int) => self.emit(Op::F2I, &[]),
                    _ => {}
                }
            }
            Expr::Xor {
                left: l, right: r, ..
            } => {
                self.compile_expr(l);
                self.compile_expr(r);
                self.emit(Op::LOGXOR, &[]);
            }
            Expr::BAnd {
                left: l, right: r, ..
            } => {
                self.compile_expr(l);
                self.compile_expr(r);
                self.emit(Op::LOGAND, &[]);
            }
            Expr::BOr {
                left: l, right: r, ..
            } => {
                self.compile_expr(l);
                self.compile_expr(r);
                self.emit(Op::LOGOR, &[]);
            }
            Expr::LAnd {
                left: l, right: r, ..
            } => {
                self.compile_expr(l);
                self.compile_expr(r);
                self.emit(Op::LOGAND, &[]);
            }
            Expr::LOr {
                left: l, right: r, ..
            } => {
                self.compile_expr(l);
                self.compile_expr(r);
                self.emit(Op::LOGOR, &[]);
            }
            Expr::BNot { expr: e, .. } => {
                self.compile_expr(e);
                self.emit(Op::BNOT, &[]);
            }
            Expr::StrCat {
                left: l, right: r, ..
            } => {
                self.compile_expr(l);
                self.compile_expr(r);
                self.emit(Op::ADD, &[]);
            }
            Expr::Inc { name, .. } => self.compile_inc_dec(name, true),
            Expr::Dec { name, .. } => self.compile_inc_dec(name, false),

            Expr::ArrayLiteral { elements, .. } => {
                for e in elements {
                    self.compile_expr(e);
                }
                self.emit(Op::NEWARRAY, &[elements.len() as u32]);
            }
            Expr::ArrayFill {
                elem_type: ty, len, ..
            } => {
                self.compile_expr(len);
                let zero_idx = self.add_const(array_fill_zero(ty));
                self.emit(Op::LOADCONST, &[zero_idx]);
                self.emit(Op::ARRAYFILL, &[]);
            }
            Expr::Index {
                array: arr,
                index: idx,
                ..
            } => {
                self.compile_expr(arr);
                self.compile_expr(idx);
                self.emit(Op::ARRAYGET, &[]);
            }
            Expr::IndexAssign {
                target: arr_idx,
                value,
                ..
            } => {
                let (slot, idx) = match arr_idx.as_ref() {
                    Expr::Index {
                        array: arr,
                        index: idx,
                        ..
                    } => match arr.as_ref() {
                        Expr::Var { name, .. } => (self.mod_var(name), idx.clone()),
                        other => {
                            panic!("Compiler: unsupported indexed assignment target {other:?}")
                        }
                    },
                    other => panic!("Compiler: unsupported indexed assignment target {other:?}"),
                };
                self.compile_expr(&idx);
                self.compile_expr(value);
                self.emit(Op::ARRAYSET, &[slot]);

                self.emit(Op::LOADVAR, &[slot]);
                self.compile_expr(&idx);
                self.emit(Op::ARRAYGET, &[]);
            }

            Expr::VarDecl {
                name, ty: _, value, ..
            }
            | Expr::ConstDecl {
                name, ty: _, value, ..
            } => {
                if let Expr::StructLiteral { name: tname, .. }
                | Expr::UnionLiteral { name: tname, .. } = value.as_ref()
                {
                    self.track_var_type(name, tname);
                }
                self.compile_expr(value);
                let slot = self.decl_var(name);
                self.emit(Op::STOREVAR, &[slot]);
            }
            Expr::GlobalVar {
                name,
                is_pub: _,
                ty: _,
                value: init,
                ..
            } => {
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
            Expr::VarAssign { name, value, .. } => {
                self.compile_expr(value);
                let slot = self.mod_var(name);
                self.emit(Op::STOREVAR, &[slot]);
            }
            Expr::AddAssign { name, value, .. }
            | Expr::SubAssign { name, value, .. }
            | Expr::MulAssign { name, value, .. }
            | Expr::DivAssign { name, value, .. }
            | Expr::ModAssign { name, value, .. }
            | Expr::AndAssign { name, value, .. }
            | Expr::OrAssign { name, value, .. }
            | Expr::XorAssign { name, value, .. }
            | Expr::ShlAssign { name, value, .. }
            | Expr::ShrAssign { name, value, .. } => {
                let slot = self.mod_var(name);
                self.compile_expr(&Expr::Var {
                    name: name.clone(),
                    span: Span::new(0, 0),
                });
                self.compile_expr(value);
                let op = match expr {
                    Expr::AddAssign { .. } => Op::ADD,
                    Expr::SubAssign { .. } => Op::SUB,
                    Expr::MulAssign { .. } => Op::MUL,
                    Expr::DivAssign { .. } => Op::DIV,
                    Expr::ModAssign { .. } => Op::MOD,
                    Expr::AndAssign { .. } => Op::LOGAND,
                    Expr::OrAssign { .. } => Op::LOGOR,
                    Expr::XorAssign { .. } => Op::LOGXOR,
                    Expr::ShlAssign { .. } => Op::SHL,
                    Expr::ShrAssign { .. } => Op::SHR,
                    _ => unreachable!(),
                };
                self.emit(op, &[]);
                self.emit(Op::STOREVAR, &[slot]);
            }

            Expr::Return { value, .. } => {
                if let Some((target, param_count)) = self.tail_self_call(value) {
                    if let Expr::Call {
                        callee: _,
                        type_args: _,
                        args,
                        ..
                    } = value.as_ref()
                    {
                        for arg in args {
                            self.compile_expr(arg);
                        }
                    }
                    self.emit(Op::TAILCALL, &[target, param_count as u32]);
                } else {
                    self.compile_expr(value);
                    self.emit(Op::RET, &[]);
                }
            }
            Expr::Block { stmts: body, .. } => {
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
            Expr::If {
                cond,
                then_branch,
                else_branch,
                ..
            } => {
                self.compile_expr(cond);
                let then_addr = self.code.len() as u32;
                self.emit(Op::JUMPIFFALSE, &[0]);

                self.compile_expr(then_branch);

                let mut else_addr: u32 = 1;
                if else_branch.is_some() {
                    else_addr = self.code.len() as u32;
                    self.emit(Op::JUMP, &[0]);
                }
                let then_end = self.code.len() as u32;
                self.patch_jump_addr(then_addr + 1, then_end);

                if let Some(else_branch) = else_branch {
                    self.compile_expr(else_branch);
                    let else_end = self.code.len() as u32;
                    self.patch_jump_addr(else_addr + 1, else_end);
                }
            }
            Expr::While { cond, body, .. } => {
                self.enter_scope();
                let break_slot = has_break_value(&body).then(|| self.decl_var("_break_val"));
                if let Some(slot) = break_slot {
                    let void_const = self.add_const(Value::Void);
                    self.emit(Op::LOADCONST, &[void_const]);
                    self.emit(Op::STOREVAR, &[slot]);
                    self.emit(Op::POP, &[]);
                }
                let loop_pos = self.code.len() as u32;
                self.compile_expr(cond);
                let iff = self.code.len() as u32;
                self.emit(Op::JUMPIFFALSE, &[0]);
                self.loop_targets.push(LoopTargets {
                    break_slot,
                    ..LoopTargets::default()
                });
                self.compile_expr(body);
                self.emit(Op::POP, &[]);
                let targets = self.loop_targets.pop().unwrap();
                self.emit(Op::JUMP, &[loop_pos]);
                let break_pos = self.code.len() as u32;
                self.patch_loop(targets, break_pos, loop_pos);
                self.patch_jump_addr(iff + 1, break_pos);
                if let Some(slot) = break_slot {
                    self.emit(Op::LOADVAR, &[slot]);
                }
                self.exit_scope();
            }
            Expr::Break { value, .. } => {
                if let Some(slot) = self.loop_targets.last().and_then(|t| t.break_slot) {
                    if let Some(v) = value {
                        self.compile_expr(&v);
                        self.emit(Op::STOREVAR, &[slot]);
                        self.emit(Op::POP, &[]);
                    }
                }
                if let Some(targets) = self.loop_targets.last_mut() {
                    let j = self.code.len() as u32;
                    targets.break_jumps.push(j);
                    self.emit(Op::JUMP, &[0]);
                } else {
                    panic!("Compiler: break outside loop for bytecode");
                }
            }
            Expr::Continue(_) => {
                if let Some(targets) = self.loop_targets.last_mut() {
                    let j = self.code.len() as u32;
                    targets.continue_jumps.push(j);
                    self.emit(Op::JUMP, &[0]);
                } else {
                    panic!("Compiler: continue outside loop for bytecode");
                }
            }
            Expr::FuncDecl {
                name,
                attrs,
                type_params: _,
                params,
                return_type: _,
                body,
                ..
            } => {
                if attrs.is_external {
                    return;
                }
                self.compile_func(name, params.len(), params, body);
            }
            Expr::Call {
                callee: f,
                type_args: _,
                args,
                ..
            } => {
                if let Expr::Var { name: n, .. } = f.as_ref() {
                    if n == "_alum_copy" && args.len() == 1 {
                        self.compile_expr(&args[0]);
                        return;
                    }
                }
                let direct_func: Option<Func> = match f.as_ref() {
                    Expr::Var { name, .. } => {
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
                        self.emit(Op::CALL, &[func.addr, func.param_count]);
                    }
                    None => {
                        if let Expr::Var { name, .. } = f.as_ref() {
                            if let Some(&idx) = self.native_idx.get(name) {
                                for arg in args {
                                    self.compile_expr(arg);
                                }
                                self.emit(Op::CALLNATIVE, &[idx, args.len() as u32]);
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

            Expr::Lambda { params, body, .. } => {
                let name = format!("_lambda_{}", self.lambda_counter);
                self.lambda_counter += 1;

                let jump_addr = self.code.len() as u32;
                self.emit(Op::JUMP, &[0]);
                let func_addr = self.code.len() as u32;

                self.funcs.last_mut().unwrap().insert(
                    name.clone(),
                    Func {
                        addr: func_addr,
                        param_count: params.len() as u32,
                    },
                );

                let saved_slot = self.next_slot;
                self.next_slot = 0;
                self.enter_scope();
                for (param, _) in params {
                    self.decl_var(param);
                }
                self.current_func = Some((name, func_addr));
                self.compile_expr(body);
                self.current_func = None;

                match &**body {
                    Expr::Return { .. } => {}
                    _ => self.emit(Op::RET, &[]),
                }
                self.exit_scope();
                self.next_slot = saved_slot;

                self.patch_jump_addr(jump_addr + 1, self.code.len() as u32);
                self.emit(Op::MAKEFUNC, &[func_addr, params.len() as u32]);
            }

            Expr::Range {
                start,
                end,
                inclusive,
                ..
            } => {
                self.compile_expr(start);
                let start_slot = self.decl_var("_range_start");
                self.emit(Op::STOREVAR, &[start_slot]);
                self.emit(Op::POP, &[]);

                self.compile_expr(end);
                let end_slot = self.decl_var("_range_end");
                self.emit(Op::STOREVAR, &[end_slot]);
                self.emit(Op::POP, &[]);

                self.emit(Op::LOADVAR, &[end_slot]);
                self.emit(Op::LOADVAR, &[start_slot]);
                self.emit(Op::SUB, &[]);
                if *inclusive {
                    let one_const = self.add_const(Value::Int(1));
                    self.emit(Op::LOADCONST, &[one_const]);
                    self.emit(Op::ADD, &[]);
                }
                let zero_const = self.add_const(Value::Int(0));
                self.emit(Op::LOADCONST, &[zero_const]);
                self.emit(Op::ARRAYFILL, &[]);

                let arr_slot = self.decl_var("_range_arr");
                self.emit(Op::STOREVAR, &[arr_slot]);
                self.emit(Op::POP, &[]);

                let idx_slot = self.decl_var("_range_idx");
                self.emit(Op::LOADCONST, &[zero_const]);
                self.emit(Op::STOREVAR, &[idx_slot]);
                self.emit(Op::POP, &[]);

                let loop_pos = self.code.len() as u32;
                self.emit(Op::LOADVAR, &[idx_slot]);
                self.emit(Op::LOADVAR, &[end_slot]);
                self.emit(Op::LOADVAR, &[start_slot]);
                self.emit(Op::SUB, &[]);
                if *inclusive {
                    let one_const = self.add_const(Value::Int(1));
                    self.emit(Op::LOADCONST, &[one_const]);
                    self.emit(Op::ADD, &[]);
                }
                self.emit(Op::LT, &[]);
                let exit_loop = self.code.len() as u32;
                self.emit(Op::JUMPIFFALSE, &[0]);

                self.emit(Op::LOADVAR, &[idx_slot]);
                self.emit(Op::LOADVAR, &[start_slot]);
                self.emit(Op::LOADVAR, &[idx_slot]);
                self.emit(Op::ADD, &[]);
                self.emit(Op::ARRAYSET, &[arr_slot]);

                self.emit(Op::LOADVAR, &[idx_slot]);
                self.emit(Op::INC, &[]);
                self.emit(Op::STOREVAR, &[idx_slot]);
                self.emit(Op::POP, &[]);

                self.emit(Op::JUMP, &[loop_pos]);
                self.patch_jump_addr(exit_loop + 1, self.code.len() as u32);

                self.emit(Op::LOADVAR, &[arr_slot]);
            }
            Expr::For {
                var,
                iterable,
                body,
                ..
            } => {
                self.enter_scope();
                if let Expr::Range {
                    start,
                    end,
                    inclusive,
                    ..
                } = iterable.as_ref()
                {
                    self.compile_expr(end);
                    let end_slot = self.decl_var("_for_end");
                    self.emit(Op::STOREVAR, &[end_slot]);
                    self.emit(Op::POP, &[]);

                    self.compile_expr(start);
                    let var_slot = self.decl_var(var);
                    self.emit(Op::STOREVAR, &[var_slot]);
                    self.emit(Op::POP, &[]);

                    let break_slot = has_break_value(&body).then(|| self.decl_var("_break_val"));
                    if let Some(slot) = break_slot {
                        let void_const = self.add_const(Value::Void);
                        self.emit(Op::LOADCONST, &[void_const]);
                        self.emit(Op::STOREVAR, &[slot]);
                        self.emit(Op::POP, &[]);
                    }
                    let loop_pos = self.code.len() as u32;
                    self.loop_targets.push(LoopTargets {
                        break_slot,
                        ..LoopTargets::default()
                    });
                    self.emit(Op::LOADVAR, &[var_slot]);
                    self.emit(Op::LOADVAR, &[end_slot]);
                    self.emit(if *inclusive { Op::LE } else { Op::LT }, &[]);
                    let exit_loop = self.code.len() as u32;
                    self.emit(Op::JUMPIFFALSE, &[0]);

                    self.compile_expr(body);
                    self.emit(Op::POP, &[]);

                    let inc_pos = self.code.len() as u32;
                    self.emit(Op::LOADVAR, &[var_slot]);
                    self.emit(Op::INC, &[]);
                    self.emit(Op::STOREVAR, &[var_slot]);
                    self.emit(Op::POP, &[]);

                    self.emit(Op::JUMP, &[loop_pos]);
                    let break_pos = self.code.len() as u32;
                    let targets = self.loop_targets.pop().unwrap();
                    self.patch_loop(targets, break_pos, inc_pos);
                    self.patch_jump_addr(exit_loop + 1, break_pos);
                    if let Some(slot) = break_slot {
                        self.emit(Op::LOADVAR, &[slot]);
                    }
                } else {
                    self.compile_expr(iterable);
                    let arr_slot = self.decl_var("_for_arr");
                    self.emit(Op::STOREVAR, &[arr_slot]);
                    self.emit(Op::POP, &[]);

                    self.emit(Op::LOADVAR, &[arr_slot]);
                    self.emit(Op::ARRAYLEN, &[]);
                    let len_slot = self.decl_var("_for_len");
                    self.emit(Op::STOREVAR, &[len_slot]);
                    self.emit(Op::POP, &[]);

                    let zero_const = self.add_const(Value::Int(0));
                    self.emit(Op::LOADCONST, &[zero_const]);
                    let idx_slot = self.decl_var("_for_idx");
                    self.emit(Op::STOREVAR, &[idx_slot]);
                    self.emit(Op::POP, &[]);

                    let var_slot = self.decl_var(var);

                    let break_slot = has_break_value(&body).then(|| self.decl_var("_break_val"));
                    if let Some(slot) = break_slot {
                        let void_const = self.add_const(Value::Void);
                        self.emit(Op::LOADCONST, &[void_const]);
                        self.emit(Op::STOREVAR, &[slot]);
                        self.emit(Op::POP, &[]);
                    }
                    let loop_pos = self.code.len() as u32;
                    self.loop_targets.push(LoopTargets {
                        break_slot,
                        ..LoopTargets::default()
                    });

                    self.emit(Op::LOADVAR, &[idx_slot]);
                    self.emit(Op::LOADVAR, &[len_slot]);
                    self.emit(Op::LT, &[]);
                    let exit_loop = self.code.len() as u32;
                    self.emit(Op::JUMPIFFALSE, &[0]);

                    self.emit(Op::LOADVAR, &[arr_slot]);
                    self.emit(Op::LOADVAR, &[idx_slot]);
                    self.emit(Op::ARRAYGET, &[]);
                    self.emit(Op::STOREVAR, &[var_slot]);
                    self.emit(Op::POP, &[]);

                    self.compile_expr(body);
                    self.emit(Op::POP, &[]);

                    let inc_pos = self.code.len() as u32;
                    self.emit(Op::LOADVAR, &[idx_slot]);
                    self.emit(Op::INC, &[]);
                    self.emit(Op::STOREVAR, &[idx_slot]);
                    self.emit(Op::POP, &[]);

                    self.emit(Op::JUMP, &[loop_pos]);
                    let break_pos = self.code.len() as u32;
                    let targets = self.loop_targets.pop().unwrap();
                    self.patch_loop(targets, break_pos, inc_pos);
                    self.patch_jump_addr(exit_loop + 1, break_pos);
                    if let Some(slot) = break_slot {
                        self.emit(Op::LOADVAR, &[slot]);
                    }
                }
                self.exit_scope();
            }
            Expr::Struct { .. } | Expr::Union { .. } | Expr::Enum { .. } => {}
            Expr::StructLiteral {
                name,
                type_args: _,
                fields,
                ..
            } => {
                let field_order = self
                    .lookup_struct_fields(name)
                    .map(|f| f.clone())
                    .unwrap_or_default();
                for (fname, fty) in &field_order {
                    if let Some((_, fval)) = fields.iter().find(|(n, _)| n == fname) {
                        self.compile_expr(fval);
                    } else {
                        let zero = self.add_const(array_fill_zero(fty));
                        self.emit(Op::LOADCONST, &[zero]);
                    }
                }
                self.emit(Op::NEWARRAY, &[field_order.len() as u32]);
            }
            Expr::UnionLiteral {
                name,
                type_args: _,
                fields,
                ..
            } => {
                let field_order = self
                    .lookup_struct_fields(name)
                    .map(|f| f.clone())
                    .unwrap_or_default();

                let mut init: Option<&Expr> = None;
                for (fname, _) in &field_order {
                    if let Some((_, fval)) = fields.iter().find(|(n, _)| n == fname) {
                        init = Some(fval);
                    }
                }
                match init {
                    Some(fval) => self.compile_expr(fval),
                    None => {
                        let zero_ty = field_order
                            .first()
                            .map(|(_, t)| t)
                            .unwrap_or(&Type::Primitive(Primitive::Int));
                        let zero = self.add_const(array_fill_zero(zero_ty));
                        self.emit(Op::LOADCONST, &[zero]);
                    }
                }
                self.emit(Op::NEWARRAY, &[1]);
            }
            Expr::MemberAccess {
                obj,
                field: field_name,
                ..
            } => {
                let (_, idx) = self.resolve_member_index(obj, field_name);
                self.compile_expr(obj);
                let idx_const = self.add_const(Value::Int(idx as i64));
                self.emit(Op::LOADCONST, &[idx_const]);
                self.emit(Op::ARRAYGET, &[]);
            }
            Expr::MemberAssign {
                obj,
                field: field_name,
                value,
                ..
            } => {
                let (_, idx) = self.resolve_member_index(obj, field_name);
                let Expr::Var { name, .. } = obj.as_ref() else {
                    panic!("Compiler: unsupported member assignment target in CTE");
                };
                let slot = self.mod_var(name);
                let idx_const = self.add_const(Value::Int(idx as i64));
                self.emit(Op::LOADCONST, &[idx_const]);
                self.compile_expr(value);
                self.emit(Op::ARRAYSET, &[slot]);

                self.emit(Op::LOADVAR, &[slot]);
                self.emit(Op::LOADCONST, &[idx_const]);
                self.emit(Op::ARRAYGET, &[]);
            }
            Expr::Match {
                target,
                branches,
                default,
                ..
            } => {
                self.compile_expr(target);
                let target_slot = self.decl_var("_match_target");
                self.emit(Op::STOREVAR, &[target_slot]);
                self.emit(Op::POP, &[]);

                let mut end_jumps: Vec<u32> = Vec::new();
                for (case_expr, guard, result_expr) in branches {
                    self.emit(Op::LOADVAR, &[target_slot]);
                    if let Expr::Range {
                        start: lo,
                        end: hi,
                        inclusive,
                        ..
                    } = case_expr
                    {
                        self.compile_expr(&lo);
                        self.emit(Op::GE, &[]);
                        self.emit(Op::LOADVAR, &[target_slot]);
                        self.compile_expr(&hi);
                        self.emit(if *inclusive { Op::LE } else { Op::LT }, &[]);
                        self.emit(Op::LOGAND, &[]);
                    } else {
                        self.compile_expr(case_expr);
                        self.emit(Op::EQ, &[]);
                    }
                    let mut skip_addrs: Vec<u32> = Vec::new();
                    skip_addrs.push(self.code.len() as u32);
                    self.emit(Op::JUMPIFFALSE, &[0]);
                    if let Some(guard) = guard {
                        self.compile_expr(&guard);
                        skip_addrs.push(self.code.len() as u32);
                        self.emit(Op::JUMPIFFALSE, &[0]);
                    }

                    self.compile_expr(result_expr);

                    end_jumps.push(self.code.len() as u32);
                    self.emit(Op::JUMP, &[0]);

                    let next_addr = self.code.len() as u32;
                    for skip_addr in skip_addrs {
                        self.patch_jump_addr(skip_addr + 1, next_addr);
                    }
                }

                if let Some(default_expr) = default {
                    self.compile_expr(default_expr);
                } else {
                    let void_const = self.add_const(Value::Void);
                    self.emit(Op::LOADCONST, &[void_const]);
                }

                let end_addr = self.code.len() as u32;
                for j in end_jumps {
                    self.patch_jump_addr(j + 1, end_addr);
                }
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
    }

    fn compile_func(
        &mut self,
        name: &str,
        num_params: usize,
        params: &[(String, Type)],
        body: &Expr,
    ) {
        let jump_addr = self.code.len() as u32;
        self.emit(Op::JUMP, &[0]);
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

        let saved_slot = self.next_slot;
        self.next_slot = 0;
        self.enter_scope();
        for (param, ty) in params {
            self.decl_var(param);
            if let Type::Struct(sname, _) | Type::Union(sname, _) = ty {
                self.track_var_type(param, sname);
            }
        }
        self.compile_expr(body);

        match body {
            Expr::Return { .. } => {}
            _ => self.emit(Op::RET, &[]),
        }
        self.exit_scope();
        self.next_slot = saved_slot;
        self.current_func = None;

        self.patch_jump_addr(jump_addr + 1, self.code.len() as u32);
    }

    fn patch_jump_addr(&mut self, pos: u32, addr: u32) {
        self.code[pos as usize..pos as usize + 4].copy_from_slice(&addr.to_le_bytes());
    }

    fn tail_self_call(&self, value: &Expr) -> Option<(u32, usize)> {
        let Expr::Call {
            callee,
            type_args: _,
            args,
            ..
        } = value
        else {
            return None;
        };
        let Expr::Var { name, .. } = callee.as_ref() else {
            return None;
        };
        let Some((fname, faddr)) = &self.current_func else {
            return None;
        };

        if self.load_var(name).is_some() {
            return None;
        }
        if name != fname {
            return None;
        }
        let func = self.load_func(name)?;
        if func.param_count as usize != args.len() {
            panic!(
                "Compiler: Function call expects {} arguments, got {}",
                func.param_count,
                args.len()
            );
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
