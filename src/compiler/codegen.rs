use crate::compiler::ast::{Expr, Program, Type};
use cranelift::{
    codegen::{ir, settings},
    prelude::{
        AbiParam, FunctionBuilder, FunctionBuilderContext, InstBuilder, Signature, Value, Variable,
        isa::{self, CallConv},
        types,
    },
};
use cranelift_module::{FuncId, Linkage, Module};
use cranelift_object::{ObjectBuilder, ObjectModule};
use std::{collections::HashMap, fmt::Display};

#[derive(Debug)]
pub enum CodeGenError {
    UnexpectedExpression { found: Expr },
    UndefinedVariable { name: String },
    UndefinedFunction { name: String },
    ModuleError(String),
}

impl Display for CodeGenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CodeGenError::UnexpectedExpression { found } => {
                write!(f, "Unexpected expression: '{:?}', expected FuncDecl", found)
            }
            CodeGenError::UndefinedVariable { name } => {
                write!(f, "Undefined variable: '{:?}'", name)
            }
            CodeGenError::UndefinedFunction { name } => {
                write!(f, "Undefined function: '{:?}'", name)
            }
            CodeGenError::ModuleError(msg) => {
                write!(f, "Module error: '{}'", msg)
            }
        }
    }
}

impl std::error::Error for CodeGenError {}

pub struct CodeGen {
    ast: Program,
    module: ObjectModule,
    builder_context: FunctionBuilderContext,
    func_signatures: HashMap<String, (FuncId, Signature)>,
}

impl CodeGen {
    fn has_return(expr: &Expr) -> bool {
        match expr {
            Expr::Return(_) => true,
            Expr::Stmt(body) => body.last().map_or(false, |e| Self::has_return(e)),
            Expr::If(_, then_branch, else_branch) => {
                Self::has_return(then_branch)
                    && else_branch.as_ref().map_or(false, |e| Self::has_return(e))
            }
            Expr::While(_, body) => Self::has_return(body),
            Expr::For(_, _, _, body) => Self::has_return(body),
            _ => false,
        }
    }

    fn infer_expr_type_with_vars(expr: &Expr, type_stack: &Vec<HashMap<String, Type>>) -> Type {
        match expr {
            Expr::Int(_) => Type::Int,
            Expr::Float(_) => Type::Float,
            Expr::Bool(_) => Type::Bool,
            Expr::String(_) => Type::String,
            Expr::Nil => Type::Void,
            Expr::Add(lhs, rhs)
            | Expr::Sub(lhs, rhs)
            | Expr::Mul(lhs, rhs)
            | Expr::Div(lhs, rhs)
            | Expr::Mod(lhs, rhs)
            | Expr::Eq(lhs, rhs)
            | Expr::Ne(lhs, rhs)
            | Expr::Lt(lhs, rhs)
            | Expr::Le(lhs, rhs)
            | Expr::Gt(lhs, rhs)
            | Expr::Ge(lhs, rhs) => {
                let lhs_type = Self::infer_expr_type_with_vars(lhs, type_stack);
                let rhs_type = Self::infer_expr_type_with_vars(rhs, type_stack);

                if matches!(lhs_type, Type::Float) || matches!(rhs_type, Type::Float) {
                    Type::Float
                } else {
                    Type::Int
                }
            }
            Expr::Var(name) => {
                for scope in type_stack.iter().rev() {
                    if let Some(ty) = scope.get(name) {
                        return ty.clone();
                    }
                }
                Type::Int
            }
            _ => Type::Int,
        }
    }

    pub fn new(ast: Program) -> Self {
        let flag_builder = settings::builder();
        let flags = settings::Flags::new(flag_builder);
        let isa_builder = isa::lookup_by_name("x86_64").unwrap();
        let isa = isa_builder.finish(flags).unwrap();
        let object_builder = ObjectBuilder::new(
            isa,
            "main".to_string(),
            cranelift_module::default_libcall_names(),
        )
        .unwrap();
        let module = ObjectModule::new(object_builder);
        Self {
            ast,
            module,
            builder_context: FunctionBuilderContext::new(),
            func_signatures: HashMap::new(),
        }
    }

    pub fn generate(mut self) -> Result<Vec<u8>, CodeGenError> {
        for expr in self.ast.body.clone() {
            match expr {
                Expr::FuncDecl(name, params, ret_type, _body) => {
                    let mut sig = self.module.make_signature();
                    sig.call_conv = CallConv::SystemV;
                    for (_, t) in params {
                        sig.params.push(AbiParam::new(get_type(t)));
                    }

                    if !matches!(ret_type, Type::Void) {
                        sig.returns.push(AbiParam::new(get_type(ret_type)));
                    }
                    let func_id = self
                        .module
                        .declare_function(name.as_str(), Linkage::Export, &sig)
                        .unwrap();
                    self.func_signatures.insert(name, (func_id, sig));
                }
                Expr::Extern(name, params, ret_type) => {
                    let mut sig = self.module.make_signature();
                    sig.call_conv = CallConv::SystemV;
                    for (_, t) in params {
                        sig.params.push(AbiParam::new(get_type(t)));
                    }

                    if !matches!(ret_type, Type::Void) {
                        sig.returns.push(AbiParam::new(get_type(ret_type)));
                    }
                    let func_id = self
                        .module
                        .declare_function(name.as_str(), Linkage::Import, &sig)
                        .unwrap();
                    self.func_signatures.insert(name, (func_id, sig));
                }
                expr => return Err(CodeGenError::UnexpectedExpression { found: expr }),
            }
        }
        let mut str_idx = 0u32;
        for expr in self.ast.body.clone() {
            match expr {
                Expr::FuncDecl(name, params, ret_type, body) => {
                    self.compile_func(name, params, ret_type, body, &mut str_idx)?;
                }
                Expr::Extern(_, _, _) => {}
                expr => return Err(CodeGenError::UnexpectedExpression { found: expr }),
            }
        }

        let product = self.module.finish();
        let object_code = product
            .emit()
            .map_err(|e| CodeGenError::ModuleError(e.to_string()))?;
        Ok(object_code.to_vec())
    }

    fn compile_func(
        &mut self,
        name: String,
        params: Vec<(String, Type)>,
        _ret_type: Type,
        body: Box<Expr>,
        str_idx: &mut u32,
    ) -> Result<(), CodeGenError> {
        let (func_id, ref sig) = self.func_signatures[&name].clone();

        let mut new_ctx = self.module.make_context();
        new_ctx.func.signature = sig.clone();

        let param_types: Vec<ir::Type> =
            params.iter().map(|(_, ty)| get_type(ty.clone())).collect();

        let mut builder = FunctionBuilder::new(&mut new_ctx.func, &mut self.builder_context);
        let entry_block = builder.create_block();
        builder.append_block_params_for_function_params(entry_block);
        builder.switch_to_block(entry_block);
        builder.seal_block(entry_block);

        let mut scope_stack: Vec<HashMap<String, Variable>> = Vec::new();
        let mut type_stack: Vec<HashMap<String, Type>> = Vec::new();
        scope_stack.push(HashMap::new());
        type_stack.push(HashMap::new());
        let mut idx = 0;

        for (i, (param_name, param_ty)) in params.iter().enumerate() {
            let val = builder.block_params(entry_block)[i];
            let var = Variable::from_u32(idx);
            idx += 1;
            builder.declare_var(param_types[i]);
            builder.def_var(var, val);
            scope_stack[0].insert(param_name.clone(), var);
            type_stack[0].insert(param_name.clone(), param_ty.clone());
        }

        Self::compile_expr(
            &body,
            &mut builder,
            &mut scope_stack,
            &mut type_stack,
            &mut idx,
            &self.func_signatures,
            &mut self.module,
            str_idx,
        )?;

        if matches!(_ret_type, Type::Void) {
            builder.ins().return_(&[]);
        }

        builder.finalize();
        self.module
            .define_function(func_id, &mut new_ctx)
            .map_err(|e| {
                eprintln!("Verification error for function '{}':", name);
                eprintln!("Function IR:\n{}", new_ctx.func.display());
                CodeGenError::ModuleError(format!("{}: {}", name, e))
            })?;

        Ok(())
    }

    fn lookup_var<'a>(
        name: &str,
        scope_stack: &'a Vec<HashMap<String, Variable>>,
    ) -> Option<&'a Variable> {
        for scope in scope_stack.iter().rev() {
            if let Some(v) = scope.get(name) {
                return Some(v);
            }
        }
        None
    }

    fn compile_expr(
        expr: &Expr,
        builder: &mut FunctionBuilder,
        scope_stack: &mut Vec<HashMap<String, Variable>>,
        type_stack: &mut Vec<HashMap<String, Type>>,
        idx: &mut u32,
        func_signatures: &HashMap<String, (FuncId, Signature)>,
        module: &mut ObjectModule,
        str_idx: &mut u32,
    ) -> Result<Value, CodeGenError> {
        match expr {
            Expr::Stmt(body) => {
                scope_stack.push(HashMap::new());
                type_stack.push(HashMap::new());
                let result = if body.len() > 1 {
                    let (last, body) = body.split_last().unwrap();
                    for expr in body {
                        Self::compile_expr(
                            expr,
                            builder,
                            scope_stack,
                            type_stack,
                            idx,
                            func_signatures,
                            module,
                            str_idx,
                        )?;
                    }
                    Self::compile_expr(
                        last,
                        builder,
                        scope_stack,
                        type_stack,
                        idx,
                        func_signatures,
                        module,
                        str_idx,
                    )
                } else {
                    Self::compile_expr(
                        body.last().unwrap(),
                        builder,
                        scope_stack,
                        type_stack,
                        idx,
                        func_signatures,
                        module,
                        str_idx,
                    )
                };
                scope_stack.pop();
                type_stack.pop();
                result
            }
            Expr::Int(i) => Ok(builder.ins().iconst(types::I64, *i as i64)),
            Expr::Float(f) => Ok(builder.ins().f64const(*f)),
            Expr::Bool(b) => Ok(builder.ins().iconst(types::I8, *b as i64)),
            Expr::String(s) => {
                let data_id = module
                    .declare_data(
                        &format!("str_{}", str_idx),
                        cranelift_module::Linkage::Local,
                        false,
                        false,
                    )
                    .map_err(|e| CodeGenError::ModuleError(e.to_string()))?;

                let mut data_desc = cranelift_module::DataDescription::new();
                let mut bytes = s.as_bytes().to_vec();
                bytes.push(0);
                data_desc.define(bytes.into());
                module
                    .define_data(data_id, &data_desc)
                    .map_err(|e| CodeGenError::ModuleError(e.to_string()))?;

                let global_value = module.declare_data_in_func(data_id, builder.func);
                let ptr = builder.ins().global_value(types::I64, global_value);
                *str_idx += 1;
                Ok(ptr)
            }
            Expr::Nil => Ok(builder.ins().iconst(types::I64, 0)),
            Expr::Add(lhs, rhs)
            | Expr::Sub(lhs, rhs)
            | Expr::Mul(lhs, rhs)
            | Expr::Div(lhs, rhs)
            | Expr::Mod(lhs, rhs) => {
                let expr_type = Self::infer_expr_type_with_vars(expr, type_stack);
                let lhs = Self::compile_expr(
                    lhs,
                    builder,
                    scope_stack,
                    type_stack,
                    idx,
                    func_signatures,
                    module,
                    str_idx,
                )?;
                let rhs = Self::compile_expr(
                    rhs,
                    builder,
                    scope_stack,
                    type_stack,
                    idx,
                    func_signatures,
                    module,
                    str_idx,
                )?;

                if matches!(expr_type, Type::Float) {
                    match expr {
                        Expr::Add(_, _) => Ok(builder.ins().fadd(lhs, rhs)),
                        Expr::Sub(_, _) => Ok(builder.ins().fsub(lhs, rhs)),
                        Expr::Mul(_, _) => Ok(builder.ins().fmul(lhs, rhs)),
                        Expr::Div(_, _) => Ok(builder.ins().fdiv(lhs, rhs)),
                        _ => unreachable!(),
                    }
                } else {
                    match expr {
                        Expr::Add(_, _) => Ok(builder.ins().iadd(lhs, rhs)),
                        Expr::Sub(_, _) => Ok(builder.ins().isub(lhs, rhs)),
                        Expr::Mul(_, _) => Ok(builder.ins().imul(lhs, rhs)),
                        Expr::Div(_, _) => Ok(builder.ins().sdiv(lhs, rhs)),
                        Expr::Mod(_, _) => Ok(builder.ins().srem(lhs, rhs)),
                        _ => unreachable!(),
                    }
                }
            }
            Expr::Eq(lhs, rhs)
            | Expr::Ne(lhs, rhs)
            | Expr::Lt(lhs, rhs)
            | Expr::Le(lhs, rhs)
            | Expr::Gt(lhs, rhs)
            | Expr::Ge(lhs, rhs) => {
                let expr_type = Self::infer_expr_type_with_vars(expr, type_stack);
                let lhs = Self::compile_expr(
                    lhs,
                    builder,
                    scope_stack,
                    type_stack,
                    idx,
                    func_signatures,
                    module,
                    str_idx,
                )?;
                let rhs = Self::compile_expr(
                    rhs,
                    builder,
                    scope_stack,
                    type_stack,
                    idx,
                    func_signatures,
                    module,
                    str_idx,
                )?;

                if matches!(expr_type, Type::Float) {
                    match expr {
                        Expr::Eq(_, _) => {
                            Ok(builder.ins().fcmp(ir::condcodes::FloatCC::Equal, lhs, rhs))
                        }
                        Expr::Ne(_, _) => {
                            Ok(builder
                                .ins()
                                .fcmp(ir::condcodes::FloatCC::NotEqual, lhs, rhs))
                        }
                        Expr::Lt(_, _) => {
                            Ok(builder
                                .ins()
                                .fcmp(ir::condcodes::FloatCC::LessThan, lhs, rhs))
                        }
                        Expr::Le(_, _) => Ok(builder.ins().fcmp(
                            ir::condcodes::FloatCC::LessThanOrEqual,
                            lhs,
                            rhs,
                        )),
                        Expr::Gt(_, _) => {
                            Ok(builder
                                .ins()
                                .fcmp(ir::condcodes::FloatCC::GreaterThan, lhs, rhs))
                        }
                        Expr::Ge(_, _) => Ok(builder.ins().fcmp(
                            ir::condcodes::FloatCC::GreaterThanOrEqual,
                            lhs,
                            rhs,
                        )),
                        _ => unreachable!(),
                    }
                } else {
                    match expr {
                        Expr::Eq(_, _) => {
                            Ok(builder.ins().icmp(ir::condcodes::IntCC::Equal, lhs, rhs))
                        }
                        Expr::Ne(_, _) => {
                            Ok(builder.ins().icmp(ir::condcodes::IntCC::NotEqual, lhs, rhs))
                        }
                        Expr::Lt(_, _) => {
                            Ok(builder
                                .ins()
                                .icmp(ir::condcodes::IntCC::SignedLessThan, lhs, rhs))
                        }
                        Expr::Le(_, _) => Ok(builder.ins().icmp(
                            ir::condcodes::IntCC::SignedLessThanOrEqual,
                            lhs,
                            rhs,
                        )),
                        Expr::Gt(_, _) => Ok(builder.ins().icmp(
                            ir::condcodes::IntCC::SignedGreaterThan,
                            lhs,
                            rhs,
                        )),
                        Expr::Ge(_, _) => Ok(builder.ins().icmp(
                            ir::condcodes::IntCC::SignedGreaterThanOrEqual,
                            lhs,
                            rhs,
                        )),
                        _ => unreachable!(),
                    }
                }
            }
            Expr::And(lhs, rhs) => {
                let lhs_val = Self::compile_expr(
                    lhs,
                    builder,
                    scope_stack,
                    type_stack,
                    idx,
                    func_signatures,
                    module,
                    str_idx,
                )?;
                let rhs_val = Self::compile_expr(
                    rhs,
                    builder,
                    scope_stack,
                    type_stack,
                    idx,
                    func_signatures,
                    module,
                    str_idx,
                )?;
                Ok(builder.ins().band(lhs_val, rhs_val))
            }
            Expr::Or(lhs, rhs) => {
                let lhs_val = Self::compile_expr(
                    lhs,
                    builder,
                    scope_stack,
                    type_stack,
                    idx,
                    func_signatures,
                    module,
                    str_idx,
                )?;
                let rhs_val = Self::compile_expr(
                    rhs,
                    builder,
                    scope_stack,
                    type_stack,
                    idx,
                    func_signatures,
                    module,
                    str_idx,
                )?;
                Ok(builder.ins().bor(lhs_val, rhs_val))
            }
            Expr::Not(expr) => {
                let val = Self::compile_expr(
                    expr,
                    builder,
                    scope_stack,
                    type_stack,
                    idx,
                    func_signatures,
                    module,
                    str_idx,
                )?;
                let one = builder.ins().iconst(types::I8, 1);
                Ok(builder.ins().bxor(val, one))
            }
            Expr::VarDecl(name, ty, value) => {
                let val = Self::compile_expr(
                    &**value,
                    builder,
                    scope_stack,
                    type_stack,
                    idx,
                    func_signatures,
                    module,
                    str_idx,
                )?;
                let var = Variable::from_u32(*idx);
                *idx += 1;
                builder.declare_var(get_type(ty.clone()));
                builder.def_var(var, val);
                scope_stack.last_mut().unwrap().insert(name.clone(), var);
                type_stack
                    .last_mut()
                    .unwrap()
                    .insert(name.clone(), ty.clone());
                Ok(Value::from_u32(0))
            }

            Expr::VarAssign(name, val) => {
                let val = Self::compile_expr(
                    &**val,
                    builder,
                    scope_stack,
                    type_stack,
                    idx,
                    func_signatures,
                    module,
                    str_idx,
                )?;
                let var = Self::lookup_var(name, scope_stack)
                    .ok_or_else(|| CodeGenError::UndefinedVariable { name: name.clone() })?;
                builder.def_var(*var, val);
                Ok(val)
            }
            Expr::Var(name) => {
                let var = Self::lookup_var(name, scope_stack)
                    .ok_or_else(|| CodeGenError::UndefinedVariable { name: name.clone() })?;
                Ok(builder.use_var(*var))
            }
            Expr::Call(callee, args) => {
                let func_name = match &**callee {
                    Expr::Var(name) => name,
                    _ => {
                        return Err(CodeGenError::ModuleError(
                            "Only direct function calls are supported currently".to_string(),
                        ));
                    }
                };

                let (func_id, _) = match func_signatures.get(func_name) {
                    Some(sig) => sig,
                    None => {
                        return Err(CodeGenError::UndefinedFunction {
                            name: func_name.clone(),
                        });
                    }
                };

                let func_ref = module.declare_func_in_func(*func_id, builder.func);

                let arg_values: Result<Vec<Value>, CodeGenError> = args
                    .iter()
                    .map(|a| {
                        Self::compile_expr(
                            a,
                            builder,
                            scope_stack,
                            type_stack,
                            idx,
                            func_signatures,
                            module,
                            str_idx,
                        )
                    })
                    .collect();

                let arg_values = arg_values?;
                let call = builder.ins().call(func_ref, &arg_values);
                let results = builder.inst_results(call);
                if results.is_empty() {
                    Ok(builder.ins().iconst(types::I64, 0))
                } else {
                    Ok(results[0])
                }
            }
            Expr::Return(value) => {
                let val = Self::compile_expr(
                    &**value,
                    builder,
                    scope_stack,
                    type_stack,
                    idx,
                    func_signatures,
                    module,
                    str_idx,
                )?;
                builder.ins().return_(&[val]);
                Ok(Value::from_u32(0))
            }
            Expr::If(cond, then_branch, else_branch) => {
                let cond_val = Self::compile_expr(
                    cond,
                    builder,
                    scope_stack,
                    type_stack,
                    idx,
                    func_signatures,
                    module,
                    str_idx,
                )?;

                let then_has_return = Self::has_return(then_branch);
                let else_has_return = else_branch.as_ref().map_or(false, |e| Self::has_return(e));
                let both_return = then_has_return && else_has_return;

                let then_block = builder.create_block();
                let else_block = builder.create_block();
                let merge_block = if both_return {
                    None
                } else {
                    Some(builder.create_block())
                };

                let cond_i64 = builder.ins().uextend(types::I64, cond_val);
                builder
                    .ins()
                    .brif(cond_i64, then_block, &[], else_block, &[]);

                builder.switch_to_block(then_block);
                scope_stack.push(HashMap::new());
                type_stack.push(HashMap::new());
                Self::compile_expr(
                    then_branch,
                    builder,
                    scope_stack,
                    type_stack,
                    idx,
                    func_signatures,
                    module,
                    str_idx,
                )?;
                scope_stack.pop();
                type_stack.pop();

                if !then_has_return {
                    if let Some(merge) = merge_block {
                        builder.ins().jump(merge, &[]);
                    }
                }
                builder.seal_block(then_block);

                builder.switch_to_block(else_block);
                if let Some(else_expr) = else_branch {
                    scope_stack.push(HashMap::new());
                    type_stack.push(HashMap::new());
                    Self::compile_expr(
                        else_expr,
                        builder,
                        scope_stack,
                        type_stack,
                        idx,
                        func_signatures,
                        module,
                        str_idx,
                    )?;
                    scope_stack.pop();
                    type_stack.pop();

                    if !else_has_return {
                        if let Some(merge) = merge_block {
                            builder.ins().jump(merge, &[]);
                        }
                    }
                } else {
                    if let Some(merge) = merge_block {
                        builder.ins().jump(merge, &[]);
                    }
                }
                builder.seal_block(else_block);

                if let Some(merge) = merge_block {
                    builder.switch_to_block(merge);
                    builder.seal_block(merge);
                    Ok(builder.ins().iconst(types::I64, 0))
                } else {
                    Ok(Value::from_u32(0))
                }
            }
            Expr::While(cond, body) => {
                let loop_header = builder.create_block();
                let loop_body = builder.create_block();
                let loop_exit = builder.create_block();

                builder.ins().jump(loop_header, &[]);

                builder.switch_to_block(loop_header);
                let cond_val = Self::compile_expr(
                    cond,
                    builder,
                    scope_stack,
                    type_stack,
                    idx,
                    func_signatures,
                    module,
                    str_idx,
                )?;
                let cond_i64 = builder.ins().uextend(types::I64, cond_val);
                builder.ins().brif(cond_i64, loop_body, &[], loop_exit, &[]);

                builder.switch_to_block(loop_body);
                scope_stack.push(HashMap::new());
                type_stack.push(HashMap::new());
                Self::compile_expr(
                    body,
                    builder,
                    scope_stack,
                    type_stack,
                    idx,
                    func_signatures,
                    module,
                    str_idx,
                )?;
                scope_stack.pop();
                type_stack.pop();
                builder.ins().jump(loop_header, &[]);

                builder.seal_block(loop_body);
                builder.seal_block(loop_header);

                builder.switch_to_block(loop_exit);
                builder.seal_block(loop_exit);

                Ok(builder.ins().iconst(types::I64, 0))
            }
            Expr::Index(array, index) => {
                let array_ptr = Self::compile_expr(
                    array,
                    builder,
                    scope_stack,
                    type_stack,
                    idx,
                    func_signatures,
                    module,
                    str_idx,
                )?;
                let index_val = Self::compile_expr(
                    index,
                    builder,
                    scope_stack,
                    type_stack,
                    idx,
                    func_signatures,
                    module,
                    str_idx,
                )?;

                let offset = builder.ins().imul_imm(index_val, 8);
                let element_ptr = builder.ins().iadd(array_ptr, offset);

                let element =
                    builder
                        .ins()
                        .load(types::I64, ir::MemFlags::trusted(), element_ptr, 0);
                Ok(element)
            }
            Expr::IndexAssign(array_index, value) => {
                if let Expr::Index(array, index) = &**array_index {
                    let array_ptr = Self::compile_expr(
                        array,
                        builder,
                        scope_stack,
                        type_stack,
                        idx,
                        func_signatures,
                        module,
                        str_idx,
                    )?;
                    let index_val = Self::compile_expr(
                        index,
                        builder,
                        scope_stack,
                        type_stack,
                        idx,
                        func_signatures,
                        module,
                        str_idx,
                    )?;
                    let value_val = Self::compile_expr(
                        value,
                        builder,
                        scope_stack,
                        type_stack,
                        idx,
                        func_signatures,
                        module,
                        str_idx,
                    )?;

                    let offset = builder.ins().imul_imm(index_val, 8);
                    let element_ptr = builder.ins().iadd(array_ptr, offset);

                    builder
                        .ins()
                        .store(ir::MemFlags::trusted(), value_val, element_ptr, 0);
                    Ok(builder.ins().iconst(types::I64, 0))
                } else {
                    Err(CodeGenError::UnexpectedExpression {
                        found: (**array_index).clone(),
                    })
                }
            }
            Expr::ArrayLiteral(elements) => {
                let data_id = module
                    .declare_data(
                        &format!("array_{}", idx),
                        cranelift_module::Linkage::Local,
                        false,
                        false,
                    )
                    .map_err(|e| CodeGenError::ModuleError(e.to_string()))?;

                let mut data_desc = cranelift_module::DataDescription::new();
                let mut bytes = Vec::new();
                let mut str_idx = 0;
                for elem in elements {
                    match elem {
                        Expr::Int(n) => {
                            let val = *n as i64;
                            bytes.extend_from_slice(&val.to_le_bytes());
                        }
                        Expr::Float(f) => {
                            let val = f.to_bits();
                            bytes.extend_from_slice(&val.to_le_bytes());
                        }
                        Expr::Bool(b) => {
                            let val = *b as i64;
                            bytes.extend_from_slice(&val.to_le_bytes());
                        }
                        Expr::String(s) => {
                            let str_data_id = module
                                .declare_data(
                                    &format!("array_str_{}_{}", idx, str_idx),
                                    cranelift_module::Linkage::Local,
                                    false,
                                    false,
                                )
                                .map_err(|e| CodeGenError::ModuleError(e.to_string()))?;
                            let mut str_data_desc = cranelift_module::DataDescription::new();
                            let mut str_bytes = s.as_bytes().to_vec();
                            str_bytes.push(0);
                            str_data_desc.define(str_bytes.into());
                            module
                                .define_data(str_data_id, &str_data_desc)
                                .map_err(|e| CodeGenError::ModuleError(e.to_string()))?;

                            bytes.extend_from_slice(&[0u8; 8]);
                            str_idx += 1;
                        }
                        Expr::Nil => {
                            bytes.extend_from_slice(&[0u8; 8]);
                        }
                        _ => {
                            return Err(CodeGenError::ModuleError(
                                "Array literals currently only support int, float, bool, string, and nil constants".to_string(),
                            ));
                        }
                    }
                }
                data_desc.define(bytes.into());
                module
                    .define_data(data_id, &data_desc)
                    .map_err(|e| CodeGenError::ModuleError(e.to_string()))?;

                let global_value = module.declare_data_in_func(data_id, builder.func);
                let ptr = builder.ins().global_value(types::I64, global_value);
                Ok(ptr)
            }
            Expr::ArrayFill(elem_type, length) => {
                let length_val = Self::compile_expr(
                    length,
                    builder,
                    scope_stack,
                    type_stack,
                    idx,
                    func_signatures,
                    module,
                    str_idx,
                )?;

                let elem_size = match elem_type {
                    Type::Int | Type::Float | Type::String => 8i64,
                    Type::Bool => 1i64,
                    Type::Void => 0i64,
                    Type::Array(_) => 8i64,
                };

                let total_size = builder.ins().imul_imm(length_val, elem_size);

                let malloc_sig = {
                    let mut sig = module.make_signature();
                    sig.call_conv = CallConv::SystemV;
                    sig.params.push(AbiParam::new(types::I64));
                    sig.returns.push(AbiParam::new(types::I64));
                    sig
                };
                let malloc_id = module
                    .declare_function("malloc", Linkage::Import, &malloc_sig)
                    .map_err(|e| CodeGenError::ModuleError(e.to_string()))?;
                let malloc_ref = module.declare_func_in_func(malloc_id, builder.func);
                let mem_ptr = builder.ins().call(malloc_ref, &[total_size]);
                let mem_ptr = builder.inst_results(mem_ptr)[0];

                let memset_sig = {
                    let mut sig = module.make_signature();
                    sig.call_conv = CallConv::SystemV;
                    sig.params.push(AbiParam::new(types::I64));
                    sig.params.push(AbiParam::new(types::I32));
                    sig.params.push(AbiParam::new(types::I64));
                    sig.returns.push(AbiParam::new(types::I64));
                    sig
                };
                let memset_id = module
                    .declare_function("memset", Linkage::Import, &memset_sig)
                    .map_err(|e| CodeGenError::ModuleError(e.to_string()))?;
                let memset_ref = module.declare_func_in_func(memset_id, builder.func);
                let zero_val = builder.ins().iconst(types::I32, 0);
                builder
                    .ins()
                    .call(memset_ref, &[mem_ptr, zero_val, total_size]);

                Ok(mem_ptr)
            }
            Expr::For(var, start, end, body) => {
                let loop_header = builder.create_block();
                let loop_body = builder.create_block();
                let loop_exit = builder.create_block();

                let start_val = Self::compile_expr(
                    start,
                    builder,
                    scope_stack,
                    type_stack,
                    idx,
                    func_signatures,
                    module,
                    str_idx,
                )?;
                let end_val = Self::compile_expr(
                    end,
                    builder,
                    scope_stack,
                    type_stack,
                    idx,
                    func_signatures,
                    module,
                    str_idx,
                )?;

                let loop_var = Variable::from_u32(*idx);
                *idx += 1;
                builder.declare_var(types::I64);
                builder.def_var(loop_var, start_val);

                builder.ins().jump(loop_header, &[]);

                builder.switch_to_block(loop_header);
                let current_val = builder.use_var(loop_var);

                let cmp = builder.ins().icmp(
                    ir::condcodes::IntCC::UnsignedLessThan,
                    current_val,
                    end_val,
                );
                let cmp_i64 = builder.ins().uextend(types::I64, cmp);
                builder.ins().brif(cmp_i64, loop_body, &[], loop_exit, &[]);

                builder.switch_to_block(loop_body);
                scope_stack.push(HashMap::new());
                type_stack.push(HashMap::new());
                scope_stack
                    .last_mut()
                    .unwrap()
                    .insert(var.clone(), loop_var);

                Self::compile_expr(
                    body,
                    builder,
                    scope_stack,
                    type_stack,
                    idx,
                    func_signatures,
                    module,
                    str_idx,
                )?;

                scope_stack.pop();
                type_stack.pop();

                let next_val = builder.ins().iadd_imm(current_val, 1);
                builder.def_var(loop_var, next_val);
                builder.ins().jump(loop_header, &[]);

                builder.seal_block(loop_body);
                builder.seal_block(loop_header);

                builder.switch_to_block(loop_exit);
                builder.seal_block(loop_exit);

                Ok(builder.ins().iconst(types::I64, 0))
            }
            Expr::Break => Ok(builder.ins().iconst(types::I64, 0)),
            Expr::Continue => Ok(builder.ins().iconst(types::I64, 0)),
            _ => Err(CodeGenError::UnexpectedExpression {
                found: expr.clone(),
            }),
        }
    }
}

fn get_type(t: Type) -> ir::Type {
    return match t {
        Type::Int => types::I64,
        Type::Float => types::F64,
        Type::Bool => types::I8,
        Type::String => types::I64,
        Type::Void => types::INVALID,
        Type::Array(_) => types::I64,
    };
}
