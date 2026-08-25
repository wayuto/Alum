use super::context::Context;
use super::ir::{IRConst, IRType, Instruction, Op, Operand};
use crate::compiler::{
    Span,
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
        if matches!(&*callee, Expr::Var(n, _) if n == "_alum_copy") {
            let arg = args.first().ok_or_else(|| CodeGenError::TypeError {
                message: String::from("_alum_copy requires exactly one argument"),
            })?;
            let src = self.compile_expr(arg.clone(), ctx)?;
            let hty = self.expr_high_type(arg, ctx);
            return match hty {
                Some(ty) if self.is_resource_type(&ty) => self.copy_resource(ctx, src, &ty),
                _ => Ok(src),
            };
        }

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

            let mut moved_args: Vec<(String, Span)> = Vec::new();
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
                        if let Expr::Var(src, sp) = arg {
                            if self.can_move_var(src, ctx) {
                                ctx.mark_moved(src, *sp);
                                moved_args.push((src.clone(), *sp));
                                operand
                            } else {
                                self.copy_resource(ctx, operand, &hty)?
                            }
                        } else {
                            self.copy_resource(ctx, operand, &hty)?
                        }
                    }
                    _ => operand,
                };
                evaluated.push((operand, param.1.clone()));
                n += 1;
            }
            let mut int_idx = 0usize;
            let mut flt_idx = 0usize;
            for (operand, ir_type) in evaluated.iter() {
                match ir_type {
                    IRType::Float => {
                        ctx.instructions.push(Instruction {
                            op: Op::FArg(flt_idx),
                            dst: None,
                            src1: Some(operand.clone()),
                            src2: None,
                        });
                        flt_idx += 1;
                    }
                    _ => {
                        ctx.instructions.push(Instruction {
                            op: Op::Arg(int_idx),
                            dst: None,
                            src1: Some(operand.clone()),
                            src2: None,
                        });
                        int_idx += 1;
                    }
                }
            }

            let zero_idx = self.get_const_index(IRConst::Int(0));
            for (src_name, _) in &moved_args {
                ctx.instructions.push(Instruction {
                    op: Op::Store,
                    dst: Some(Operand::Var(ctx.slot(src_name.as_str()))),
                    src1: Some(Operand::ConstIdx(zero_idx)),
                    src2: None,
                });
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
                    Some(ty) => {
                        if let Expr::Var(src, arg_span) = arg {
                            if self.can_move_var(src, ctx) {
                                let h = ctx.new_tmp(IRType::Int);
                                ctx.instructions.push(Instruction {
                                    op: Op::Load,
                                    dst: Some(h.clone()),
                                    src1: Some(operand.clone()),
                                    src2: None,
                                });
                                let zero_idx = self.get_const_index(IRConst::Int(0));
                                let src_name = src.clone();
                                ctx.instructions.push(Instruction {
                                    op: Op::Store,
                                    dst: Some(Operand::Var(ctx.slot(&src_name))),
                                    src1: Some(Operand::ConstIdx(zero_idx)),
                                    src2: None,
                                });
                                ctx.mark_moved(src, *arg_span);
                                evaluated.push(h);
                                continue;
                            }
                        }
                        self.copy_resource(ctx, operand, &ty)?
                    }
                    None => operand,
                };
                evaluated.push(operand);
            }
            let mut int_idx = 0usize;
            let mut flt_idx = 0usize;
            for operand in evaluated.iter() {
                let op = if matches!(
                    ctx.get_operand_type(operand, &self.constants)?,
                    IRType::Float
                ) {
                    flt_idx += 1;
                    Op::FArg(flt_idx - 1)
                } else {
                    int_idx += 1;
                    Op::Arg(int_idx - 1)
                };
                ctx.instructions.push(Instruction {
                    op,
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

        let is_ptr_src = matches!(self.expr_high_type(&arr, ctx), Some(Type::Pointer(_)));
        let arr_op = self.compile_expr(*arr, ctx)?;
        let offset = self.compile_expr(*idx, ctx)?;
        let res_tmp = ctx.new_tmp(elem_ir_type.clone());
        if is_ptr_src && !byte {
            let scaled = ctx.new_tmp(IRType::Int);
            let scale_idx = self.get_const_index(IRConst::Int(8));
            ctx.instructions.push(Instruction {
                op: Op::Mul,
                dst: Some(scaled.clone()),
                src1: Some(offset),
                src2: Some(Operand::ConstIdx(scale_idx)),
            });

            let load_op = if elem_ir_type == IRType::Float {
                Op::FLoadAt
            } else {
                Op::LoadAt
            };
            ctx.instructions.push(Instruction {
                op: load_op,
                dst: Some(res_tmp.clone()),
                src1: Some(arr_op),
                src2: Some(scaled),
            });
            return Ok(res_tmp);
        }
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

            let mut int_idx = 0usize;
            let mut flt_idx = 0usize;
            for op in [&s_op, &i_op, &v_op] {
                let ty = ctx.get_operand_type(op, &self.constants)?;
                if ty == IRType::Float {
                    ctx.instructions.push(Instruction {
                        op: Op::FArg(flt_idx),
                        dst: None,
                        src1: Some(op.clone()),
                        src2: None,
                    });
                    flt_idx += 1;
                } else {
                    ctx.instructions.push(Instruction {
                        op: Op::Arg(int_idx),
                        dst: None,
                        src1: Some(op.clone()),
                        src2: None,
                    });
                    int_idx += 1;
                }
            }
            let res_tmp = ctx.new_tmp(IRType::Void);
            ctx.instructions.push(Instruction {
                op: Op::Call,
                dst: Some(res_tmp.clone()),
                src1: Some(fn_ptr),
                src2: None,
            });
            return Ok(v_op);
        }
        let (elem_type, byte) = self.index_info(&arr, ctx);

        let is_ptr_src = matches!(self.expr_high_type(&arr, ctx), Some(Type::Pointer(_)));
        let arr_op = self.compile_expr(*arr, ctx)?;
        let offset = self.compile_expr(*idx, ctx)?;
        let value_copy_info = self.resource_copy_info(&value, ctx);
        let val = self.compile_expr(*value, ctx)?;
        let val = if let Some(et) = &elem_type {
            if self.is_resource_type(et) {
                let val = match value_copy_info {
                    Some(ty) => self.copy_resource(ctx, val, &ty)?,
                    None => val,
                };
                let old_tmp = ctx.new_tmp(IRType::Int);
                if is_ptr_src && !byte {
                    let scaled_old = ctx.new_tmp(IRType::Int);
                    let scale_idx = self.get_const_index(IRConst::Int(8));
                    ctx.instructions.push(Instruction {
                        op: Op::Mul,
                        dst: Some(scaled_old.clone()),
                        src1: Some(offset.clone()),
                        src2: Some(Operand::ConstIdx(scale_idx)),
                    });
                    ctx.instructions.push(Instruction {
                        op: Op::LoadAt,
                        dst: Some(old_tmp.clone()),
                        src1: Some(arr_op.clone()),
                        src2: Some(scaled_old),
                    });
                } else {
                    ctx.instructions.push(Instruction {
                        op: Op::ArrayAccess,
                        dst: Some(old_tmp.clone()),
                        src1: Some(arr_op.clone()),
                        src2: Some(offset.clone()),
                    });
                }
                self.emit_free(ctx, old_tmp, &et)?;
                val
            } else {
                val
            }
        } else {
            val
        };
        let result = val.clone();
        if is_ptr_src && !byte {
            let scaled = ctx.new_tmp(IRType::Int);
            let scale_idx = self.get_const_index(IRConst::Int(8));
            ctx.instructions.push(Instruction {
                op: Op::Mul,
                dst: Some(scaled.clone()),
                src1: Some(offset),
                src2: Some(Operand::ConstIdx(scale_idx)),
            });

            let elem_is_float = elem_type
                .as_ref()
                .map(|t| Context::type_to_ir_type(t) == IRType::Float)
                .unwrap_or(false);
            let store_op = if elem_is_float {
                Op::FStoreAt
            } else {
                Op::StoreAt
            };
            ctx.instructions.push(Instruction {
                op: store_op,
                dst: Some(arr_op),
                src1: Some(scaled),
                src2: Some(val),
            });
            return Ok(result);
        }
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
