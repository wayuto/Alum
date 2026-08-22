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

        for (i, (field_name, field_ty)) in fields.iter().enumerate() {
            let Some((_, field_expr)) = field_values.iter().find(|(n, _)| n == field_name) else {
                let zero_const = match field_ty {
                    Type::Primitive(Primitive::Float) => IRConst::Float(OrderedFloat(0.0)),
                    _ => IRConst::Int(0),
                };
                let zero_idx = self.get_const_index(zero_const);
                let offset_idx = self.get_const_index(IRConst::Int((i * 8) as i64));
                ctx.instructions.push(Instruction {
                    op: Op::StoreAt,
                    dst: Some(ptr_tmp.clone()),
                    src1: Some(Operand::ConstIdx(offset_idx)),
                    src2: Some(Operand::ConstIdx(zero_idx)),
                });
                continue;
            };
            let copy_info = self.resource_copy_info(field_expr, ctx);
            let val = self.compile_expr(field_expr.clone(), ctx)?;
            let val = match copy_info {
                Some(ty) => self.copy_resource(ctx, val, &ty)?,
                None => val,
            };
            let offset_idx = self.get_const_index(IRConst::Int((i * 8) as i64));
            ctx.instructions.push(Instruction {
                op: Op::StoreAt,
                dst: Some(ptr_tmp.clone()),
                src1: Some(Operand::ConstIdx(offset_idx)),
                src2: Some(val),
            });
        }

        Ok(ptr_tmp)
    }

    pub(super) fn compile_union_literal(
        &mut self,
        union_name: &str,
        field_values: Vec<(String, Expr)>,
        ctx: &mut Context,
    ) -> Result<Operand, CodeGenError> {
        let (_, fields) = self
            .unions
            .get(union_name)
            .ok_or_else(|| CodeGenError::NameError {
                message: format!("undefined union '{}'", union_name),
            })?;
        let fields = fields.clone();
        let size_idx = self.get_const_index(IRConst::Int(8));
        let ptr_tmp = ctx.new_tmp(IRType::Int);

        ctx.instructions.push(Instruction {
            op: Op::Malloc,
            dst: Some(ptr_tmp.clone()),
            src1: Some(Operand::ConstIdx(size_idx)),
            src2: None,
        });

        for (field_name, _) in &fields {
            if let Some((_, field_expr)) = field_values.iter().find(|(n, _)| n == field_name) {
                let val = self.compile_expr(field_expr.clone(), ctx)?;
                let offset_idx = self.get_const_index(IRConst::Int(0));
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

    pub(super) fn enum_member_value(&self, name: &str) -> Result<Option<isize>, CodeGenError> {
        let mut found: Vec<(&str, isize)> = Vec::new();
        for (enum_name, members) in &self.enums {
            for (member_name, value) in members {
                if member_name == name {
                    found.push((enum_name.as_str(), *value));
                }
            }
        }
        if found.len() > 1 {
            let mut names: Vec<String> = found.iter().map(|(n, _)| n.to_string()).collect();
            names.sort();
            Err(CodeGenError::NameError {
                message: format!(
                    "enum member '{}' is ambiguous (defined in {}); qualify it as <EnumName>.{}",
                    name,
                    names.join(", "),
                    name
                ),
            })
        } else {
            Ok(found.first().map(|(_, v)| *v))
        }
    }

    fn member_addr(
        &mut self,
        expr: &Expr,
        ctx: &mut Context,
    ) -> Result<(Operand, Type), CodeGenError> {
        match expr {
            Expr::Var(name, _) => {
                let hty = ctx
                    .get_var_high_type(name)
                    .ok_or_else(|| CodeGenError::TypeError {
                        message: format!("member access on non-struct variable '{}'", name),
                    })?
                    .clone();
                let op = self.compile_expr(expr.clone(), ctx)?;
                Ok((op, hty))
            }
            Expr::MemberAccess(obj, field_name, _) => {
                let (obj_addr, obj_type) = self.member_addr(obj, ctx)?;
                let (offset, field_type) = self.member_offset_and_type(&obj_type, field_name)?;
                let field_addr = if offset == 0 {
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
                let is_container = matches!(&field_type, Type::Struct(_, _) | Type::Union(_, _))
                    || matches!(&field_type, Type::Pointer(inner) if matches!(inner.as_ref(), Type::Struct(_, _) | Type::Union(_, _)));
                if is_container {
                    let ptr_tmp = ctx.new_tmp(IRType::Int);
                    let zero_idx = self.get_const_index(IRConst::Int(0));
                    ctx.instructions.push(Instruction {
                        op: Op::LoadAt,
                        dst: Some(ptr_tmp.clone()),
                        src1: Some(field_addr),
                        src2: Some(Operand::ConstIdx(zero_idx)),
                    });
                    Ok((ptr_tmp, field_type))
                } else {
                    Ok((field_addr, field_type))
                }
            }
            _ => Err(CodeGenError::TypeError {
                message: "member access on non-variable expression".to_string(),
            }),
        }
    }

    pub(super) fn struct_field_fn_ret(
        &self,
        struct_name: &str,
        type_args: &[Type],
        method: &str,
    ) -> Option<Type> {
        let (_, fields) = self.structs.get(struct_name)?;
        for (fname, fty) in fields {
            if fname == method {
                let substituted = fty.substitute(type_args);
                if let Type::Function(_, ret) = substituted {
                    return Some(*ret);
                }
            }
        }
        None
    }

    pub(super) fn load_function_field(
        &mut self,
        obj_op: Operand,
        obj_type: &Type,
        field: &str,
        ctx: &mut Context,
    ) -> Result<Operand, CodeGenError> {
        let (offset, _) = self.member_offset_and_type(obj_type, field)?;
        let addr = if offset == 0 {
            obj_op
        } else {
            let addr_tmp = ctx.new_tmp(IRType::Int);
            let offset_idx = self.get_const_index(IRConst::Int(offset as i64));
            ctx.instructions.push(Instruction {
                op: Op::Add,
                dst: Some(addr_tmp.clone()),
                src1: Some(obj_op),
                src2: Some(Operand::ConstIdx(offset_idx)),
            });
            addr_tmp
        };
        let fn_tmp = ctx.new_tmp(IRType::Int);
        let zero_idx = self.get_const_index(IRConst::Int(0));
        ctx.instructions.push(Instruction {
            op: Op::LoadAt,
            dst: Some(fn_tmp.clone()),
            src1: Some(addr),
            src2: Some(Operand::ConstIdx(zero_idx)),
        });
        Ok(fn_tmp)
    }

    fn member_offset_and_type(
        &self,
        obj_type: &Type,
        field_name: &str,
    ) -> Result<(usize, Type), CodeGenError> {
        let (type_name, type_args) = match obj_type {
            Type::Struct(sname, args) => (sname.clone(), args.clone()),
            Type::Union(sname, args) => (sname.clone(), args.clone()),
            Type::Pointer(inner) => match inner.as_ref() {
                Type::Struct(sname, args) => (sname.clone(), args.clone()),
                Type::Union(sname, args) => (sname.clone(), args.clone()),
                _ => {
                    return Err(CodeGenError::TypeError {
                        message: "member access on non-struct type".to_string(),
                    });
                }
            },
            _ => {
                return Err(CodeGenError::TypeError {
                    message: format!("member access on non-struct type '{}'", obj_type),
                });
            }
        };
        let is_union = self.unions.contains_key(&type_name);
        let type_def = if is_union {
            self.unions
                .get(&type_name)
                .ok_or_else(|| CodeGenError::NameError {
                    message: format!("undefined union '{}'", type_name),
                })?
        } else {
            self.structs
                .get(&type_name)
                .ok_or_else(|| CodeGenError::NameError {
                    message: format!("undefined struct '{}'", type_name),
                })?
        };
        let mut offset = 0;
        let mut found = false;
        let mut field_type = None;
        for (i, (fname, ftype)) in type_def.1.iter().enumerate() {
            if fname == field_name {
                found = true;
                offset = if is_union { 0 } else { i * 8 };
                field_type = Some(ftype.clone());
                break;
            }
        }
        if !found {
            return Err(CodeGenError::NameError {
                message: format!("type '{}' has no field '{}'", type_name, field_name),
            });
        }
        Ok((offset, field_type.unwrap().substitute(&type_args)))
    }

    fn extern_op(&self, name: &str) -> Option<Operand> {
        if self.extern_vars.contains_key(name) || self.global_storage.contains_key(name) {
            Some(Operand::Global(name.to_string()))
        } else {
            None
        }
    }

    fn glob_ir_type(&self, name: &str) -> Option<IRType> {
        if let Some(t) = self.extern_vars.get(name) {
            Some(Context::type_to_ir_type(t))
        } else {
            self.global_storage.get(name).map(|(t, _, _)| t.clone())
        }
    }

    fn extern_load_op(&self, name: &str) -> Option<Op> {
        self.glob_ir_type(name).map(|t| match t {
            IRType::Float => Op::FGlobLoad,
            _ => Op::GlobLoad,
        })
    }

    fn extern_store_op(&self, name: &str) -> Option<Op> {
        self.glob_ir_type(name).map(|t| match t {
            IRType::Float => Op::FGlobStore,
            _ => Op::GlobStore,
        })
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

    pub(super) fn is_resource_type(&self, ty: &Type) -> bool {
        match ty {
            Type::Struct(name, _) => self.structs.contains_key(name),
            Type::Union(name, _) => self.unions.contains_key(name),
            Type::Array(_) => true,
            _ => false,
        }
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

    pub(super) fn resource_copy_info(&self, e: &Expr, ctx: &Context) -> Option<Type> {
        let ty = self.expr_high_type(e, ctx)?;
        if self.is_resource_type(&ty) && !self.is_fresh_expr(e) {
            Some(ty)
        } else {
            None
        }
    }

    fn elem_scale(ty: &Type) -> i64 {
        match ty {
            Type::Primitive(Primitive::Boolean) => 8,
            _ => 8,
        }
    }

    fn emit_field_addr(&mut self, ctx: &mut Context, base_op: Operand, base: usize) -> Operand {
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

    fn emit_load_at(&mut self, ctx: &mut Context, addr: Operand) -> Operand {
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

    fn emit_const_store_at(&mut self, ctx: &mut Context, addr: Operand, val: Operand) {
        let zero_idx = self.get_const_index(IRConst::Int(0));
        ctx.instructions.push(Instruction {
            op: Op::StoreAt,
            dst: Some(addr),
            src1: Some(Operand::ConstIdx(zero_idx)),
            src2: Some(val),
        });
    }

    fn emit_malloc(&mut self, ctx: &mut Context, size: i64) -> Operand {
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
            let rhs = match ctx.get_operand_type(&rhs, &self.constants)? {
                IRType::Float => rhs,
                _ => {
                    let cvt_tmp = ctx.new_tmp(IRType::Float);
                    ctx.instructions.push(Instruction {
                        op: Op::IntToFloat,
                        dst: Some(cvt_tmp.clone()),
                        src1: Some(rhs),
                        src2: None,
                    });
                    cvt_tmp
                }
            };
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
        ctx.instructions.push(Instruction {
            op: Op::Free,
            dst: None,
            src1: Some(ptr),
            src2: None,
        });
    }

    fn is_float_var(&self, name: &str, ctx: &Context) -> bool {
        match ctx.get_var_type(name) {
            Ok(t) => matches!(t, IRType::Float),
            Err(_) => matches!(self.glob_ir_type(name), Some(IRType::Float)),
        }
    }

    fn emit_float_inc_dec(
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

    fn emit_array_len(&mut self, ctx: &mut Context, arr: Operand) -> Operand {
        let len_tmp = ctx.new_tmp(IRType::Int);
        ctx.instructions.push(Instruction {
            op: Op::SizeOf,
            dst: Some(len_tmp.clone()),
            src1: Some(arr),
            src2: None,
        });
        len_tmp
    }

    fn emit_array_elem_loop<F>(
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
                Ok(())
            }
            Type::Union(..) => Ok(()),
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
                let copy_info = self.resource_copy_info(&value, ctx);
                let resolved_typ =
                    if matches!(typ, Type::Unknown | Type::TypeVar(_) | Type::Param(_)) {
                        self.expr_high_type(&value, ctx)
                            .unwrap_or_else(|| typ.clone())
                    } else {
                        typ.clone()
                    };
                let is_struct = matches!(&resolved_typ, Type::Struct(_, _) | Type::Union(_, _));
                let mut value: Operand = if is_struct {
                    self.compile_expr(*value, ctx)?
                } else {
                    match self.eval_const(&value, Some(&*ctx)) {
                        Some((cv, IRType::Int | IRType::Float | IRType::Bool | IRType::Array)) => {
                            Operand::ConstIdx(self.get_const_index(cv))
                        }
                        _ => self.compile_expr(*value, ctx)?,
                    }
                };
                if let Some(copy_ty) = copy_info {
                    value = self.copy_resource(ctx, value, &copy_ty)?;
                }
                let var_ir_type = Context::type_to_ir_type(&resolved_typ);

                ctx.declare_var_with_type(name.clone(), var_ir_type.clone(), resolved_typ)?;
                if matches!(var_ir_type, IRType::Array) {
                    if let Some(len) = self.const_array_len(&value, ctx) {
                        ctx.array_lengths.insert(name.clone(), len);
                    } else if let Operand::ConstIdx(idx) = &value {
                        if let IRConst::Array(elems) = &self.constants[*idx] {
                            ctx.array_lengths.insert(name.clone(), elems.len());
                        }
                    }
                }

                let result = value.clone();
                match var_ir_type {
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
                Ok(result)
            }

            Expr::ConstDecl(name, typ, value, _, _) => {
                let resolved_typ =
                    if matches!(typ, Type::Unknown | Type::TypeVar(_) | Type::Param(_)) {
                        self.expr_high_type(&value, ctx)
                            .unwrap_or_else(|| typ.clone())
                    } else {
                        typ.clone()
                    };
                let is_struct = matches!(&resolved_typ, Type::Struct(_, _) | Type::Union(_, _));
                let value = if is_struct {
                    self.compile_expr(*value, ctx)?
                } else {
                    match self.eval_const(&value, Some(&*ctx)) {
                        Some((cv, IRType::Int | IRType::Float | IRType::Bool | IRType::Array)) => {
                            Operand::ConstIdx(self.get_const_index(cv))
                        }
                        _ => self.compile_expr(*value, ctx)?,
                    }
                };
                let var_ir_type = Context::type_to_ir_type(&resolved_typ);

                ctx.declare_var_with_type(name.clone(), var_ir_type.clone(), resolved_typ)?;
                if matches!(var_ir_type, IRType::Array) {
                    if let Some(len) = self.const_array_len(&value, ctx) {
                        ctx.array_lengths.insert(name.clone(), len);
                    } else if let Operand::ConstIdx(idx) = &value {
                        if let IRConst::Array(elems) = &self.constants[*idx] {
                            ctx.array_lengths.insert(name.clone(), elems.len());
                        }
                    }
                }

                let result = value.clone();
                match var_ir_type {
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
                Ok(result)
            }

            Expr::GlobalVar(_, _, _, _, _) => Ok(ctx.new_tmp(IRType::Void)),

            Expr::VarAssign(name, value, _) => {
                let value_copy_info = self.resource_copy_info(&value, ctx);
                let value = self.compile_expr(*value, ctx)?;
                let typ = ctx.get_operand_type(&value, &self.constants)?;

                if ctx.get_var_type(&name).is_err() {
                    if let Some(store_op) = self.extern_store_op(&name) {
                        let is_float = matches!(store_op, Op::FGlobStore);
                        let var_typ = if is_float { IRType::Float } else { IRType::Int };
                        if typ != var_typ && typ != IRType::Array {
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
                        return Ok(result);
                    }
                }
                let var_typ = ctx.get_var_type(&name)?;
                if typ != var_typ {
                    return Err(CodeGenError::TypeError {
                        message: format!("unexpected type: {:?}", typ),
                    });
                }
                if matches!(var_typ, IRType::Array) {
                    if let Some(len) = self.const_array_len(&value, ctx) {
                        ctx.array_lengths.insert(name.clone(), len);
                    } else if let Operand::ConstIdx(idx) = &value {
                        if let IRConst::Array(elems) = &self.constants[*idx] {
                            ctx.array_lengths.insert(name.clone(), elems.len());
                        }
                    }
                }
                if let Some(hty) = ctx.var_types.get(&name) {
                    if self.is_resource_type(hty) {
                        let value = match value_copy_info {
                            Some(ty) => self.copy_resource(ctx, value, &ty)?,
                            None => value,
                        };
                        let result = value.clone();
                        self.emit_var_free(ctx, &name)?;
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
                Ok(result)
            }

            Expr::Var(name, _) => {
                if let Ok(var_type) = ctx.get_var_type(&name) {
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
                } else if let Ok(func) = self.find_func(&name) {
                    Ok(Operand::Function(func.name))
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

            Expr::Inc(name, _) => {
                if self.is_float_var(&name, ctx) {
                    return self.emit_float_inc_dec(&name, true, ctx);
                }
                if let Some(load_op) = self.extern_load_op(&name) {
                    let dst = self.extern_op(&name).unwrap();
                    let res_tmp = ctx.new_tmp(IRType::Int);
                    ctx.instructions.push(Instruction {
                        op: load_op,
                        dst: Some(res_tmp.clone()),
                        src1: Some(dst.clone()),
                        src2: None,
                    });
                    ctx.instructions.push(Instruction {
                        op: Op::Inc,
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
                let var_op = Operand::Var(ctx.slot(&name));
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
                if self.is_float_var(&name, ctx) {
                    return self.emit_float_inc_dec(&name, false, ctx);
                }
                if let Some(load_op) = self.extern_load_op(&name) {
                    let dst = self.extern_op(&name).unwrap();
                    let res_tmp = ctx.new_tmp(IRType::Int);
                    ctx.instructions.push(Instruction {
                        op: load_op,
                        dst: Some(res_tmp.clone()),
                        src1: Some(dst.clone()),
                        src2: None,
                    });
                    ctx.instructions.push(Instruction {
                        op: Op::Dec,
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
                let var_op = Operand::Var(ctx.slot(&name));
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
                self.emit_compound_assign(name, *value, Op::LAnd, ctx)
            }

            Expr::OrAssign(name, value, _) => self.emit_compound_assign(name, *value, Op::LOr, ctx),

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
                let copy_info = self.resource_copy_info(&val, ctx);
                let res_op = self.compile_expr(*val, ctx)?;
                let res_op = match copy_info {
                    Some(ty) => self.copy_resource(ctx, res_op, &ty)?,
                    None => res_op,
                };
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

            Expr::Break(_) => self.compile_break(ctx),

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
                let res_tmp = ctx.new_tmp(field_ir_type);
                let zero_idx = self.get_const_index(IRConst::Int(0));
                ctx.instructions.push(Instruction {
                    op: Op::LoadAt,
                    dst: Some(res_tmp.clone()),
                    src1: Some(addr),
                    src2: Some(Operand::ConstIdx(zero_idx)),
                });
                Ok(res_tmp)
            }

            Expr::MemberAssign(obj, _field_name, value, _) => {
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
                ctx.instructions.push(Instruction {
                    op: Op::StoreAt,
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
            | Expr::Enum(_, _, _)
            | Expr::Lambda(_, _, _, _) => Ok(ctx.new_tmp(IRType::Void)),

            Expr::Match(target, branches, default, _) => {
                self.compile_match(target, branches, default, ctx)
            }
            Expr::FString(_, _) => {
                unreachable!("f-string should have been desugared in checker")
            }
            Expr::Cast(inner, target_ty, _) => {
                let src_ty = self.expr_high_type(&inner, ctx);
                let src = self.compile_expr(*inner, ctx)?;
                match &target_ty {
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
                    _ => Err(CodeGenError::UnsupportedOperation {
                        message: format!("cast to {:?} is not supported", target_ty),
                    }),
                }
            }
        }
    }
}
