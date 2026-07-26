use super::ir::{IRConst, IRFunction, IRProgram, IRType, Instruction, Op, Operand};
use super::types::CodeGenError;
use crate::compiler::parser::{Expr, Program, Type};
use ordered_float::OrderedFloat;
use std::{collections::HashMap, iter::zip, mem::take};

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
    pub loop_end_labels: Vec<String>,
    pub loop_inc_labels: Vec<String>,
    pub var_types: HashMap<String, Type>,
}

impl Context {
    pub fn new() -> Self {
        Self {
            instructions: Vec::new(),
            tmp_cnt: 0,
            scope: Vec::new(),
            label_cnt: 0,
            loop_end_labels: Vec::new(),
            loop_inc_labels: Vec::new(),
            var_types: HashMap::new(),
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

    pub fn exit_scope(&mut self) -> Result<(), CodeGenError> {
        self.scope.pop().ok_or_else(|| CodeGenError::ScopeError {
            message: "Tried to pop the root scope.".to_string(),
        })?;
        Ok(())
    }

    fn get_var_type(&self, name: &str) -> Result<IRType, CodeGenError> {
        for scope in self.scope.iter().rev() {
            if let Some(symbol) = scope.get(name) {
                return Ok(symbol.ir_type.clone());
            }
        }
        Err(CodeGenError::UndefinedVariable {
            name: name.to_string(),
            span: crate::compiler::Span::new(0, 0),
        })
    }

    fn type_to_ir_type(typ: &Type) -> IRType {
        match typ {
            Type::Named(name) => match name.as_str() {
                "int" | "i64" | "isize" | "usize" => IRType::Int,
                "float" | "f64" => IRType::Float,
                "bool" => IRType::Bool,
                "string" => IRType::String,
                "void" => IRType::Void,
                _ => IRType::Int,
            },
            Type::Array(_, len) => IRType::Array(Some(*len)),
            Type::Pointer(_) => IRType::Int,
            _ => IRType::Int,
        }
    }

    pub fn get_operand_type(
        &self,
        operand: &Operand,
        constants: &[IRConst],
    ) -> Result<IRType, CodeGenError> {
        match operand {
            Operand::ConstIdx(c) => match &constants[*c] {
                IRConst::Int(_) => Ok(IRType::Int),
                IRConst::Float(_) => Ok(IRType::Float),
                IRConst::Str(_) => Ok(IRType::String),
                IRConst::Array(len, _) => Ok(IRType::Array(Some(len.to_owned()))),
            },
            Operand::Var(name) => self.get_var_type(name),
            Operand::Temp(_, t) => Ok(t.to_owned()),
            Operand::Label(_) => Ok(IRType::Void),
            Operand::Function(_) => Ok(IRType::Void),
        }
    }

    pub fn declare_var(&mut self, name: String, ir_type: IRType) -> Result<(), CodeGenError> {
        let current_scope = self
            .scope
            .last_mut()
            .ok_or_else(|| CodeGenError::ScopeError {
                message: "No scope available".to_string(),
            })?;
        if current_scope.contains_key(&name) {
            return Err(CodeGenError::NameError {
                message: format!("variable '{}' already declared in this scope.", name),
            });
        }
        current_scope.insert(
            name.clone(),
            Symbol {
                name: name.clone(),
                ir_type,
            },
        );
        Ok(())
    }

    pub fn declare_var_with_type(
        &mut self,
        name: String,
        ir_type: IRType,
        htype: Type,
    ) -> Result<(), CodeGenError> {
        self.declare_var(name.clone(), ir_type)?;
        self.var_types.insert(name, htype);
        Ok(())
    }

    fn get_var_high_type(&self, name: &str) -> Option<&Type> {
        self.var_types.get(name)
    }
}

pub struct IRGen {
    functions: Vec<IRFunction>,
    constants: Vec<IRConst>,
    constant_pool: HashMap<IRConst, usize>,
    structs: HashMap<String, Vec<(String, Type)>>,
    lambda_counter: u32,
}

impl IRGen {
    pub fn new() -> Self {
        Self {
            functions: Vec::new(),
            constants: Vec::new(),
            constant_pool: HashMap::new(),
            structs: HashMap::new(),
            lambda_counter: 0,
        }
    }

    fn convert_lambdas_to_functions(&mut self, program: Program) -> Program {
        let mut new_body = Vec::new();
        let mut lambda_map: HashMap<String, Expr> = HashMap::new();

        fn process_expr(
            expr: Expr,
            lambda_counter: &mut u32,
            lambda_map: &mut HashMap<String, Expr>,
        ) -> Expr {
            match expr {
                Expr::Lambda(params, body, ret_type, _) => {
                    let lambda_name = format!("_lambda_{}", lambda_counter);
                    *lambda_counter += 1;

                    let lambda_func = Expr::FuncDecl(
                        lambda_name.clone(),
                        params,
                        ret_type,
                        body,
                        crate::compiler::Span::new(0, 0),
                    );
                    lambda_map.insert(lambda_name.clone(), lambda_func);

                    Expr::Var(lambda_name, crate::compiler::Span::new(0, 0))
                }
                Expr::Block(body, _) => Expr::Block(
                    body.into_iter()
                        .map(|e| process_expr(e, lambda_counter, lambda_map))
                        .collect(),
                    crate::compiler::Span::new(0, 0),
                ),
                Expr::Add(l, r, _) => Expr::Add(
                    Box::new(process_expr(*l, lambda_counter, lambda_map)),
                    Box::new(process_expr(*r, lambda_counter, lambda_map)),
                    crate::compiler::Span::new(0, 0),
                ),
                Expr::Sub(l, r, _) => Expr::Sub(
                    Box::new(process_expr(*l, lambda_counter, lambda_map)),
                    Box::new(process_expr(*r, lambda_counter, lambda_map)),
                    crate::compiler::Span::new(0, 0),
                ),
                Expr::Mul(l, r, _) => Expr::Mul(
                    Box::new(process_expr(*l, lambda_counter, lambda_map)),
                    Box::new(process_expr(*r, lambda_counter, lambda_map)),
                    crate::compiler::Span::new(0, 0),
                ),
                Expr::Div(l, r, _) => Expr::Div(
                    Box::new(process_expr(*l, lambda_counter, lambda_map)),
                    Box::new(process_expr(*r, lambda_counter, lambda_map)),
                    crate::compiler::Span::new(0, 0),
                ),
                Expr::Mod(l, r, _) => Expr::Mod(
                    Box::new(process_expr(*l, lambda_counter, lambda_map)),
                    Box::new(process_expr(*r, lambda_counter, lambda_map)),
                    crate::compiler::Span::new(0, 0),
                ),
                Expr::FAdd(l, r, _) => Expr::FAdd(
                    Box::new(process_expr(*l, lambda_counter, lambda_map)),
                    Box::new(process_expr(*r, lambda_counter, lambda_map)),
                    crate::compiler::Span::new(0, 0),
                ),
                Expr::FSub(l, r, _) => Expr::FSub(
                    Box::new(process_expr(*l, lambda_counter, lambda_map)),
                    Box::new(process_expr(*r, lambda_counter, lambda_map)),
                    crate::compiler::Span::new(0, 0),
                ),
                Expr::FMul(l, r, _) => Expr::FMul(
                    Box::new(process_expr(*l, lambda_counter, lambda_map)),
                    Box::new(process_expr(*r, lambda_counter, lambda_map)),
                    crate::compiler::Span::new(0, 0),
                ),
                Expr::FDiv(l, r, _) => Expr::FDiv(
                    Box::new(process_expr(*l, lambda_counter, lambda_map)),
                    Box::new(process_expr(*r, lambda_counter, lambda_map)),
                    crate::compiler::Span::new(0, 0),
                ),
                Expr::Eq(l, r, _) => Expr::Eq(
                    Box::new(process_expr(*l, lambda_counter, lambda_map)),
                    Box::new(process_expr(*r, lambda_counter, lambda_map)),
                    crate::compiler::Span::new(0, 0),
                ),
                Expr::Ne(l, r, _) => Expr::Ne(
                    Box::new(process_expr(*l, lambda_counter, lambda_map)),
                    Box::new(process_expr(*r, lambda_counter, lambda_map)),
                    crate::compiler::Span::new(0, 0),
                ),
                Expr::Lt(l, r, _) => Expr::Lt(
                    Box::new(process_expr(*l, lambda_counter, lambda_map)),
                    Box::new(process_expr(*r, lambda_counter, lambda_map)),
                    crate::compiler::Span::new(0, 0),
                ),
                Expr::Le(l, r, _) => Expr::Le(
                    Box::new(process_expr(*l, lambda_counter, lambda_map)),
                    Box::new(process_expr(*r, lambda_counter, lambda_map)),
                    crate::compiler::Span::new(0, 0),
                ),
                Expr::Gt(l, r, _) => Expr::Gt(
                    Box::new(process_expr(*l, lambda_counter, lambda_map)),
                    Box::new(process_expr(*r, lambda_counter, lambda_map)),
                    crate::compiler::Span::new(0, 0),
                ),
                Expr::Ge(l, r, _) => Expr::Ge(
                    Box::new(process_expr(*l, lambda_counter, lambda_map)),
                    Box::new(process_expr(*r, lambda_counter, lambda_map)),
                    crate::compiler::Span::new(0, 0),
                ),
                Expr::FEq(l, r, _) => Expr::FEq(
                    Box::new(process_expr(*l, lambda_counter, lambda_map)),
                    Box::new(process_expr(*r, lambda_counter, lambda_map)),
                    crate::compiler::Span::new(0, 0),
                ),
                Expr::FNe(l, r, _) => Expr::FNe(
                    Box::new(process_expr(*l, lambda_counter, lambda_map)),
                    Box::new(process_expr(*r, lambda_counter, lambda_map)),
                    crate::compiler::Span::new(0, 0),
                ),
                Expr::FLt(l, r, _) => Expr::FLt(
                    Box::new(process_expr(*l, lambda_counter, lambda_map)),
                    Box::new(process_expr(*r, lambda_counter, lambda_map)),
                    crate::compiler::Span::new(0, 0),
                ),
                Expr::FLe(l, r, _) => Expr::FLe(
                    Box::new(process_expr(*l, lambda_counter, lambda_map)),
                    Box::new(process_expr(*r, lambda_counter, lambda_map)),
                    crate::compiler::Span::new(0, 0),
                ),
                Expr::FGt(l, r, _) => Expr::FGt(
                    Box::new(process_expr(*l, lambda_counter, lambda_map)),
                    Box::new(process_expr(*r, lambda_counter, lambda_map)),
                    crate::compiler::Span::new(0, 0),
                ),
                Expr::FGe(l, r, _) => Expr::FGe(
                    Box::new(process_expr(*l, lambda_counter, lambda_map)),
                    Box::new(process_expr(*r, lambda_counter, lambda_map)),
                    crate::compiler::Span::new(0, 0),
                ),
                Expr::And(l, r, _) => Expr::And(
                    Box::new(process_expr(*l, lambda_counter, lambda_map)),
                    Box::new(process_expr(*r, lambda_counter, lambda_map)),
                    crate::compiler::Span::new(0, 0),
                ),
                Expr::Or(l, r, _) => Expr::Or(
                    Box::new(process_expr(*l, lambda_counter, lambda_map)),
                    Box::new(process_expr(*r, lambda_counter, lambda_map)),
                    crate::compiler::Span::new(0, 0),
                ),
                Expr::Not(e, _) => Expr::Not(
                    Box::new(process_expr(*e, lambda_counter, lambda_map)),
                    crate::compiler::Span::new(0, 0),
                ),
                Expr::StrCat(l, r, _) => Expr::StrCat(
                    Box::new(process_expr(*l, lambda_counter, lambda_map)),
                    Box::new(process_expr(*r, lambda_counter, lambda_map)),
                    crate::compiler::Span::new(0, 0),
                ),
                Expr::VarDecl(name, ty, val, _) => Expr::VarDecl(
                    name,
                    ty,
                    Box::new(process_expr(*val, lambda_counter, lambda_map)),
                    crate::compiler::Span::new(0, 0),
                ),
                Expr::VarAssign(name, val, _) => Expr::VarAssign(
                    name,
                    Box::new(process_expr(*val, lambda_counter, lambda_map)),
                    crate::compiler::Span::new(0, 0),
                ),
                Expr::Call(func, args, _) => Expr::Call(
                    Box::new(process_expr(*func, lambda_counter, lambda_map)),
                    args.into_iter()
                        .map(|a| process_expr(a, lambda_counter, lambda_map))
                        .collect(),
                    crate::compiler::Span::new(0, 0),
                ),
                Expr::Return(e, _) => Expr::Return(
                    Box::new(process_expr(*e, lambda_counter, lambda_map)),
                    crate::compiler::Span::new(0, 0),
                ),
                Expr::If(cond, then_branch, else_branch, _) => Expr::If(
                    Box::new(process_expr(*cond, lambda_counter, lambda_map)),
                    Box::new(process_expr(*then_branch, lambda_counter, lambda_map)),
                    else_branch.map(|e| Box::new(process_expr(*e, lambda_counter, lambda_map))),
                    crate::compiler::Span::new(0, 0),
                ),
                Expr::While(cond, body, _) => Expr::While(
                    Box::new(process_expr(*cond, lambda_counter, lambda_map)),
                    Box::new(process_expr(*body, lambda_counter, lambda_map)),
                    crate::compiler::Span::new(0, 0),
                ),
                Expr::For(var, array, body, _) => Expr::For(
                    var,
                    Box::new(process_expr(*array, lambda_counter, lambda_map)),
                    Box::new(process_expr(*body, lambda_counter, lambda_map)),
                    crate::compiler::Span::new(0, 0),
                ),
                Expr::Index(arr, idx, _) => Expr::Index(
                    Box::new(process_expr(*arr, lambda_counter, lambda_map)),
                    Box::new(process_expr(*idx, lambda_counter, lambda_map)),
                    crate::compiler::Span::new(0, 0),
                ),
                Expr::IndexAssign(arr, val, _) => Expr::IndexAssign(
                    Box::new(process_expr(*arr, lambda_counter, lambda_map)),
                    Box::new(process_expr(*val, lambda_counter, lambda_map)),
                    crate::compiler::Span::new(0, 0),
                ),
                Expr::ArrayLiteral(elements, _) => Expr::ArrayLiteral(
                    elements
                        .into_iter()
                        .map(|e| process_expr(e, lambda_counter, lambda_map))
                        .collect(),
                    crate::compiler::Span::new(0, 0),
                ),
                Expr::ArrayFill(ty, len, _) => Expr::ArrayFill(
                    ty,
                    Box::new(process_expr(*len, lambda_counter, lambda_map)),
                    crate::compiler::Span::new(0, 0),
                ),
                Expr::Range(start, end, _) => Expr::Range(
                    Box::new(process_expr(*start, lambda_counter, lambda_map)),
                    Box::new(process_expr(*end, lambda_counter, lambda_map)),
                    crate::compiler::Span::new(0, 0),
                ),
                Expr::StructLiteral(name, fields, _) => Expr::StructLiteral(
                    name,
                    fields
                        .into_iter()
                        .map(|(n, e)| (n, process_expr(e, lambda_counter, lambda_map)))
                        .collect(),
                    crate::compiler::Span::new(0, 0),
                ),
                Expr::MemberAccess(obj, field, _) => Expr::MemberAccess(
                    Box::new(process_expr(*obj, lambda_counter, lambda_map)),
                    field,
                    crate::compiler::Span::new(0, 0),
                ),
                Expr::MemberAssign(obj, field, val, _) => Expr::MemberAssign(
                    Box::new(process_expr(*obj, lambda_counter, lambda_map)),
                    field,
                    Box::new(process_expr(*val, lambda_counter, lambda_map)),
                    crate::compiler::Span::new(0, 0),
                ),
                Expr::AddressOf(expr, _) => Expr::AddressOf(
                    Box::new(process_expr(*expr, lambda_counter, lambda_map)),
                    crate::compiler::Span::new(0, 0),
                ),
                Expr::Deref(expr, _) => Expr::Deref(
                    Box::new(process_expr(*expr, lambda_counter, lambda_map)),
                    crate::compiler::Span::new(0, 0),
                ),
                Expr::DerefAssign(ptr, val, _) => Expr::DerefAssign(
                    Box::new(process_expr(*ptr, lambda_counter, lambda_map)),
                    Box::new(process_expr(*val, lambda_counter, lambda_map)),
                    crate::compiler::Span::new(0, 0),
                ),
                Expr::FuncDecl(name, params, ret_type, body, _) => Expr::FuncDecl(
                    name,
                    params,
                    ret_type,
                    Box::new(process_expr(*body, lambda_counter, lambda_map)),
                    crate::compiler::Span::new(0, 0),
                ),
                _ => expr,
            }
        }

        for expr in program.body {
            let processed = process_expr(expr, &mut self.lambda_counter, &mut lambda_map);
            new_body.push(processed);
        }

        let lambda_funcs: Vec<Expr> = lambda_map.into_values().collect();
        new_body.splice(0..0, lambda_funcs);

        Program { body: new_body }
    }

    pub fn compile(&mut self, program: Program) -> Result<IRProgram, CodeGenError> {
        let program = self.convert_lambdas_to_functions(program);

        for expr in &program.body {
            match expr {
                Expr::FuncDecl(name, params, ret_type, _, _) => {
                    self.func_decl(name.clone(), params.clone(), ret_type.clone())?;
                }
                Expr::Extern(name, params, ret_type, _) => {
                    self.extern_decl(name.clone(), params.clone(), ret_type.clone())?;
                }
                Expr::Struct(name, fields, _) => {
                    self.structs.insert(name.clone(), fields.clone());
                }
                _ => {}
            }
        }

        for expr in program.body {
            match expr {
                Expr::FuncDecl(name, params, _, body, _) => {
                    self.compile_fn(name, params, *body)?;
                }
                Expr::Int(_, _)
                | Expr::Float(_, _)
                | Expr::Bool(_, _)
                | Expr::String(_, _)
                | Expr::Nil(_)
                | Expr::Var(_, _) => {
                    let mut ctx = Context::new();
                    ctx.enter_scope();
                    self.compile_expr(
                        Expr::VarDecl(
                            "_global".to_string(),
                            Type::Named("int".to_string()),
                            Box::new(expr),
                            crate::compiler::Span::new(0, 0),
                        ),
                        &mut ctx,
                    )?;
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

    fn get_binop_parts(expr: Expr) -> Result<(Op, Box<Expr>, Box<Expr>), CodeGenError> {
        let _span = crate::compiler::Span::new(0, 0);
        match expr {
            Expr::Add(l, r, _) => Ok((Op::Add, l, r)),
            Expr::Sub(l, r, _) => Ok((Op::Sub, l, r)),
            Expr::Mul(l, r, _) => Ok((Op::Mul, l, r)),
            Expr::Div(l, r, _) => Ok((Op::Div, l, r)),
            Expr::Mod(l, r, _) => Ok((Op::Mod, l, r)),
            Expr::FAdd(l, r, _) => Ok((Op::FAdd, l, r)),
            Expr::FSub(l, r, _) => Ok((Op::FSub, l, r)),
            Expr::FMul(l, r, _) => Ok((Op::FMul, l, r)),
            Expr::FDiv(l, r, _) => Ok((Op::FDiv, l, r)),
            Expr::Eq(l, r, _) => Ok((Op::Eq, l, r)),
            Expr::Ne(l, r, _) => Ok((Op::Ne, l, r)),
            Expr::Lt(l, r, _) => Ok((Op::Lt, l, r)),
            Expr::Le(l, r, _) => Ok((Op::Le, l, r)),
            Expr::Gt(l, r, _) => Ok((Op::Gt, l, r)),
            Expr::Ge(l, r, _) => Ok((Op::Ge, l, r)),
            Expr::FEq(l, r, _) => Ok((Op::FEq, l, r)),
            Expr::FNe(l, r, _) => Ok((Op::FNe, l, r)),
            Expr::FLt(l, r, _) => Ok((Op::FLt, l, r)),
            Expr::FLe(l, r, _) => Ok((Op::FLe, l, r)),
            Expr::FGt(l, r, _) => Ok((Op::FGt, l, r)),
            Expr::FGe(l, r, _) => Ok((Op::FGe, l, r)),
            Expr::And(l, r, _) => Ok((Op::And, l, r)),
            Expr::Or(l, r, _) => Ok((Op::Or, l, r)),
            Expr::Xor(l, r, _) => Ok((Op::Xor, l, r)),
            Expr::LAnd(l, r, _) => Ok((Op::LAnd, l, r)),
            Expr::LOr(l, r, _) => Ok((Op::LOr, l, r)),
            Expr::StrCat(l, r, _) => Ok((Op::StrCat, l, r)),
            _ => Err(CodeGenError::UnsupportedOperation {
                message: "not a binary operation".to_string(),
            }),
        }
    }

    fn compile_struct_literal(
        &mut self,
        struct_name: &str,
        field_values: Vec<(String, Expr)>,
        ctx: &mut Context,
    ) -> Result<Operand, CodeGenError> {
        let fields = self
            .structs
            .get(struct_name)
            .ok_or_else(|| CodeGenError::NameError {
                message: format!("undefined struct '{}'", struct_name),
            })?;
        let fields = fields.clone();
        let total_size = fields.len() * 8;
        let size_idx = self.get_const_index(IRConst::Int(total_size as i64));
        let ptr_tmp = ctx.new_tmp(IRType::Int);

        ctx.instructions.push(Instruction {
            op: Op::Malloc,
            dst: Some(ptr_tmp.clone()),
            src1: Some(Operand::ConstIdx(size_idx)),
            src2: None,
        });

        for (i, (field_name, _)) in fields.iter().enumerate() {
            if let Some((_, field_expr)) = field_values.iter().find(|(n, _)| n == field_name) {
                let val = self.compile_expr(field_expr.clone(), ctx)?;
                let offset_idx = self.get_const_index(IRConst::Int((i * 8) as i64));
                ctx.instructions.push(Instruction {
                    op: Op::StoreAt,
                    dst: Some(ptr_tmp.clone()),
                    src1: Some(Operand::ConstIdx(offset_idx)),
                    src2: Some(val),
                });
            }
        }

        Ok(ptr_tmp)
    }

    fn compile_expr(&mut self, expr: Expr, ctx: &mut Context) -> Result<Operand, CodeGenError> {
        match expr {
            Expr::Int(n, _) => {
                let ir_type = IRType::Int;
                let ir_const = IRConst::Int(n as i64);
                let res_tmp = ctx.new_tmp(ir_type);
                let const_idx = self.get_const_index(ir_const);
                ctx.instructions.push(Instruction {
                    op: Op::Move,
                    dst: Some(res_tmp.clone()),
                    src1: Some(Operand::ConstIdx(const_idx)),
                    src2: None,
                });
                Ok(res_tmp)
            }
            Expr::Float(f, _) => {
                let ir_type = IRType::Float;
                let ir_const = IRConst::Float(OrderedFloat(f));
                let res_tmp = ctx.new_tmp(ir_type);
                let const_idx = self.get_const_index(ir_const);
                ctx.instructions.push(Instruction {
                    op: Op::FMove,
                    dst: Some(res_tmp.clone()),
                    src1: Some(Operand::ConstIdx(const_idx)),
                    src2: None,
                });
                Ok(res_tmp)
            }
            Expr::Bool(b, _) => {
                let ir_const = IRConst::Int(if b { 1 } else { 0 });
                let res_tmp = ctx.new_tmp(IRType::Bool);
                let const_idx = self.get_const_index(ir_const);
                ctx.instructions.push(Instruction {
                    op: Op::Move,
                    dst: Some(res_tmp.clone()),
                    src1: Some(Operand::ConstIdx(const_idx)),
                    src2: None,
                });
                Ok(res_tmp)
            }
            Expr::String(s, _) => {
                let ir_const = IRConst::Str(s);
                let res_tmp = ctx.new_tmp(IRType::String);
                let const_idx = self.get_const_index(ir_const);
                ctx.instructions.push(Instruction {
                    op: Op::Move,
                    dst: Some(res_tmp.clone()),
                    src1: Some(Operand::ConstIdx(const_idx)),
                    src2: None,
                });
                Ok(res_tmp)
            }
            Expr::Nil(_) => {
                let res_tmp = ctx.new_tmp(IRType::Void);
                Ok(res_tmp)
            }

            Expr::VarDecl(name, typ, value, _) => {
                let value = self.compile_expr(*value, ctx)?;
                let value_type = ctx.get_operand_type(&value, &self.constants)?;
                let var_ir_type = Context::type_to_ir_type(&typ);

                let var_ir_type = match &typ {
                    Type::Array(_, declared_len) if *declared_len > 0 => {
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
                                                return Ok(Operand::Temp(
                                                    ctx.tmp_cnt - 1,
                                                    IRType::Array(Some(*declared_len)),
                                                ));
                                            }
                                        }
                                    }
                                }
                            } else if *declared_len != *actual_len {
                                return Err(CodeGenError::TypeError {
                                    message: "array length mismatch".to_string(),
                                });
                            }
                            IRType::Array(Some(*declared_len))
                        } else {
                            return Err(CodeGenError::TypeError {
                                message: "expected array".to_string(),
                            });
                        }
                    }
                    _ => var_ir_type,
                };

                ctx.declare_var_with_type(name.clone(), var_ir_type.clone(), typ.clone())?;
                match var_ir_type {
                    IRType::Float => ctx.instructions.push(Instruction {
                        op: Op::FStore,
                        dst: Some(Operand::Var(name)),
                        src1: Some(value),
                        src2: None,
                    }),
                    _ => ctx.instructions.push(Instruction {
                        op: Op::Store,
                        dst: Some(Operand::Var(name)),
                        src1: Some(value),
                        src2: None,
                    }),
                }
                Ok(ctx.new_tmp(IRType::Void))
            }

            Expr::VarAssign(name, value, _) => {
                let value = self.compile_expr(*value, ctx)?;
                let typ = ctx.get_operand_type(&value, &self.constants)?;
                let var_typ = ctx.get_var_type(&name)?;
                if typ != var_typ {
                    return Err(CodeGenError::TypeError {
                        message: format!("unexpected type: {:?}", typ),
                    });
                }
                match typ {
                    IRType::Float => ctx.instructions.push(Instruction {
                        op: Op::FStore,
                        dst: Some(Operand::Var(name)),
                        src1: Some(value),
                        src2: None,
                    }),
                    _ => ctx.instructions.push(Instruction {
                        op: Op::Store,
                        dst: Some(Operand::Var(name)),
                        src1: Some(value),
                        src2: None,
                    }),
                }
                Ok(ctx.new_tmp(IRType::Void))
            }

            Expr::Var(name, _) => {
                if let Ok(var_type) = ctx.get_var_type(&name) {
                    let res_tmp = ctx.new_tmp(var_type.clone());
                    match var_type {
                        IRType::Float => ctx.instructions.push(Instruction {
                            op: Op::FLoad,
                            dst: Some(res_tmp.clone()),
                            src1: Some(Operand::Var(name)),
                            src2: None,
                        }),
                        _ => ctx.instructions.push(Instruction {
                            op: Op::Load,
                            dst: Some(res_tmp.clone()),
                            src1: Some(Operand::Var(name)),
                            src2: None,
                        }),
                    }
                    Ok(res_tmp)
                } else if let Ok(func) = self.find_func(&name) {
                    Ok(Operand::Function(func.name))
                } else {
                    Err(CodeGenError::UndefinedVariable {
                        name: name.clone(),
                        span: crate::compiler::Span::new(0, 0),
                    })
                }
            }

            Expr::Add(_, _, _)
            | Expr::Sub(_, _, _)
            | Expr::Mul(_, _, _)
            | Expr::Div(_, _, _)
            | Expr::Mod(_, _, _)
            | Expr::FAdd(_, _, _)
            | Expr::FSub(_, _, _)
            | Expr::FMul(_, _, _)
            | Expr::FDiv(_, _, _)
            | Expr::Eq(_, _, _)
            | Expr::Ne(_, _, _)
            | Expr::Lt(_, _, _)
            | Expr::Le(_, _, _)
            | Expr::Gt(_, _, _)
            | Expr::Ge(_, _, _)
            | Expr::FEq(_, _, _)
            | Expr::FNe(_, _, _)
            | Expr::FLt(_, _, _)
            | Expr::FLe(_, _, _)
            | Expr::FGt(_, _, _)
            | Expr::FGe(_, _, _)
            | Expr::And(_, _, _)
            | Expr::Or(_, _, _)
            | Expr::Xor(_, _, _)
            | Expr::LAnd(_, _, _)
            | Expr::LOr(_, _, _)
            | Expr::StrCat(_, _, _) => {
                let (op, l, r) = Self::get_binop_parts(expr)?;
                let left = self.compile_expr(*l, ctx)?;
                let right = self.compile_expr(*r, ctx)?;
                let typ = ctx.get_operand_type(&left, &self.constants)?;

                let res_tmp = match op {
                    Op::StrCat => ctx.new_tmp(IRType::String),
                    _ => ctx.new_tmp(typ.clone()),
                };

                ctx.instructions.push(Instruction {
                    op,
                    dst: Some(res_tmp.clone()),
                    src1: Some(left),
                    src2: Some(right),
                });
                Ok(res_tmp)
            }

            Expr::Not(e, _) => {
                let arg = self.compile_expr(*e, ctx)?;
                let res_tmp = ctx.new_tmp(IRType::Bool);
                ctx.instructions.push(Instruction {
                    op: Op::Not,
                    dst: Some(res_tmp.clone()),
                    src1: Some(arg),
                    src2: None,
                });
                Ok(res_tmp)
            }

            Expr::Neg(expr, _) => {
                let arg = self.compile_expr(*expr, ctx)?;
                let res_tmp = ctx.new_tmp(IRType::Int);
                ctx.instructions.push(Instruction {
                    op: Op::Neg,
                    dst: Some(res_tmp.clone()),
                    src1: Some(arg),
                    src2: None,
                });
                Ok(res_tmp)
            }

            Expr::FNeg(expr, _) => {
                let arg = self.compile_expr(*expr, ctx)?;
                let res_tmp = ctx.new_tmp(IRType::Float);
                ctx.instructions.push(Instruction {
                    op: Op::FNeg,
                    dst: Some(res_tmp.clone()),
                    src1: Some(arg),
                    src2: None,
                });
                Ok(res_tmp)
            }

            Expr::Inc(name, _) => {
                let var_op = Operand::Var(name);
                let res_tmp = ctx.new_tmp(IRType::Int);
                ctx.instructions.push(Instruction {
                    op: Op::Load,
                    dst: Some(res_tmp.clone()),
                    src1: Some(var_op.clone()),
                    src2: None,
                });
                ctx.instructions.push(Instruction {
                    op: Op::Inc,
                    dst: Some(res_tmp.clone()),
                    src1: Some(res_tmp.clone()),
                    src2: None,
                });
                ctx.instructions.push(Instruction {
                    op: Op::Store,
                    dst: Some(var_op),
                    src1: Some(res_tmp.clone()),
                    src2: None,
                });
                Ok(res_tmp)
            }

            Expr::Dec(name, _) => {
                let var_op = Operand::Var(name);
                let res_tmp = ctx.new_tmp(IRType::Int);
                ctx.instructions.push(Instruction {
                    op: Op::Load,
                    dst: Some(res_tmp.clone()),
                    src1: Some(var_op.clone()),
                    src2: None,
                });
                ctx.instructions.push(Instruction {
                    op: Op::Dec,
                    dst: Some(res_tmp.clone()),
                    src1: Some(res_tmp.clone()),
                    src2: None,
                });
                ctx.instructions.push(Instruction {
                    op: Op::Store,
                    dst: Some(var_op),
                    src1: Some(res_tmp.clone()),
                    src2: None,
                });
                Ok(res_tmp)
            }

            Expr::AddAssign(name, value, _) => {
                let var_op = Operand::Var(name.clone());
                let rhs = self.compile_expr(*value, ctx)?;
                let var_tmp = ctx.new_tmp(IRType::Int);
                ctx.instructions.push(Instruction {
                    op: Op::Load,
                    dst: Some(var_tmp.clone()),
                    src1: Some(var_op.clone()),
                    src2: None,
                });
                let res_tmp = ctx.new_tmp(IRType::Int);
                ctx.instructions.push(Instruction {
                    op: Op::Add,
                    dst: Some(res_tmp.clone()),
                    src1: Some(var_tmp),
                    src2: Some(rhs),
                });
                ctx.instructions.push(Instruction {
                    op: Op::Store,
                    dst: Some(var_op),
                    src1: Some(res_tmp.clone()),
                    src2: None,
                });
                Ok(res_tmp)
            }

            Expr::SubAssign(name, value, _) => {
                let var_op = Operand::Var(name.clone());
                let rhs = self.compile_expr(*value, ctx)?;
                let var_tmp = ctx.new_tmp(IRType::Int);
                ctx.instructions.push(Instruction {
                    op: Op::Load,
                    dst: Some(var_tmp.clone()),
                    src1: Some(var_op.clone()),
                    src2: None,
                });
                let res_tmp = ctx.new_tmp(IRType::Int);
                ctx.instructions.push(Instruction {
                    op: Op::Sub,
                    dst: Some(res_tmp.clone()),
                    src1: Some(var_tmp),
                    src2: Some(rhs),
                });
                ctx.instructions.push(Instruction {
                    op: Op::Store,
                    dst: Some(var_op),
                    src1: Some(res_tmp.clone()),
                    src2: None,
                });
                Ok(res_tmp)
            }

            Expr::Block(body, _) => {
                ctx.enter_scope();
                let body_len = body.len();
                for i in 0..body_len.saturating_sub(1) {
                    self.compile_expr(body[i].clone(), ctx)?;
                }
                let result_operand = if let Some(last_expr) = body.last() {
                    self.compile_expr(last_expr.clone(), ctx)?
                } else {
                    ctx.new_tmp(IRType::Void)
                };
                ctx.exit_scope()?;
                Ok(result_operand)
            }

            Expr::Return(val, _) => {
                let res_op = self.compile_expr(*val, ctx)?;
                match ctx.get_operand_type(&res_op, &self.constants)? {
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
                Ok(ctx.new_tmp(IRType::Void))
            }

            Expr::If(condition, then_branch, else_branch, _) => {
                let label_else = ctx.new_label("else");
                let label_end = ctx.new_label("endif");

                let cond = self.compile_expr(*condition, ctx)?;

                ctx.instructions.push(Instruction {
                    op: Op::JumpIfFalse,
                    dst: None,
                    src1: Some(cond),
                    src2: Some(Operand::Label(label_else.clone())),
                });

                let res_tmp = ctx.new_tmp(IRType::Void);

                ctx.enter_scope();
                let then_op = self.compile_expr(*then_branch, ctx)?;
                ctx.instructions.push(Instruction {
                    op: Op::Move,
                    dst: Some(res_tmp.clone()),
                    src1: Some(then_op),
                    src2: None,
                });
                ctx.exit_scope()?;

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

                if let Some(else_expr) = else_branch {
                    ctx.enter_scope();
                    let else_op = self.compile_expr(*else_expr, ctx)?;
                    ctx.instructions.push(Instruction {
                        op: Op::Move,
                        dst: Some(res_tmp.clone()),
                        src1: Some(else_op),
                        src2: None,
                    });
                    ctx.exit_scope()?;
                }

                ctx.instructions.push(Instruction {
                    op: Op::Label(label_end),
                    dst: None,
                    src1: None,
                    src2: None,
                });

                Ok(res_tmp)
            }

            Expr::While(condition, body, _) => {
                let label_start = ctx.new_label("while_start");
                let label_body = ctx.new_label("while_body");
                let label_end = ctx.new_label("while_end");

                ctx.loop_end_labels.push(label_end.clone());
                ctx.loop_inc_labels.push(label_body.clone());

                ctx.instructions.push(Instruction {
                    op: Op::Label(label_start.clone()),
                    dst: None,
                    src1: None,
                    src2: None,
                });

                let cond = self.compile_expr(*condition, ctx)?;
                ctx.instructions.push(Instruction {
                    op: Op::JumpIfFalse,
                    dst: None,
                    src1: Some(cond),
                    src2: Some(Operand::Label(label_end.clone())),
                });

                ctx.instructions.push(Instruction {
                    op: Op::Label(label_body.clone()),
                    dst: None,
                    src1: None,
                    src2: None,
                });

                ctx.enter_scope();
                self.compile_expr(*body, ctx)?;
                ctx.exit_scope()?;

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

                ctx.loop_end_labels.pop();
                ctx.loop_inc_labels.pop();

                Ok(ctx.new_tmp(IRType::Void))
            }

            Expr::For(var, iter, body, _) => {
                let array_operand = self.compile_expr(*iter, ctx)?;
                let array_type = ctx.get_operand_type(&array_operand, &self.constants)?;

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
                        return Err(CodeGenError::TypeError {
                            message: format!(
                                "can only iterate over arrays, found {:?}",
                                array_type
                            ),
                        });
                    }
                };

                let label_cond = ctx.new_label("for_cond");
                let label_end = ctx.new_label("for_end");
                let label_inc = ctx.new_label("for_inc");

                ctx.loop_end_labels.push(label_end.clone());
                ctx.loop_inc_labels.push(label_inc.clone());

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

                ctx.declare_var(var.clone(), IRType::Int)?;
                let element_tmp = ctx.new_tmp(IRType::Int);

                ctx.instructions.push(Instruction {
                    op: Op::ArrayAccess,
                    dst: Some(element_tmp.clone()),
                    src1: Some(array_operand),
                    src2: Some(curr_idx.clone()),
                });

                ctx.instructions.push(Instruction {
                    op: Op::Store,
                    dst: Some(Operand::Var(var)),
                    src1: Some(element_tmp),
                    src2: None,
                });

                self.compile_expr(*body, ctx)?;

                ctx.instructions.push(Instruction {
                    op: Op::Label(label_inc.clone()),
                    dst: None,
                    src1: None,
                    src2: None,
                });

                let one_idx = self.get_const_index(IRConst::Int(1));
                let curr_idx2 = ctx.new_tmp(IRType::Int);
                ctx.instructions.push(Instruction {
                    op: Op::Load,
                    dst: Some(curr_idx2.clone()),
                    src1: Some(idx_var.clone()),
                    src2: None,
                });
                let next_idx = ctx.new_tmp(IRType::Int);
                ctx.instructions.push(Instruction {
                    op: Op::Add,
                    dst: Some(next_idx.clone()),
                    src1: Some(curr_idx2),
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
                ctx.loop_end_labels.pop();
                ctx.loop_inc_labels.pop();

                Ok(ctx.new_tmp(IRType::Void))
            }

            Expr::Break(_) => {
                let end = ctx
                    .loop_end_labels
                    .last()
                    .ok_or_else(|| CodeGenError::SyntaxError {
                        message: "break outside of loop".to_string(),
                    })?;
                ctx.instructions.push(Instruction {
                    op: Op::Jump,
                    dst: None,
                    src1: Some(Operand::Label(end.clone())),
                    src2: None,
                });
                Ok(ctx.new_tmp(IRType::Void))
            }

            Expr::Continue(_) => {
                let inc = ctx
                    .loop_inc_labels
                    .last()
                    .ok_or_else(|| CodeGenError::SyntaxError {
                        message: "continue outside of loop".to_string(),
                    })?;
                ctx.instructions.push(Instruction {
                    op: Op::Jump,
                    dst: None,
                    src1: Some(Operand::Label(inc.clone())),
                    src2: None,
                });
                Ok(ctx.new_tmp(IRType::Void))
            }

            Expr::FuncDecl(_, _, _, _, _) => Err(CodeGenError::SyntaxError {
                message: "cannot declare a function in a function".to_string(),
            }),

            Expr::Call(callee, args, _) => {
                let func_name = match &*callee {
                    Expr::Var(name, _) => {
                        if self.find_func(name).is_ok() {
                            Some(name.clone())
                        } else {
                            None
                        }
                    }
                    _ => None,
                };

                if let Some(ref name) = func_name {
                    let func = self.find_func(name)?;
                    if args.len() != func.params.len() {
                        return Err(CodeGenError::TypeError {
                            message: format!(
                                "expected {} arguments, got {}",
                                func.params.len(),
                                args.len()
                            ),
                        });
                    }
                    let mut n = 0;
                    for (arg, param) in zip(args.iter(), func.params.iter()) {
                        let operand = self.compile_expr(arg.clone(), ctx)?;
                        let operand_type = ctx.get_operand_type(&operand, &self.constants)?;
                        if operand_type != param.1 {
                            return Err(CodeGenError::TypeError {
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
                    let res_tmp = ctx.new_tmp(func.ret_type);
                    ctx.instructions.push(Instruction {
                        op: Op::Call,
                        dst: Some(res_tmp.clone()),
                        src1: Some(Operand::Function(name.clone())),
                        src2: None,
                    });
                    Ok(res_tmp)
                } else {
                    let callee_op = self.compile_expr(*callee, ctx)?;
                    for (n, arg) in args.iter().enumerate() {
                        let operand = self.compile_expr(arg.clone(), ctx)?;
                        ctx.instructions.push(Instruction {
                            op: Op::Arg(n as usize),
                            dst: None,
                            src1: Some(operand),
                            src2: None,
                        });
                    }
                    let res_tmp = ctx.new_tmp(IRType::Int);
                    ctx.instructions.push(Instruction {
                        op: Op::Call,
                        dst: Some(res_tmp.clone()),
                        src1: Some(callee_op),
                        src2: None,
                    });
                    Ok(res_tmp)
                }
            }

            Expr::Index(arr, idx, _) => {
                let arr_op = self.compile_expr(*arr, ctx)?;
                let offset = self.compile_expr(*idx, ctx)?;
                let res_tmp = ctx.new_tmp(IRType::Int);
                ctx.instructions.push(Instruction {
                    op: Op::ArrayAccess,
                    dst: Some(res_tmp.clone()),
                    src1: Some(arr_op),
                    src2: Some(offset),
                });
                Ok(res_tmp)
            }

            Expr::IndexAssign(arr_idx, value, _) => {
                let (arr, idx) = match *arr_idx {
                    Expr::Index(arr, idx, _) => (arr, idx),
                    expr => {
                        return Err(CodeGenError::TypeError {
                            message: format!("expected index expression, got {:?}", expr),
                        });
                    }
                };
                let arr_op = self.compile_expr(*arr, ctx)?;
                let offset = self.compile_expr(*idx, ctx)?;
                let val = self.compile_expr(*value, ctx)?;
                let res_tmp = ctx.new_tmp(IRType::Void);
                ctx.instructions.push(Instruction {
                    op: Op::ArrayAssign,
                    dst: Some(arr_op),
                    src1: Some(offset),
                    src2: Some(val),
                });
                Ok(res_tmp)
            }

            Expr::ArrayLiteral(elements, _) => {
                let is_fill = false;
                let mut compiled = Vec::new();
                for e in elements.iter() {
                    compiled.push(self.compile_expr(e.clone(), ctx)?);
                }

                let (ir_const, ir_type) = if is_fill && compiled.len() == 1 {
                    let fill_element = compiled[0].clone();
                    let mut elems = Vec::new();
                    for _ in 0..elements.len() {
                        elems.push(fill_element.clone());
                    }
                    (
                        IRConst::Array(elems.len(), elems.clone()),
                        IRType::Array(Some(elems.len())),
                    )
                } else {
                    if elements.len() != compiled.len() {
                        return Err(CodeGenError::TypeError {
                            message: format!(
                                "Array literal length mismatch: declared {}, actual {}",
                                elements.len(),
                                compiled.len()
                            ),
                        });
                    }
                    (
                        IRConst::Array(compiled.len(), compiled.clone()),
                        IRType::Array(Some(compiled.len())),
                    )
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

            Expr::ArrayFill(typ, len, _) => {
                let len_op = self.compile_expr(*len, ctx)?;
                let elem_size = match &typ {
                    Type::Named(n) if n == "bool" => 1i64,
                    Type::Named(n) if matches!(n.as_str(), "int" | "float" | "string") => 8i64,
                    _ => 8i64,
                };
                let total_size_idx = self.get_const_index(IRConst::Int(
                    if let IRType::Array(Some(l)) = Context::type_to_ir_type(&typ) {
                        (l as i64 + 1) * 8
                    } else {
                        8 + elem_size * 8
                    },
                ));
                let ptr_tmp = ctx.new_tmp(IRType::Int);
                ctx.instructions.push(Instruction {
                    op: Op::Malloc,
                    dst: Some(ptr_tmp.clone()),
                    src1: Some(Operand::ConstIdx(total_size_idx)),
                    src2: None,
                });
                let zero_idx = self.get_const_index(IRConst::Int(0));
                ctx.instructions.push(Instruction {
                    op: Op::StoreAt,
                    dst: Some(ptr_tmp.clone()),
                    src1: Some(Operand::ConstIdx(zero_idx)),
                    src2: Some(len_op),
                });
                Ok(ptr_tmp)
            }

            Expr::Range(start, end, _) => {
                let start_op = self.compile_expr(*start, ctx)?;
                let end_op = self.compile_expr(*end, ctx)?;
                let res_tmp = ctx.new_tmp(IRType::Array(None));
                ctx.instructions.push(Instruction {
                    op: Op::Range,
                    dst: Some(res_tmp.clone()),
                    src1: Some(start_op),
                    src2: Some(end_op),
                });
                Ok(res_tmp)
            }

            Expr::Extern(_, _, _, _) => Err(CodeGenError::SyntaxError {
                message: "cannot extern a function in a function".to_string(),
            }),

            Expr::StructLiteral(name, fields, _) => self.compile_struct_literal(&name, fields, ctx),

            Expr::MemberAccess(obj, field_name, _) => {
                let struct_name = match &*obj {
                    Expr::Var(name, _) => {
                        if let Some(Type::Named(sname)) = ctx.get_var_high_type(name) {
                            sname.clone()
                        } else {
                            return Err(CodeGenError::TypeError {
                                message: "member access on non-struct variable".to_string(),
                            });
                        }
                    }
                    _ => {
                        return Err(CodeGenError::TypeError {
                            message: "member access on non-variable expression".to_string(),
                        });
                    }
                };

                let struct_def =
                    self.structs
                        .get(&struct_name)
                        .ok_or_else(|| CodeGenError::NameError {
                            message: format!("undefined struct '{}'", struct_name),
                        })?;

                let mut offset = 0;
                let mut found = false;
                for (i, (fname, _)) in struct_def.iter().enumerate() {
                    if fname == &field_name {
                        found = true;
                        offset = i * 8;
                        break;
                    }
                }
                if !found {
                    return Err(CodeGenError::NameError {
                        message: format!("struct '{}' has no field '{}'", struct_name, field_name),
                    });
                }

                let obj_op = self.compile_expr(*obj, ctx)?;
                let offset_idx = self.get_const_index(IRConst::Int(offset as i64));
                let res_tmp = ctx.new_tmp(IRType::Int);
                ctx.instructions.push(Instruction {
                    op: Op::LoadAt,
                    dst: Some(res_tmp.clone()),
                    src1: Some(obj_op),
                    src2: Some(Operand::ConstIdx(offset_idx)),
                });
                Ok(res_tmp)
            }

            Expr::MemberAssign(obj, field_name, value, _) => {
                let struct_name = match &*obj {
                    Expr::Var(name, _) => {
                        if let Some(Type::Named(sname)) = ctx.get_var_high_type(name) {
                            sname.clone()
                        } else {
                            return Err(CodeGenError::TypeError {
                                message: "member assign on non-struct variable".to_string(),
                            });
                        }
                    }
                    _ => {
                        return Err(CodeGenError::TypeError {
                            message: "member assign on non-variable expression".to_string(),
                        });
                    }
                };

                let struct_def =
                    self.structs
                        .get(&struct_name)
                        .ok_or_else(|| CodeGenError::NameError {
                            message: format!("undefined struct '{}'", struct_name),
                        })?;

                let mut offset = 0;
                let mut found = false;
                for (i, (fname, _)) in struct_def.iter().enumerate() {
                    if fname == &field_name {
                        found = true;
                        offset = i * 8;
                        break;
                    }
                }
                if !found {
                    return Err(CodeGenError::NameError {
                        message: format!("struct '{}' has no field '{}'", struct_name, field_name),
                    });
                }

                let obj_op = self.compile_expr(*obj, ctx)?;
                let val_op = self.compile_expr(*value, ctx)?;
                let offset_idx = self.get_const_index(IRConst::Int(offset as i64));
                ctx.instructions.push(Instruction {
                    op: Op::StoreAt,
                    dst: Some(obj_op),
                    src1: Some(Operand::ConstIdx(offset_idx)),
                    src2: Some(val_op),
                });
                Ok(ctx.new_tmp(IRType::Void))
            }

            Expr::AddressOf(inner, _) => {
                let name = match &*inner {
                    Expr::Var(name, _) => name.clone(),
                    _ => {
                        return Err(CodeGenError::UnsupportedOperation {
                            message: "address of non-variable".to_string(),
                        });
                    }
                };
                let res_tmp = ctx.new_tmp(IRType::Int);
                ctx.instructions.push(Instruction {
                    op: Op::Lea,
                    dst: Some(res_tmp.clone()),
                    src1: Some(Operand::Var(name)),
                    src2: None,
                });
                Ok(res_tmp)
            }

            Expr::Deref(inner, _) => {
                let ptr = self.compile_expr(*inner, ctx)?;
                let res_tmp = ctx.new_tmp(IRType::Int);
                ctx.instructions.push(Instruction {
                    op: Op::LoadAt,
                    dst: Some(res_tmp.clone()),
                    src1: Some(ptr),
                    src2: Some(Operand::ConstIdx(self.get_const_index(IRConst::Int(0)))),
                });
                Ok(res_tmp)
            }

            Expr::DerefAssign(ptr, val, _) => {
                let ptr_op = self.compile_expr(*ptr, ctx)?;
                let val_op = self.compile_expr(*val, ctx)?;
                ctx.instructions.push(Instruction {
                    op: Op::StoreAt,
                    dst: Some(ptr_op),
                    src1: Some(Operand::ConstIdx(self.get_const_index(IRConst::Int(0)))),
                    src2: Some(val_op),
                });
                Ok(ctx.new_tmp(IRType::Void))
            }

            Expr::TypeDef(_) | Expr::Struct(_, _, _) | Expr::Lambda(_, _, _, _) => {
                Ok(ctx.new_tmp(IRType::Void))
            }
        }
    }

    fn func_decl(
        &mut self,
        name: String,
        params: Vec<(String, Type)>,
        ret_type: Type,
    ) -> Result<(), CodeGenError> {
        let ir_params: Vec<(Operand, IRType)> = params
            .iter()
            .map(|(name, typ)| (Operand::Var(name.clone()), Context::type_to_ir_type(typ)))
            .collect();
        let ir_ret_type = Context::type_to_ir_type(&ret_type);
        self.functions.push(IRFunction {
            name,
            params: ir_params,
            ret_type: ir_ret_type,
            instructions: Vec::new(),
            is_pub: true,
            is_external: false,
        });
        Ok(())
    }

    fn compile_fn(
        &mut self,
        name: String,
        _params: Vec<(String, Type)>,
        body: Expr,
    ) -> Result<(), CodeGenError> {
        let func = self.find_func(&name)?;

        let mut ctx = Context::new();
        ctx.enter_scope();

        for (_i, (param, ty)) in func.params.iter().enumerate() {
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

    fn extern_decl(
        &mut self,
        name: String,
        params: Vec<(String, Type)>,
        ret_type: Type,
    ) -> Result<(), CodeGenError> {
        let ir_params: Vec<(Operand, IRType)> = params
            .into_iter()
            .enumerate()
            .map(|(i, (_, typ))| {
                let param_name = format!("a{}", i);
                (Operand::Var(param_name), Context::type_to_ir_type(&typ))
            })
            .collect();
        let ir_ret_type = Context::type_to_ir_type(&ret_type);
        let signature = IRFunction {
            name: name.clone(),
            params: ir_params,
            ret_type: ir_ret_type,
            instructions: Vec::new(),
            is_pub: false,
            is_external: true,
        };
        self.functions.push(signature);
        Ok(())
    }

    fn find_func(&self, name: &str) -> Result<IRFunction, CodeGenError> {
        for func in self.functions.iter().rev() {
            if func.name == *name {
                return Ok(func.to_owned());
            }
        }
        Err(CodeGenError::UndefinedFunction {
            name: name.to_string(),
            span: crate::compiler::Span::new(0, 0),
        })
    }
}
