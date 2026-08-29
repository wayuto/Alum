use super::context::Context;
use super::ir::{IRConst, IRType, Instruction, Op, Operand};
use crate::compiler::{
    Span,
    codegen::CodeGenError,
    irgen::IRGen,
    parser::{Expr, Primitive, Type},
};
use ordered_float::OrderedFloat;

impl IRGen {
    fn compile_local_decl(
        &mut self,
        name: &str,
        typ: &Type,
        value: Expr,
        ctx: &mut Context,
        allow_move: bool,
    ) -> Result<Operand, CodeGenError> {
        let resolved_typ = if matches!(typ, Type::Unknown | Type::TypeVar(_) | Type::Param(_)) {
            self.expr_high_type(&value, ctx)
                .unwrap_or_else(|| typ.clone())
        } else {
            typ.clone()
        };
        let is_struct = matches!(&resolved_typ, Type::Struct(_, _) | Type::Union(_, _));

        let copy_info = if allow_move && self.is_resource_type(&resolved_typ) {
            self.resource_copy_info(&value, ctx)
        } else {
            None
        };

        let moved_src = if !allow_move {
            None
        } else {
            self.detect_move_source(
                &value,
                ctx,
                super::expr_resource::MoveOpts {
                    exclude: Some(name.to_string()),
                    binding_ty: Some(resolved_typ.clone()),
                },
            )
        };
        let is_whole_name_move = moved_src.is_some();
        let mut value: Operand = if is_whole_name_move || is_struct {
            self.compile_expr(value, ctx)?
        } else if allow_move && Self::array_has_string_elems(&value) {
            self.compile_expr(value, ctx)?
        } else {
            match self.eval_const(&value, Some(&*ctx)) {
                Some((cv, IRType::Int | IRType::Float | IRType::Bool | IRType::Array)) => {
                    Operand::ConstIdx(self.get_const_index(cv))
                }
                _ => self.compile_expr(value, ctx)?,
            }
        };
        if let Some(copy_ty) = copy_info {
            if !is_whole_name_move {
                value = self.copy_resource(ctx, value, &copy_ty)?;
            }
        }
        let var_ir_type = Context::type_to_ir_type(&resolved_typ);

        ctx.declare_var_with_type(name.to_string(), var_ir_type.clone(), resolved_typ)?;
        if matches!(var_ir_type, IRType::Array) {
            if let Some(len) = self.const_array_len(&value, ctx) {
                ctx.array_lengths.insert(name.to_string(), len);
            } else if let Operand::ConstIdx(idx) = &value {
                if let IRConst::Array(elems) = &self.constants[*idx] {
                    ctx.array_lengths.insert(name.to_string(), elems.len());
                }
            }
        }

        let result = value.clone();
        match var_ir_type {
            IRType::Float => ctx.instructions.push(Instruction {
                op: Op::FStore,
                dst: Some(Operand::Var(ctx.slot(name))),
                src1: Some(value),
                src2: None,
            }),
            _ => ctx.instructions.push(Instruction {
                op: Op::Store,
                dst: Some(Operand::Var(ctx.slot(name))),
                src1: Some(value),
                src2: None,
            }),
        }

        if let Some(msrc) = &moved_src {
            self.invalidate_and_mark_move(msrc, ctx)?;
        }
        Ok(result)
    }

    fn emit_int_inc_dec(
        &mut self,
        name: &str,
        op: Op,
        ctx: &mut Context,
    ) -> Result<Operand, CodeGenError> {
        if self.is_float_var(name, ctx) {
            return self.emit_float_inc_dec(name, op == Op::Inc, ctx);
        }
        if let Some(load_op) = self.extern_load_op(name) {
            let dst = self.extern_op(name).unwrap();
            let res_tmp = ctx.new_tmp(IRType::Int);
            ctx.instructions.push(Instruction {
                op: load_op,
                dst: Some(res_tmp.clone()),
                src1: Some(dst.clone()),
                src2: None,
            });
            ctx.instructions.push(Instruction {
                op: op.clone(),
                dst: Some(res_tmp.clone()),
                src1: Some(res_tmp.clone()),
                src2: None,
            });
            ctx.instructions.push(Instruction {
                op: Op::GlobStore,
                dst: Some(dst),
                src1: Some(res_tmp.clone()),
                src2: None,
            });
            return Ok(res_tmp);
        }
        let var_op = Operand::Var(ctx.slot(name));
        let res_tmp = ctx.new_tmp(IRType::Int);
        ctx.instructions.push(Instruction {
            op: Op::Load,
            dst: Some(res_tmp.clone()),
            src1: Some(var_op.clone()),
            src2: None,
        });
        ctx.instructions.push(Instruction {
            op,
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

    fn get_binop_parts(expr: Expr) -> Result<(Op, Box<Expr>, Box<Expr>), CodeGenError> {
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
            Expr::Xor(l, r, _) => Ok((Op::Xor, l, r)),
            Expr::BAnd(l, r, _) => Ok((Op::And, l, r)),
            Expr::BOr(l, r, _) => Ok((Op::Or, l, r)),
            Expr::LAnd(l, r, _) => Ok((Op::LAnd, l, r)),
            Expr::LOr(l, r, _) => Ok((Op::LOr, l, r)),
            Expr::Shl(l, r, _) => Ok((Op::Shl, l, r)),
            Expr::Shr(l, r, _) => Ok((Op::Shr, l, r)),
            Expr::StrCat(l, r, _) => Ok((Op::StrCat, l, r)),
            _ => Err(CodeGenError::UnsupportedOperation {
                message: "not a binary operation".to_string(),
            }),
        }
    }

    pub(super) fn compile_expr(
        &mut self,
        expr: Expr,
        ctx: &mut Context,
    ) -> Result<Operand, CodeGenError> {
        const MAX_EXPR_DEPTH: usize = 1000;
        if self.expr_depth > MAX_EXPR_DEPTH {
            return Err(CodeGenError::TypeError {
                message: format!("expression nesting exceeds {} levels", MAX_EXPR_DEPTH),
            });
        }
        let span = expr.span();
        self.expr_depth += 1;
        let result = self
            .compile_expr_inner(expr, ctx)
            .map_err(|e| e.with_fallback_span(span));
        self.expr_depth -= 1;
        result
    }

    fn compile_expr_inner(
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
            Expr::Char(c, _) => {
                let ir_type = IRType::Int;
                let ir_const = IRConst::Int(c as i64);
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
                self.compile_local_decl(&name, &typ, *value, ctx, true)
            }

            Expr::ConstDecl(name, typ, value, _, _) => {
                self.compile_local_decl(&name, &typ, *value, ctx, false)
            }

            Expr::GlobalVar(_, _, _, _, _) => Ok(ctx.new_tmp(IRType::Void)),

            Expr::VarAssign(name, value, _) => {
                ctx.unmark_moved(&name);

                let moved_src = self.detect_move_source(
                    &value,
                    ctx,
                    super::expr_resource::MoveOpts {
                        exclude: Some(name.clone()),
                        binding_ty: None,
                    },
                );
                let value_copy_info = self.resource_copy_info(&value, ctx);
                let value = self.compile_expr(*value, ctx)?;
                let typ = ctx.get_operand_type(&value, &self.constants)?;

                if ctx.get_var_type(&name).is_err() {
                    if let Some(store_op) = self.extern_store_op(&name) {
                        let var_typ = self.glob_ir_type(&name).unwrap_or({
                            if matches!(store_op, Op::FGlobStore) {
                                IRType::Float
                            } else {
                                IRType::Int
                            }
                        });
                        let typ_ok = typ == var_typ
                            || typ == IRType::Array
                            || (var_typ == IRType::Int && typ == IRType::Bool);
                        if !typ_ok {
                            return Err(CodeGenError::TypeError {
                                message: format!("unexpected type: {:?}", typ),
                            });
                        }
                        let dst = self.extern_op(&name).unwrap();
                        let result = value.clone();
                        ctx.instructions.push(Instruction {
                            op: store_op,
                            dst: Some(dst),
                            src1: Some(value),
                            src2: None,
                        });

                        if let Some(msrc) = &moved_src {
                            self.invalidate_and_mark_move(msrc, ctx)?;
                        }

                        ctx.unmark_moved(&name);
                        return Ok(result);
                    }
                }
                let var_typ = ctx.get_var_type(&name)?;

                let typ_ok = typ == var_typ || (var_typ == IRType::String && typ == IRType::Int);
                if !typ_ok {
                    return Err(CodeGenError::TypeError {
                        message: format!("unexpected type: {:?}", typ),
                    });
                }
                if matches!(var_typ, IRType::Array) {
                    let mut new_len: Option<usize> = None;
                    if let Some(len) = self.const_array_len(&value, ctx) {
                        new_len = Some(len);
                    } else if let Operand::ConstIdx(idx) = &value {
                        if let IRConst::Array(elems) = &self.constants[*idx] {
                            new_len = Some(elems.len());
                        }
                    }
                    match new_len {
                        Some(len) => {
                            ctx.array_lengths.insert(name.clone(), len);
                        }

                        None => {
                            ctx.array_lengths.remove(&name);
                        }
                    }
                } else {
                    ctx.array_lengths.remove(&name);
                }
                if let Some(hty) = ctx.var_types.get(&name) {
                    if self.is_resource_type(hty) {
                        let is_whole_name_move = moved_src.is_some();
                        let value = match (&value_copy_info, is_whole_name_move) {
                            (_, true) => value,
                            (Some(ty), false) => self.copy_resource(ctx, value, ty)?,
                            (None, false) => value,
                        };
                        let result = value.clone();

                        let old = ctx.new_tmp(IRType::Int);
                        ctx.instructions.push(Instruction {
                            op: Op::Load,
                            dst: Some(old.clone()),
                            src1: Some(Operand::Var(ctx.slot(&name))),
                            src2: None,
                        });
                        self.emit_free_ptr(ctx, old);
                        match typ {
                            IRType::Float => ctx.instructions.push(Instruction {
                                op: Op::FStore,
                                dst: Some(Operand::Var(ctx.slot(&name))),
                                src1: Some(value),
                                src2: None,
                            }),
                            _ => ctx.instructions.push(Instruction {
                                op: Op::Store,
                                dst: Some(Operand::Var(ctx.slot(&name))),
                                src1: Some(value),
                                src2: None,
                            }),
                        }

                        if let Some(msrc) = &moved_src {
                            self.invalidate_and_mark_move(msrc, ctx)?;
                        }

                        ctx.unmark_moved(&name);
                        return Ok(result);
                    }
                }
                let result = value.clone();
                match typ {
                    IRType::Float => ctx.instructions.push(Instruction {
                        op: Op::FStore,
                        dst: Some(Operand::Var(ctx.slot(&name))),
                        src1: Some(value),
                        src2: None,
                    }),
                    _ => ctx.instructions.push(Instruction {
                        op: Op::Store,
                        dst: Some(Operand::Var(ctx.slot(&name))),
                        src1: Some(value),
                        src2: None,
                    }),
                }
                ctx.unmark_moved(&name);
                Ok(result)
            }

            Expr::Var(name, span) => {
                if let Ok(var_type) = ctx.get_var_type(&name) {
                    self.check_use_after_move(&name, span, ctx)?;
                    let res_tmp = ctx.new_tmp(var_type.clone());
                    match var_type {
                        IRType::Float => ctx.instructions.push(Instruction {
                            op: Op::FLoad,
                            dst: Some(res_tmp.clone()),
                            src1: Some(Operand::Var(ctx.slot(&name))),
                            src2: None,
                        }),
                        _ => ctx.instructions.push(Instruction {
                            op: Op::Load,
                            dst: Some(res_tmp.clone()),
                            src1: Some(Operand::Var(ctx.slot(&name))),
                            src2: None,
                        }),
                    }
                    Ok(res_tmp)
                } else if let Some(ir_type) = self.glob_ir_type(&name) {
                    let res_tmp = ctx.new_tmp(ir_type.clone());
                    let op = match ir_type {
                        IRType::Float => Op::FGlobLoad,
                        _ => Op::GlobLoad,
                    };
                    ctx.instructions.push(Instruction {
                        op,
                        dst: Some(res_tmp.clone()),
                        src1: Some(Operand::Global(name)),
                        src2: None,
                    });
                    Ok(res_tmp)
                } else if let Some(func) = self.lookup_func(&name) {
                    Ok(Operand::Function(func.name.clone()))
                } else if let Some((ir_const, ir_type)) = self.globals.get(&name).cloned() {
                    let const_idx = self.get_const_index(ir_const);
                    let res_tmp = ctx.new_tmp(ir_type);
                    let op = if matches!(res_tmp, Operand::Temp(_, IRType::Float)) {
                        Op::FMove
                    } else {
                        Op::Move
                    };
                    ctx.instructions.push(Instruction {
                        op,
                        dst: Some(res_tmp.clone()),
                        src1: Some(Operand::ConstIdx(const_idx)),
                        src2: None,
                    });
                    Ok(res_tmp)
                } else if let Some(value) = self.enum_member_value(&name)? {
                    let const_idx = self.get_const_index(IRConst::Int(value as i64));
                    let res_tmp = ctx.new_tmp(IRType::Int);
                    ctx.instructions.push(Instruction {
                        op: Op::Move,
                        dst: Some(res_tmp.clone()),
                        src1: Some(Operand::ConstIdx(const_idx)),
                        src2: None,
                    });
                    Ok(res_tmp)
                } else {
                    Err(CodeGenError::UndefinedVariable {
                        name: name.clone(),
                        span: Span::new(0, 0),
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
                            let scale = Self::ptr_scale(&pointee);
                            let base_e = l;
                            let off_e = r;
                            let base_op = self.compile_expr(*base_e, ctx)?;
                            let off_op = self.compile_expr(*off_e, ctx)?;
                            let res_tmp = ctx.new_tmp(IRType::Int);
                            if scale == 1 {
                                ctx.instructions.push(Instruction {
                                    op: op.clone(),
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
                                    op: op.clone(),
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
                                let scale = Self::ptr_scale(&pointee);
                                let base_e = r;
                                let off_e = l;
                                let base_op = self.compile_expr(*base_e, ctx)?;
                                let off_op = self.compile_expr(*off_e, ctx)?;
                                let res_tmp = ctx.new_tmp(IRType::Int);
                                if scale == 1 {
                                    ctx.instructions.push(Instruction {
                                        op: Op::Add,
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
                                        op: Op::Add,
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

            Expr::LAnd(_, _, _) | Expr::LOr(_, _, _) => {
                let (op, l, r) = IRGen::get_binop_parts(expr)?;
                let is_and = matches!(op, Op::LAnd);
                let res_tmp = ctx.new_tmp(IRType::Bool);

                let left = self.compile_expr(*l, ctx)?;
                ctx.instructions.push(Instruction {
                    op: Op::Move,
                    dst: Some(res_tmp.clone()),
                    src1: Some(left),
                    src2: None,
                });

                if is_and {
                    let label_end = ctx.new_label("land_end");
                    ctx.instructions.push(Instruction {
                        op: Op::JumpIfFalse,
                        dst: None,
                        src1: Some(res_tmp.clone()),
                        src2: Some(Operand::Label(label_end.clone())),
                    });
                    let right = self.compile_expr(*r, ctx)?;
                    ctx.instructions.push(Instruction {
                        op: Op::Move,
                        dst: Some(res_tmp.clone()),
                        src1: Some(right),
                        src2: None,
                    });
                    ctx.instructions.push(Instruction {
                        op: Op::Label(label_end),
                        dst: None,
                        src1: None,
                        src2: None,
                    });
                } else {
                    let label_rhs = ctx.new_label("lor_rhs");
                    let label_end = ctx.new_label("lor_end");
                    ctx.instructions.push(Instruction {
                        op: Op::JumpIfFalse,
                        dst: None,
                        src1: Some(res_tmp.clone()),
                        src2: Some(Operand::Label(label_rhs.clone())),
                    });
                    ctx.instructions.push(Instruction {
                        op: Op::Jump,
                        dst: None,
                        src1: Some(Operand::Label(label_end.clone())),
                        src2: None,
                    });
                    ctx.instructions.push(Instruction {
                        op: Op::Label(label_rhs),
                        dst: None,
                        src1: None,
                        src2: None,
                    });
                    let right = self.compile_expr(*r, ctx)?;
                    ctx.instructions.push(Instruction {
                        op: Op::Move,
                        dst: Some(res_tmp.clone()),
                        src1: Some(right),
                        src2: None,
                    });
                    ctx.instructions.push(Instruction {
                        op: Op::Label(label_end),
                        dst: None,
                        src1: None,
                        src2: None,
                    });
                }
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
            | Expr::BAnd(_, _, _)
            | Expr::BOr(_, _, _)
            | Expr::Shl(_, _, _)
            | Expr::Shr(_, _, _)
            | Expr::StrCat(_, _, _) => {
                let (op, l, r) = IRGen::get_binop_parts(expr)?;
                let left = self.compile_expr(*l, ctx)?;
                let right = self.compile_expr(*r, ctx)?;
                let typ = ctx.get_operand_type(&left, &self.constants)?;

                let op = if matches!(typ, IRType::String)
                    && matches!(op, Op::Eq | Op::Ne | Op::Lt | Op::Le | Op::Gt | Op::Ge)
                {
                    match op {
                        Op::Eq => Op::StrEq,
                        Op::Ne => Op::StrNe,
                        Op::Lt => Op::StrLt,
                        Op::Le => Op::StrLe,
                        Op::Gt => Op::StrGt,
                        Op::Ge => Op::StrGe,
                        _ => unreachable!(),
                    }
                } else {
                    op
                };

                let res_tmp = match op {
                    Op::StrCat => ctx.new_tmp(IRType::String),
                    Op::StrEq | Op::StrNe | Op::StrLt | Op::StrLe | Op::StrGt | Op::StrGe => {
                        ctx.new_tmp(IRType::Bool)
                    }
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

            Expr::BNot(e, _) => {
                let arg = self.compile_expr(*e, ctx)?;
                let res_tmp = ctx.new_tmp(IRType::Int);
                ctx.instructions.push(Instruction {
                    op: Op::BNot,
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

            Expr::Inc(name, _) => self.emit_int_inc_dec(&name, Op::Inc, ctx),

            Expr::Dec(name, _) => self.emit_int_inc_dec(&name, Op::Dec, ctx),

            Expr::AddAssign(name, value, _) => {
                self.emit_compound_assign(name, *value, Op::Add, ctx)
            }

            Expr::SubAssign(name, value, _) => {
                self.emit_compound_assign(name, *value, Op::Sub, ctx)
            }

            Expr::MulAssign(name, value, _) => {
                self.emit_compound_assign(name, *value, Op::Mul, ctx)
            }

            Expr::DivAssign(name, value, _) => {
                self.emit_compound_assign(name, *value, Op::Div, ctx)
            }

            Expr::ModAssign(name, value, _) => {
                self.emit_compound_assign(name, *value, Op::Mod, ctx)
            }

            Expr::AndAssign(name, value, _) => {
                self.emit_compound_assign(name, *value, Op::And, ctx)
            }

            Expr::OrAssign(name, value, _) => self.emit_compound_assign(name, *value, Op::Or, ctx),

            Expr::XorAssign(name, value, _) => {
                self.emit_compound_assign(name, *value, Op::Xor, ctx)
            }

            Expr::ShlAssign(name, value, _) => {
                self.emit_compound_assign(name, *value, Op::Shl, ctx)
            }

            Expr::ShrAssign(name, value, _) => {
                self.emit_compound_assign(name, *value, Op::Shr, ctx)
            }

            Expr::Block(body, _) => {
                ctx.enter_scope();
                let body_len = body.len();
                for i in 0..body_len.saturating_sub(1) {
                    self.compile_expr(body[i].clone(), ctx)?;
                }
                let result_operand = if let Some(last_expr) = body.last() {
                    let op = self.compile_expr(last_expr.clone(), ctx)?;
                    if let Some(ty) = self.resource_copy_info(last_expr, ctx) {
                        if !matches!(ctx.get_operand_type(&op, &self.constants)?, IRType::Void) {
                            self.copy_resource(ctx, op, &ty)?
                        } else {
                            op
                        }
                    } else {
                        op
                    }
                } else {
                    ctx.new_tmp(IRType::Void)
                };
                self.emit_scope_frees(ctx)?;
                ctx.exit_scope()?;
                Ok(result_operand)
            }

            Expr::Return(val, _) => {
                let moved_src = self.detect_move_source(
                    &val,
                    ctx,
                    super::expr_resource::MoveOpts {
                        exclude: None,
                        binding_ty: None,
                    },
                );
                let copy_info = self.resource_copy_info(&val, ctx);
                let res_op = self.compile_expr(*val, ctx)?;
                let res_op = match (&moved_src, copy_info) {
                    (Some(_), _) => res_op,
                    (None, Some(ty)) => self.copy_resource(ctx, res_op, &ty)?,
                    (None, None) => res_op,
                };

                if let Some(msrc) = &moved_src {
                    self.invalidate_and_mark_move(msrc, ctx)?;
                }
                self.emit_all_scope_frees(ctx)?;
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
                self.compile_if(condition, then_branch, else_branch, ctx)
            }

            Expr::While(condition, body, _) => self.compile_while(condition, body, ctx),

            Expr::For(var, iter, body, _) => self.compile_for(var, iter, body, ctx),

            Expr::Break(value, _) => self.compile_break(value, ctx),

            Expr::Continue(_) => self.compile_continue(ctx),

            Expr::FuncDecl(_, _, _, _, _, _, _) => Err(CodeGenError::SyntaxError {
                message: "cannot declare a function in a function".to_string(),
            }),

            Expr::Call(callee, type_args, args, _) => {
                self.compile_call(callee, type_args, args, ctx)
            }

            Expr::Index(arr, idx, _) => self.compile_index(arr, idx, ctx),

            Expr::IndexAssign(arr_idx, value, _) => self.compile_index_assign(arr_idx, value, ctx),

            Expr::ArrayLiteral(elements, _) => self.compile_array_literal(elements, ctx),

            Expr::ArrayFill(typ, len, _) => self.compile_array_fill(typ, len, ctx),

            Expr::Range(start, end, _) => self.compile_range(start, end, ctx),

            Expr::ExternVar(name, _, _) => {
                if let Some(ty) = self.extern_vars.get(&name).cloned() {
                    let ir_ty = Context::type_to_ir_type(&ty);
                    Ok(ctx.new_tmp(ir_ty))
                } else {
                    Err(CodeGenError::UndefinedVariable {
                        name: name.clone(),
                        span: Span::new(0, 0),
                    })
                }
            }

            Expr::StructLiteral(name, _, fields, _) => {
                self.compile_struct_literal(&name, fields, ctx)
            }

            Expr::UnionLiteral(name, _, fields, _) => {
                self.compile_union_literal(&name, fields, ctx)
            }

            Expr::MemberAccess(obj, field_name, _) => {
                if let Expr::Var(name, _) = obj.as_ref() {
                    if let Some(members) = self.enums.get(name) {
                        for (member_name, value) in members {
                            if member_name == &field_name {
                                let const_idx = self.get_const_index(IRConst::Int(*value as i64));
                                let res_tmp = ctx.new_tmp(IRType::Int);
                                ctx.instructions.push(Instruction {
                                    op: Op::Move,
                                    dst: Some(res_tmp.clone()),
                                    src1: Some(Operand::ConstIdx(const_idx)),
                                    src2: None,
                                });
                                return Ok(res_tmp);
                            }
                        }
                        return Err(CodeGenError::NameError {
                            message: format!("enum '{}' has no member '{}'", name, field_name),
                        });
                    }
                }

                if let Expr::Var(base, mspan) = obj.as_ref() {
                    let path = format!("{base}.{field_name}");
                    if ctx.moved.contains(&path) {
                        return Err(CodeGenError::UseAfterMove {
                            moved_at: ctx.moved_at.get(&path).copied().unwrap_or(*mspan),
                            span: *mspan,
                            name: path,
                        });
                    }
                }
                let (obj_addr, obj_type) = self.member_addr(&obj, ctx)?;
                let (offset, field_type) = self.member_offset_and_type(&obj_type, &field_name)?;
                let field_ir_type = Context::type_to_ir_type(&field_type);
                let addr = if offset == 0 {
                    obj_addr
                } else {
                    let addr_tmp = ctx.new_tmp(IRType::Int);
                    let offset_idx = self.get_const_index(IRConst::Int(offset as i64));
                    ctx.instructions.push(Instruction {
                        op: Op::Add,
                        dst: Some(addr_tmp.clone()),
                        src1: Some(obj_addr),
                        src2: Some(Operand::ConstIdx(offset_idx)),
                    });
                    addr_tmp
                };
                let res_tmp = ctx.new_tmp(field_ir_type.clone());
                let zero_idx = self.get_const_index(IRConst::Int(0));

                let load_op = if field_ir_type == IRType::Float {
                    Op::FLoadAt
                } else {
                    Op::LoadAt
                };
                ctx.instructions.push(Instruction {
                    op: load_op,
                    dst: Some(res_tmp.clone()),
                    src1: Some(addr),
                    src2: Some(Operand::ConstIdx(zero_idx)),
                });
                Ok(res_tmp)
            }

            Expr::MemberAssign(obj, _field_name, value, _) => {
                if let Expr::Var(base, _) = obj.as_ref() {
                    ctx.unmark_moved(&format!("{base}.{_field_name}"));
                }
                let value_copy_info = self.resource_copy_info(&value, ctx);
                let val_op = self.compile_expr(*value, ctx)?;
                let (obj_addr, obj_type) = self.member_addr(&obj, ctx)?;
                let (offset, field_ty) = self.member_offset_and_type(&obj_type, &_field_name)?;
                let addr = if offset == 0 {
                    obj_addr
                } else {
                    let addr_tmp = ctx.new_tmp(IRType::Int);
                    let offset_idx = self.get_const_index(IRConst::Int(offset as i64));
                    ctx.instructions.push(Instruction {
                        op: Op::Add,
                        dst: Some(addr_tmp.clone()),
                        src1: Some(obj_addr),
                        src2: Some(Operand::ConstIdx(offset_idx)),
                    });
                    addr_tmp
                };
                let zero_idx = self.get_const_index(IRConst::Int(0));
                let val_op = if self.is_resource_type(&field_ty) {
                    let val_op = match value_copy_info {
                        Some(ty) => self.copy_resource(ctx, val_op, &ty)?,
                        None => val_op,
                    };
                    let old_tmp = ctx.new_tmp(IRType::Int);
                    ctx.instructions.push(Instruction {
                        op: Op::LoadAt,
                        dst: Some(old_tmp.clone()),
                        src1: Some(addr.clone()),
                        src2: Some(Operand::ConstIdx(zero_idx.clone())),
                    });
                    self.emit_free(ctx, old_tmp, &field_ty)?;
                    val_op
                } else {
                    val_op
                };
                let result = val_op.clone();

                let store_op = if Context::type_to_ir_type(&field_ty) == IRType::Float {
                    Op::FStoreAt
                } else {
                    Op::StoreAt
                };
                ctx.instructions.push(Instruction {
                    op: store_op,
                    dst: Some(addr),
                    src1: Some(Operand::ConstIdx(zero_idx)),
                    src2: Some(val_op),
                });
                Ok(result)
            }

            Expr::AddressOf(inner, _) => match &*inner {
                Expr::Var(name, _) => {
                    if let Some(Operand::Global(gname)) = self.extern_op(name) {
                        let res_tmp = ctx.new_tmp(IRType::Int);
                        ctx.instructions.push(Instruction {
                            op: Op::Lea,
                            dst: Some(res_tmp.clone()),
                            src1: Some(Operand::Global(gname)),
                            src2: None,
                        });
                        return Ok(res_tmp);
                    }
                    let is_struct = match ctx.get_var_high_type(name) {
                        Some(Type::Struct(sname, _)) => self.structs.contains_key(sname),
                        Some(Type::Union(sname, _)) => self.unions.contains_key(sname),
                        _ => false,
                    };
                    if is_struct {
                        self.compile_expr(*inner, ctx)
                    } else {
                        let res_tmp = ctx.new_tmp(IRType::Int);
                        ctx.instructions.push(Instruction {
                            op: Op::Lea,
                            dst: Some(res_tmp.clone()),
                            src1: Some(Operand::Var(ctx.slot(name))),
                            src2: None,
                        });
                        Ok(res_tmp)
                    }
                }
                Expr::MemberAccess(obj, field_name, _) => {
                    let obj_type =
                        self.expr_high_type(obj, ctx)
                            .ok_or_else(|| CodeGenError::TypeError {
                                message: "cannot determine type for address-of".to_string(),
                            })?;
                    let (offset, _field_ty) = self.member_offset_and_type(&obj_type, field_name)?;
                    let base_op = self.compile_expr((**obj).clone(), ctx)?;
                    Ok(self.emit_field_addr(ctx, base_op, offset))
                }
                Expr::Index(arr, idx, _) => {
                    let arr_high = self.expr_high_type(arr, ctx);
                    let is_array = matches!(arr_high, Some(Type::Array(_)));
                    let (elem_type, byte) = self.index_info(arr, ctx);
                    if !is_array && elem_type.is_none() {
                        return Err(CodeGenError::UnsupportedOperation {
                            message: "cannot take address of index expression".to_string(),
                        });
                    }
                    let arr_op = self.compile_expr((**arr).clone(), ctx)?;
                    let off_op = self.compile_expr((**idx).clone(), ctx)?;
                    let scaled = if byte {
                        off_op
                    } else {
                        let s = ctx.new_tmp(IRType::Int);
                        let scale_idx = self.get_const_index(IRConst::Int(8));
                        ctx.instructions.push(Instruction {
                            op: Op::Mul,
                            dst: Some(s.clone()),
                            src1: Some(off_op),
                            src2: Some(Operand::ConstIdx(scale_idx)),
                        });
                        s
                    };
                    let sum = ctx.new_tmp(IRType::Int);
                    ctx.instructions.push(Instruction {
                        op: Op::Add,
                        dst: Some(sum.clone()),
                        src1: Some(arr_op),
                        src2: Some(scaled),
                    });

                    if is_array {
                        let with_prefix = self.emit_field_addr(ctx, sum, 8);
                        return Ok(with_prefix);
                    }
                    Ok(sum)
                }
                _ => Err(CodeGenError::UnsupportedOperation {
                    message: "address of non-variable".to_string(),
                }),
            },

            Expr::Deref(inner, _) => {
                let pointee_is_aggregate = match self.expr_high_type(&inner, ctx) {
                    Some(Type::Pointer(t)) => {
                        matches!(*t, Type::Struct(_, _) | Type::Union(_, _))
                    }
                    Some(Type::Struct(_, _)) | Some(Type::Union(_, _)) => true,
                    _ => false,
                };
                let ptr = self.compile_expr(*inner, ctx)?;
                if pointee_is_aggregate {
                    return Ok(ptr);
                }
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
                let result = val_op.clone();
                ctx.instructions.push(Instruction {
                    op: Op::StoreAt,
                    dst: Some(ptr_op),
                    src1: Some(Operand::ConstIdx(self.get_const_index(IRConst::Int(0)))),
                    src2: Some(val_op),
                });
                Ok(result)
            }

            Expr::TypeDef(_)
            | Expr::Struct(_, _, _, _)
            | Expr::Union(_, _, _, _)
            | Expr::Enum(_, _, _) => Ok(ctx.new_tmp(IRType::Void)),
            Expr::Lambda(_, _, _, _) => Err(CodeGenError::UnsupportedOperation {
                message: "internal error: unhoisted lambda reached code generation".to_string(),
            }),

            Expr::Match(target, branches, default, _) => {
                self.compile_match(target, branches, default, ctx)
            }
            Expr::FString(_, _) => {
                unreachable!("f-string should have been desugared in checker")
            }
            Expr::Cast(inner, target_ty, _) => {
                let src_ty = self.expr_high_type(&inner, ctx);
                let src_is_void = matches!(src_ty, Some(Type::Primitive(Primitive::Void)));
                let src = self.compile_expr(*inner, ctx)?;
                match &target_ty {
                    Type::Primitive(Primitive::Void) => Ok(ctx.new_tmp(IRType::Void)),

                    _ if src_is_void => {
                        let zero: Expr = match &target_ty {
                            Type::Primitive(Primitive::Float) => Expr::Float(0.0, Span::new(0, 0)),
                            Type::Primitive(Primitive::Boolean) => {
                                Expr::Bool(false, Span::new(0, 0))
                            }
                            Type::Primitive(Primitive::String) => {
                                Expr::String(String::new(), Span::new(0, 0))
                            }
                            _ => Expr::Int(0, Span::new(0, 0)),
                        };
                        return self.compile_expr(zero, ctx);
                    }
                    Type::Primitive(Primitive::Float) => {
                        if matches!(src_ty, Some(Type::Primitive(Primitive::Float))) {
                            return Ok(src);
                        }
                        let res_tmp = ctx.new_tmp(IRType::Float);
                        ctx.instructions.push(Instruction {
                            op: Op::IntToFloat,
                            dst: Some(res_tmp.clone()),
                            src1: Some(src),
                            src2: None,
                        });
                        Ok(res_tmp)
                    }
                    Type::Primitive(Primitive::Int) => {
                        if matches!(src_ty, Some(Type::Primitive(Primitive::Int))) {
                            return Ok(src);
                        }
                        let res_tmp = ctx.new_tmp(IRType::Int);
                        if matches!(src_ty, Some(Type::Primitive(Primitive::Boolean))) {
                            ctx.instructions.push(Instruction {
                                op: Op::Move,
                                dst: Some(res_tmp.clone()),
                                src1: Some(src),
                                src2: None,
                            });
                        } else {
                            ctx.instructions.push(Instruction {
                                op: Op::FloatToInt,
                                dst: Some(res_tmp.clone()),
                                src1: Some(src),
                                src2: None,
                            });
                        }
                        Ok(res_tmp)
                    }

                    Type::Primitive(Primitive::Boolean) => {
                        let (ne_op, zero_const) =
                            if matches!(src_ty, Some(Type::Primitive(Primitive::Float))) {
                                (Op::FNe, IRConst::Float(OrderedFloat(0.0)))
                            } else {
                                (Op::Ne, IRConst::Int(0))
                            };
                        let zero_idx = self.get_const_index(zero_const);
                        let res_tmp = ctx.new_tmp(IRType::Bool);
                        ctx.instructions.push(Instruction {
                            op: ne_op,
                            dst: Some(res_tmp.clone()),
                            src1: Some(src),
                            src2: Some(Operand::ConstIdx(zero_idx)),
                        });
                        Ok(res_tmp)
                    }

                    Type::Pointer(_) => Ok(src),

                    _ => Err(CodeGenError::UnsupportedOperation {
                        message: format!("cast to {:?} is not supported", target_ty),
                    }),
                }
            }
        }
    }
}
