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
    pub(super) fn is_resource_type(&self, ty: &Type) -> bool {
        match ty {
            Type::Struct(name, _) => self.structs.contains_key(name),
            Type::Union(name, _) => self.unions.contains_key(name),
            Type::Array(_) => true,
            Type::Primitive(Primitive::String) => true,
            _ => false,
        }
    }

    pub(super) fn can_move_var(&self, name: &str, ctx: &Context) -> bool {
        ctx.get_var_type(name).is_ok() && !ctx.borrowed.contains(name)
    }

    pub(super) fn check_use_after_move(
        &self,
        name: &str,
        span: Span,
        ctx: &Context,
    ) -> Result<(), CodeGenError> {
        if !ctx.moved.contains(name) || ctx.get_var_type(name).is_err() {
            return Ok(());
        }
        let is_resource = ctx
            .get_var_high_type(name)
            .map(|t| self.is_resource_type(t))
            .unwrap_or(false);
        if is_resource {
            return Err(CodeGenError::UseAfterMove {
                name: name.to_string(),
                moved_at: ctx.moved_at.get(name).copied().unwrap_or(span),
                span,
            });
        }
        Ok(())
    }

    pub(super) fn is_fresh_expr(&self, e: &Expr) -> bool {
        matches!(
            e,
            Expr::StructLiteral(..)
                | Expr::UnionLiteral(..)
                | Expr::ArrayLiteral(..)
                | Expr::ArrayFill(..)
                | Expr::Range(..)
                | Expr::If(..)
                | Expr::Match(..)
                | Expr::Call(..)
        )
    }

    pub(super) fn array_has_string_elems(e: &Expr) -> bool {
        match e {
            Expr::ArrayLiteral(items, _) => items.iter().any(|i| Self::expr_holds_string(i)),
            Expr::ArrayFill(ty, len, _) => {
                matches!(ty, Type::Primitive(Primitive::String))
                    || Self::array_has_string_elems(len)
            }
            _ => false,
        }
    }

    fn expr_holds_string(e: &Expr) -> bool {
        match e {
            Expr::String(..) => true,
            Expr::ArrayLiteral(..) => Self::array_has_string_elems(e),
            Expr::ArrayFill(ty, _, _) => matches!(ty, Type::Primitive(Primitive::String)),
            Expr::StructLiteral(_, _, fields, _) => {
                fields.iter().any(|(_, fe)| Self::expr_holds_string(fe))
            }
            Expr::UnionLiteral(_, _, fields, _) => {
                fields.iter().any(|(_, fe)| Self::expr_holds_string(fe))
            }
            _ => false,
        }
    }

    pub(super) fn resource_copy_info(&self, e: &Expr, ctx: &Context) -> Option<Type> {
        let ty = self.expr_high_type(e, ctx)?;
        if self.is_resource_type(&ty) && !self.is_fresh_expr(e) {
            Some(ty)
        } else {
            None
        }
    }

    pub(super) fn check_loop_moves(
        &self,
        before: &std::collections::HashSet<String>,
        ctx: &Context,
    ) -> Result<(), CodeGenError> {
        let newly: Vec<(String, Span)> = ctx
            .moved
            .iter()
            .filter(|n| !before.contains(*n))
            .filter_map(|n| ctx.moved_at.get(n).map(|sp| (n.clone(), *sp)))
            .collect();
        for (name, at) in newly {
            return Err(CodeGenError::UseAfterMove {
                name,
                moved_at: at,
                span: at,
            });
        }
        Ok(())
    }

    pub(super) fn elem_scale(ty: &Type) -> i64 {
        match ty {
            Type::Primitive(Primitive::Boolean) => 8,
            _ => 8,
        }
    }

    pub(super) fn emit_field_addr(
        &mut self,
        ctx: &mut Context,
        base_op: Operand,
        base: usize,
    ) -> Operand {
        if base == 0 {
            base_op
        } else {
            let addr_tmp = ctx.new_tmp(IRType::Int);
            let off_idx = self.get_const_index(IRConst::Int(base as i64));
            ctx.instructions.push(Instruction {
                op: Op::Add,
                dst: Some(addr_tmp.clone()),
                src1: Some(base_op),
                src2: Some(Operand::ConstIdx(off_idx)),
            });
            addr_tmp
        }
    }

    pub(super) fn emit_load_at(&mut self, ctx: &mut Context, addr: Operand) -> Operand {
        let val_tmp = ctx.new_tmp(IRType::Int);
        let zero_idx = self.get_const_index(IRConst::Int(0));
        ctx.instructions.push(Instruction {
            op: Op::LoadAt,
            dst: Some(val_tmp.clone()),
            src1: Some(addr),
            src2: Some(Operand::ConstIdx(zero_idx)),
        });
        val_tmp
    }

    pub(super) fn emit_const_store_at(&mut self, ctx: &mut Context, addr: Operand, val: Operand) {
        let zero_idx = self.get_const_index(IRConst::Int(0));
        ctx.instructions.push(Instruction {
            op: Op::StoreAt,
            dst: Some(addr),
            src1: Some(Operand::ConstIdx(zero_idx)),
            src2: Some(val),
        });
    }

    pub(super) fn emit_malloc(&mut self, ctx: &mut Context, size: i64) -> Operand {
        let ptr_tmp = ctx.new_tmp(IRType::Int);
        let size_idx = self.get_const_index(IRConst::Int(size));
        ctx.instructions.push(Instruction {
            op: Op::Malloc,
            dst: Some(ptr_tmp.clone()),
            src1: Some(Operand::ConstIdx(size_idx)),
            src2: None,
        });
        ptr_tmp
    }

    pub(super) fn emit_compound_assign(
        &mut self,
        name: String,
        value: Expr,
        bin_op: Op,
        ctx: &mut Context,
    ) -> Result<Operand, CodeGenError> {
        let is_local = ctx.get_var_type(&name).is_ok();
        let is_extern = !is_local
            && (self.extern_vars.contains_key(name.as_str())
                || self.global_storage.contains_key(name.as_str()));
        let var_op = if is_extern {
            Operand::Global(name.clone())
        } else {
            Operand::Var(ctx.slot(&name))
        };
        let is_float = if is_extern {
            matches!(self.glob_ir_type(&name), Some(IRType::Float))
        } else {
            matches!(ctx.get_var_type(&name), Ok(IRType::Float))
        };
        if is_float {
            let f_op = match bin_op {
                Op::Add => Op::FAdd,
                Op::Sub => Op::FSub,
                Op::Mul => Op::FMul,
                Op::Div => Op::FDiv,
                other => {
                    return Err(CodeGenError::TypeError {
                        message: format!(
                            "unsupported compound assignment {:?} on float variable '{}'",
                            other, name
                        ),
                    });
                }
            };
            let rhs = self.compile_expr(value, ctx)?;

            if ctx.get_operand_type(&rhs, &self.constants)? != IRType::Float {
                return Err(CodeGenError::TypeError {
                    message: String::from(
                        "float compound assignment requires a float right-hand side (use '@float')",
                    ),
                });
            }
            let rhs = rhs;
            let var_tmp = ctx.new_tmp(IRType::Float);
            ctx.instructions.push(Instruction {
                op: if is_extern { Op::FGlobLoad } else { Op::FLoad },
                dst: Some(var_tmp.clone()),
                src1: Some(var_op.clone()),
                src2: None,
            });
            let res_tmp = ctx.new_tmp(IRType::Float);
            ctx.instructions.push(Instruction {
                op: f_op,
                dst: Some(res_tmp.clone()),
                src1: Some(var_tmp),
                src2: Some(rhs),
            });
            ctx.instructions.push(Instruction {
                op: if is_extern {
                    Op::FGlobStore
                } else {
                    Op::FStore
                },
                dst: Some(var_op),
                src1: Some(res_tmp.clone()),
                src2: None,
            });
            return Ok(res_tmp);
        }
        let var_high = ctx.get_var_high_type(name.as_str()).cloned();
        let scale = if matches!(bin_op, Op::Add | Op::Sub) {
            var_high
                .as_ref()
                .and_then(|t| t.pointee())
                .map(Self::ptr_scale)
        } else {
            None
        };
        let rhs_raw = self.compile_expr(value, ctx)?;
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
            op: if is_extern { Op::GlobLoad } else { Op::Load },
            dst: Some(var_tmp.clone()),
            src1: Some(var_op.clone()),
            src2: None,
        });
        let res_tmp = ctx.new_tmp(IRType::Int);
        ctx.instructions.push(Instruction {
            op: bin_op,
            dst: Some(res_tmp.clone()),
            src1: Some(var_tmp),
            src2: Some(rhs),
        });
        ctx.instructions.push(Instruction {
            op: if is_extern { Op::GlobStore } else { Op::Store },
            dst: Some(var_op),
            src1: Some(res_tmp.clone()),
            src2: None,
        });
        Ok(res_tmp)
    }

    pub(super) fn emit_free_ptr(&mut self, ctx: &mut Context, ptr: Operand) {
        let zero_idx = self.get_const_index(IRConst::Int(0));
        let skip = ctx.new_label("freed");
        let chk = ctx.new_tmp(IRType::Bool);
        ctx.instructions.push(Instruction {
            op: Op::Ne,
            dst: Some(chk.clone()),
            src1: Some(ptr.clone()),
            src2: Some(Operand::ConstIdx(zero_idx)),
        });
        ctx.instructions.push(Instruction {
            op: Op::JumpIfFalse,
            dst: None,
            src1: Some(chk),
            src2: Some(Operand::Label(skip.clone())),
        });
        ctx.instructions.push(Instruction {
            op: Op::Free,
            dst: None,
            src1: Some(ptr),
            src2: None,
        });
        ctx.instructions.push(Instruction {
            op: Op::Label(skip),
            dst: None,
            src1: None,
            src2: None,
        });
    }

    pub(super) fn is_float_var(&self, name: &str, ctx: &Context) -> bool {
        match ctx.get_var_type(name) {
            Ok(t) => matches!(t, IRType::Float),
            Err(_) => matches!(self.glob_ir_type(name), Some(IRType::Float)),
        }
    }

    pub(super) fn emit_float_inc_dec(
        &mut self,
        name: &str,
        is_inc: bool,
        ctx: &mut Context,
    ) -> Result<Operand, CodeGenError> {
        let is_extern = ctx.get_var_type(name).is_err()
            && (self.extern_vars.contains_key(name) || self.global_storage.contains_key(name));
        let var_op = if is_extern {
            Operand::Global(name.to_string())
        } else {
            Operand::Var(ctx.slot(name))
        };
        let res_tmp = ctx.new_tmp(IRType::Float);
        ctx.instructions.push(Instruction {
            op: if is_extern { Op::FGlobLoad } else { Op::FLoad },
            dst: Some(res_tmp.clone()),
            src1: Some(var_op.clone()),
            src2: None,
        });
        let one_idx = self.get_const_index(IRConst::Float(OrderedFloat(1.0)));
        ctx.instructions.push(Instruction {
            op: if is_inc { Op::FAdd } else { Op::FSub },
            dst: Some(res_tmp.clone()),
            src1: Some(res_tmp.clone()),
            src2: Some(Operand::ConstIdx(one_idx)),
        });
        ctx.instructions.push(Instruction {
            op: if is_extern {
                Op::FGlobStore
            } else {
                Op::FStore
            },
            dst: Some(var_op),
            src1: Some(res_tmp.clone()),
            src2: None,
        });
        Ok(res_tmp)
    }

    pub(super) fn emit_array_len(&mut self, ctx: &mut Context, arr: Operand) -> Operand {
        let len_tmp = ctx.new_tmp(IRType::Int);
        ctx.instructions.push(Instruction {
            op: Op::SizeOf,
            dst: Some(len_tmp.clone()),
            src1: Some(arr),
            src2: None,
        });
        len_tmp
    }

    pub(super) fn emit_array_elem_loop<F>(
        ir: &mut IRGen,
        ctx: &mut Context,
        arr: Operand,
        len_op: Operand,
        elem_ir: IRType,
        mut body: F,
    ) -> Result<(), CodeGenError>
    where
        F: FnMut(&mut IRGen, &mut Context, Operand, Operand),
    {
        let label_cond = ctx.new_label("raii_cond");
        let label_inc = ctx.new_label("raii_inc");
        let label_end = ctx.new_label("raii_end");

        let idx_name = ctx.new_label("raii_idx");
        let idx_var = Operand::Var(idx_name.clone());
        ctx.declare_var(idx_name.clone(), IRType::Int)?;
        let zero_idx = ir.get_const_index(IRConst::Int(0));
        let one_idx = ir.get_const_index(IRConst::Int(1));
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
        let curr = ctx.new_tmp(IRType::Int);
        ctx.instructions.push(Instruction {
            op: Op::Load,
            dst: Some(curr.clone()),
            src1: Some(idx_var.clone()),
            src2: None,
        });
        let cond = ctx.new_tmp(IRType::Bool);
        ctx.instructions.push(Instruction {
            op: Op::Lt,
            dst: Some(cond.clone()),
            src1: Some(curr.clone()),
            src2: Some(len_op.clone()),
        });
        ctx.instructions.push(Instruction {
            op: Op::JumpIfFalse,
            dst: None,
            src1: Some(cond),
            src2: Some(Operand::Label(label_end.clone())),
        });
        let elem = ctx.new_tmp(elem_ir);
        ctx.instructions.push(Instruction {
            op: Op::ArrayAccess,
            dst: Some(elem.clone()),
            src1: Some(arr.clone()),
            src2: Some(curr.clone()),
        });
        body(ir, ctx, curr, elem);
        ctx.instructions.push(Instruction {
            op: Op::Label(label_inc.clone()),
            dst: None,
            src1: None,
            src2: None,
        });
        let curr2 = ctx.new_tmp(IRType::Int);
        ctx.instructions.push(Instruction {
            op: Op::Load,
            dst: Some(curr2.clone()),
            src1: Some(idx_var.clone()),
            src2: None,
        });
        let next = ctx.new_tmp(IRType::Int);
        ctx.instructions.push(Instruction {
            op: Op::Add,
            dst: Some(next.clone()),
            src1: Some(curr2),
            src2: Some(Operand::ConstIdx(one_idx)),
        });
        ctx.instructions.push(Instruction {
            op: Op::Store,
            dst: Some(idx_var),
            src1: Some(next),
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
        Ok(())
    }

    pub(super) fn emit_free(
        &mut self,
        ctx: &mut Context,
        ptr: Operand,
        ty: &Type,
    ) -> Result<(), CodeGenError> {
        let zero_idx = self.get_const_index(IRConst::Int(0));
        let nonnull = ctx.new_tmp(IRType::Bool);
        let end = ctx.new_label("freeend");
        ctx.instructions.push(Instruction {
            op: Op::Ne,
            dst: Some(nonnull.clone()),
            src1: Some(ptr.clone()),
            src2: Some(Operand::ConstIdx(zero_idx)),
        });
        ctx.instructions.push(Instruction {
            op: Op::JumpIfFalse,
            dst: None,
            src1: Some(nonnull),
            src2: Some(Operand::Label(end.clone())),
        });
        self.emit_free_inner(ctx, ptr, ty)?;
        ctx.instructions.push(Instruction {
            op: Op::Label(end),
            dst: None,
            src1: None,
            src2: None,
        });
        Ok(())
    }

    fn emit_free_inner(
        &mut self,
        ctx: &mut Context,
        ptr: Operand,
        ty: &Type,
    ) -> Result<(), CodeGenError> {
        match ty {
            Type::Struct(name, targs) => {
                let Some((_, fields)) = self.structs.get(name).cloned() else {
                    return Ok(());
                };
                for (i, (_, fty)) in fields.iter().enumerate() {
                    let fty = fty.substitute(targs);
                    if !self.is_resource_type(&fty) {
                        continue;
                    }
                    let addr = self.emit_field_addr(ctx, ptr.clone(), i * 8);
                    let fval = self.emit_load_at(ctx, addr);
                    self.emit_free(ctx, fval, &fty)?;
                }
                self.emit_free_ptr(ctx, ptr);
                Ok(())
            }
            Type::Union(..) => {
                self.emit_free_ptr(ctx, ptr);
                Ok(())
            }
            Type::Primitive(Primitive::String) => {
                self.emit_free_ptr(ctx, ptr);
                Ok(())
            }
            Type::Array(elem) => {
                if self.is_resource_type(elem) {
                    let len = self.emit_array_len(ctx, ptr.clone());
                    let elem_ty = *elem.clone();
                    let elem_ir = Context::type_to_ir_type(&elem_ty);
                    Self::emit_array_elem_loop(
                        self,
                        ctx,
                        ptr.clone(),
                        len,
                        elem_ir,
                        |ir, c, _, e| {
                            let _ = ir.emit_free(c, e, &elem_ty);
                        },
                    )?;
                }
                self.emit_free_ptr(ctx, ptr);
                Ok(())
            }
            _ => Ok(()),
        }
    }

    pub(super) fn emit_var_free(
        &mut self,
        ctx: &mut Context,
        name: &str,
    ) -> Result<(), CodeGenError> {
        if ctx.borrowed.contains(name) {
            return Ok(());
        }
        let Some(hty) = ctx.var_types.get(name).cloned() else {
            return Ok(());
        };
        if !self.is_resource_type(&hty) {
            return Ok(());
        }
        let val = ctx.new_tmp(IRType::Int);
        ctx.instructions.push(Instruction {
            op: Op::Load,
            dst: Some(val.clone()),
            src1: Some(Operand::Var(ctx.slot(&name))),
            src2: None,
        });
        self.emit_free(ctx, val, &hty)
    }

    pub(super) fn emit_scope_frees(&mut self, ctx: &mut Context) -> Result<(), CodeGenError> {
        let Some(scope) = ctx.scope.last() else {
            return Ok(());
        };
        let names: Vec<String> = scope.keys().cloned().collect();
        for name in names {
            self.emit_var_free(ctx, &name)?;
        }
        Ok(())
    }

    pub(super) fn emit_all_scope_frees(&mut self, ctx: &mut Context) -> Result<(), CodeGenError> {
        let names: std::collections::HashSet<String> =
            ctx.scope.iter().flat_map(|s| s.keys().cloned()).collect();
        for name in names {
            self.emit_var_free(ctx, &name)?;
        }
        Ok(())
    }

    pub(super) fn emit_scope_frees_from(
        &mut self,
        ctx: &mut Context,
        min_depth: usize,
    ) -> Result<(), CodeGenError> {
        let names: std::collections::HashSet<String> = ctx
            .scope
            .iter()
            .skip(min_depth)
            .flat_map(|s| s.keys().cloned())
            .collect();
        for name in names {
            self.emit_var_free(ctx, &name)?;
        }
        Ok(())
    }

    pub(super) fn copy_resource(
        &mut self,
        ctx: &mut Context,
        src: Operand,
        ty: &Type,
    ) -> Result<Operand, CodeGenError> {
        if matches!(ctx.get_operand_type(&src, &self.constants)?, IRType::Void) {
            return Ok(src);
        }
        match ty {
            Type::Struct(name, targs) => {
                let Some((_, fields)) = self.structs.get(name).cloned() else {
                    return Ok(src);
                };
                let dst = self.emit_malloc(ctx, (fields.len() * 8) as i64);
                for (i, (_, fty)) in fields.iter().enumerate() {
                    let fty = fty.substitute(targs);
                    let src_addr = self.emit_field_addr(ctx, src.clone(), i * 8);
                    let fval = self.emit_load_at(ctx, src_addr);
                    let fcopy = if self.is_resource_type(&fty) {
                        self.copy_resource(ctx, fval, &fty)?
                    } else {
                        fval
                    };
                    let dst_addr = self.emit_field_addr(ctx, dst.clone(), i * 8);
                    self.emit_const_store_at(ctx, dst_addr, fcopy);
                }
                Ok(dst)
            }
            Type::Union(..) => {
                let dst = self.emit_malloc(ctx, 8);
                let fval = self.emit_load_at(ctx, src);
                self.emit_const_store_at(ctx, dst.clone(), fval);
                Ok(dst)
            }
            Type::Primitive(Primitive::String) => {
                let len_tmp = ctx.new_tmp(IRType::Int);
                ctx.instructions.push(Instruction {
                    op: Op::Arg(0),
                    dst: None,
                    src1: Some(src.clone()),
                    src2: None,
                });
                ctx.instructions.push(Instruction {
                    op: Op::Call,
                    dst: Some(len_tmp.clone()),
                    src1: Some(Operand::Function("strlen".to_string())),
                    src2: None,
                });
                let one_idx = self.get_const_index(IRConst::Int(1));
                let size_tmp = ctx.new_tmp(IRType::Int);
                ctx.instructions.push(Instruction {
                    op: Op::Add,
                    dst: Some(size_tmp.clone()),
                    src1: Some(len_tmp),
                    src2: Some(Operand::ConstIdx(one_idx)),
                });
                let dst = ctx.new_tmp(IRType::String);
                ctx.instructions.push(Instruction {
                    op: Op::Malloc,
                    dst: Some(dst.clone()),
                    src1: Some(size_tmp),
                    src2: None,
                });
                ctx.instructions.push(Instruction {
                    op: Op::Arg(0),
                    dst: None,
                    src1: Some(dst.clone()),
                    src2: None,
                });
                ctx.instructions.push(Instruction {
                    op: Op::Arg(1),
                    dst: None,
                    src1: Some(src),
                    src2: None,
                });
                let ret_tmp = ctx.new_tmp(IRType::Int);
                ctx.instructions.push(Instruction {
                    op: Op::Call,
                    dst: Some(ret_tmp),
                    src1: Some(Operand::Function("strcpy".to_string())),
                    src2: None,
                });
                Ok(dst)
            }
            Type::Array(elem) => {
                let len = self.emit_array_len(ctx, src.clone());
                let elem_ty = *elem.clone();
                let esize = Self::elem_scale(&elem_ty);
                let esize_idx = self.get_const_index(IRConst::Int(esize));
                let byte_len = ctx.new_tmp(IRType::Int);
                ctx.instructions.push(Instruction {
                    op: Op::Mul,
                    dst: Some(byte_len.clone()),
                    src1: Some(len.clone()),
                    src2: Some(Operand::ConstIdx(esize_idx)),
                });
                let eight_idx = self.get_const_index(IRConst::Int(8));
                let total = ctx.new_tmp(IRType::Int);
                ctx.instructions.push(Instruction {
                    op: Op::Add,
                    dst: Some(total.clone()),
                    src1: Some(byte_len),
                    src2: Some(Operand::ConstIdx(eight_idx)),
                });
                let dst = ctx.new_tmp(IRType::Int);
                ctx.instructions.push(Instruction {
                    op: Op::Malloc,
                    dst: Some(dst.clone()),
                    src1: Some(total),
                    src2: None,
                });
                let zero_idx = self.get_const_index(IRConst::Int(0));
                ctx.instructions.push(Instruction {
                    op: Op::StoreAt,
                    dst: Some(dst.clone()),
                    src1: Some(Operand::ConstIdx(zero_idx)),
                    src2: Some(len.clone()),
                });
                let elem_ir = Context::type_to_ir_type(&elem_ty);
                let deep_elem = self.is_resource_type(&elem_ty);
                let dst_c = dst.clone();
                Self::emit_array_elem_loop(self, ctx, src, len, elem_ir, |ir, c, idx, e| {
                    let ev = if deep_elem {
                        match ir.copy_resource(c, e.clone(), &elem_ty) {
                            Ok(copied) => copied,
                            Err(_) => e,
                        }
                    } else {
                        e
                    };
                    c.instructions.push(Instruction {
                        op: Op::ArrayAssign,
                        dst: Some(dst_c.clone()),
                        src1: Some(idx),
                        src2: Some(ev),
                    });
                })?;
                Ok(dst)
            }
            _ => Ok(src),
        }
    }
}
