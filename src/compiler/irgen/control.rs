use super::context::Context;
use super::ir::{IRConst, IRType, Instruction, Op, Operand};
use crate::compiler::{
    codegen::CodeGenError,
    irgen::IRGen,
    parser::{Expr, Primitive, Type},
};

impl IRGen {
    pub(super) fn compile_scoped_value(
        &mut self,
        expr: Expr,
        res_tmp: &Operand,
        ctx: &mut Context,
    ) -> Result<(), CodeGenError> {
        ctx.enter_scope();
        let info = self.resource_copy_info(&expr, ctx);
        let value = self.compile_expr(expr, ctx)?;
        let value = match info {
            Some(ty) => self.copy_resource(ctx, value, &ty)?,
            None => value,
        };
        ctx.instructions.push(Instruction {
            op: Op::Move,
            dst: Some(res_tmp.clone()),
            src1: Some(value),
            src2: None,
        });
        self.emit_scope_frees(ctx)?;
        ctx.exit_scope()?;
        Ok(())
    }

    pub(super) fn compile_if(
        &mut self,
        condition: Box<Expr>,
        then_branch: Box<Expr>,
        else_branch: Option<Box<Expr>>,
        ctx: &mut Context,
    ) -> Result<Operand, CodeGenError> {
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

        self.compile_scoped_value(*then_branch, &res_tmp, ctx)?;

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
            self.compile_scoped_value(*else_expr, &res_tmp, ctx)?;
        }

        ctx.instructions.push(Instruction {
            op: Op::Label(label_end),
            dst: None,
            src1: None,
            src2: None,
        });

        Ok(res_tmp)
    }

    pub(super) fn compile_while(
        &mut self,
        condition: Box<Expr>,
        body: Box<Expr>,
        ctx: &mut Context,
    ) -> Result<Operand, CodeGenError> {
        let label_start = ctx.new_label("while_start");
        let label_cont = ctx.new_label("while_cont");
        let label_end = ctx.new_label("while_end");

        ctx.loop_end_labels.push(label_end.clone());
        ctx.loop_inc_labels.push(label_cont.clone());

        ctx.loop_scope_depths.push(ctx.scope.len());
        ctx.loop_results.push(None);

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

        let moved_before = ctx.moved.clone();
        ctx.enter_scope();
        self.compile_expr(*body, ctx)?;
        self.emit_scope_frees(ctx)?;
        ctx.exit_scope()?;
        self.check_loop_moves(&moved_before, ctx)?;

        ctx.instructions.push(Instruction {
            op: Op::Label(label_cont.clone()),
            dst: None,
            src1: None,
            src2: None,
        });

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
        ctx.loop_scope_depths.pop();

        Ok(self.take_loop_result(ctx))
    }

    pub(super) fn compile_for(
        &mut self,
        var: String,
        iter: Box<Expr>,
        body: Box<Expr>,
        ctx: &mut Context,
    ) -> Result<Operand, CodeGenError> {
        if let Expr::Range(start, end, _) = *iter {
            let start_op = self.compile_expr(*start, ctx)?;
            let end_op = self.compile_expr(*end, ctx)?;
            let label_cond = ctx.new_label("rfor_cond");
            let label_end = ctx.new_label("rfor_end");
            let label_inc = ctx.new_label("rfor_inc");
            ctx.loop_end_labels.push(label_end.clone());
            ctx.loop_inc_labels.push(label_inc.clone());
            ctx.loop_scope_depths.push(ctx.scope.len());
            ctx.loop_results.push(None);
            let moved_before = ctx.moved.clone();
            ctx.enter_scope();
            let idx_name = ctx.new_label("idx");
            let idx_var = Operand::Var(idx_name.clone());
            ctx.declare_var(idx_name.clone(), IRType::Int)?;
            ctx.instructions.push(Instruction {
                op: Op::Store,
                dst: Some(idx_var.clone()),
                src1: Some(start_op),
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
                src2: Some(end_op),
            });
            ctx.instructions.push(Instruction {
                op: Op::JumpIfFalse,
                dst: None,
                src1: Some(cond_tmp),
                src2: Some(Operand::Label(label_end.clone())),
            });
            ctx.declare_var(var.clone(), IRType::Int)?;
            ctx.var_types
                .insert(var.clone(), Type::Primitive(Primitive::Int));
            ctx.instructions.push(Instruction {
                op: Op::Store,
                dst: Some(Operand::Var(ctx.slot(&var))),
                src1: Some(curr_idx),
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
            self.emit_scope_frees(ctx)?;
            ctx.exit_scope()?;
            self.check_loop_moves(&moved_before, ctx)?;
            ctx.loop_end_labels.pop();
            ctx.loop_inc_labels.pop();
            ctx.loop_scope_depths.pop();
            return Ok(self.take_loop_result(ctx));
        }
        if let Some(Type::Struct(sname, ta)) = self.expr_high_type(&iter, ctx) {
            let maybe_ty = self
                .struct_field_fn_ret(&sname, &ta, "next")
                .ok_or_else(|| CodeGenError::TypeError {
                    message: format!("type '{}' has no 'next' method", sname),
                })?;
            let elem_ir = match &maybe_ty {
                Type::Struct(mname, margs) if crate::compiler::is_maybe_type_name(mname) => margs
                    .first()
                    .map(Context::type_to_ir_type)
                    .unwrap_or(IRType::Int),
                _ => {
                    return Err(CodeGenError::TypeError {
                        message: format!(
                            "'next' of '{}' must return Maybe<T>, got {}",
                            sname, maybe_ty
                        ),
                    });
                }
            };
            let elem_hty = match &maybe_ty {
                Type::Struct(mname, margs) if crate::compiler::is_maybe_type_name(mname) => {
                    margs.first().cloned()
                }
                _ => None,
            };
            let s_op = self.compile_expr(*iter, ctx)?;
            let fn_ptr =
                self.load_function_field(s_op.clone(), &Type::Struct(sname, ta), "next", ctx)?;

            let label_cond = ctx.new_label("nfor_cond");
            let label_end = ctx.new_label("nfor_end");

            ctx.loop_end_labels.push(label_end.clone());
            ctx.loop_inc_labels.push(label_cond.clone());

            ctx.loop_scope_depths.push(ctx.scope.len());
            ctx.loop_results.push(None);
            let moved_before = ctx.moved.clone();
            ctx.enter_scope();

            ctx.instructions.push(Instruction {
                op: Op::Label(label_cond.clone()),
                dst: None,
                src1: None,
                src2: None,
            });

            ctx.instructions.push(Instruction {
                op: Op::Arg(0),
                dst: None,
                src1: Some(s_op),
                src2: None,
            });
            let maybe_tmp = ctx.new_tmp(IRType::Int);
            ctx.instructions.push(Instruction {
                op: Op::Call,
                dst: Some(maybe_tmp.clone()),
                src1: Some(fn_ptr),
                src2: None,
            });

            let zero_idx = self.get_const_index(IRConst::Int(0));
            let tag_tmp = ctx.new_tmp(IRType::Int);
            ctx.instructions.push(Instruction {
                op: Op::LoadAt,
                dst: Some(tag_tmp.clone()),
                src1: Some(maybe_tmp.clone()),
                src2: Some(Operand::ConstIdx(zero_idx)),
            });

            let cond_tmp = ctx.new_tmp(IRType::Bool);
            ctx.instructions.push(Instruction {
                op: Op::Ne,
                dst: Some(cond_tmp.clone()),
                src1: Some(tag_tmp),
                src2: Some(Operand::ConstIdx(zero_idx)),
            });
            ctx.instructions.push(Instruction {
                op: Op::JumpIfFalse,
                dst: None,
                src1: Some(cond_tmp),
                src2: Some(Operand::Label(label_end.clone())),
            });

            let eight_idx = self.get_const_index(IRConst::Int(8));
            let val_tmp = ctx.new_tmp(elem_ir.clone());
            ctx.instructions.push(Instruction {
                op: Op::LoadAt,
                dst: Some(val_tmp.clone()),
                src1: Some(maybe_tmp),
                src2: Some(Operand::ConstIdx(eight_idx)),
            });

            ctx.declare_var(var.clone(), elem_ir.clone())?;
            ctx.borrowed.insert(var.clone());
            if let Some(elem_hty) = elem_hty {
                ctx.var_types.insert(var.clone(), elem_hty);
            }
            ctx.instructions.push(Instruction {
                op: Op::Store,
                dst: Some(Operand::Var(ctx.slot(&var))),
                src1: Some(val_tmp),
                src2: None,
            });

            self.compile_expr(*body, ctx)?;

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

            self.emit_scope_frees(ctx)?;
            ctx.exit_scope()?;
            self.check_loop_moves(&moved_before, ctx)?;
            ctx.loop_end_labels.pop();
            ctx.loop_inc_labels.pop();
            ctx.loop_scope_depths.pop();

            return Ok(self.take_loop_result(ctx));
        }
        let is_string = matches!(
            self.expr_high_type(&iter, ctx),
            Some(Type::Primitive(Primitive::String))
        );
        let (elem_hty, _) = self.index_info(&iter, ctx);
        let elem_ir_type = elem_hty
            .as_ref()
            .map_or(IRType::Int, |t| Context::type_to_ir_type(t));
        let known_len = match &*iter {
            Expr::ArrayLiteral(elements, _) => Some(elements.len()),
            Expr::Var(name, _) => ctx.array_lengths.get(name).copied(),
            _ => None,
        };
        let array_operand = self.compile_expr(*iter, ctx)?;

        let array_len_operand = if is_string {
            let zero_idx = self.get_const_index(IRConst::Int(0));
            ctx.instructions.push(Instruction {
                op: Op::Arg(0),
                dst: None,
                src1: Some(array_operand.clone()),
                src2: Some(Operand::ConstIdx(zero_idx)),
            });
            let len_tmp = ctx.new_tmp(IRType::Int);
            ctx.instructions.push(Instruction {
                op: Op::Call,
                dst: Some(len_tmp.clone()),
                src1: Some(Operand::Function("strlen".to_string())),
                src2: None,
            });
            len_tmp
        } else if let Some(len) = known_len {
            let idx = self.get_const_index(IRConst::Int(len as i64));
            Operand::ConstIdx(idx)
        } else {
            let len_tmp = ctx.new_tmp(IRType::Int);
            ctx.instructions.push(Instruction {
                op: Op::SizeOf,
                dst: Some(len_tmp.clone()),
                src1: Some(array_operand.clone()),
                src2: None,
            });
            len_tmp
        };

        let label_cond = ctx.new_label("for_cond");
        let label_end = ctx.new_label("for_end");
        let label_inc = ctx.new_label("for_inc");

        ctx.loop_end_labels.push(label_end.clone());
        ctx.loop_inc_labels.push(label_inc.clone());

        ctx.loop_scope_depths.push(ctx.scope.len());
        ctx.loop_results.push(None);
        let moved_before = ctx.moved.clone();
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

        ctx.declare_var(var.clone(), elem_ir_type.clone())?;
        ctx.borrowed.insert(var.clone());
        if let Some(et) = elem_hty {
            ctx.var_types.insert(var.clone(), et);
        }
        let element_tmp = ctx.new_tmp(elem_ir_type);

        ctx.instructions.push(Instruction {
            op: if is_string {
                Op::StrByte
            } else {
                Op::ArrayAccess
            },
            dst: Some(element_tmp.clone()),
            src1: Some(array_operand),
            src2: Some(curr_idx.clone()),
        });

        ctx.instructions.push(Instruction {
            op: Op::Store,
            dst: Some(Operand::Var(ctx.slot(&var))),
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

        self.emit_scope_frees(ctx)?;
        ctx.exit_scope()?;
        self.check_loop_moves(&moved_before, ctx)?;
        ctx.loop_end_labels.pop();
        ctx.loop_inc_labels.pop();
        ctx.loop_scope_depths.pop();

        Ok(self.take_loop_result(ctx))
    }

    fn take_loop_result(&mut self, ctx: &mut Context) -> Operand {
        match ctx.loop_results.pop().flatten() {
            Some(op) => op,
            None => ctx.new_tmp(IRType::Void),
        }
    }

    pub(super) fn compile_break(
        &mut self,
        value: Option<Box<Expr>>,
        ctx: &mut Context,
    ) -> Result<Operand, CodeGenError> {
        let end = ctx
            .loop_end_labels
            .last()
            .ok_or_else(|| CodeGenError::SyntaxError {
                message: "break outside of loop".to_string(),
            })?
            .clone();

        if let Some(v) = value {
            let val_op = self.compile_expr(*v, ctx)?;
            let has_loop = ctx.loop_results.last().is_some();
            if !has_loop {
                return Err(CodeGenError::SyntaxError {
                    message: "break outside of loop".to_string(),
                });
            }
            let already = ctx.loop_results.last().cloned().flatten();
            let slot = match already {
                Some(op) => op,
                None => {
                    let ty = ctx.get_operand_type(&val_op, &self.constants)?;
                    let tmp = ctx.new_tmp(ty);
                    *ctx.loop_results.last_mut().unwrap() = Some(tmp.clone());
                    tmp
                }
            };
            ctx.instructions.push(Instruction {
                op: Op::Move,
                dst: Some(slot),
                src1: Some(val_op),
                src2: None,
            });
        }

        if let Some(depth) = ctx.loop_scope_depths.last() {
            self.emit_scope_frees_from(ctx, *depth)?;
        }
        ctx.instructions.push(Instruction {
            op: Op::Jump,
            dst: None,
            src1: Some(Operand::Label(end.clone())),
            src2: None,
        });
        Ok(ctx.new_tmp(IRType::Void))
    }

    pub(super) fn compile_continue(&mut self, ctx: &mut Context) -> Result<Operand, CodeGenError> {
        let inc = ctx
            .loop_inc_labels
            .last()
            .ok_or_else(|| CodeGenError::SyntaxError {
                message: "continue outside of loop".to_string(),
            })?
            .clone();

        if let Some(depth) = ctx.loop_scope_depths.last() {
            self.emit_scope_frees_from(ctx, *depth)?;
        }
        ctx.instructions.push(Instruction {
            op: Op::Jump,
            dst: None,
            src1: Some(Operand::Label(inc.clone())),
            src2: None,
        });
        Ok(ctx.new_tmp(IRType::Void))
    }
}
