use super::context::Context;
use super::ir::{IRType, Instruction, Op, Operand};
use crate::compiler::{
    codegen::CodeGenError,
    irgen::IRGen,
    parser::{Expr, Primitive, Type},
};
use std::iter::zip;

impl IRGen {
    pub(super) fn compile_call(
        &mut self,
        callee: Box<Expr>,
        type_args: Vec<Type>,
        args: Vec<Expr>,
        ctx: &mut Context,
    ) -> Result<Operand, CodeGenError> {
        let func_name = match &*callee {
            Expr::Var(name, _) => {
                if self.find_func(name).is_ok() {
                    Some(name.clone())
                } else if self.generic_funcs.contains_key(name) {
                    Some(self.monomorphize(name, &type_args)?)
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
            let mut evaluated: Vec<(Operand, IRType)> = Vec::new();
            let mut n = 0;
            for (arg, param) in zip(args.iter(), func.params.iter()) {
                let operand = self.compile_expr(arg.clone(), ctx)?;
                let operand_type = ctx.get_operand_type(&operand, &self.constants)?;
                let type_matches = operand_type == param.1
                    || (operand_type == IRType::Array && param.1 == IRType::Int)
                    || (matches!(operand, Operand::Function(_)) && param.1 == IRType::Int);
                if !type_matches {
                    return Err(CodeGenError::TypeError {
                        message: format!(
                            "unexpected type {:?}, expected {:?} (arg {} of '{}')",
                            operand_type, param.1, n, name
                        ),
                    });
                }
                let operand = match self.expr_high_type(arg, ctx) {
                    Some(hty) if self.is_resource_type(&hty) && !self.is_fresh_expr(arg) => {
                        self.copy_resource(ctx, operand, &hty)?
                    }
                    _ => operand,
                };
                evaluated.push((operand, param.1.clone()));
                n += 1;
            }
            for (n, (operand, ir_type)) in evaluated.iter().enumerate() {
                match ir_type {
                    IRType::Float => ctx.instructions.push(Instruction {
                        op: Op::FArg(n),
                        dst: None,
                        src1: Some(operand.clone()),
                        src2: None,
                    }),
                    _ => ctx.instructions.push(Instruction {
                        op: Op::Arg(n),
                        dst: None,
                        src1: Some(operand.clone()),
                        src2: None,
                    }),
                }
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
            let ret_ir_type = self.member_call_ret_type(&callee, ctx);
            let callee_op = self.compile_expr(*callee, ctx)?;
            let mut evaluated: Vec<Operand> = Vec::new();
            for arg in args.iter() {
                let operand = self.compile_expr(arg.clone(), ctx)?;
                let operand = match self.resource_copy_info(arg, ctx) {
                    Some(ty) => self.copy_resource(ctx, operand, &ty)?,
                    None => operand,
                };
                evaluated.push(operand);
            }
            for (n, operand) in evaluated.iter().enumerate() {
                ctx.instructions.push(Instruction {
                    op: Op::Arg(n as usize),
                    dst: None,
                    src1: Some(operand.clone()),
                    src2: None,
                });
            }
            let res_tmp = ctx.new_tmp(ret_ir_type);
            ctx.instructions.push(Instruction {
                op: Op::Call,
                dst: Some(res_tmp.clone()),
                src1: Some(callee_op),
                src2: None,
            });
            Ok(res_tmp)
        }
    }

    pub(super) fn compile_index(
        &mut self,
        arr: Box<Expr>,
        idx: Box<Expr>,
        ctx: &mut Context,
    ) -> Result<Operand, CodeGenError> {
        if let Some(Type::Struct(sname, ta)) = self.expr_high_type(&arr, ctx) {
            let ret_ty = self
                .struct_field_fn_ret(&sname, &ta, "nth")
                .ok_or_else(|| CodeGenError::TypeError {
                    message: format!("type '{}' has no 'nth' method", sname),
                })?;
            let ret_ir = Context::type_to_ir_type(&ret_ty);
            let s_op = self.compile_expr(*arr, ctx)?;
            let i_op = self.compile_expr(*idx, ctx)?;
            let fn_ptr =
                self.load_function_field(s_op.clone(), &Type::Struct(sname, ta), "nth", ctx)?;
            ctx.instructions.push(Instruction {
                op: Op::Arg(0),
                dst: None,
                src1: Some(s_op),
                src2: None,
            });
            ctx.instructions.push(Instruction {
                op: Op::Arg(1),
                dst: None,
                src1: Some(i_op),
                src2: None,
            });
            let res_tmp = ctx.new_tmp(ret_ir);
            ctx.instructions.push(Instruction {
                op: Op::Call,
                dst: Some(res_tmp.clone()),
                src1: Some(fn_ptr),
                src2: None,
            });
            return Ok(res_tmp);
        }
        let (elem_type, byte) = self.index_info(&arr, ctx);
        let is_string_index = byte && matches!(elem_type, Some(Type::Primitive(Primitive::String)));
        let elem_ir_type = if is_string_index {
            IRType::String
        } else {
            elem_type.map_or(IRType::Int, |t| Context::type_to_ir_type(&t))
        };
        let arr_op = self.compile_expr(*arr, ctx)?;
        let offset = self.compile_expr(*idx, ctx)?;
        let res_tmp = ctx.new_tmp(elem_ir_type);
        ctx.instructions.push(Instruction {
            op: if is_string_index {
                Op::StrByte
            } else if byte {
                Op::ByteAccess
            } else {
                Op::ArrayAccess
            },
            dst: Some(res_tmp.clone()),
            src1: Some(arr_op),
            src2: Some(offset),
        });
        Ok(res_tmp)
    }

    pub(super) fn compile_index_assign(
        &mut self,
        arr_idx: Box<Expr>,
        value: Box<Expr>,
        ctx: &mut Context,
    ) -> Result<Operand, CodeGenError> {
        let (arr, idx) = match *arr_idx {
            Expr::Index(arr, idx, _) => (arr, idx),
            expr => {
                return Err(CodeGenError::TypeError {
                    message: format!("expected index expression, got {:?}", expr),
                });
            }
        };
        if let Some(Type::Struct(sname, ta)) = self.expr_high_type(&arr, ctx) {
            self.struct_field_fn_ret(&sname, &ta, "set_nth")
                .ok_or_else(|| CodeGenError::TypeError {
                    message: format!("type '{}' has no 'set_nth' method", sname),
                })?;
            let s_op = self.compile_expr(*arr, ctx)?;
            let i_op = self.compile_expr(*idx, ctx)?;
            let v_copy_info = self.resource_copy_info(&value, ctx);
            let v_op = self.compile_expr(*value, ctx)?;
            let v_op = match v_copy_info {
                Some(ty) => self.copy_resource(ctx, v_op, &ty)?,
                None => v_op,
            };
            let fn_ptr =
                self.load_function_field(s_op.clone(), &Type::Struct(sname, ta), "set_nth", ctx)?;
            ctx.instructions.push(Instruction {
                op: Op::Arg(0),
                dst: None,
                src1: Some(s_op),
                src2: None,
            });
            ctx.instructions.push(Instruction {
                op: Op::Arg(1),
                dst: None,
                src1: Some(i_op),
                src2: None,
            });
            let v_result = v_op.clone();
            ctx.instructions.push(Instruction {
                op: Op::Arg(2),
                dst: None,
                src1: Some(v_op),
                src2: None,
            });
            let res_tmp = ctx.new_tmp(IRType::Void);
            ctx.instructions.push(Instruction {
                op: Op::Call,
                dst: Some(res_tmp.clone()),
                src1: Some(fn_ptr),
                src2: None,
            });
            return Ok(v_result);
        }
        let (elem_type, byte) = self.index_info(&arr, ctx);
        let arr_op = self.compile_expr(*arr, ctx)?;
        let offset = self.compile_expr(*idx, ctx)?;
        let value_copy_info = self.resource_copy_info(&value, ctx);
        let val = self.compile_expr(*value, ctx)?;
        let val = if let Some(et) = elem_type {
            if self.is_resource_type(&et) {
                let val = match value_copy_info {
                    Some(ty) => self.copy_resource(ctx, val, &ty)?,
                    None => val,
                };
                let old_tmp = ctx.new_tmp(IRType::Int);
                ctx.instructions.push(Instruction {
                    op: Op::ArrayAccess,
                    dst: Some(old_tmp.clone()),
                    src1: Some(arr_op.clone()),
                    src2: Some(offset.clone()),
                });
                self.emit_free(ctx, old_tmp, &et)?;
                val
            } else {
                val
            }
        } else {
            val
        };
        let result = val.clone();
        ctx.instructions.push(Instruction {
            op: if byte {
                Op::ByteAssign
            } else {
                Op::ArrayAssign
            },
            dst: Some(arr_op),
            src1: Some(offset),
            src2: Some(val),
        });
        Ok(result)
    }
}
