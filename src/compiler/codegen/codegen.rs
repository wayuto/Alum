use super::types::{CodeGenError, LoopContext, Slot, StructDef, StructField, get_type};
use crate::compiler::Span;
use crate::compiler::parser::{Expr, Program, Type};
use cranelift::{
    codegen::{ir, settings},
    prelude::{
        AbiParam, Configurable, FunctionBuilder, FunctionBuilderContext, InstBuilder, Signature,
        Value,
        isa::{self, CallConv},
        types,
    },
};
use cranelift_module::{FuncId, Linkage, Module};
use cranelift_object::{ObjectBuilder, ObjectModule};
use std::collections::{HashMap, HashSet};

pub struct CodeGen {
    pub ast: Program,
    pub module: ObjectModule,
    pub builder_context: FunctionBuilderContext,
    pub func_signatures: HashMap<String, (FuncId, Signature)>,
    pub loop_stack: Vec<LoopContext>,
    pub type_map: HashMap<String, ir::Type>,
    pub structs: HashMap<String, StructDef>,
    pub lambda_counter: u32,
}

impl CodeGen {
    pub(crate) fn get_type_size(
        ty: &Type,
        _type_map: &HashMap<String, ir::Type>,
        structs: &HashMap<String, StructDef>,
    ) -> i64 {
        match ty {
            Type::Named(name) => match name.as_str() {
                "int" | "float" | "gen" => 8,
                "string" => 8,
                "void" => 0,
                _ => {
                    if let Some(struct_def) = structs.get(name) {
                        struct_def.size
                    } else {
                        8
                    }
                }
            },
            Type::Array(inner, len) => {
                let elem_size = Self::get_type_size(inner, _type_map, structs);
                8 + elem_size * (*len as i64)
            }
            Type::Pointer(_) => 8,
            Type::Function(_, _) => 8,
            Type::TypeVar(_) => 8,
            Type::Auto => 8,
            Type::Gen => 8,
        }
    }

    fn get_type_align(
        ty: &Type,
        type_map: &HashMap<String, ir::Type>,
        structs: &HashMap<String, StructDef>,
    ) -> i64 {
        match ty {
            Type::Named(name) => match name.as_str() {
                "int" | "float" | "gen" => 8,
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
            Type::Array(inner, _) => Self::get_type_align(inner, type_map, structs),
            Type::Pointer(_) => 8,
            Type::Function(_, _) => 8,
            Type::TypeVar(_) => 8,
            Type::Auto => 8,
            Type::Gen => 8,
        }
    }

    pub fn new(ast: Program) -> Self {
        let mut flag_builder = settings::builder();
        flag_builder.set("opt_level", "speed").unwrap();
        flag_builder.set("enable_alias_analysis", "true").unwrap();
        flag_builder
            .set("regalloc_algorithm", "backtracking")
            .unwrap();
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
        type_map.insert("any".to_string(), types::I64);
        Self {
            ast,
            module,
            builder_context: FunctionBuilderContext::new(),
            func_signatures: HashMap::new(),
            loop_stack: Vec::new(),
            type_map,
            structs: HashMap::new(),
            lambda_counter: 0,
        }
    }

    fn convert_lambdas_to_functions(&mut self, program: Program) -> Program {
        let mut new_body = Vec::new();
        let mut lambda_map: std::collections::HashMap<String, Expr> =
            std::collections::HashMap::new();

        fn process_expr(
            expr: Expr,
            lambda_counter: &mut u32,
            lambda_map: &mut std::collections::HashMap<String, Expr>,
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
                        Span::new(0, 0),
                    );
                    lambda_map.insert(lambda_name.clone(), lambda_func);

                    Expr::Var(lambda_name, Span::new(0, 0))
                }
                Expr::Block(body, _) => Expr::Block(
                    body.into_iter()
                        .map(|e| process_expr(e, lambda_counter, lambda_map))
                        .collect(),
                    Span::new(0, 0),
                ),
                Expr::Add(l, r, _) => Expr::Add(
                    Box::new(process_expr(*l, lambda_counter, lambda_map)),
                    Box::new(process_expr(*r, lambda_counter, lambda_map)),
                    Span::new(0, 0),
                ),
                Expr::Sub(l, r, _) => Expr::Sub(
                    Box::new(process_expr(*l, lambda_counter, lambda_map)),
                    Box::new(process_expr(*r, lambda_counter, lambda_map)),
                    Span::new(0, 0),
                ),
                Expr::Mul(l, r, _) => Expr::Mul(
                    Box::new(process_expr(*l, lambda_counter, lambda_map)),
                    Box::new(process_expr(*r, lambda_counter, lambda_map)),
                    Span::new(0, 0),
                ),
                Expr::Div(l, r, _) => Expr::Div(
                    Box::new(process_expr(*l, lambda_counter, lambda_map)),
                    Box::new(process_expr(*r, lambda_counter, lambda_map)),
                    Span::new(0, 0),
                ),
                Expr::Mod(l, r, _) => Expr::Mod(
                    Box::new(process_expr(*l, lambda_counter, lambda_map)),
                    Box::new(process_expr(*r, lambda_counter, lambda_map)),
                    Span::new(0, 0),
                ),
                Expr::FAdd(l, r, _) => Expr::FAdd(
                    Box::new(process_expr(*l, lambda_counter, lambda_map)),
                    Box::new(process_expr(*r, lambda_counter, lambda_map)),
                    Span::new(0, 0),
                ),
                Expr::FSub(l, r, _) => Expr::FSub(
                    Box::new(process_expr(*l, lambda_counter, lambda_map)),
                    Box::new(process_expr(*r, lambda_counter, lambda_map)),
                    Span::new(0, 0),
                ),
                Expr::FMul(l, r, _) => Expr::FMul(
                    Box::new(process_expr(*l, lambda_counter, lambda_map)),
                    Box::new(process_expr(*r, lambda_counter, lambda_map)),
                    Span::new(0, 0),
                ),
                Expr::FDiv(l, r, _) => Expr::FDiv(
                    Box::new(process_expr(*l, lambda_counter, lambda_map)),
                    Box::new(process_expr(*r, lambda_counter, lambda_map)),
                    Span::new(0, 0),
                ),
                Expr::Eq(l, r, _) => Expr::Eq(
                    Box::new(process_expr(*l, lambda_counter, lambda_map)),
                    Box::new(process_expr(*r, lambda_counter, lambda_map)),
                    Span::new(0, 0),
                ),
                Expr::Ne(l, r, _) => Expr::Ne(
                    Box::new(process_expr(*l, lambda_counter, lambda_map)),
                    Box::new(process_expr(*r, lambda_counter, lambda_map)),
                    Span::new(0, 0),
                ),
                Expr::Lt(l, r, _) => Expr::Lt(
                    Box::new(process_expr(*l, lambda_counter, lambda_map)),
                    Box::new(process_expr(*r, lambda_counter, lambda_map)),
                    Span::new(0, 0),
                ),
                Expr::Le(l, r, _) => Expr::Le(
                    Box::new(process_expr(*l, lambda_counter, lambda_map)),
                    Box::new(process_expr(*r, lambda_counter, lambda_map)),
                    Span::new(0, 0),
                ),
                Expr::Gt(l, r, _) => Expr::Gt(
                    Box::new(process_expr(*l, lambda_counter, lambda_map)),
                    Box::new(process_expr(*r, lambda_counter, lambda_map)),
                    Span::new(0, 0),
                ),
                Expr::Ge(l, r, _) => Expr::Ge(
                    Box::new(process_expr(*l, lambda_counter, lambda_map)),
                    Box::new(process_expr(*r, lambda_counter, lambda_map)),
                    Span::new(0, 0),
                ),
                Expr::FEq(l, r, _) => Expr::FEq(
                    Box::new(process_expr(*l, lambda_counter, lambda_map)),
                    Box::new(process_expr(*r, lambda_counter, lambda_map)),
                    Span::new(0, 0),
                ),
                Expr::FNe(l, r, _) => Expr::FNe(
                    Box::new(process_expr(*l, lambda_counter, lambda_map)),
                    Box::new(process_expr(*r, lambda_counter, lambda_map)),
                    Span::new(0, 0),
                ),
                Expr::FLt(l, r, _) => Expr::FLt(
                    Box::new(process_expr(*l, lambda_counter, lambda_map)),
                    Box::new(process_expr(*r, lambda_counter, lambda_map)),
                    Span::new(0, 0),
                ),
                Expr::FLe(l, r, _) => Expr::FLe(
                    Box::new(process_expr(*l, lambda_counter, lambda_map)),
                    Box::new(process_expr(*r, lambda_counter, lambda_map)),
                    Span::new(0, 0),
                ),
                Expr::FGt(l, r, _) => Expr::FGt(
                    Box::new(process_expr(*l, lambda_counter, lambda_map)),
                    Box::new(process_expr(*r, lambda_counter, lambda_map)),
                    Span::new(0, 0),
                ),
                Expr::FGe(l, r, _) => Expr::FGe(
                    Box::new(process_expr(*l, lambda_counter, lambda_map)),
                    Box::new(process_expr(*r, lambda_counter, lambda_map)),
                    Span::new(0, 0),
                ),
                Expr::And(l, r, _) => Expr::And(
                    Box::new(process_expr(*l, lambda_counter, lambda_map)),
                    Box::new(process_expr(*r, lambda_counter, lambda_map)),
                    Span::new(0, 0),
                ),
                Expr::Or(l, r, _) => Expr::Or(
                    Box::new(process_expr(*l, lambda_counter, lambda_map)),
                    Box::new(process_expr(*r, lambda_counter, lambda_map)),
                    Span::new(0, 0),
                ),
                Expr::Not(e, _) => Expr::Not(
                    Box::new(process_expr(*e, lambda_counter, lambda_map)),
                    Span::new(0, 0),
                ),
                Expr::StrCat(l, r, _) => Expr::StrCat(
                    Box::new(process_expr(*l, lambda_counter, lambda_map)),
                    Box::new(process_expr(*r, lambda_counter, lambda_map)),
                    Span::new(0, 0),
                ),
                Expr::VarDecl(name, ty, val, _) => Expr::VarDecl(
                    name,
                    ty,
                    Box::new(process_expr(*val, lambda_counter, lambda_map)),
                    Span::new(0, 0),
                ),
                Expr::VarAssign(name, val, _) => Expr::VarAssign(
                    name,
                    Box::new(process_expr(*val, lambda_counter, lambda_map)),
                    Span::new(0, 0),
                ),
                Expr::Call(func, args, _) => Expr::Call(
                    Box::new(process_expr(*func, lambda_counter, lambda_map)),
                    args.into_iter()
                        .map(|a| process_expr(a, lambda_counter, lambda_map))
                        .collect(),
                    Span::new(0, 0),
                ),
                Expr::Return(e, _) => Expr::Return(
                    Box::new(process_expr(*e, lambda_counter, lambda_map)),
                    Span::new(0, 0),
                ),
                Expr::If(cond, then_branch, else_branch, _) => Expr::If(
                    Box::new(process_expr(*cond, lambda_counter, lambda_map)),
                    Box::new(process_expr(*then_branch, lambda_counter, lambda_map)),
                    else_branch.map(|e| Box::new(process_expr(*e, lambda_counter, lambda_map))),
                    Span::new(0, 0),
                ),
                Expr::While(cond, body, _) => Expr::While(
                    Box::new(process_expr(*cond, lambda_counter, lambda_map)),
                    Box::new(process_expr(*body, lambda_counter, lambda_map)),
                    Span::new(0, 0),
                ),
                Expr::For(var, array, body, _) => Expr::For(
                    var,
                    Box::new(process_expr(*array, lambda_counter, lambda_map)),
                    Box::new(process_expr(*body, lambda_counter, lambda_map)),
                    Span::new(0, 0),
                ),
                Expr::Index(arr, idx, _) => Expr::Index(
                    Box::new(process_expr(*arr, lambda_counter, lambda_map)),
                    Box::new(process_expr(*idx, lambda_counter, lambda_map)),
                    Span::new(0, 0),
                ),
                Expr::IndexAssign(arr, val, _) => Expr::IndexAssign(
                    Box::new(process_expr(*arr, lambda_counter, lambda_map)),
                    Box::new(process_expr(*val, lambda_counter, lambda_map)),
                    Span::new(0, 0),
                ),
                Expr::ArrayLiteral(elements, _) => Expr::ArrayLiteral(
                    elements
                        .into_iter()
                        .map(|e| process_expr(e, lambda_counter, lambda_map))
                        .collect(),
                    Span::new(0, 0),
                ),
                Expr::ArrayFill(ty, len, _) => Expr::ArrayFill(
                    ty,
                    Box::new(process_expr(*len, lambda_counter, lambda_map)),
                    Span::new(0, 0),
                ),
                Expr::Range(start, end, _) => Expr::Range(
                    Box::new(process_expr(*start, lambda_counter, lambda_map)),
                    Box::new(process_expr(*end, lambda_counter, lambda_map)),
                    Span::new(0, 0),
                ),
                Expr::StructLiteral(name, fields, _) => Expr::StructLiteral(
                    name,
                    fields
                        .into_iter()
                        .map(|(n, e)| (n, process_expr(e, lambda_counter, lambda_map)))
                        .collect(),
                    Span::new(0, 0),
                ),
                Expr::MemberAccess(obj, field, _) => Expr::MemberAccess(
                    Box::new(process_expr(*obj, lambda_counter, lambda_map)),
                    field,
                    Span::new(0, 0),
                ),
                Expr::MemberAssign(obj, field, val, _) => Expr::MemberAssign(
                    Box::new(process_expr(*obj, lambda_counter, lambda_map)),
                    field,
                    Box::new(process_expr(*val, lambda_counter, lambda_map)),
                    Span::new(0, 0),
                ),
                Expr::AddressOf(expr, _) => Expr::AddressOf(
                    Box::new(process_expr(*expr, lambda_counter, lambda_map)),
                    Span::new(0, 0),
                ),
                Expr::Deref(expr, _) => Expr::Deref(
                    Box::new(process_expr(*expr, lambda_counter, lambda_map)),
                    Span::new(0, 0),
                ),
                Expr::DerefAssign(ptr, val, _) => Expr::DerefAssign(
                    Box::new(process_expr(*ptr, lambda_counter, lambda_map)),
                    Box::new(process_expr(*val, lambda_counter, lambda_map)),
                    Span::new(0, 0),
                ),
                Expr::FuncDecl(name, params, ret_type, body, _) => Expr::FuncDecl(
                    name,
                    params,
                    ret_type,
                    Box::new(process_expr(*body, lambda_counter, lambda_map)),
                    Span::new(0, 0),
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

    pub fn generate(mut self) -> Result<Vec<u8>, CodeGenError> {
        self.ast = self.convert_lambdas_to_functions(self.ast.clone());

        for expr in self.ast.body.clone() {
            match expr {
                Expr::FuncDecl(name, params, ret_type, _body, _) => {
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
                Expr::Extern(name, params, ret_type, _) => {
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
                Expr::TypeDef(_) => {}
                Expr::Struct(name, fields, _) => {
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
                expr => {
                    let span = expr.span();
                    return Err(CodeGenError::UnexpectedExpression { found: expr, span });
                }
            }
        }
        let mut str_idx = 0u32;
        for expr in self.ast.body.clone() {
            match expr {
                Expr::FuncDecl(name, params, ret_type, body, _) => {
                    self.compile_func(name, params, ret_type, body, &mut str_idx)?;
                }
                Expr::Extern(_, _, _, _) => {}
                Expr::TypeDef(_) => {}
                Expr::Struct(_, _, _) => {}
                expr => {
                    let span = expr.span();
                    return Err(CodeGenError::UnexpectedExpression { found: expr, span });
                }
            }
        }

        let product = self.module.finish();
        let object_code = product
            .emit()
            .map_err(|e| CodeGenError::ModuleError(e.to_string(), Span::new(0, 0)))?;
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

        let mut builder = FunctionBuilder::new(&mut new_ctx.func, &mut self.builder_context);
        let entry_block = builder.create_block();
        builder.append_block_params_for_function_params(entry_block);
        builder.switch_to_block(entry_block);
        builder.seal_block(entry_block);

        let mut scope_stack: Vec<HashMap<String, Slot>> = Vec::new();
        let mut type_stack: Vec<HashMap<String, Type>> = Vec::new();
        scope_stack.push(HashMap::new());
        type_stack.push(HashMap::new());
        let mut idx = 0;

        for (i, (param_name, param_ty)) in params.iter().enumerate() {
            let val = builder.block_params(entry_block)[i];

            let slot_size = Self::get_type_size(param_ty, &self.type_map, &self.structs) as u32;
            let slot = builder.create_sized_stack_slot(ir::StackSlotData::new(
                ir::StackSlotKind::ExplicitSlot,
                slot_size,
                0,
            ));

            builder.ins().stack_store(val, slot, 0);
            scope_stack[0].insert(param_name.clone(), Slot::StackSlot(slot));
            type_stack[0].insert(param_name.clone(), param_ty.clone());
            idx += 1;
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
                CodeGenError::ModuleError(format!("{}: {}", name, e), Span::new(0, 0))
            })?;

        Ok(())
    }

    pub(crate) fn find_var_assignments(expr: &Expr, modified_vars: &mut HashSet<String>) {
        match expr {
            Expr::Block(body, _) => {
                for e in body {
                    Self::find_var_assignments(e, modified_vars);
                }
            }
            Expr::VarAssign(name, _, _) => {
                modified_vars.insert(name.clone());
            }
            Expr::If(cond, t, e, _) => {
                Self::find_var_assignments(cond, modified_vars);
                Self::find_var_assignments(t, modified_vars);
                if let Some(e) = e {
                    Self::find_var_assignments(e, modified_vars);
                }
            }
            Expr::While(cond, body, _) => {
                Self::find_var_assignments(cond, modified_vars);
                Self::find_var_assignments(body, modified_vars);
            }
            Expr::For(_, array, body, _) => {
                Self::find_var_assignments(array, modified_vars);
                Self::find_var_assignments(body, modified_vars);
            }
            Expr::Add(l, r, _)
            | Expr::Sub(l, r, _)
            | Expr::Mul(l, r, _)
            | Expr::Div(l, r, _)
            | Expr::Mod(l, r, _)
            | Expr::FAdd(l, r, _)
            | Expr::FSub(l, r, _)
            | Expr::FMul(l, r, _)
            | Expr::FDiv(l, r, _)
            | Expr::Eq(l, r, _)
            | Expr::Ne(l, r, _)
            | Expr::Lt(l, r, _)
            | Expr::Le(l, r, _)
            | Expr::Gt(l, r, _)
            | Expr::Ge(l, r, _)
            | Expr::FEq(l, r, _)
            | Expr::FNe(l, r, _)
            | Expr::FLt(l, r, _)
            | Expr::FLe(l, r, _)
            | Expr::FGt(l, r, _)
            | Expr::FGe(l, r, _)
            | Expr::And(l, r, _)
            | Expr::Or(l, r, _)
            | Expr::StrCat(l, r, _) => {
                Self::find_var_assignments(l, modified_vars);
                Self::find_var_assignments(r, modified_vars);
            }
            Expr::Not(e, _) | Expr::Return(e, _) => {
                Self::find_var_assignments(e, modified_vars);
            }
            Expr::Call(func, args, _) => {
                Self::find_var_assignments(func, modified_vars);
                for a in args {
                    Self::find_var_assignments(a, modified_vars);
                }
            }
            Expr::Index(arr, idx, _) => {
                Self::find_var_assignments(arr, modified_vars);
                Self::find_var_assignments(idx, modified_vars);
            }
            Expr::IndexAssign(arr, _, _) => {
                Self::find_var_assignments(arr, modified_vars);
            }
            Expr::ArrayLiteral(elements, _) => {
                for e in elements {
                    Self::find_var_assignments(e, modified_vars);
                }
            }
            Expr::ArrayFill(_, len, _) => {
                Self::find_var_assignments(len, modified_vars);
            }
            Expr::StructLiteral(_, fields, _) => {
                for (_, f) in fields {
                    Self::find_var_assignments(f, modified_vars);
                }
            }
            Expr::MemberAccess(obj, _, _) => {
                Self::find_var_assignments(obj, modified_vars);
            }
            Expr::MemberAssign(obj, _, val, _) => {
                Self::find_var_assignments(obj, modified_vars);
                Self::find_var_assignments(val, modified_vars);
            }
            Expr::AddressOf(expr, _) => {
                Self::find_var_assignments(expr, modified_vars);
            }
            Expr::Deref(expr, _) => {
                Self::find_var_assignments(expr, modified_vars);
            }
            Expr::DerefAssign(ptr, val, _) => {
                Self::find_var_assignments(ptr, modified_vars);
                Self::find_var_assignments(val, modified_vars);
            }
            _ => {}
        }
    }
}
impl CodeGen {
    fn lookup_var<'a>(name: &str, scope_stack: &'a Vec<HashMap<String, Slot>>) -> Option<&'a Slot> {
        for scope in scope_stack.iter().rev() {
            if let Some(v) = scope.get(name) {
                return Some(v);
            }
        }
        None
    }

    fn get_var_type(name: &str, type_stack: &Vec<HashMap<String, Type>>) -> Option<Type> {
        for scope in type_stack.iter().rev() {
            if let Some(ty) = scope.get(name) {
                return Some(ty.clone());
            }
        }
        None
    }

    fn get_expr_type(expr: &Expr, type_stack: &Vec<HashMap<String, Type>>) -> Type {
        match expr {
            Expr::Int(_, _) => Type::Named("int".to_string()),
            Expr::Float(_, _) => Type::Named("float".to_string()),
            Expr::Bool(_, _) => Type::Named("bool".to_string()),
            Expr::String(_, _) => Type::Named("string".to_string()),
            Expr::Nil(_) => Type::Named("void".to_string()),
            Expr::Var(name, _) => {
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
            Expr::Return(_, _) => true,
            Expr::Block(body, _) => body.last().map_or(false, |e| Self::has_return(e)),
            Expr::If(_, then_branch, else_branch, _) => {
                Self::has_return(then_branch)
                    && else_branch.as_ref().map_or(false, |e| Self::has_return(e))
            }
            Expr::While(_, body, _) => Self::has_return(body),
            Expr::For(_, _, body, _) => Self::has_return(body),
            _ => false,
        }
    }

    fn has_break_or_continue(expr: &Expr) -> bool {
        match expr {
            Expr::Break(_) | Expr::Continue(_) => true,
            Expr::Block(body, _) => body.iter().any(|e| Self::has_break_or_continue(e)),
            Expr::If(_, then_branch, else_branch, _) => {
                Self::has_break_or_continue(then_branch)
                    || else_branch
                        .as_ref()
                        .map_or(false, |e| Self::has_break_or_continue(e))
            }
            Expr::While(_, body, _) => Self::has_break_or_continue(body),
            Expr::For(_, _, body, _) => Self::has_break_or_continue(body),
            _ => false,
        }
    }

    pub(crate) fn compile_expr(
        expr: &Expr,
        builder: &mut FunctionBuilder,
        scope_stack: &mut Vec<HashMap<String, Slot>>,
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
            Expr::Block(body, _) => {
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
            Expr::Int(i, _) => Ok(builder.ins().iconst(types::I64, *i as i64)),
            Expr::Float(f, _) => Ok(builder.ins().f64const(*f)),
            Expr::Bool(b, _) => Ok(builder.ins().iconst(types::I8, *b as i64)),
            Expr::String(s, _) => {
                let data_id = module
                    .declare_data(
                        &format!("str_{}", str_idx),
                        cranelift_module::Linkage::Local,
                        true,
                        false,
                    )
                    .map_err(|e| CodeGenError::ModuleError(e.to_string(), Span::new(0, 0)))?;

                let mut data_desc = cranelift_module::DataDescription::new();
                let mut bytes = s.as_bytes().to_vec();
                bytes.push(0);
                data_desc.define(bytes.into());
                module
                    .define_data(data_id, &data_desc)
                    .map_err(|e| CodeGenError::ModuleError(e.to_string(), Span::new(0, 0)))?;

                let global_value = module.declare_data_in_func(data_id, builder.func);
                let ptr = builder.ins().global_value(types::I64, global_value);
                *str_idx += 1;
                Ok(ptr)
            }
            Expr::Nil(_) => Ok(builder.ins().iconst(types::I64, 0)),

            Expr::Add(lhs, rhs, _) => {
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
                        .map_err(|e| CodeGenError::ModuleError(e.to_string(), Span::new(0, 0)))?;
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
                        .map_err(|e| CodeGenError::ModuleError(e.to_string(), Span::new(0, 0)))?;
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
                        .map_err(|e| CodeGenError::ModuleError(e.to_string(), Span::new(0, 0)))?;
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
                        .map_err(|e| CodeGenError::ModuleError(e.to_string(), Span::new(0, 0)))?;
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
                        .map_err(|e| CodeGenError::ModuleError(e.to_string(), Span::new(0, 0)))?;
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
            Expr::Sub(lhs, rhs, _) => {
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
            Expr::Mul(lhs, rhs, _) => {
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
            Expr::Div(lhs, rhs, _) => {
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
            Expr::Mod(lhs, rhs, _) => {
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

            Expr::FAdd(lhs, rhs, _) => {
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
            Expr::FSub(lhs, rhs, _) => {
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
            Expr::FMul(lhs, rhs, _) => {
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
            Expr::FDiv(lhs, rhs, _) => {
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

            Expr::Eq(lhs, rhs, _) => {
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
            Expr::Ne(lhs, rhs, _) => {
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
            Expr::Lt(lhs, rhs, _) => {
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
            Expr::Le(lhs, rhs, _) => {
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
            Expr::Gt(lhs, rhs, _) => {
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
            Expr::Ge(lhs, rhs, _) => {
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

            Expr::FEq(lhs, rhs, _) => {
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
            Expr::FNe(lhs, rhs, _) => {
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
            Expr::FLt(lhs, rhs, _) => {
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
            Expr::FLe(lhs, rhs, _) => {
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
            Expr::FGt(lhs, rhs, _) => {
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
            Expr::FGe(lhs, rhs, _) => {
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

            Expr::StrCat(lhs, rhs, _) => {
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
                    .map_err(|e| CodeGenError::ModuleError(e.to_string(), Span::new(0, 0)))?;
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
                    .map_err(|e| CodeGenError::ModuleError(e.to_string(), Span::new(0, 0)))?;
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
                    .map_err(|e| CodeGenError::ModuleError(e.to_string(), Span::new(0, 0)))?;
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
                    .map_err(|e| CodeGenError::ModuleError(e.to_string(), Span::new(0, 0)))?;
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
                    .map_err(|e| CodeGenError::ModuleError(e.to_string(), Span::new(0, 0)))?;
                let strcat_ref = module.declare_func_in_func(strcat_id, builder.func);
                builder.ins().call(strcat_ref, &[result_ptr, rhs_ptr]);

                Ok(result_ptr)
            }
            Expr::And(lhs, rhs, _) => {
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
            Expr::Or(lhs, rhs, _) => {
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
            Expr::Not(expr, _) => {
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
            Expr::VarDecl(name, ty, value, _) => {
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

                let decl_type = if let Type::Named(n) = ty {
                    if n == "gen" {
                        match &**value {
                            Expr::Int(_, _) => Type::Named("int".to_string()),
                            Expr::Float(_, _) => Type::Named("float".to_string()),
                            Expr::Bool(_, _) => Type::Named("bool".to_string()),
                            Expr::String(_, _) => Type::Named("string".to_string()),
                            _ => Type::Named("int".to_string()),
                        }
                    } else {
                        ty.clone()
                    }
                } else {
                    ty.clone()
                };

                let slot_size = Self::get_type_size(&decl_type, type_map, structs) as u32;
                let slot = builder.create_sized_stack_slot(ir::StackSlotData::new(
                    ir::StackSlotKind::ExplicitSlot,
                    slot_size,
                    0,
                ));

                builder.ins().stack_store(val, slot, 0);
                scope_stack
                    .last_mut()
                    .unwrap()
                    .insert(name.clone(), Slot::StackSlot(slot));
                type_stack
                    .last_mut()
                    .unwrap()
                    .insert(name.clone(), ty.clone());
                *idx += 1;
                Ok(Value::from_u32(0))
            }

            Expr::VarAssign(name, val, _) => {
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
                let slot = Self::lookup_var(name, scope_stack).ok_or_else(|| {
                    CodeGenError::UndefinedVariable {
                        name: name.clone(),
                        span: Span::new(0, 0),
                    }
                })?;
                match slot {
                    Slot::StackSlot(s) => {
                        builder.ins().stack_store(val, *s, 0);
                    }
                }
                Ok(val)
            }
            Expr::Var(name, var_span) => {
                if let Some(slot) = Self::lookup_var(name, scope_stack) {
                    return match slot {
                        Slot::StackSlot(s) => {
                            let mut found_type = None;
                            for scope in type_stack.iter().rev() {
                                if let Some(ty) = scope.get(name) {
                                    found_type = Some(ty.clone());
                                    break;
                                }
                            }

                            let ty = match found_type {
                                Some(t) => t,
                                None => Type::Named("int".to_string()),
                            };

                            let ir_type = get_type(&ty, type_map);
                            let val = builder.ins().stack_load(ir_type, *s, 0);

                            if ir_type == types::I8 {
                                Ok(builder.ins().uextend(types::I64, val))
                            } else {
                                Ok(val)
                            }
                        }
                    };
                }

                if let Some((func_id, _)) = func_signatures.get(name) {
                    let func_ref = module.declare_func_in_func(*func_id, builder.func);
                    let addr = builder.ins().func_addr(types::I64, func_ref);
                    return Ok(addr);
                }

                Err(CodeGenError::UndefinedVariable {
                    name: name.clone(),
                    span: *var_span,
                })
            }
            Expr::Call(callee, args, _) => {
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

                if let Expr::Var(name, _) = &**callee {
                    if let Some((func_id, _)) = func_signatures.get(name) {
                        let func_ref = module.declare_func_in_func(*func_id, builder.func);
                        let call = builder.ins().call(func_ref, &arg_values);
                        let results = builder.inst_results(call);
                        if results.is_empty() {
                            return Ok(builder.ins().iconst(types::I64, 0));
                        } else {
                            return Ok(results[0]);
                        }
                    }
                }

                let callee_type = Self::get_expr_type(callee, type_stack);

                let callee_val = Self::compile_expr(
                    callee,
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

                let sig = {
                    let mut sig = module.make_signature();
                    sig.call_conv = CallConv::SystemV;
                    for arg in &arg_values {
                        sig.params
                            .push(AbiParam::new(builder.func.dfg.value_type(*arg)));
                    }

                    let return_type = if let Type::Function(_, ret) = callee_type {
                        get_type(&ret, type_map)
                    } else {
                        types::I64
                    };

                    if return_type != types::INVALID {
                        sig.returns.push(AbiParam::new(return_type));
                    }
                    sig
                };

                let sig_ref = builder.import_signature(sig);

                let call = builder
                    .ins()
                    .call_indirect(sig_ref, callee_val, &arg_values);
                let results = builder.inst_results(call);
                if results.is_empty() {
                    Ok(builder.ins().iconst(types::I64, 0))
                } else {
                    Ok(results[0])
                }
            }
            Expr::Return(value, _) => {
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
            Expr::If(cond, then_branch, else_branch, _) => {
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

                let then_has_break = Self::has_break_or_continue(then_branch);
                let else_has_break = else_branch
                    .as_ref()
                    .map_or(false, |e| Self::has_break_or_continue(e));
                let has_break_or_continue = then_has_break || else_has_break;

                let in_loop = !loop_stack.is_empty();
                let then_has_terminal = then_has_return || (in_loop && then_has_break);
                let else_has_terminal = else_has_return || (in_loop && else_has_break);
                let both_terminal = then_has_terminal && else_has_terminal;

                let in_loop_with_increment = loop_stack
                    .last()
                    .and_then(|ctx| ctx.increment_block)
                    .is_some();

                let then_block = builder.create_block();
                let else_block = builder.create_block();

                let merge_block =
                    if both_terminal || in_loop_with_increment || has_break_or_continue {
                        None
                    } else {
                        Some(builder.create_block())
                    };

                let cond_type = builder.func.dfg.value_type(cond_val);
                let cond_i64 = if cond_type == types::I64 {
                    cond_val
                } else {
                    builder.ins().uextend(types::I64, cond_val)
                };

                builder
                    .ins()
                    .brif(cond_i64, then_block, &[], else_block, &[]);

                builder.switch_to_block(then_block);
                scope_stack.push(HashMap::new());
                type_stack.push(HashMap::new());
                let then_val = Self::compile_expr(
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

                if !then_has_terminal {
                    if let Some(merge) = merge_block {
                        if else_branch.is_some() {
                            builder.ins().jump(merge, &[then_val.into()]);
                        } else {
                            builder.ins().jump(merge, &[]);
                        }
                    } else if in_loop_with_increment {
                        if let Some(loop_ctx) = loop_stack.last() {
                            if let Some(inc_block) = loop_ctx.increment_block {
                                builder.ins().jump(inc_block, &[]);
                            }
                        }
                    }
                }
                builder.seal_block(then_block);

                builder.switch_to_block(else_block);
                let else_val = if let Some(else_expr) = else_branch {
                    scope_stack.push(HashMap::new());
                    type_stack.push(HashMap::new());
                    let val = Self::compile_expr(
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

                    if !else_has_terminal {
                        if let Some(merge) = merge_block {
                            builder.ins().jump(merge, &[val.into()]);
                        } else if in_loop_with_increment {
                            if let Some(loop_ctx) = loop_stack.last() {
                                if let Some(inc_block) = loop_ctx.increment_block {
                                    builder.ins().jump(inc_block, &[]);
                                }
                            }
                        }
                    }
                    Some(val)
                } else {
                    if let Some(merge) = merge_block {
                        builder.ins().jump(merge, &[]);
                    }
                    None
                };
                builder.seal_block(else_block);

                if let Some(merge) = merge_block {
                    if else_val.is_some() {
                        let merge_val = builder
                            .append_block_param(merge, builder.func.dfg.value_type(then_val));

                        builder.switch_to_block(merge);
                        builder.seal_block(merge);
                        Ok(merge_val)
                    } else {
                        builder.switch_to_block(merge);
                        builder.seal_block(merge);
                        Ok(then_val)
                    }
                } else {
                    Ok(then_val)
                }
            }
            Expr::While(cond, body, _) => {
                let loop_header = builder.create_block();
                let loop_body = builder.create_block();
                let loop_exit = builder.create_block();

                loop_stack.push(LoopContext {
                    header_block: loop_header,
                    exit_block: loop_exit,
                    increment_block: None,
                    loop_params: Vec::new(),
                });

                let mut modified_vars = HashSet::new();
                Self::find_var_assignments(body, &mut modified_vars);

                let mut loop_params: Vec<(String, Value, ir::StackSlot)> = Vec::new();
                let mut param_types = Vec::new();
                for var_name in &modified_vars {
                    if let Some(slot) = Self::lookup_var(var_name, scope_stack) {
                        match slot {
                            Slot::StackSlot(s) => {
                                let var_value = builder.ins().stack_load(types::I64, *s, 0);
                                let var_type = builder.func.dfg.value_type(var_value);
                                param_types.push(var_type);
                                let param = builder.append_block_param(loop_header, var_type);
                                loop_params.push((var_name.clone(), param, *s));
                            }
                        }
                    }
                }

                if let Some(loop_ctx) = loop_stack.last_mut() {
                    loop_ctx.loop_params = loop_params
                        .iter()
                        .map(|(name, _, s)| (name.clone(), Slot::StackSlot(*s)))
                        .collect();
                }

                let initial_values: Vec<Value> = loop_params
                    .iter()
                    .map(|(_, _, s)| builder.ins().stack_load(types::I64, *s, 0))
                    .collect();
                let initial_args: Vec<ir::BlockArg> =
                    initial_values.iter().map(|v| (*v).into()).collect();
                builder.ins().jump(loop_header, initial_args.as_slice());

                builder.switch_to_block(loop_header);

                for (_, param, s) in &loop_params {
                    builder.ins().stack_store(*param, *s, 0);
                }

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

                let mut loop_values = Vec::new();
                for (var_name, _, s) in &loop_params {
                    if let Some(slot) = Self::lookup_var(var_name, scope_stack) {
                        match slot {
                            Slot::StackSlot(ss) => {
                                loop_values.push(builder.ins().stack_load(types::I64, *ss, 0));
                            }
                        }
                    } else {
                        loop_values.push(builder.ins().stack_load(types::I64, *s, 0));
                    }
                }
                let loop_args: Vec<ir::BlockArg> =
                    loop_values.iter().map(|v| (*v).into()).collect();
                builder.ins().jump(loop_header, loop_args.as_slice());

                builder.seal_block(loop_body);
                builder.seal_block(loop_header);

                builder.switch_to_block(loop_exit);
                builder.seal_block(loop_exit);

                Ok(builder.ins().iconst(types::I64, 0))
            }
            Expr::Index(array, index, _) => {
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
            Expr::IndexAssign(array_index, value, _) => {
                if let Expr::Index(array, index, _) = &**array_index {
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
                        span: array_index.span(),
                    })
                }
            }
            Expr::ArrayLiteral(elements, _) => {
                let data_id = module
                    .declare_data(
                        &format!("array_{}", idx),
                        cranelift_module::Linkage::Local,
                        true,
                        false,
                    )
                    .map_err(|e| CodeGenError::ModuleError(e.to_string(), Span::new(0, 0)))?;

                let mut data_desc = cranelift_module::DataDescription::new();
                let mut bytes = Vec::new();

                let mut str_idx = 0;
                for elem in elements {
                    match elem {
                        Expr::Int(n, _) => {
                            let val = *n as i64;
                            bytes.extend_from_slice(&val.to_le_bytes());
                        }
                        Expr::Float(f, _) => {
                            let val = f.to_bits();
                            bytes.extend_from_slice(&val.to_le_bytes());
                        }
                        Expr::Bool(b, _) => {
                            let val = *b as i64;
                            bytes.extend_from_slice(&val.to_le_bytes());
                        }
                        Expr::String(s, _) => {
                            let str_data_id = module
                                .declare_data(
                                    &format!("array_str_{}_{}", idx, str_idx),
                                    cranelift_module::Linkage::Local,
                                    true,
                                    false,
                                )
                                .map_err(|e| {
                                    CodeGenError::ModuleError(e.to_string(), Span::new(0, 0))
                                })?;
                            let mut str_data_desc = cranelift_module::DataDescription::new();
                            let mut str_bytes = s.as_bytes().to_vec();
                            str_bytes.push(0);
                            str_data_desc.define(str_bytes.into());
                            module
                                .define_data(str_data_id, &str_data_desc)
                                .map_err(|e| {
                                    CodeGenError::ModuleError(e.to_string(), Span::new(0, 0))
                                })?;

                            bytes.extend_from_slice(&[0u8; 8]);
                            str_idx += 1;
                        }
                        Expr::Nil(_) => {
                            bytes.extend_from_slice(&[0u8; 8]);
                        }
                        _ => {
                            return Err(CodeGenError::ModuleError(
                                "Array literals currently only support int, float, bool, string, and nil constants".to_string(),
                                Span::new(0, 0),
                            ));
                        }
                    }
                }
                data_desc.define(bytes.into());
                module
                    .define_data(data_id, &data_desc)
                    .map_err(|e| CodeGenError::ModuleError(e.to_string(), Span::new(0, 0)))?;

                let global_value = module.declare_data_in_func(data_id, builder.func);
                let ptr = builder.ins().global_value(types::I64, global_value);
                Ok(ptr)
            }
            Expr::ArrayFill(elem_type, length, _) => {
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
                    Type::Array(_, _) => 8i64,
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
                    .map_err(|e| CodeGenError::ModuleError(e.to_string(), Span::new(0, 0)))?;
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
                    .map_err(|e| CodeGenError::ModuleError(e.to_string(), Span::new(0, 0)))?;
                let memset_ref = module.declare_func_in_func(memset_id, builder.func);
                let zero_val = builder.ins().iconst(types::I32, 0);

                builder
                    .ins()
                    .call(memset_ref, &[mem_ptr, zero_val, total_size]);

                Ok(mem_ptr)
            }
            Expr::Range(start, end, _) => {
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

                let len_val = builder.ins().isub(end_val, start_val);

                let zero = builder.ins().iconst(types::I64, 0);
                let is_neg =
                    builder
                        .ins()
                        .icmp(ir::condcodes::IntCC::SignedLessThan, len_val, zero);
                let final_len = builder.ins().select(is_neg, zero, len_val);

                let total_size = builder.ins().imul_imm(final_len, 8);

                let malloc_sig = {
                    let mut sig = module.make_signature();
                    sig.call_conv = CallConv::SystemV;
                    sig.params.push(AbiParam::new(types::I64));
                    sig.returns.push(AbiParam::new(types::I64));
                    sig
                };
                let malloc_id = module
                    .declare_function("malloc", Linkage::Import, &malloc_sig)
                    .map_err(|e| CodeGenError::ModuleError(e.to_string(), Span::new(0, 0)))?;
                let malloc_ref = module.declare_func_in_func(malloc_id, builder.func);
                let mem_ptr = builder.ins().call(malloc_ref, &[total_size]);
                let mem_ptr = builder.inst_results(mem_ptr)[0];

                let fill_header = builder.create_block();
                let fill_body = builder.create_block();
                let fill_exit = builder.create_block();

                let idx_slot =
                    builder.create_sized_stack_slot(cranelift::codegen::ir::StackSlotData::new(
                        cranelift::codegen::ir::StackSlotKind::ExplicitSlot,
                        8,
                        0,
                    ));
                builder.ins().stack_store(zero, idx_slot, 0);

                builder.ins().jump(fill_header, &[]);

                builder.switch_to_block(fill_header);
                let current_idx = builder.ins().stack_load(types::I64, idx_slot, 0);
                let cmp = builder.ins().icmp(
                    ir::condcodes::IntCC::UnsignedLessThan,
                    current_idx,
                    final_len,
                );
                let cmp_i64 = builder.ins().uextend(types::I64, cmp);
                builder.ins().brif(cmp_i64, fill_body, &[], fill_exit, &[]);

                builder.switch_to_block(fill_body);

                let elem_val = builder.ins().iadd(start_val, current_idx);

                let elem_offset = builder.ins().imul_imm(current_idx, 8);
                let elem_ptr = builder.ins().iadd(mem_ptr, elem_offset);
                builder
                    .ins()
                    .store(ir::MemFlags::new(), elem_val, elem_ptr, 0);

                let next_idx = builder.ins().iadd_imm(current_idx, 1);
                builder.ins().stack_store(next_idx, idx_slot, 0);
                builder.ins().jump(fill_header, &[]);

                builder.switch_to_block(fill_exit);
                builder.seal_block(fill_header);
                builder.seal_block(fill_body);
                builder.seal_block(fill_exit);

                Ok(mem_ptr)
            }
            Expr::For(var, array, body, _) => {
                let loop_header = builder.create_block();
                let loop_body = builder.create_block();
                let loop_increment = builder.create_block();
                let loop_exit = builder.create_block();

                loop_stack.push(LoopContext {
                    header_block: loop_header,
                    exit_block: loop_exit,
                    increment_block: Some(loop_increment),
                    loop_params: Vec::new(),
                });

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

                let array_len = match array.as_ref() {
                    Expr::ArrayLiteral(elements, _) => {
                        builder.ins().iconst(types::I64, elements.len() as i64)
                    }

                    Expr::Range(start, end, _) => {
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
                        let len = builder.ins().isub(end_val, start_val);
                        let zero = builder.ins().iconst(types::I64, 0);
                        let is_neg =
                            builder
                                .ins()
                                .icmp(ir::condcodes::IntCC::SignedLessThan, len, zero);
                        builder.ins().select(is_neg, zero, len)
                    }

                    Expr::Var(name, _) => {
                        if let Some(ty) = Self::get_var_type(name, type_stack) {
                            if let Type::Array(_, len) = ty {
                                if len > 0 {
                                    builder.ins().iconst(types::I64, len as i64)
                                } else {
                                    builder.ins().iconst(types::I64, 0)
                                }
                            } else {
                                builder.ins().iconst(types::I64, 0)
                            }
                        } else {
                            builder.ins().iconst(types::I64, 0)
                        }
                    }

                    _ => builder.ins().iconst(types::I64, 0),
                };

                let index_slot =
                    builder.create_sized_stack_slot(cranelift::codegen::ir::StackSlotData::new(
                        cranelift::codegen::ir::StackSlotKind::ExplicitSlot,
                        8,
                        0,
                    ));
                let zero = builder.ins().iconst(types::I64, 0);
                builder.ins().stack_store(zero, index_slot, 0);

                let elem_slot =
                    builder.create_sized_stack_slot(cranelift::codegen::ir::StackSlotData::new(
                        cranelift::codegen::ir::StackSlotKind::ExplicitSlot,
                        8,
                        0,
                    ));

                builder.ins().jump(loop_header, &[]);

                builder.switch_to_block(loop_header);
                let current_idx = builder.ins().stack_load(types::I64, index_slot, 0);

                let cmp = builder.ins().icmp(
                    ir::condcodes::IntCC::UnsignedLessThan,
                    current_idx,
                    array_len,
                );
                let cmp_i64 = builder.ins().uextend(types::I64, cmp);
                builder.ins().brif(cmp_i64, loop_body, &[], loop_exit, &[]);

                builder.switch_to_block(loop_body);

                let elem_offset = builder.ins().imul_imm(current_idx, 8);
                let elem_ptr = builder.ins().iadd(array_ptr, elem_offset);
                let elem_val = builder
                    .ins()
                    .load(types::I64, ir::MemFlags::new(), elem_ptr, 0);
                builder.ins().stack_store(elem_val, elem_slot, 0);

                scope_stack.push(HashMap::new());
                type_stack.push(HashMap::new());

                scope_stack
                    .last_mut()
                    .unwrap()
                    .insert(var.clone(), Slot::StackSlot(elem_slot));

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
                let current_idx_inc = builder.ins().stack_load(types::I64, index_slot, 0);
                let next_idx = builder.ins().iadd_imm(current_idx_inc, 1);
                builder.ins().stack_store(next_idx, index_slot, 0);
                builder.ins().jump(loop_header, &[]);

                loop_stack.pop();

                builder.seal_block(loop_increment);
                builder.seal_block(loop_body);
                builder.seal_block(loop_header);

                builder.switch_to_block(loop_exit);
                builder.seal_block(loop_exit);

                Ok(builder.ins().iconst(types::I64, 0))
            }
            Expr::Break(_) => {
                if let Some(loop_ctx) = loop_stack.last() {
                    builder.ins().jump(loop_ctx.exit_block, &[]);
                    Ok(Value::from_u32(0))
                } else {
                    Err(CodeGenError::ModuleError(
                        "break statement outside of loop".to_string(),
                        Span::new(0, 0),
                    ))
                }
            }
            Expr::Continue(_) => {
                if let Some(loop_ctx) = loop_stack.last() {
                    if let Some(inc_block) = loop_ctx.increment_block {
                        builder.ins().jump(inc_block, &[]);
                    } else {
                        let mut loop_values = Vec::new();
                        for (var_name, slot) in &loop_ctx.loop_params {
                            if let Some(s) = Self::lookup_var(var_name, scope_stack) {
                                match s {
                                    Slot::StackSlot(ss) => {
                                        loop_values.push(builder.ins().stack_load(
                                            types::I64,
                                            *ss,
                                            0,
                                        ));
                                    }
                                }
                            } else {
                                match slot {
                                    Slot::StackSlot(ss) => {
                                        loop_values.push(builder.ins().stack_load(
                                            types::I64,
                                            *ss,
                                            0,
                                        ));
                                    }
                                }
                            }
                        }
                        let loop_args: Vec<ir::BlockArg> =
                            loop_values.iter().map(|v| (*v).into()).collect();
                        builder
                            .ins()
                            .jump(loop_ctx.header_block, loop_args.as_slice());
                    }
                    Ok(Value::from_u32(0))
                } else {
                    Err(CodeGenError::ModuleError(
                        "continue statement outside of loop".to_string(),
                        Span::new(0, 0),
                    ))
                }
            }
            Expr::TypeDef(_) => Ok(Value::from_u32(0)),
            Expr::Struct(name, fields, _) => {
                let _ = (name, fields);
                Ok(Value::from_u32(0))
            }
            Expr::StructLiteral(name, field_values, _) => {
                let struct_def = structs.get(name).ok_or_else(|| {
                    CodeGenError::ModuleError(
                        format!("Undefined struct type: {}", name),
                        Span::new(0, 0),
                    )
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
                    .map_err(|e| CodeGenError::ModuleError(e.to_string(), Span::new(0, 0)))?;
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
            Expr::MemberAccess(obj, field_name, _) => {
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

                let (struct_name, is_pointer) = match &**obj {
                    Expr::Var(name, _) => {
                        let mut found_type = None;
                        for scope in type_stack.iter().rev() {
                            if let Some(ty) = scope.get(name) {
                                found_type = Some(ty.clone());
                                break;
                            }
                        }
                        match found_type {
                            Some(Type::Named(name)) => (name, false),
                            Some(Type::Pointer(inner_type)) => {
                                if let Type::Named(name) = *inner_type {
                                    (name, true)
                                } else {
                                    return Err(CodeGenError::ModuleError(
                                        "Pointer to non-struct type".to_string(),
                                        Span::new(0, 0),
                                    ));
                                }
                            }
                            _ => {
                                return Err(CodeGenError::ModuleError(
                                    "Member access on non-struct type".to_string(),
                                    Span::new(0, 0),
                                ));
                            }
                        }
                    }
                    _ => {
                        return Err(CodeGenError::ModuleError(
                            "Member access only supported on variables".to_string(),
                            Span::new(0, 0),
                        ));
                    }
                };

                let obj_ptr = if is_pointer {
                    builder
                        .ins()
                        .load(types::I64, ir::MemFlags::trusted(), obj_ptr, 0)
                } else {
                    obj_ptr
                };

                let struct_def = structs.get(&struct_name).ok_or_else(|| {
                    CodeGenError::ModuleError(
                        format!("Undefined struct type: {}", struct_name),
                        Span::new(0, 0),
                    )
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
                    Err(CodeGenError::ModuleError(
                        format!("Struct {} has no field {}", struct_name, field_name),
                        Span::new(0, 0),
                    ))
                }
            }
            Expr::MemberAssign(obj, field_name, value, _) => {
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

                let (struct_name, is_pointer) = match &**obj {
                    Expr::Var(name, _) => {
                        let mut found_type = None;
                        for scope in type_stack.iter().rev() {
                            if let Some(ty) = scope.get(name) {
                                found_type = Some(ty.clone());
                                break;
                            }
                        }
                        match found_type {
                            Some(Type::Named(name)) => (name, false),
                            Some(Type::Pointer(inner_type)) => {
                                if let Type::Named(name) = *inner_type {
                                    (name, true)
                                } else {
                                    return Err(CodeGenError::ModuleError(
                                        "Pointer to non-struct type".to_string(),
                                        Span::new(0, 0),
                                    ));
                                }
                            }
                            _ => {
                                return Err(CodeGenError::ModuleError(
                                    "Member assign on non-struct type".to_string(),
                                    Span::new(0, 0),
                                ));
                            }
                        }
                    }
                    _ => {
                        return Err(CodeGenError::ModuleError(
                            "Member assign only supported on variables".to_string(),
                            Span::new(0, 0),
                        ));
                    }
                };

                let obj_ptr = if is_pointer {
                    builder
                        .ins()
                        .load(types::I64, ir::MemFlags::trusted(), obj_ptr, 0)
                } else {
                    obj_ptr
                };

                let struct_def = structs.get(&struct_name).ok_or_else(|| {
                    CodeGenError::ModuleError(
                        format!("Undefined struct type: {}", struct_name),
                        Span::new(0, 0),
                    )
                })?;

                let field_info = struct_def.fields.iter().find(|f| &f.name == field_name);
                if let Some(field) = field_info {
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

                    let field_ptr = builder.ins().iadd_imm(obj_ptr, field.offset);
                    builder
                        .ins()
                        .store(ir::MemFlags::trusted(), value_val, field_ptr, 0);
                    Ok(builder.ins().iconst(types::I64, 0))
                } else {
                    Err(CodeGenError::ModuleError(
                        format!("Struct {} has no field {}", struct_name, field_name),
                        Span::new(0, 0),
                    ))
                }
            }
            Expr::AddressOf(expr, _) => match &**expr {
                Expr::Var(name, _) => {
                    if let Some(slot) = Self::lookup_var(name, scope_stack) {
                        match slot {
                            Slot::StackSlot(s) => Ok(builder.ins().stack_addr(types::I64, *s, 0)),
                        }
                    } else {
                        Err(CodeGenError::ModuleError(
                            format!("Undefined variable: {}", name),
                            Span::new(0, 0),
                        ))
                    }
                }
                Expr::MemberAccess(obj, field_name, _) => {
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
                        Expr::Var(name, _) => {
                            let mut found_type = None;
                            for scope in type_stack.iter().rev() {
                                if let Some(ty) = scope.get(name) {
                                    found_type = Some(ty.clone());
                                    break;
                                }
                            }
                            match found_type {
                                Some(Type::Pointer(inner_type)) => {
                                    if let Type::Named(name) = *inner_type {
                                        name
                                    } else {
                                        return Err(CodeGenError::ModuleError(
                                            "Pointer to non-struct type".to_string(),
                                            Span::new(0, 0),
                                        ));
                                    }
                                }
                                Some(Type::Named(name)) => name,
                                _ => {
                                    return Err(CodeGenError::ModuleError(
                                        "Member access on non-struct type".to_string(),
                                        Span::new(0, 0),
                                    ));
                                }
                            }
                        }
                        Expr::Deref(inner, _) => match &**inner {
                            Expr::Var(name, _) => {
                                let mut found_type = None;
                                for scope in type_stack.iter().rev() {
                                    if let Some(ty) = scope.get(name) {
                                        found_type = Some(ty.clone());
                                        break;
                                    }
                                }
                                match found_type {
                                    Some(Type::Pointer(inner_type)) => {
                                        if let Type::Named(name) = *inner_type {
                                            name
                                        } else {
                                            return Err(CodeGenError::ModuleError(
                                                "Deref of non-pointer type".to_string(),
                                                Span::new(0, 0),
                                            ));
                                        }
                                    }
                                    _ => {
                                        return Err(CodeGenError::ModuleError(
                                            "Member access on non-struct type".to_string(),
                                            Span::new(0, 0),
                                        ));
                                    }
                                }
                            }
                            _ => {
                                return Err(CodeGenError::ModuleError(
                                    "Member access on non-struct type".to_string(),
                                    Span::new(0, 0),
                                ));
                            }
                        },
                        _ => {
                            return Err(CodeGenError::ModuleError(
                                "Member access only supported on variables".to_string(),
                                Span::new(0, 0),
                            ));
                        }
                    };

                    let struct_def = structs.get(&struct_name).ok_or_else(|| {
                        CodeGenError::ModuleError(
                            format!("Undefined struct type: {}", struct_name),
                            Span::new(0, 0),
                        )
                    })?;

                    let field_info = struct_def.fields.iter().find(|f| &f.name == field_name);
                    if let Some(field) = field_info {
                        let field_ptr = builder.ins().iadd_imm(obj_ptr, field.offset);
                        Ok(field_ptr)
                    } else {
                        Err(CodeGenError::ModuleError(
                            format!("Struct {} has no field {}", struct_name, field_name),
                            Span::new(0, 0),
                        ))
                    }
                }
                Expr::Index(arr, index_expr, _) => {
                    let arr_ptr = Self::compile_expr(
                        arr,
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
                        index_expr,
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
                    let element_ptr = builder.ins().iadd(arr_ptr, offset);
                    Ok(element_ptr)
                }
                _ => Err(CodeGenError::ModuleError(
                    "AddressOf only supported on variables, member access, or array index"
                        .to_string(),
                    Span::new(0, 0),
                )),
            },
            Expr::Deref(expr, _) => {
                let ptr = Self::compile_expr(
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
                let value = builder
                    .ins()
                    .load(types::I64, ir::MemFlags::trusted(), ptr, 0);
                Ok(value)
            }
            Expr::DerefAssign(ptr_expr, val_expr, _) => {
                let ptr = Self::compile_expr(
                    ptr_expr,
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
                let val = Self::compile_expr(
                    val_expr,
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
                builder.ins().store(ir::MemFlags::trusted(), val, ptr, 0);
                Ok(builder.ins().iconst(types::I64, 0))
            }
            _ => Err(CodeGenError::UnexpectedExpression {
                found: expr.clone(),
                span: expr.span(),
            }),
        }
    }
}
