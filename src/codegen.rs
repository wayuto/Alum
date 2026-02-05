use crate::ast::{Expr, Program, Type};
use cranelift::{
    codegen::{
        Context,
        ir::{self, BlockArg, Function, UserFuncName},
        settings,
    },
    prelude::{
        AbiParam, Configurable, FunctionBuilder, FunctionBuilderContext, InstBuilder, Signature,
        Value, Variable,
        isa::{self, CallConv},
        types,
    },
};
use cranelift_module::{FuncId, Linkage, Module};
use cranelift_object::{ObjectBuilder, ObjectModule, ObjectProduct};
use std::{collections::HashMap, error::Error, fmt::Display};

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
    ctx: Context,
    func_signatures: HashMap<String, (FuncId, Signature)>,
}

impl CodeGen {
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
        let ctx = module.make_context();
        Self {
            ast,
            module,
            builder_context: FunctionBuilderContext::new(),
            ctx,
            func_signatures: HashMap::new(),
        }
    }

    pub fn generate(mut self) -> Result<Vec<u8>, CodeGenError> {
        for expr in self.ast.body.clone() {
            match expr {
                Expr::FuncDecl(name, params, ret_type, body) => {
                    let mut sig = self.module.make_signature();
                    for (_, t) in params {
                        sig.params.push(AbiParam::new(get_type(t)));
                    }
                    sig.returns.push(AbiParam::new(get_type(ret_type)));
                    let func_id = self
                        .module
                        .declare_function(name.as_str(), Linkage::Export, &sig)
                        .unwrap();
                    self.func_signatures.insert(name, (func_id, sig));
                }
                expr => return Err(CodeGenError::UnexpectedExpression { found: expr }),
            }
        }
        for expr in self.ast.body.clone() {
            match expr {
                Expr::FuncDecl(name, params, ret_type, body) => {
                    self.compile_func(name, params, ret_type, body)?;
                }
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
        ret_type: Type,
        body: Box<Expr>,
    ) -> Result<(), CodeGenError> {
        let (func_id, ref sig) = self.func_signatures[&name].clone();

        let mut new_ctx = self.module.make_context();
        new_ctx.func.signature = sig.clone();

        let param_types: Vec<ir::Type> = params.iter().map(|(_, ty)| get_type(*ty)).collect();

        let mut builder = FunctionBuilder::new(&mut new_ctx.func, &mut self.builder_context);
        let entry_block = builder.create_block();
        builder.append_block_params_for_function_params(entry_block);
        builder.switch_to_block(entry_block);
        builder.seal_block(entry_block);

        let mut vars: HashMap<String, Variable> = HashMap::new();
        let mut idx = 0;

        for (i, (param_name, _)) in params.iter().enumerate() {
            let val = builder.block_params(entry_block)[i];
            let var = Variable::from_u32(idx);
            idx += 1;
            builder.declare_var(param_types[i]);
            builder.def_var(var, val);
            vars.insert(param_name.clone(), var);
        }

        Self::compile_expr(
            &body,
            &mut builder,
            &mut vars,
            &mut idx,
            &self.func_signatures,
            &mut self.module,
        )?;

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

    fn compile_expr(
        expr: &Expr,
        builder: &mut FunctionBuilder,
        vars: &mut HashMap<String, Variable>,
        idx: &mut u32,
        func_signatures: &HashMap<String, (FuncId, Signature)>,
        module: &mut ObjectModule,
    ) -> Result<Value, CodeGenError> {
        match expr {
            Expr::Stmt(body) => {
                if body.len() > 1 {
                    let (last, body) = body.split_last().unwrap();
                    for expr in body {
                        Self::compile_expr(expr, builder, vars, idx, func_signatures, module)?;
                    }
                    Self::compile_expr(last, builder, vars, idx, func_signatures, module)
                } else {
                    Self::compile_expr(
                        body.last().unwrap(),
                        builder,
                        vars,
                        idx,
                        func_signatures,
                        module,
                    )
                }
            }
            Expr::Int(i) => Ok(builder.ins().iconst(types::I64, *i as i64)),
            Expr::Bool(b) => Ok(builder.ins().iconst(types::I8, *b as i64)),
            Expr::Add(lhs, rhs)
            | Expr::Sub(lhs, rhs)
            | Expr::Mul(lhs, rhs)
            | Expr::Div(lhs, rhs) => {
                let lhs = Self::compile_expr(lhs, builder, vars, idx, func_signatures, module)?;
                let rhs = Self::compile_expr(rhs, builder, vars, idx, func_signatures, module)?;
                match expr {
                    Expr::Add(_, _) => Ok(builder.ins().iadd(lhs, rhs)),
                    Expr::Sub(_, _) => Ok(builder.ins().isub(lhs, rhs)),
                    Expr::Mul(_, _) => Ok(builder.ins().imul(lhs, rhs)),
                    Expr::Div(_, _) => Ok(builder.ins().sdiv(lhs, rhs)),
                    _ => unreachable!(),
                }
            }
            Expr::VarDecl(name, ty, value) => {
                let val =
                    Self::compile_expr(&**value, builder, vars, idx, func_signatures, module)?;
                let var = Variable::from_u32(*idx);
                *idx += 1;
                builder.declare_var(get_type(*ty));
                builder.def_var(var, val);
                vars.insert(name.clone(), var);
                Ok(Value::from_u32(0))
            }
            Expr::Var(name) => {
                let var = match vars.get(name) {
                    Some(v) => v,
                    None => return Err(CodeGenError::UndefinedVariable { name: name.clone() }),
                };
                Ok(builder.use_var(*var))
            }
            Expr::FuncCall(name, args) => {
                let (func_id, _) = match func_signatures.get(name) {
                    Some(sig) => sig,
                    None => return Err(CodeGenError::UndefinedFunction { name: name.clone() }),
                };

                let func_ref = module.declare_func_in_func(*func_id, builder.func);

                let arg_values: Result<Vec<Value>, CodeGenError> = args
                    .iter()
                    .map(|a| Self::compile_expr(a, builder, vars, idx, func_signatures, module))
                    .collect();

                let arg_values = arg_values?;
                let call = builder.ins().call(func_ref, &arg_values);
                Ok(builder.inst_results(call)[0])
            }
            Expr::Return(value) => {
                let val =
                    Self::compile_expr(&**value, builder, vars, idx, func_signatures, module)?;
                builder.ins().return_(&[val]);
                Ok(Value::from_u32(0))
            }
            Expr::If(cond, then_branch, else_branch) => {
                let cond_val =
                    Self::compile_expr(cond, builder, vars, idx, func_signatures, module)?;

                let then_block = builder.create_block();
                let else_block = builder.create_block();
                let merge_block = builder.create_block();

                let cond_i64 = builder.ins().uextend(types::I64, cond_val);
                builder
                    .ins()
                    .brif(cond_i64, then_block, &[], else_block, &[]);

                builder.switch_to_block(then_block);
                let then_val =
                    Self::compile_expr(then_branch, builder, vars, idx, func_signatures, module)?;
                builder
                    .ins()
                    .jump(merge_block, &[BlockArg::Value(then_val)]);
                builder.seal_block(then_block);

                builder.switch_to_block(else_block);
                let else_val = if let Some(else_expr) = else_branch {
                    Self::compile_expr(else_expr, builder, vars, idx, func_signatures, module)?
                } else {
                    builder.ins().iconst(types::I64, 0)
                };
                builder
                    .ins()
                    .jump(merge_block, &[BlockArg::Value(else_val)]);
                builder.seal_block(else_block);

                builder.switch_to_block(merge_block);

                let merge_param = builder.append_block_param(merge_block, types::I64);
                builder.seal_block(merge_block);

                Ok(merge_param)
            }
            Expr::While(cond, body) => {
                let loop_header = builder.create_block();
                let loop_body = builder.create_block();
                let loop_exit = builder.create_block();

                builder.ins().jump(loop_header, &[]);

                builder.switch_to_block(loop_header);
                let cond_val =
                    Self::compile_expr(cond, builder, vars, idx, func_signatures, module)?;
                let cond_i64 = builder.ins().uextend(types::I64, cond_val);
                builder.ins().brif(cond_i64, loop_body, &[], loop_exit, &[]);

                builder.switch_to_block(loop_body);
                Self::compile_expr(body, builder, vars, idx, func_signatures, module)?;
                builder.ins().jump(loop_header, &[]);

                builder.seal_block(loop_body);
                builder.seal_block(loop_header);

                builder.switch_to_block(loop_exit);
                builder.seal_block(loop_exit);

                Ok(builder.ins().iconst(types::I64, 0))
            }
            _ => Err(CodeGenError::UnexpectedExpression {
                found: expr.clone(),
            }),
        }
    }
}

fn get_type(t: Type) -> ir::Type {
    return match t {
        Type::Int => types::I64,
        Type::Bool => types::I8,
        Type::Void => types::INVALID,
    };
}
