use super::context::{Context, Symbol};
use super::ir::{IRFunction, IRType, Instruction, Op, Operand};
use crate::compiler::{
    codegen::CodeGenError,
    irgen::IRGen,
    parser::{Expr, Type},
};
use std::mem::take;

impl IRGen {
    pub(super) fn func_decl(
        &mut self,
        name: String,
        params: Vec<(String, Type)>,
        ret_type: Type,
    ) -> Result<(), CodeGenError> {
        let ir_params: Vec<(Operand, IRType)> = params
            .iter()
            .map(|(name, typ)| (Operand::Var(name.clone()), Context::type2ir_type(typ)))
            .collect();
        let ir_ret_type = Context::type2ir_type(&ret_type);
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

    pub(super) fn compile_fn(
        &mut self,
        name: String,
        params: Vec<(String, Type)>,
        body: Expr,
    ) -> Result<(), CodeGenError> {
        let func = self.find_func(&name)?;

        let mut ctx = Context::new(name.clone());
        ctx.enter_scope();

        for (i, (param, ty)) in func.params.iter().enumerate() {
            if let Operand::Var(pname) = param {
                if let Some(scope) = ctx.scope.last_mut() {
                    scope.insert(
                        pname.clone(),
                        Symbol {
                            name: pname.clone(),
                            ir_type: ty.clone(),
                        },
                    );
                }
                if let Some((_, hty)) = params.get(i) {
                    ctx.var_types.insert(pname.clone(), hty.clone());
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

        if let Some(f) = self.functions.iter_mut().rev().find(|f| f.name == name) {
            f.instructions = take(&mut ctx.instructions);
        }
        Ok(())
    }

    pub(super) fn extern_decl(
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
                (Operand::Var(param_name), Context::type2ir_type(&typ))
            })
            .collect();
        let ir_ret_type = Context::type2ir_type(&ret_type);
        let signature = IRFunction {
            name: name.clone(),
            params: ir_params,
            ret_type: ir_ret_type,
            instructions: Vec::new(),
            is_pub: false,
            is_external: true,
        };
        let already_defined = self
            .functions
            .iter()
            .any(|f| f.name == name && !f.is_external);
        if !already_defined {
            self.functions.push(signature);
        }
        Ok(())
    }

    pub(super) fn find_func(&self, name: &str) -> Result<IRFunction, CodeGenError> {
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
