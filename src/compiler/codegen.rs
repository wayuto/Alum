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

#[derive(Clone)]
struct LoopContext {
    header_block: ir::Block,
    exit_block: ir::Block,
    increment_block: Option<ir::Block>,
}

#[derive(Clone, Debug)]
struct StructField {
    name: String,
    ty: Type,
    offset: i64,
}

#[derive(Clone, Debug)]
struct StructDef {
    fields: Vec<StructField>,
    size: i64,
    #[allow(dead_code)]
    align: i64,
}

pub struct CodeGen {
    ast: Program,
    module: ObjectModule,
    builder_context: FunctionBuilderContext,
    func_signatures: HashMap<String, (FuncId, Signature)>,
    loop_stack: Vec<LoopContext>,
    type_map: HashMap<String, ir::Type>,
    structs: HashMap<String, StructDef>,
}

impl CodeGen {
    fn get_type_size(
        ty: &Type,
        _type_map: &HashMap<String, ir::Type>,
        structs: &HashMap<String, StructDef>,
    ) -> i64 {
        match ty {
            Type::Named(name) => match name.as_str() {
                "int" | "float" => 8,
                "string" => 8,
                "bool" => 1,
                "void" => 0,
                _ => {
                    if let Some(struct_def) = structs.get(name) {
                        struct_def.size
                    } else {
                        8
                    }
                }
            },
            Type::Array(_) => 8,
        }
    }

    fn get_type_align(
        ty: &Type,
        type_map: &HashMap<String, ir::Type>,
        structs: &HashMap<String, StructDef>,
    ) -> i64 {
        match ty {
            Type::Named(name) => match name.as_str() {
                "int" | "float" => 8,
                "string" => 8,
                "bool" => 1,
                "void" => 1,
                _ => {
                    if let Some(struct_def) = structs.get(name) {
                        struct_def
                            .fields
                            .iter()
                            .map(|f| Self::get_type_align(&f.ty, type_map, structs))
                            .max()
                            .unwrap_or(1)
                    } else {
                        8
                    }
                }
            },
            Type::Array(_) => 8,
        }
    }

    fn get_expr_type(expr: &Expr, type_stack: &Vec<HashMap<String, Type>>) -> Type {
        match expr {
            Expr::Int(_) => Type::Named("int".to_string()),
            Expr::Float(_) => Type::Named("float".to_string()),
            Expr::Bool(_) => Type::Named("bool".to_string()),
            Expr::String(_) => Type::Named("string".to_string()),
            Expr::Nil => Type::Named("void".to_string()),
            Expr::Var(name) => {
                for scope in type_stack.iter().rev() {
                    if let Some(ty) = scope.get(name) {
                        return ty.clone();
                    }
                }
                Type::Named("int".to_string())
            }
            _ => Type::Named("int".to_string()),
        }
    }

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
        let mut type_map = HashMap::new();
        type_map.insert("int".to_string(), types::I64);
        type_map.insert("float".to_string(), types::F64);
        type_map.insert("bool".to_string(), types::I8);
        type_map.insert("string".to_string(), types::I64);
        type_map.insert("void".to_string(), types::INVALID);
        Self {
            ast,
            module,
            builder_context: FunctionBuilderContext::new(),
            func_signatures: HashMap::new(),
            loop_stack: Vec::new(),
            type_map,
            structs: HashMap::new(),
        }
    }

    pub fn generate(mut self) -> Result<Vec<u8>, CodeGenError> {
        for expr in self.ast.body.clone() {
            match expr {
                Expr::FuncDecl(name, params, ret_type, _body) => {
                    let mut sig = self.module.make_signature();
                    sig.call_conv = CallConv::SystemV;
                    for (_, t) in params {
                        sig.params.push(AbiParam::new(get_type(&t, &self.type_map)));
                    }

                    if !matches!(ret_type, Type::Named(ref n) if n == "void") {
                        sig.returns
                            .push(AbiParam::new(get_type(&ret_type, &self.type_map)));
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
                        sig.params.push(AbiParam::new(get_type(&t, &self.type_map)));
                    }

                    if !matches!(ret_type, Type::Named(ref n) if n == "void") {
                        sig.returns
                            .push(AbiParam::new(get_type(&ret_type, &self.type_map)));
                    }
                    let func_id = self
                        .module
                        .declare_function(name.as_str(), Linkage::Import, &sig)
                        .unwrap();
                    self.func_signatures.insert(name, (func_id, sig));
                }
                Expr::TypeDef => {}
                Expr::Struct(name, fields) => {
                    let mut offset = 0i64;
                    let mut max_align = 1i64;
                    let mut struct_fields = Vec::new();

                    for (field_name, field_ty) in fields {
                        let field_align =
                            Self::get_type_align(&field_ty, &self.type_map, &self.structs);
                        let field_size =
                            Self::get_type_size(&field_ty, &self.type_map, &self.structs);

                        if offset % field_align != 0 {
                            offset = ((offset / field_align) + 1) * field_align;
                        }

                        struct_fields.push(StructField {
                            name: field_name,
                            ty: field_ty,
                            offset,
                        });

                        offset += field_size;
                        max_align = max_align.max(field_align);
                    }

                    let size = if offset % max_align != 0 {
                        ((offset / max_align) + 1) * max_align
                    } else {
                        offset
                    };

                    self.structs.insert(
                        name,
                        StructDef {
                            fields: struct_fields,
                            size,
                            align: max_align,
                        },
                    );
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
                Expr::TypeDef => {}
                Expr::Struct(_, _) => {}
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

        let param_types: Vec<ir::Type> = params
            .iter()
            .map(|(_, ty)| get_type(ty, &self.type_map))
            .collect();

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
            &mut self.loop_stack,
            &self.type_map,
            &self.structs,
        )?;

        if matches!(_ret_type, Type::Named(ref n) if n == "void") {
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
        loop_stack: &mut Vec<LoopContext>,
        type_map: &HashMap<String, ir::Type>,
        structs: &HashMap<String, StructDef>,
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
                            loop_stack,
                            type_map,
                            structs,
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
                        loop_stack,
                        type_map,
                        structs,
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
                        loop_stack,
                        type_map,
                        structs,
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
                        true,
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

            Expr::Add(lhs, rhs) => {
                let lhs_type = Self::get_expr_type(lhs, type_stack);
                let rhs_type = Self::get_expr_type(rhs, type_stack);
                let is_string = matches!(lhs_type, Type::Named(ref n) if n == "string")
                    || matches!(rhs_type, Type::Named(ref n) if n == "string");

                if is_string {
                    let lhs_ptr = Self::compile_expr(
                        lhs,
                        builder,
                        scope_stack,
                        type_stack,
                        idx,
                        func_signatures,
                        module,
                        str_idx,
                        loop_stack,
                        type_map,
                        structs,
                    )?;
                    let rhs_ptr = Self::compile_expr(
                        rhs,
                        builder,
                        scope_stack,
                        type_stack,
                        idx,
                        func_signatures,
                        module,
                        str_idx,
                        loop_stack,
                        type_map,
                        structs,
                    )?;

                    let strlen_sig = {
                        let mut sig = module.make_signature();
                        sig.call_conv = CallConv::SystemV;
                        sig.params.push(AbiParam::new(types::I64));
                        sig.returns.push(AbiParam::new(types::I64));
                        sig
                    };
                    let strlen_id = module
                        .declare_function("strlen", Linkage::Import, &strlen_sig)
                        .map_err(|e| CodeGenError::ModuleError(e.to_string()))?;
                    let strlen_ref = module.declare_func_in_func(strlen_id, builder.func);

                    let lhs_len = builder.ins().call(strlen_ref, &[lhs_ptr]);
                    let lhs_len = builder.inst_results(lhs_len)[0];
                    let rhs_len = builder.ins().call(strlen_ref, &[rhs_ptr]);
                    let rhs_len = builder.inst_results(rhs_len)[0];

                    let total_len = builder.ins().iadd(lhs_len, rhs_len);
                    let alloc_size = builder.ins().iadd_imm(total_len, 1);

                    let calloc_sig = {
                        let mut sig = module.make_signature();
                        sig.call_conv = CallConv::SystemV;
                        sig.params.push(AbiParam::new(types::I64));
                        sig.returns.push(AbiParam::new(types::I64));
                        sig
                    };
                    let calloc_id = module
                        .declare_function("malloc", Linkage::Import, &calloc_sig)
                        .map_err(|e| CodeGenError::ModuleError(e.to_string()))?;
                    let calloc_ref = module.declare_func_in_func(calloc_id, builder.func);
                    let result_ptr = builder.ins().call(calloc_ref, &[alloc_size]);
                    let result_ptr = builder.inst_results(result_ptr)[0];

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
                    let zero_i32 = builder.ins().iconst(types::I32, 0);
                    builder
                        .ins()
                        .call(memset_ref, &[result_ptr, zero_i32, alloc_size]);

                    let strcpy_sig = {
                        let mut sig = module.make_signature();
                        sig.call_conv = CallConv::SystemV;
                        sig.params.push(AbiParam::new(types::I64));
                        sig.params.push(AbiParam::new(types::I64));
                        sig.returns.push(AbiParam::new(types::I64));
                        sig
                    };
                    let strcpy_id = module
                        .declare_function("strcpy", Linkage::Import, &strcpy_sig)
                        .map_err(|e| CodeGenError::ModuleError(e.to_string()))?;
                    let strcpy_ref = module.declare_func_in_func(strcpy_id, builder.func);
                    builder.ins().call(strcpy_ref, &[result_ptr, lhs_ptr]);

                    let strcat_sig = {
                        let mut sig = module.make_signature();
                        sig.call_conv = CallConv::SystemV;
                        sig.params.push(AbiParam::new(types::I64));
                        sig.params.push(AbiParam::new(types::I64));
                        sig.returns.push(AbiParam::new(types::I64));
                        sig
                    };
                    let strcat_id = module
                        .declare_function("strcat", Linkage::Import, &strcat_sig)
                        .map_err(|e| CodeGenError::ModuleError(e.to_string()))?;
                    let strcat_ref = module.declare_func_in_func(strcat_id, builder.func);
                    builder.ins().call(strcat_ref, &[result_ptr, rhs_ptr]);

                    Ok(result_ptr)
                } else {
                    let lhs = Self::compile_expr(
                        lhs,
                        builder,
                        scope_stack,
                        type_stack,
                        idx,
                        func_signatures,
                        module,
                        str_idx,
                        loop_stack,
                        type_map,
                        structs,
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
                        loop_stack,
                        type_map,
                        structs,
                    )?;
                    Ok(builder.ins().iadd(lhs, rhs))
                }
            }
            Expr::Sub(lhs, rhs) => {
                let lhs = Self::compile_expr(
                    lhs,
                    builder,
                    scope_stack,
                    type_stack,
                    idx,
                    func_signatures,
                    module,
                    str_idx,
                    loop_stack,
                    type_map,
                    structs,
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
                    loop_stack,
                    type_map,
                    structs,
                )?;
                Ok(builder.ins().isub(lhs, rhs))
            }
            Expr::Mul(lhs, rhs) => {
                let lhs = Self::compile_expr(
                    lhs,
                    builder,
                    scope_stack,
                    type_stack,
                    idx,
                    func_signatures,
                    module,
                    str_idx,
                    loop_stack,
                    type_map,
                    structs,
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
                    loop_stack,
                    type_map,
                    structs,
                )?;
                Ok(builder.ins().imul(lhs, rhs))
            }
            Expr::Div(lhs, rhs) => {
                let lhs = Self::compile_expr(
                    lhs,
                    builder,
                    scope_stack,
                    type_stack,
                    idx,
                    func_signatures,
                    module,
                    str_idx,
                    loop_stack,
                    type_map,
                    structs,
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
                    loop_stack,
                    type_map,
                    structs,
                )?;
                Ok(builder.ins().sdiv(lhs, rhs))
            }
            Expr::Mod(lhs, rhs) => {
                let lhs = Self::compile_expr(
                    lhs,
                    builder,
                    scope_stack,
                    type_stack,
                    idx,
                    func_signatures,
                    module,
                    str_idx,
                    loop_stack,
                    type_map,
                    structs,
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
                    loop_stack,
                    type_map,
                    structs,
                )?;
                Ok(builder.ins().srem(lhs, rhs))
            }

            Expr::FAdd(lhs, rhs) => {
                let lhs = Self::compile_expr(
                    lhs,
                    builder,
                    scope_stack,
                    type_stack,
                    idx,
                    func_signatures,
                    module,
                    str_idx,
                    loop_stack,
                    type_map,
                    structs,
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
                    loop_stack,
                    type_map,
                    structs,
                )?;
                Ok(builder.ins().fadd(lhs, rhs))
            }
            Expr::FSub(lhs, rhs) => {
                let lhs = Self::compile_expr(
                    lhs,
                    builder,
                    scope_stack,
                    type_stack,
                    idx,
                    func_signatures,
                    module,
                    str_idx,
                    loop_stack,
                    type_map,
                    structs,
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
                    loop_stack,
                    type_map,
                    structs,
                )?;
                Ok(builder.ins().fsub(lhs, rhs))
            }
            Expr::FMul(lhs, rhs) => {
                let lhs = Self::compile_expr(
                    lhs,
                    builder,
                    scope_stack,
                    type_stack,
                    idx,
                    func_signatures,
                    module,
                    str_idx,
                    loop_stack,
                    type_map,
                    structs,
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
                    loop_stack,
                    type_map,
                    structs,
                )?;
                Ok(builder.ins().fmul(lhs, rhs))
            }
            Expr::FDiv(lhs, rhs) => {
                let lhs = Self::compile_expr(
                    lhs,
                    builder,
                    scope_stack,
                    type_stack,
                    idx,
                    func_signatures,
                    module,
                    str_idx,
                    loop_stack,
                    type_map,
                    structs,
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
                    loop_stack,
                    type_map,
                    structs,
                )?;
                Ok(builder.ins().fdiv(lhs, rhs))
            }

            Expr::Eq(lhs, rhs) => {
                let lhs = Self::compile_expr(
                    lhs,
                    builder,
                    scope_stack,
                    type_stack,
                    idx,
                    func_signatures,
                    module,
                    str_idx,
                    loop_stack,
                    type_map,
                    structs,
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
                    loop_stack,
                    type_map,
                    structs,
                )?;
                Ok(builder.ins().icmp(ir::condcodes::IntCC::Equal, lhs, rhs))
            }
            Expr::Ne(lhs, rhs) => {
                let lhs = Self::compile_expr(
                    lhs,
                    builder,
                    scope_stack,
                    type_stack,
                    idx,
                    func_signatures,
                    module,
                    str_idx,
                    loop_stack,
                    type_map,
                    structs,
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
                    loop_stack,
                    type_map,
                    structs,
                )?;
                Ok(builder.ins().icmp(ir::condcodes::IntCC::NotEqual, lhs, rhs))
            }
            Expr::Lt(lhs, rhs) => {
                let lhs = Self::compile_expr(
                    lhs,
                    builder,
                    scope_stack,
                    type_stack,
                    idx,
                    func_signatures,
                    module,
                    str_idx,
                    loop_stack,
                    type_map,
                    structs,
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
                    loop_stack,
                    type_map,
                    structs,
                )?;
                Ok(builder
                    .ins()
                    .icmp(ir::condcodes::IntCC::SignedLessThan, lhs, rhs))
            }
            Expr::Le(lhs, rhs) => {
                let lhs = Self::compile_expr(
                    lhs,
                    builder,
                    scope_stack,
                    type_stack,
                    idx,
                    func_signatures,
                    module,
                    str_idx,
                    loop_stack,
                    type_map,
                    structs,
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
                    loop_stack,
                    type_map,
                    structs,
                )?;
                Ok(builder
                    .ins()
                    .icmp(ir::condcodes::IntCC::SignedLessThanOrEqual, lhs, rhs))
            }
            Expr::Gt(lhs, rhs) => {
                let lhs = Self::compile_expr(
                    lhs,
                    builder,
                    scope_stack,
                    type_stack,
                    idx,
                    func_signatures,
                    module,
                    str_idx,
                    loop_stack,
                    type_map,
                    structs,
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
                    loop_stack,
                    type_map,
                    structs,
                )?;
                Ok(builder
                    .ins()
                    .icmp(ir::condcodes::IntCC::SignedGreaterThan, lhs, rhs))
            }
            Expr::Ge(lhs, rhs) => {
                let lhs = Self::compile_expr(
                    lhs,
                    builder,
                    scope_stack,
                    type_stack,
                    idx,
                    func_signatures,
                    module,
                    str_idx,
                    loop_stack,
                    type_map,
                    structs,
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
                    loop_stack,
                    type_map,
                    structs,
                )?;
                Ok(builder
                    .ins()
                    .icmp(ir::condcodes::IntCC::SignedGreaterThanOrEqual, lhs, rhs))
            }

            Expr::FEq(lhs, rhs) => {
                let lhs = Self::compile_expr(
                    lhs,
                    builder,
                    scope_stack,
                    type_stack,
                    idx,
                    func_signatures,
                    module,
                    str_idx,
                    loop_stack,
                    type_map,
                    structs,
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
                    loop_stack,
                    type_map,
                    structs,
                )?;
                Ok(builder.ins().fcmp(ir::condcodes::FloatCC::Equal, lhs, rhs))
            }
            Expr::FNe(lhs, rhs) => {
                let lhs = Self::compile_expr(
                    lhs,
                    builder,
                    scope_stack,
                    type_stack,
                    idx,
                    func_signatures,
                    module,
                    str_idx,
                    loop_stack,
                    type_map,
                    structs,
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
                    loop_stack,
                    type_map,
                    structs,
                )?;
                Ok(builder
                    .ins()
                    .fcmp(ir::condcodes::FloatCC::NotEqual, lhs, rhs))
            }
            Expr::FLt(lhs, rhs) => {
                let lhs = Self::compile_expr(
                    lhs,
                    builder,
                    scope_stack,
                    type_stack,
                    idx,
                    func_signatures,
                    module,
                    str_idx,
                    loop_stack,
                    type_map,
                    structs,
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
                    loop_stack,
                    type_map,
                    structs,
                )?;
                Ok(builder
                    .ins()
                    .fcmp(ir::condcodes::FloatCC::LessThan, lhs, rhs))
            }
            Expr::FLe(lhs, rhs) => {
                let lhs = Self::compile_expr(
                    lhs,
                    builder,
                    scope_stack,
                    type_stack,
                    idx,
                    func_signatures,
                    module,
                    str_idx,
                    loop_stack,
                    type_map,
                    structs,
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
                    loop_stack,
                    type_map,
                    structs,
                )?;
                Ok(builder
                    .ins()
                    .fcmp(ir::condcodes::FloatCC::LessThanOrEqual, lhs, rhs))
            }
            Expr::FGt(lhs, rhs) => {
                let lhs = Self::compile_expr(
                    lhs,
                    builder,
                    scope_stack,
                    type_stack,
                    idx,
                    func_signatures,
                    module,
                    str_idx,
                    loop_stack,
                    type_map,
                    structs,
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
                    loop_stack,
                    type_map,
                    structs,
                )?;
                Ok(builder
                    .ins()
                    .fcmp(ir::condcodes::FloatCC::GreaterThan, lhs, rhs))
            }
            Expr::FGe(lhs, rhs) => {
                let lhs = Self::compile_expr(
                    lhs,
                    builder,
                    scope_stack,
                    type_stack,
                    idx,
                    func_signatures,
                    module,
                    str_idx,
                    loop_stack,
                    type_map,
                    structs,
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
                    loop_stack,
                    type_map,
                    structs,
                )?;
                Ok(builder
                    .ins()
                    .fcmp(ir::condcodes::FloatCC::GreaterThanOrEqual, lhs, rhs))
            }

            Expr::StrConcat(lhs, rhs) => {
                let lhs_ptr = Self::compile_expr(
                    lhs,
                    builder,
                    scope_stack,
                    type_stack,
                    idx,
                    func_signatures,
                    module,
                    str_idx,
                    loop_stack,
                    type_map,
                    structs,
                )?;
                let rhs_ptr = Self::compile_expr(
                    rhs,
                    builder,
                    scope_stack,
                    type_stack,
                    idx,
                    func_signatures,
                    module,
                    str_idx,
                    loop_stack,
                    type_map,
                    structs,
                )?;

                let strlen_sig = {
                    let mut sig = module.make_signature();
                    sig.call_conv = CallConv::SystemV;
                    sig.params.push(AbiParam::new(types::I64));
                    sig.returns.push(AbiParam::new(types::I64));
                    sig
                };
                let strlen_id = module
                    .declare_function("strlen", Linkage::Import, &strlen_sig)
                    .map_err(|e| CodeGenError::ModuleError(e.to_string()))?;
                let strlen_ref = module.declare_func_in_func(strlen_id, builder.func);

                let lhs_len = builder.ins().call(strlen_ref, &[lhs_ptr]);
                let lhs_len = builder.inst_results(lhs_len)[0];
                let rhs_len = builder.ins().call(strlen_ref, &[rhs_ptr]);
                let rhs_len = builder.inst_results(rhs_len)[0];

                let total_len = builder.ins().iadd(lhs_len, rhs_len);
                let alloc_size = builder.ins().iadd_imm(total_len, 1);

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
                let result_ptr = builder.ins().call(malloc_ref, &[alloc_size]);
                let result_ptr = builder.inst_results(result_ptr)[0];

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
                let zero_i32 = builder.ins().iconst(types::I32, 0);
                builder
                    .ins()
                    .call(memset_ref, &[result_ptr, zero_i32, alloc_size]);

                let strcpy_sig = {
                    let mut sig = module.make_signature();
                    sig.call_conv = CallConv::SystemV;
                    sig.params.push(AbiParam::new(types::I64));
                    sig.params.push(AbiParam::new(types::I64));
                    sig.returns.push(AbiParam::new(types::I64));
                    sig
                };
                let strcpy_id = module
                    .declare_function("strcpy", Linkage::Import, &strcpy_sig)
                    .map_err(|e| CodeGenError::ModuleError(e.to_string()))?;
                let strcpy_ref = module.declare_func_in_func(strcpy_id, builder.func);
                builder.ins().call(strcpy_ref, &[result_ptr, lhs_ptr]);

                let strcat_sig = {
                    let mut sig = module.make_signature();
                    sig.call_conv = CallConv::SystemV;
                    sig.params.push(AbiParam::new(types::I64));
                    sig.params.push(AbiParam::new(types::I64));
                    sig.returns.push(AbiParam::new(types::I64));
                    sig
                };
                let strcat_id = module
                    .declare_function("strcat", Linkage::Import, &strcat_sig)
                    .map_err(|e| CodeGenError::ModuleError(e.to_string()))?;
                let strcat_ref = module.declare_func_in_func(strcat_id, builder.func);
                builder.ins().call(strcat_ref, &[result_ptr, rhs_ptr]);

                Ok(result_ptr)
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
                    loop_stack,
                    type_map,
                    structs,
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
                    loop_stack,
                    type_map,
                    structs,
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
                    loop_stack,
                    type_map,
                    structs,
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
                    loop_stack,
                    type_map,
                    structs,
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
                    loop_stack,
                    type_map,
                    structs,
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
                    loop_stack,
                    type_map,
                    structs,
                )?;
                let var = Variable::from_u32(*idx);
                *idx += 1;
                builder.declare_var(get_type(ty, type_map));
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
                    loop_stack,
                    type_map,
                    structs,
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
                            loop_stack,
                            type_map,
                            structs,
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
                    loop_stack,
                    type_map,
                    structs,
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
                    loop_stack,
                    type_map,
                    structs,
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
                    loop_stack,
                    type_map,
                    structs,
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
                        loop_stack,
                        type_map,
                        structs,
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

                loop_stack.push(LoopContext {
                    header_block: loop_header,
                    exit_block: loop_exit,
                    increment_block: None,
                });

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
                    loop_stack,
                    type_map,
                    structs,
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
                    loop_stack,
                    type_map,
                    structs,
                )?;
                scope_stack.pop();
                type_stack.pop();
                loop_stack.pop();
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
                    loop_stack,
                    type_map,
                    structs,
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
                    loop_stack,
                    type_map,
                    structs,
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
                        loop_stack,
                        type_map,
                        structs,
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
                        loop_stack,
                        type_map,
                        structs,
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
                        loop_stack,
                        type_map,
                        structs,
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
                        true,
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
                                    true,
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
                    loop_stack,
                    type_map,
                    structs,
                )?;

                let elem_size = match &elem_type {
                    Type::Named(n) if matches!(n.as_str(), "int" | "float" | "string") => 8i64,
                    Type::Named(n) if n == "bool" => 1i64,
                    Type::Named(n) if n == "void" => 0i64,
                    Type::Array(_) => 8i64,
                    _ => 8i64,
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
                let loop_increment = builder.create_block();
                let loop_exit = builder.create_block();

                loop_stack.push(LoopContext {
                    header_block: loop_header,
                    exit_block: loop_exit,
                    increment_block: Some(loop_increment),
                });

                let start_val = Self::compile_expr(
                    start,
                    builder,
                    scope_stack,
                    type_stack,
                    idx,
                    func_signatures,
                    module,
                    str_idx,
                    loop_stack,
                    type_map,
                    structs,
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
                    loop_stack,
                    type_map,
                    structs,
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
                    loop_stack,
                    type_map,
                    structs,
                )?;

                scope_stack.pop();
                type_stack.pop();

                builder.ins().jump(loop_increment, &[]);

                builder.switch_to_block(loop_increment);
                let current_val_inc = builder.use_var(loop_var);
                let next_val = builder.ins().iadd_imm(current_val_inc, 1);
                builder.def_var(loop_var, next_val);
                builder.ins().jump(loop_header, &[]);

                loop_stack.pop();

                builder.seal_block(loop_increment);
                builder.seal_block(loop_body);
                builder.seal_block(loop_header);

                builder.switch_to_block(loop_exit);
                builder.seal_block(loop_exit);

                Ok(builder.ins().iconst(types::I64, 0))
            }
            Expr::Break => {
                if let Some(loop_ctx) = loop_stack.last() {
                    builder.ins().jump(loop_ctx.exit_block, &[]);
                    let unreachable_block = builder.create_block();
                    builder.switch_to_block(unreachable_block);
                    Ok(Value::from_u32(0))
                } else {
                    Err(CodeGenError::ModuleError(
                        "break statement outside of loop".to_string(),
                    ))
                }
            }
            Expr::Continue => {
                if let Some(loop_ctx) = loop_stack.last() {
                    if let Some(inc_block) = loop_ctx.increment_block {
                        builder.ins().jump(inc_block, &[]);
                    } else {
                        builder.ins().jump(loop_ctx.header_block, &[]);
                    }
                    let unreachable_block = builder.create_block();
                    builder.switch_to_block(unreachable_block);
                    Ok(Value::from_u32(0))
                } else {
                    Err(CodeGenError::ModuleError(
                        "continue statement outside of loop".to_string(),
                    ))
                }
            }
            Expr::TypeDef => Ok(Value::from_u32(0)),
            Expr::Struct(name, fields) => {
                let _ = (name, fields);
                Ok(Value::from_u32(0))
            }
            Expr::StructLiteral(name, field_values) => {
                let struct_def = structs.get(name).ok_or_else(|| {
                    CodeGenError::ModuleError(format!("Undefined struct type: {}", name))
                })?;

                let struct_size = struct_def.size;

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
                let size_val = builder.ins().iconst(types::I64, struct_size);
                let mem_ptr = builder.ins().call(malloc_ref, &[size_val]);
                let mem_ptr = builder.inst_results(mem_ptr)[0];

                for (field_name, field_expr) in field_values {
                    let field_info = struct_def.fields.iter().find(|f| &f.name == field_name);
                    if let Some(field) = field_info {
                        let field_val = Self::compile_expr(
                            field_expr,
                            builder,
                            scope_stack,
                            type_stack,
                            idx,
                            func_signatures,
                            module,
                            str_idx,
                            loop_stack,
                            type_map,
                            structs,
                        )?;

                        let field_ptr = builder.ins().iadd_imm(mem_ptr, field.offset);

                        let field_ir_type = get_type(&field.ty, type_map);
                        if field_ir_type == types::I8 {
                            builder
                                .ins()
                                .store(ir::MemFlags::trusted(), field_val, field_ptr, 0);
                        } else {
                            builder
                                .ins()
                                .store(ir::MemFlags::trusted(), field_val, field_ptr, 0);
                        }
                    }
                }

                Ok(mem_ptr)
            }
            Expr::MemberAccess(obj, field_name) => {
                let obj_ptr = Self::compile_expr(
                    obj,
                    builder,
                    scope_stack,
                    type_stack,
                    idx,
                    func_signatures,
                    module,
                    str_idx,
                    loop_stack,
                    type_map,
                    structs,
                )?;

                let struct_name = match &**obj {
                    Expr::Var(name) => {
                        let mut found_type = None;
                        for scope in type_stack.iter().rev() {
                            if let Some(ty) = scope.get(name) {
                                found_type = Some(ty.clone());
                                break;
                            }
                        }
                        match found_type {
                            Some(Type::Named(name)) => name,
                            _ => {
                                return Err(CodeGenError::ModuleError(
                                    "Member access on non-struct type".to_string(),
                                ));
                            }
                        }
                    }
                    _ => {
                        return Err(CodeGenError::ModuleError(
                            "Member access only supported on variables".to_string(),
                        ));
                    }
                };

                let struct_def = structs.get(&struct_name).ok_or_else(|| {
                    CodeGenError::ModuleError(format!("Undefined struct type: {}", struct_name))
                })?;

                let field_info = struct_def.fields.iter().find(|f| &f.name == field_name);
                if let Some(field) = field_info {
                    let field_ptr = builder.ins().iadd_imm(obj_ptr, field.offset);
                    let field_val =
                        builder
                            .ins()
                            .load(types::I64, ir::MemFlags::trusted(), field_ptr, 0);
                    Ok(field_val)
                } else {
                    Err(CodeGenError::ModuleError(format!(
                        "Struct {} has no field {}",
                        struct_name, field_name
                    )))
                }
            }
            _ => Err(CodeGenError::UnexpectedExpression {
                found: expr.clone(),
            }),
        }
    }
}

fn get_type(t: &Type, type_map: &HashMap<String, ir::Type>) -> ir::Type {
    return match t {
        Type::Named(name) => *type_map.get(name).unwrap_or(&types::I64),
        Type::Array(_) => types::I64,
    };
}
