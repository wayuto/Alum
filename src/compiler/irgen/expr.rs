use super::context::Context;
use super::ir::{IRConst, IRType, Instruction, Op, Operand};
use crate::compiler::{
    codegen::CodeGenError,
    irgen::IRGen,
    parser::{Expr, Primitive, Type},
};
use ordered_float::OrderedFloat;
use std::iter::zip;

impl IRGen {
    pub(super) fn compile_struct_literal(
        &mut self,
        struct_name: &str,
        field_values: Vec<(String, Expr)>,
        ctx: &mut Context,
    ) -> Result<Operand, CodeGenError> {
        let (_, fields) = self
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

    pub(super) fn const_array_len(&self, value: &Operand, ctx: &Context) -> Option<usize> {
        let last_inst = ctx.instructions.last()?;
        if !matches!(last_inst.op, Op::Move | Op::FMove) {
            return None;
        }
        if last_inst.dst.as_ref() != Some(value) {
            return None;
        }
        let Operand::ConstIdx(idx) = last_inst.src1.as_ref()? else {
            return None;
        };
        match &self.constants[*idx] {
            IRConst::Array(elems) => Some(elems.len()),
            _ => None,
        }
    }

    pub(super) fn compile_expr(
        &mut self,
        expr: Expr,
        ctx: &mut Context,
    ) -> Result<Operand, CodeGenError> {
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
                let res_tmp = ctx.new_tmp(IRType::Int);
                let zero_idx = self.get_const_index(IRConst::Int(0));
                ctx.instructions.push(Instruction {
                    op: Op::Move,
                    dst: Some(res_tmp.clone()),
                    src1: Some(Operand::ConstIdx(zero_idx)),
                    src2: None,
                });
                Ok(res_tmp)
            }

            Expr::VarDecl(name, typ, value, _) => {
                let value = self.compile_expr(*value, ctx)?;
                let var_ir_type = Context::type2ir_type(&typ);

                if matches!(var_ir_type, IRType::Array) {
                    if let Some(len) = self.const_array_len(&value, ctx) {
                        ctx.array_lengths.insert(name.clone(), len);
                    }
                }

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
                if matches!(var_typ, IRType::Array) {
                    if let Some(len) = self.const_array_len(&value, ctx) {
                        ctx.array_lengths.insert(name.clone(), len);
                    }
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

            Expr::Add(_, _, _) | Expr::Sub(_, _, _) => {
                let (op, l, r) = IRGen::get_binop_parts(expr)?;
                {
                    let is_add = op == Op::Add;
                    let int_like = |e: &Expr| -> bool {
                        matches!(e, Expr::Int(_, _))
                            || matches!(
                                self.expr_high_type(e, ctx),
                                Some(Type::Primitive(Primitive::Int))
                            )
                    };
                    let pointee_of = |e: &Expr| -> Option<Type> {
                        match self.expr_high_type(e, ctx) {
                            Some(Type::Pointer(inner)) => Some(*inner),
                            _ => None,
                        }
                    };
                    let l_pointee = pointee_of(&*l);
                    let r_pointee = pointee_of(&*r);
                    if let Some(pointee) = l_pointee {
                        if int_like(&*r) {
                            let (scale, l_is_ptr) = (Self::ptr_scale(&pointee), true);
                            let base_e = l;
                            let off_e = r;
                            let base_op = self.compile_expr(*base_e, ctx)?;
                            let off_op = self.compile_expr(*off_e, ctx)?;
                            let res_tmp = ctx.new_tmp(IRType::Int);
                            if scale == 1 {
                                ctx.instructions.push(Instruction {
                                    op: if l_is_ptr { Op::Add } else { Op::Sub },
                                    dst: Some(res_tmp.clone()),
                                    src1: Some(base_op),
                                    src2: Some(off_op),
                                });
                            } else {
                                let scaled = ctx.new_tmp(IRType::Int);
                                let scale_idx = self.get_const_index(IRConst::Int(scale as i64));
                                ctx.instructions.push(Instruction {
                                    op: Op::Mul,
                                    dst: Some(scaled.clone()),
                                    src1: Some(off_op),
                                    src2: Some(Operand::ConstIdx(scale_idx)),
                                });
                                ctx.instructions.push(Instruction {
                                    op: if l_is_ptr { Op::Add } else { Op::Sub },
                                    dst: Some(res_tmp.clone()),
                                    src1: Some(base_op),
                                    src2: Some(scaled),
                                });
                            }
                            return Ok(res_tmp);
                        }
                    }
                    if is_add {
                        if let Some(pointee) = r_pointee {
                            if int_like(&*l) {
                                let (scale, l_is_ptr) = (Self::ptr_scale(&pointee), false);
                                let base_e = r;
                                let off_e = l;
                                let base_op = self.compile_expr(*base_e, ctx)?;
                                let off_op = self.compile_expr(*off_e, ctx)?;
                                let res_tmp = ctx.new_tmp(IRType::Int);
                                if scale == 1 {
                                    ctx.instructions.push(Instruction {
                                        op: if l_is_ptr { Op::Add } else { Op::Sub },
                                        dst: Some(res_tmp.clone()),
                                        src1: Some(base_op),
                                        src2: Some(off_op),
                                    });
                                } else {
                                    let scaled = ctx.new_tmp(IRType::Int);
                                    let scale_idx =
                                        self.get_const_index(IRConst::Int(scale as i64));
                                    ctx.instructions.push(Instruction {
                                        op: Op::Mul,
                                        dst: Some(scaled.clone()),
                                        src1: Some(off_op),
                                        src2: Some(Operand::ConstIdx(scale_idx)),
                                    });
                                    ctx.instructions.push(Instruction {
                                        op: if l_is_ptr { Op::Add } else { Op::Sub },
                                        dst: Some(res_tmp.clone()),
                                        src1: Some(base_op),
                                        src2: Some(scaled),
                                    });
                                }
                                return Ok(res_tmp);
                            }
                        }
                    }
                }
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

            Expr::Mul(_, _, _)
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
            | Expr::Xor(_, _, _)
            | Expr::LAnd(_, _, _)
            | Expr::LOr(_, _, _)
            | Expr::StrCat(_, _, _) => {
                let (op, l, r) = IRGen::get_binop_parts(expr)?;
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
                let var_high = ctx.get_var_high_type(name.as_str()).cloned();
                let scale = var_high
                    .as_ref()
                    .and_then(|t| t.pointee())
                    .map(Self::ptr_scale);
                let rhs_raw = self.compile_expr(*value, ctx)?;
                let rhs = if let Some(s) = scale {
                    if s == 1 {
                        rhs_raw
                    } else {
                        let scaled = ctx.new_tmp(IRType::Int);
                        let scale_idx = self.get_const_index(IRConst::Int(s as i64));
                        ctx.instructions.push(Instruction {
                            op: Op::Mul,
                            dst: Some(scaled.clone()),
                            src1: Some(rhs_raw),
                            src2: Some(Operand::ConstIdx(scale_idx)),
                        });
                        scaled
                    }
                } else {
                    rhs_raw
                };
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
                let var_high = ctx.get_var_high_type(name.as_str()).cloned();
                let scale = var_high
                    .as_ref()
                    .and_then(|t| t.pointee())
                    .map(Self::ptr_scale);
                let rhs_raw = self.compile_expr(*value, ctx)?;
                let rhs = if let Some(s) = scale {
                    if s == 1 {
                        rhs_raw
                    } else {
                        let scaled = ctx.new_tmp(IRType::Int);
                        let scale_idx = self.get_const_index(IRConst::Int(s as i64));
                        ctx.instructions.push(Instruction {
                            op: Op::Mul,
                            dst: Some(scaled.clone()),
                            src1: Some(rhs_raw),
                            src2: Some(Operand::ConstIdx(scale_idx)),
                        });
                        scaled
                    }
                } else {
                    rhs_raw
                };
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
                let known_len = match &*iter {
                    Expr::ArrayLiteral(elements, _) => Some(elements.len()),
                    Expr::Var(name, _) => ctx.array_lengths.get(name).copied(),
                    _ => None,
                };
                let array_operand = self.compile_expr(*iter, ctx)?;

                let array_len_operand = if let Some(len) = known_len {
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

            Expr::FuncDecl(_, _, _, _, _, _) => Err(CodeGenError::SyntaxError {
                message: "cannot declare a function in a function".to_string(),
            }),

            Expr::Call(callee, type_args, args, _) => {
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
                    let mut n = 0;
                    for (arg, param) in zip(args.iter(), func.params.iter()) {
                        let operand = self.compile_expr(arg.clone(), ctx)?;
                        let operand_type = ctx.get_operand_type(&operand, &self.constants)?;
                        let type_matches = operand_type == param.1
                            || (operand_type == IRType::Array && param.1 == IRType::Int);
                        if !type_matches {
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
                    let ret_ir_type = self.member_call_ret_type(&callee, ctx);
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

            Expr::Index(arr, idx, _) => {
                let (elem_type, byte) = self.index_info(&arr, ctx);
                let elem_ir_type = elem_type.map_or(IRType::Int, |t| Context::type2ir_type(&t));
                let arr_op = self.compile_expr(*arr, ctx)?;
                let offset = self.compile_expr(*idx, ctx)?;
                let res_tmp = ctx.new_tmp(elem_ir_type);
                ctx.instructions.push(Instruction {
                    op: if byte {
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

            Expr::IndexAssign(arr_idx, value, _) => {
                let (arr, idx) = match *arr_idx {
                    Expr::Index(arr, idx, _) => (arr, idx),
                    expr => {
                        return Err(CodeGenError::TypeError {
                            message: format!("expected index expression, got {:?}", expr),
                        });
                    }
                };
                let (_elem_type, byte) = self.index_info(&arr, ctx);
                let arr_op = self.compile_expr(*arr, ctx)?;
                let offset = self.compile_expr(*idx, ctx)?;
                let val = self.compile_expr(*value, ctx)?;
                let res_tmp = ctx.new_tmp(IRType::Void);
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
                Ok(res_tmp)
            }

            Expr::ArrayLiteral(elements, _) => {
                let mut compiled = Vec::new();
                for e in elements.iter() {
                    compiled.push(self.compile_expr(e.clone(), ctx)?);
                }

                let ir_const = IRConst::Array(compiled.clone());
                let ir_type = IRType::Array;

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
                    Type::Primitive(crate::compiler::parser::Primitive::Boolean) => 1i64,
                    _ => 8i64,
                };
                let ptr_tmp = ctx.new_tmp(IRType::Int);
                if let Operand::ConstIdx(n) = &len_op {
                    let n_val = if let IRConst::Int(v) = &self.constants[*n] {
                        *v
                    } else {
                        elem_size * 8
                    };
                    let total_size = (n_val + 1) * 8;
                    let total_size_idx = self.get_const_index(IRConst::Int(total_size));
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
                } else {
                    let esize_idx = self.get_const_index(IRConst::Int(elem_size));
                    let byte_len_tmp = ctx.new_tmp(IRType::Int);
                    ctx.instructions.push(Instruction {
                        op: Op::Mul,
                        dst: Some(byte_len_tmp.clone()),
                        src1: Some(len_op.clone()),
                        src2: Some(Operand::ConstIdx(esize_idx)),
                    });
                    let header_idx = self.get_const_index(IRConst::Int(8));
                    let total_size_tmp = ctx.new_tmp(IRType::Int);
                    ctx.instructions.push(Instruction {
                        op: Op::Add,
                        dst: Some(total_size_tmp.clone()),
                        src1: Some(byte_len_tmp),
                        src2: Some(Operand::ConstIdx(header_idx)),
                    });
                    ctx.instructions.push(Instruction {
                        op: Op::Malloc,
                        dst: Some(ptr_tmp.clone()),
                        src1: Some(total_size_tmp),
                        src2: None,
                    });
                    let zero_idx = self.get_const_index(IRConst::Int(0));
                    ctx.instructions.push(Instruction {
                        op: Op::StoreAt,
                        dst: Some(ptr_tmp.clone()),
                        src1: Some(Operand::ConstIdx(zero_idx)),
                        src2: Some(len_op),
                    });
                }
                Ok(ptr_tmp)
            }

            Expr::Range(start, end, _) => {
                let start_op = self.compile_expr(*start, ctx)?;
                let end_op = self.compile_expr(*end, ctx)?;
                let res_tmp = ctx.new_tmp(IRType::Array);
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

            Expr::StructLiteral(name, _, fields, _) => {
                self.compile_struct_literal(&name, fields, ctx)
            }

            Expr::MemberAccess(obj, field_name, _) => {
                let struct_name = match &*obj {
                    Expr::Var(name, _) => match ctx.get_var_high_type(name) {
                        Some(Type::Struct(sname, _)) => sname.clone(),
                        Some(Type::Pointer(box_ty)) => {
                            if let Type::Struct(sname, _) = box_ty.as_ref() {
                                sname.clone()
                            } else {
                                return Err(CodeGenError::TypeError {
                                    message: "member access on non-struct variable".to_string(),
                                });
                            }
                        }
                        _ => {
                            return Err(CodeGenError::TypeError {
                                message: "member access on non-struct variable".to_string(),
                            });
                        }
                    },
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
                for (i, (fname, _)) in struct_def.1.iter().enumerate() {
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
                    Expr::Var(name, _) => match ctx.get_var_high_type(name) {
                        Some(Type::Struct(sname, _)) => sname.clone(),
                        Some(Type::Pointer(box_ty)) => {
                            if let Type::Struct(sname, _) = box_ty.as_ref() {
                                sname.clone()
                            } else {
                                return Err(CodeGenError::TypeError {
                                    message: "member assign on non-struct variable".to_string(),
                                });
                            }
                        }
                        _ => {
                            return Err(CodeGenError::TypeError {
                                message: "member assign on non-struct variable".to_string(),
                            });
                        }
                    },
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
                for (i, (fname, _)) in struct_def.1.iter().enumerate() {
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
                let is_struct = match ctx.get_var_high_type(&name) {
                    Some(Type::Struct(sname, _)) => self.structs.contains_key(sname),
                    _ => false,
                };
                if is_struct {
                    self.compile_expr(*inner, ctx)
                } else {
                    let res_tmp = ctx.new_tmp(IRType::Int);
                    ctx.instructions.push(Instruction {
                        op: Op::Lea,
                        dst: Some(res_tmp.clone()),
                        src1: Some(Operand::Var(name)),
                        src2: None,
                    });
                    Ok(res_tmp)
                }
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

            Expr::TypeDef(_) | Expr::Struct(_, _, _, _) | Expr::Lambda(_, _, _, _) => {
                Ok(ctx.new_tmp(IRType::Void))
            }
        }
    }

    fn member_call_ret_type(&self, callee: &Expr, ctx: &Context) -> IRType {
        if let Expr::MemberAccess(obj, field_name, _) = callee {
            if let Expr::Var(name, _) = &**obj {
                if let Some(Type::Struct(sname, type_args)) = ctx.get_var_high_type(name) {
                    if let Some((_, fields)) = self.structs.get(sname) {
                        for (fname, ftype) in fields {
                            if fname == field_name {
                                if let Type::Function(_, ret) = ftype {
                                    let concrete = ret.substitute(type_args);
                                    return Context::type2ir_type(&concrete);
                                }
                            }
                        }
                    }
                }
            }
        }
        IRType::Int
    }

    fn ptr_scale(ty: &Type) -> usize {
        match ty {
            Type::Primitive(Primitive::Void) => 1,
            _ => 8,
        }
    }

    fn expr_high_type(&self, e: &Expr, ctx: &Context) -> Option<Type> {
        match e {
            Expr::Var(name, _) => ctx.get_var_high_type(name.as_str()).cloned(),
            Expr::AddressOf(inner, _) => match inner.as_ref() {
                Expr::Var(n, _) => ctx
                    .get_var_high_type(n.as_str())
                    .cloned()
                    .map(|t| Type::Pointer(Box::new(t))),
                _ => None,
            },
            Expr::Deref(inner, _) => match inner.as_ref() {
                Expr::Var(n, _) => match ctx.get_var_high_type(n.as_str()) {
                    Some(Type::Pointer(t)) => Some(*t.clone()),
                    _ => None,
                },
                _ => match &self.expr_high_type(inner, ctx) {
                    Some(Type::Pointer(t)) => Some(*t.clone()),
                    _ => None,
                },
            },
            Expr::Index(arr, _, _) => self.index_info(arr, ctx).0,
            _ => None,
        }
    }

    fn index_info(&self, arr: &Expr, ctx: &Context) -> (Option<Type>, bool) {
        let (sname, type_args, field_name) = match arr {
            Expr::Var(name, _) => match ctx.get_var_high_type(name) {
                Some(Type::Array(elem)) => return (Some(elem.as_ref().clone()), false),
                Some(Type::Primitive(Primitive::String)) => {
                    return (Some(Type::Primitive(Primitive::Int)), true);
                }
                Some(Type::Pointer(inner)) => {
                    return (Some(*inner.clone()), Self::ptr_scale(inner) == 1);
                }
                Some(Type::Struct(sname, ta)) => (sname.clone(), ta.clone(), None),
                #[allow(warnings)]
                Some(Type::Pointer(box_ty)) => {
                    if let Type::Struct(sname, ta) = box_ty.as_ref() {
                        (sname.clone(), ta.clone(), None)
                    } else {
                        return (None, false);
                    }
                }
                _ => return (None, false),
            },
            Expr::MemberAccess(obj, field_name, _) => match &**obj {
                Expr::Var(name, _) => match ctx.get_var_high_type(name) {
                    Some(Type::Struct(sname, type_args)) => {
                        (sname.clone(), type_args.clone(), Some(field_name.clone()))
                    }
                    Some(Type::Pointer(box_ty)) => {
                        if let Type::Struct(sname, type_args) = box_ty.as_ref() {
                            (sname.clone(), type_args.clone(), Some(field_name.clone()))
                        } else {
                            return (None, false);
                        }
                    }
                    _ => return (None, false),
                },
                _ => return (None, false),
            },
            _ => return (None, false),
        };

        if let Some((_, fields)) = self.structs.get(&sname) {
            for (fname, ftype) in fields {
                if Some(fname.as_str()) == field_name.as_deref() {
                    let byte = match ftype {
                        Type::Primitive(Primitive::String) => true,
                        Type::Array(elem) => false,
                        Type::Pointer(inner) => Self::ptr_scale(&inner) == 1,
                        _ => false,
                    };
                    let elem = match ftype {
                        Type::Pointer(inner) => *inner.clone(),
                        Type::Array(elem) => {
                            let concrete = elem.substitute(&type_args);
                            concrete
                        }
                        Type::Primitive(Primitive::String) => Type::Primitive(Primitive::Int),
                        _ => Type::Primitive(Primitive::Int),
                    };
                    return (Some(elem), byte);
                }
            }
        }
        (None, false)
    }
}
