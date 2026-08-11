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

    fn struct_field_fn_ret(
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

    fn load_function_field(
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
            Some(Context::type2ir_type(t))
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

    fn is_fresh_expr(&self, e: &Expr) -> bool {
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

    fn resource_copy_info(&self, e: &Expr, ctx: &Context) -> Option<Type> {
        let ty = self.expr_high_type(e, ctx)?;
        if self.is_resource_type(&ty) && !self.is_fresh_expr(e) {
            Some(ty)
        } else {
            None
        }
    }

    fn elem_scale(ty: &Type) -> i64 {
        match ty {
            Type::Primitive(Primitive::Boolean) => 1,
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

    fn emit_free_ptr(&mut self, ctx: &mut Context, ptr: Operand) {
        ctx.instructions.push(Instruction {
            op: Op::Free,
            dst: None,
            src1: Some(ptr),
            src2: None,
        });
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

    fn emit_free(
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
                    let elem_ir = Context::type2ir_type(&elem_ty);
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
                let elem_ir = Context::type2ir_type(&elem_ty);
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
                let mut value: Operand = match self.eval_const(&value) {
                    Some((cv, IRType::Int | IRType::Float | IRType::Bool | IRType::Array)) => {
                        Operand::ConstIdx(self.get_const_index(cv))
                    }
                    _ => self.compile_expr(*value, ctx)?,
                };
                if let Some(copy_ty) = copy_info {
                    value = self.copy_resource(ctx, value, &copy_ty)?;
                }
                let var_ir_type = Context::type2ir_type(&resolved_typ);

                if matches!(var_ir_type, IRType::Array) {
                    if let Some(len) = self.const_array_len(&value, ctx) {
                        ctx.array_lengths.insert(name.clone(), len);
                    } else if let Operand::ConstIdx(idx) = &value {
                        if let IRConst::Array(elems) = &self.constants[*idx] {
                            ctx.array_lengths.insert(name.clone(), elems.len());
                        }
                    }
                }

                ctx.declare_var_with_type(name.clone(), var_ir_type.clone(), resolved_typ)?;
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
                Ok(ctx.new_tmp(IRType::Void))
            }

            Expr::ConstDecl(name, typ, value, _, _) => {
                let resolved_typ =
                    if matches!(typ, Type::Unknown | Type::TypeVar(_) | Type::Param(_)) {
                        self.expr_high_type(&value, ctx)
                            .unwrap_or_else(|| typ.clone())
                    } else {
                        typ.clone()
                    };
                let value = match self.eval_const(&value) {
                    Some((cv, IRType::Int | IRType::Float | IRType::Bool | IRType::Array)) => {
                        Operand::ConstIdx(self.get_const_index(cv))
                    }
                    _ => self.compile_expr(*value, ctx)?,
                };
                let var_ir_type = Context::type2ir_type(&resolved_typ);

                if matches!(var_ir_type, IRType::Array) {
                    if let Some(len) = self.const_array_len(&value, ctx) {
                        ctx.array_lengths.insert(name.clone(), len);
                    } else if let Operand::ConstIdx(idx) = &value {
                        if let IRConst::Array(elems) = &self.constants[*idx] {
                            ctx.array_lengths.insert(name.clone(), elems.len());
                        }
                    }
                }

                ctx.declare_var_with_type(name.clone(), var_ir_type.clone(), resolved_typ)?;
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
                Ok(ctx.new_tmp(IRType::Void))
            }

            Expr::GlobalVar(_, _, _, _, _) => Ok(ctx.new_tmp(IRType::Void)),

            Expr::VarAssign(name, value, _) => {
                let value_copy_info = self.resource_copy_info(&value, ctx);
                let value = self.compile_expr(*value, ctx)?;
                let typ = ctx.get_operand_type(&value, &self.constants)?;
                if let Some(store_op) = self.extern_store_op(&name) {
                    let is_float = matches!(store_op, Op::FGlobStore);
                    let var_typ = if is_float { IRType::Float } else { IRType::Int };
                    if typ != var_typ && typ != IRType::Array {
                        return Err(CodeGenError::TypeError {
                            message: format!("unexpected type: {:?}", typ),
                        });
                    }
                    let dst = self.extern_op(&name).unwrap();
                    ctx.instructions.push(Instruction {
                        op: store_op,
                        dst: Some(dst),
                        src1: Some(value),
                        src2: None,
                    });
                    return Ok(ctx.new_tmp(IRType::Void));
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
                        return Ok(ctx.new_tmp(IRType::Void));
                    }
                }
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
                Ok(ctx.new_tmp(IRType::Void))
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
                let is_extern = self.extern_vars.contains_key(name.as_str())
                    || self.global_storage.contains_key(name.as_str());
                let var_op = if is_extern {
                    Operand::Global(name.clone())
                } else {
                    Operand::Var(ctx.slot(&name))
                };
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
                    op: if is_extern { Op::GlobLoad } else { Op::Load },
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
                    op: if is_extern { Op::GlobStore } else { Op::Store },
                    dst: Some(var_op),
                    src1: Some(res_tmp.clone()),
                    src2: None,
                });
                Ok(res_tmp)
            }

            Expr::SubAssign(name, value, _) => {
                let is_extern = self.extern_vars.contains_key(name.as_str())
                    || self.global_storage.contains_key(name.as_str());
                let var_op = if is_extern {
                    Operand::Global(name.clone())
                } else {
                    Operand::Var(ctx.slot(&name))
                };
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
                    op: if is_extern { Op::GlobLoad } else { Op::Load },
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
                    op: if is_extern { Op::GlobStore } else { Op::Store },
                    dst: Some(var_op),
                    src1: Some(res_tmp.clone()),
                    src2: None,
                });
                Ok(res_tmp)
            }

            Expr::Block(body, _) => {
                let is_loop_body = ctx.loop_body_pending;
                ctx.loop_body_pending = false;
                let cont_label = if is_loop_body && !ctx.loop_inc_labels.is_empty() {
                    let cont = ctx.new_label("block_cont");
                    ctx.loop_inc_labels.push(cont.clone());
                    Some(cont)
                } else {
                    None
                };
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
                if let Some(cont) = cont_label {
                    ctx.instructions.push(Instruction {
                        op: Op::Label(cont),
                        dst: None,
                        src1: None,
                        src2: None,
                    });
                    ctx.loop_inc_labels.pop();
                }
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
                let then_info = self.resource_copy_info(&then_branch, ctx);
                let then_op = self.compile_expr(*then_branch, ctx)?;
                let then_op = match then_info {
                    Some(ty) => self.copy_resource(ctx, then_op, &ty)?,
                    None => then_op,
                };
                ctx.instructions.push(Instruction {
                    op: Op::Move,
                    dst: Some(res_tmp.clone()),
                    src1: Some(then_op),
                    src2: None,
                });
                self.emit_scope_frees(ctx)?;
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
                    let else_info = self.resource_copy_info(&else_expr, ctx);
                    let else_op = self.compile_expr(*else_expr, ctx)?;
                    let else_op = match else_info {
                        Some(ty) => self.copy_resource(ctx, else_op, &ty)?,
                        None => else_op,
                    };
                    ctx.instructions.push(Instruction {
                        op: Op::Move,
                        dst: Some(res_tmp.clone()),
                        src1: Some(else_op),
                        src2: None,
                    });
                    self.emit_scope_frees(ctx)?;
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
                let label_cont = ctx.new_label("while_cont");
                let label_end = ctx.new_label("while_end");

                ctx.loop_end_labels.push(label_end.clone());
                ctx.loop_inc_labels.push(label_cont.clone());

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
                ctx.loop_body_pending = true;
                self.compile_expr(*body, ctx)?;
                ctx.loop_body_pending = false;
                self.emit_scope_frees(ctx)?;
                ctx.exit_scope()?;

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

                Ok(ctx.new_tmp(IRType::Void))
            }

            Expr::For(var, iter, body, _) => {
                if let Expr::Range(start, end, _) = *iter {
                    let start_op = self.compile_expr(*start, ctx)?;
                    let end_op = self.compile_expr(*end, ctx)?;
                    let label_cond = ctx.new_label("rfor_cond");
                    let label_end = ctx.new_label("rfor_end");
                    let label_inc = ctx.new_label("rfor_inc");
                    ctx.loop_end_labels.push(label_end.clone());
                    ctx.loop_inc_labels.push(label_inc.clone());
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
                    ctx.loop_body_pending = true;
                    self.compile_expr(*body, ctx)?;
                    ctx.loop_body_pending = false;
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
                    ctx.loop_end_labels.pop();
                    ctx.loop_inc_labels.pop();
                    return Ok(ctx.new_tmp(IRType::Void));
                }
                if let Some(Type::Struct(sname, ta)) = self.expr_high_type(&iter, ctx) {
                    let maybe_ty =
                        self.struct_field_fn_ret(&sname, &ta, "next")
                            .ok_or_else(|| CodeGenError::TypeError {
                                message: format!("type '{}' has no 'next' method", sname),
                            })?;
                    let elem_ir = match &maybe_ty {
                        Type::Struct(mname, margs) if mname == "Maybe" => margs
                            .first()
                            .map(Context::type2ir_type)
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
                        Type::Struct(mname, margs) if mname == "Maybe" => margs.first().cloned(),
                        _ => None,
                    };
                    let s_op = self.compile_expr(*iter, ctx)?;
                    let fn_ptr = self.load_function_field(
                        s_op.clone(),
                        &Type::Struct(sname, ta),
                        "next",
                        ctx,
                    )?;

                    let label_cond = ctx.new_label("nfor_cond");
                    let label_end = ctx.new_label("nfor_end");

                    ctx.loop_end_labels.push(label_end.clone());
                    ctx.loop_inc_labels.push(label_cond.clone());

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

                    ctx.loop_body_pending = true;
                    self.compile_expr(*body, ctx)?;
                    ctx.loop_body_pending = false;

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
                    ctx.loop_end_labels.pop();
                    ctx.loop_inc_labels.pop();

                    return Ok(ctx.new_tmp(IRType::Void));
                }
                let is_string = matches!(
                    self.expr_high_type(&iter, ctx),
                    Some(Type::Primitive(Primitive::String))
                );
                let (elem_hty, _) = self.index_info(&iter, ctx);
                let elem_ir_type = elem_hty
                    .as_ref()
                    .map_or(IRType::Int, |t| Context::type2ir_type(t));
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

                ctx.loop_body_pending = true;
                self.compile_expr(*body, ctx)?;
                ctx.loop_body_pending = false;

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

            Expr::FuncDecl(_, _, _, _, _, _, _) => Err(CodeGenError::SyntaxError {
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
                    let mut evaluated: Vec<(Operand, IRType)> = Vec::new();
                    let mut n = 0;
                    for (arg, param) in zip(args.iter(), func.params.iter()) {
                        let operand = self.compile_expr(arg.clone(), ctx)?;
                        let operand_type = ctx.get_operand_type(&operand, &self.constants)?;
                        let type_matches = operand_type == param.1
                            || (operand_type == IRType::Array && param.1 == IRType::Int);
                        if !type_matches {
                            return Err(CodeGenError::TypeError {
                                message: format!(
                                    "unexpected type {:?}, expected {:?} (arg {} of '{}')",
                                    operand_type, param.1, n, name
                                ),
                            });
                        }
                        let operand = match self.expr_high_type(arg, ctx) {
                            Some(hty)
                                if self.is_resource_type(&hty) && !self.is_fresh_expr(arg) =>
                            {
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

            Expr::Index(arr, idx, _) => {
                if let Some(Type::Struct(sname, ta)) = self.expr_high_type(&arr, ctx) {
                    let ret_ty = self
                        .struct_field_fn_ret(&sname, &ta, "nth")
                        .ok_or_else(|| CodeGenError::TypeError {
                            message: format!("type '{}' has no 'nth' method", sname),
                        })?;
                    let ret_ir = Context::type2ir_type(&ret_ty);
                    let s_op = self.compile_expr(*arr, ctx)?;
                    let i_op = self.compile_expr(*idx, ctx)?;
                    let fn_ptr = self.load_function_field(
                        s_op.clone(),
                        &Type::Struct(sname, ta),
                        "nth",
                        ctx,
                    )?;
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
                let is_string_index =
                    byte && matches!(elem_type, Some(Type::Primitive(Primitive::String)));
                let elem_ir_type = if is_string_index {
                    IRType::String
                } else {
                    elem_type.map_or(IRType::Int, |t| Context::type2ir_type(&t))
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

            Expr::IndexAssign(arr_idx, value, _) => {
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
                    let fn_ptr = self.load_function_field(
                        s_op.clone(),
                        &Type::Struct(sname, ta),
                        "set_nth",
                        ctx,
                    )?;
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
                    return Ok(res_tmp);
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
                        elem_size
                    };
                    let total_size = n_val * elem_size + 8;
                    let total_size_idx = self.get_const_index(IRConst::Int(total_size));
                    let len_idx = self.get_const_index(IRConst::Int(n_val));
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
                        src2: Some(Operand::ConstIdx(len_idx)),
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
                    let eight_idx = self.get_const_index(IRConst::Int(8));
                    let total_tmp = ctx.new_tmp(IRType::Int);
                    ctx.instructions.push(Instruction {
                        op: Op::Add,
                        dst: Some(total_tmp.clone()),
                        src1: Some(byte_len_tmp),
                        src2: Some(Operand::ConstIdx(eight_idx)),
                    });
                    ctx.instructions.push(Instruction {
                        op: Op::Malloc,
                        dst: Some(ptr_tmp.clone()),
                        src1: Some(total_tmp),
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

            Expr::ExternVar(name, _, _) => {
                if let Some(ty) = self.extern_vars.get(&name).cloned() {
                    let ir_ty = Context::type2ir_type(&ty);
                    Ok(ctx.new_tmp(ir_ty))
                } else {
                    Err(CodeGenError::UndefinedVariable {
                        name: name.clone(),
                        span: crate::compiler::Span::new(0, 0),
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
                let field_ir_type = Context::type2ir_type(&field_type);
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
                ctx.instructions.push(Instruction {
                    op: Op::StoreAt,
                    dst: Some(addr),
                    src1: Some(Operand::ConstIdx(zero_idx)),
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
                        src1: Some(Operand::Var(ctx.slot(&name))),
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

            Expr::TypeDef(_)
            | Expr::Struct(_, _, _, _)
            | Expr::Union(_, _, _, _)
            | Expr::Enum(_, _, _)
            | Expr::Lambda(_, _, _, _) => Ok(ctx.new_tmp(IRType::Void)),

            Expr::Match(target, branches, default, _) => {
                let target = self.compile_expr(*target, ctx)?;
                let end_label = ctx.new_label("end_match");
                let mut case_labels: Vec<String> = Vec::new();
                let mut case_cnt = 0usize;
                let res_tmp = ctx.new_tmp(IRType::Void);

                for _ in branches.clone() {
                    case_labels.push(format!("case{}", case_cnt));
                    case_cnt += 1;
                }

                case_cnt = 0;
                for (case, _) in branches.clone() {
                    let cond = ctx.new_tmp(IRType::Bool);
                    let case = self.compile_expr(case, ctx)?;
                    ctx.instructions.push(Instruction {
                        op: Op::Eq,
                        dst: Some(cond.clone()),
                        src1: Some(target.clone()),
                        src2: Some(case),
                    });
                    ctx.instructions.push(Instruction {
                        op: Op::JumpIfTrue,
                        dst: None,
                        src1: Some(cond),
                        src2: Some(Operand::Label(
                            case_labels.iter().nth(case_cnt).unwrap().clone(),
                        )),
                    });
                    case_cnt += 1;
                }

                if let Some(d) = default {
                    ctx.enter_scope();
                    let d_info = self.resource_copy_info(&d, ctx);
                    let ret = self.compile_expr(*d, ctx)?;
                    let ret = match d_info {
                        Some(ty) => self.copy_resource(ctx, ret, &ty)?,
                        None => ret,
                    };
                    ctx.instructions.push(Instruction {
                        op: Op::Move,
                        dst: Some(res_tmp.clone()),
                        src1: Some(ret),
                        src2: None,
                    });
                    self.emit_scope_frees(ctx)?;
                    ctx.exit_scope()?;
                }

                ctx.instructions.push(Instruction {
                    op: Op::Jump,
                    dst: None,
                    src1: Some(Operand::Label(end_label.clone())),
                    src2: None,
                });

                case_cnt = 0;
                for (_, ret) in branches.clone() {
                    ctx.instructions.push(Instruction {
                        op: Op::Label(case_labels.iter().nth(case_cnt).unwrap().clone()),
                        dst: None,
                        src1: None,
                        src2: None,
                    });
                    ctx.enter_scope();
                    let ret_info = self.resource_copy_info(&ret, ctx);
                    let ret = self.compile_expr(ret, ctx)?;
                    let ret = match ret_info {
                        Some(ty) => self.copy_resource(ctx, ret, &ty)?,
                        None => ret,
                    };
                    ctx.instructions.push(Instruction {
                        op: Op::Move,
                        dst: Some(res_tmp.clone()),
                        src1: Some(ret),
                        src2: None,
                    });
                    self.emit_scope_frees(ctx)?;
                    ctx.exit_scope()?;
                    ctx.instructions.push(Instruction {
                        op: Op::Jump,
                        dst: None,
                        src1: Some(Operand::Label(end_label.clone())),
                        src2: None,
                    });
                    case_cnt += 1;
                }
                ctx.instructions.push(Instruction {
                    op: Op::Label(end_label),
                    dst: None,
                    src1: None,
                    src2: None,
                });
                Ok(res_tmp)
            }
            Expr::FString(_, _) => {
                unreachable!("f-string should have been desugared in checker")
            }
        }
    }

    fn member_call_ret_type(&self, callee: &Expr, ctx: &Context) -> IRType {
        if let Expr::MemberAccess(obj, field_name, _) = callee {
            if let Some(obj_ty) = self.expr_high_type(obj, ctx) {
                if let Some(ftype) = self.member_field_type(&obj_ty, field_name) {
                    if let Type::Function(_, ret) = ftype {
                        return Context::type2ir_type(&ret);
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
            Expr::Int(_, _) => Some(Type::Primitive(Primitive::Int)),
            Expr::Float(_, _) => Some(Type::Primitive(Primitive::Float)),
            Expr::Bool(_, _) => Some(Type::Primitive(Primitive::Boolean)),
            Expr::String(_, _) => Some(Type::Primitive(Primitive::String)),
            Expr::Nil(_) => Some(Type::Primitive(Primitive::Void)),
            Expr::Var(name, _) => ctx
                .get_var_high_type(name.as_str())
                .cloned()
                .or_else(|| self.extern_vars.get(name).cloned()),
            Expr::AddressOf(inner, _) => match inner.as_ref() {
                Expr::Var(n, _) => ctx
                    .get_var_high_type(n.as_str())
                    .cloned()
                    .map(|t| Type::Pointer(Box::new(t))),
                _ => self
                    .expr_high_type(inner, ctx)
                    .map(|t| Type::Pointer(Box::new(t))),
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
            Expr::Index(arr, _, _) => match self.expr_high_type(arr, ctx) {
                Some(Type::Array(elem)) => Some(*elem),
                Some(Type::Pointer(elem)) => Some(*elem),
                Some(Type::Struct(sname, ta)) => self.struct_field_fn_ret(&sname, &ta, "nth"),
                _ => self.index_info(arr, ctx).0,
            },
            Expr::MemberAccess(obj, field_name, _) => {
                let obj_ty = self.expr_high_type(obj, ctx)?;
                self.member_field_type(&obj_ty, field_name)
            }
            Expr::Call(callee, type_args, _, _) => self.call_ret_high_type(callee, type_args, ctx),
            Expr::StructLiteral(name, type_args, _, _) => {
                Some(Type::Struct(name.clone(), type_args.clone()))
            }
            Expr::UnionLiteral(name, type_args, _, _) => {
                Some(Type::Union(name.clone(), type_args.clone()))
            }
            Expr::ArrayLiteral(items, _) => items
                .first()
                .and_then(|i| self.expr_high_type(i, ctx))
                .map(|t| Type::Array(Box::new(t))),
            Expr::ArrayFill(ty, _, _) => Some(Type::Array(Box::new(ty.clone()))),
            Expr::StrCat(_, _, _) => Some(Type::Primitive(Primitive::String)),
            Expr::Add(_, _, _)
            | Expr::Sub(_, _, _)
            | Expr::Mul(_, _, _)
            | Expr::Div(_, _, _)
            | Expr::Mod(_, _, _) => {
                let (l, r) = match e {
                    Expr::Add(l, r, _)
                    | Expr::Sub(l, r, _)
                    | Expr::Mul(l, r, _)
                    | Expr::Div(l, r, _)
                    | Expr::Mod(l, r, _) => (l, r),
                    _ => unreachable!(),
                };
                let l_float = matches!(
                    self.expr_high_type(l, ctx),
                    Some(Type::Primitive(Primitive::Float))
                );
                let r_float = matches!(
                    self.expr_high_type(r, ctx),
                    Some(Type::Primitive(Primitive::Float))
                );
                if l_float || r_float {
                    Some(Type::Primitive(Primitive::Float))
                } else {
                    Some(Type::Primitive(Primitive::Int))
                }
            }
            Expr::FAdd(_, _, _)
            | Expr::FSub(_, _, _)
            | Expr::FMul(_, _, _)
            | Expr::FDiv(_, _, _) => Some(Type::Primitive(Primitive::Float)),
            Expr::Neg(inner, _) => self.expr_high_type(inner, ctx),
            Expr::FNeg(_, _) => Some(Type::Primitive(Primitive::Float)),
            Expr::Not(_, _)
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
            | Expr::FGe(_, _, _) => Some(Type::Primitive(Primitive::Boolean)),
            Expr::Xor(_, _, _)
            | Expr::LAnd(_, _, _)
            | Expr::LOr(_, _, _)
            | Expr::Inc(_, _)
            | Expr::Dec(_, _) => Some(Type::Primitive(Primitive::Int)),
            Expr::VarDecl(_, _, value, _) | Expr::VarAssign(_, value, _) => {
                self.expr_high_type(value, ctx)
            }
            Expr::If(_, then_branch, else_branch, _) => {
                self.expr_high_type(then_branch, ctx).or_else(|| {
                    else_branch
                        .as_ref()
                        .and_then(|e| self.expr_high_type(e, ctx))
                })
            }
            Expr::Match(_, branches, default, _) => branches
                .iter()
                .find_map(|(_, ret)| self.expr_high_type(ret, ctx))
                .or_else(|| default.as_ref().and_then(|e| self.expr_high_type(e, ctx))),
            Expr::Lambda(params, _, ret_type, _) => {
                let param_types = params.iter().map(|(_, t)| t.clone()).collect();
                Some(Type::Function(param_types, Box::new(ret_type.clone())))
            }
            Expr::Return(value, _) => self.expr_high_type(value, ctx),
            Expr::ExternVar(_, ty, _) => Some(ty.clone()),
            Expr::GlobalVar(_, _, ty, value, _) => value
                .as_ref()
                .and_then(|v| self.expr_high_type(v, ctx))
                .or_else(|| {
                    if matches!(ty, Type::Unknown) {
                        None
                    } else {
                        Some(ty.clone())
                    }
                }),
            Expr::IndexAssign(arr, value, _) => self
                .expr_high_type(value, ctx)
                .or_else(|| self.index_info(arr, ctx).0),
            Expr::MemberAssign(obj, field_name, value, _) => {
                self.expr_high_type(value, ctx).or_else(|| {
                    let obj_ty = self.expr_high_type(obj, ctx)?;
                    self.member_field_type(&obj_ty, field_name)
                })
            }
            _ => None,
        }
    }

    fn member_field_type(&self, obj_type: &Type, field: &str) -> Option<Type> {
        let (sname, type_args) = match obj_type {
            Type::Struct(sname, args) => (sname, args),
            Type::Union(sname, args) => (sname, args),
            Type::Pointer(inner) => match inner.as_ref() {
                Type::Struct(sname, args) => (sname, args),
                Type::Union(sname, args) => (sname, args),
                _ => return None,
            },
            _ => return None,
        };
        let fields = match self.structs.get(sname).or_else(|| self.unions.get(sname)) {
            Some((_, fields)) => fields,
            None => return None,
        };
        fields
            .iter()
            .find(|(fname, _)| fname == field)
            .map(|(_, ftype)| ftype.substitute(type_args))
    }

    fn call_ret_high_type(&self, callee: &Expr, type_args: &[Type], ctx: &Context) -> Option<Type> {
        match callee {
            Expr::Var(fname, _) => {
                if let Some((_, _, ret, _)) = self.generic_funcs.get(fname) {
                    Some(ret.substitute(type_args))
                } else {
                    self.func_high_returns.get(fname).cloned()
                }
            }
            Expr::MemberAccess(obj, field_name, _) => {
                let obj_ty = self.expr_high_type(obj, ctx)?;
                self.member_field_type(&obj_ty, field_name)
            }
            _ => None,
        }
    }

    fn index_info(&self, arr: &Expr, ctx: &Context) -> (Option<Type>, bool) {
        if let Some(ty) = self.expr_high_type(arr, ctx) {
            match ty {
                Type::Array(elem) => return (Some(*elem), false),
                Type::Primitive(Primitive::String) => {
                    return (Some(Type::Primitive(Primitive::String)), true);
                }
                Type::Pointer(inner) => {
                    let pointee = *inner;
                    return (Some(pointee.clone()), Self::ptr_scale(&pointee) == 1);
                }
                _ => {}
            }
        }

        let (sname, type_args, field_name) = match arr {
            Expr::Var(name, _) => match ctx.get_var_high_type(name) {
                Some(Type::Array(elem)) => return (Some(elem.as_ref().clone()), false),
                Some(Type::Primitive(Primitive::String)) => {
                    return (Some(Type::Primitive(Primitive::String)), true);
                }
                Some(Type::Pointer(inner)) => {
                    return (Some(*inner.clone()), Self::ptr_scale(inner) == 1);
                }
                Some(Type::Struct(sname, ta)) => (sname.clone(), ta.clone(), None),
                Some(Type::Union(sname, ta)) => (sname.clone(), ta.clone(), None),
                #[allow(warnings)]
                Some(Type::Pointer(box_ty)) => match box_ty.as_ref() {
                    Type::Struct(sname, ta) => (sname.clone(), ta.clone(), None),
                    Type::Union(sname, ta) => (sname.clone(), ta.clone(), None),
                    _ => return (None, false),
                },
                _ => return (None, false),
            },
            Expr::MemberAccess(obj, field_name, _) => match &**obj {
                Expr::Var(name, _) => match ctx.get_var_high_type(name) {
                    Some(Type::Struct(sname, type_args)) => {
                        (sname.clone(), type_args.clone(), Some(field_name.clone()))
                    }
                    Some(Type::Union(sname, type_args)) => {
                        (sname.clone(), type_args.clone(), Some(field_name.clone()))
                    }
                    Some(Type::Pointer(box_ty)) => match box_ty.as_ref() {
                        Type::Struct(sname, type_args) => {
                            (sname.clone(), type_args.clone(), Some(field_name.clone()))
                        }
                        Type::Union(sname, type_args) => {
                            (sname.clone(), type_args.clone(), Some(field_name.clone()))
                        }
                        _ => return (None, false),
                    },
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
                        Type::Array(_elem) => false,
                        Type::Pointer(inner) => Self::ptr_scale(&inner) == 1,
                        _ => false,
                    };
                    let elem = match ftype {
                        Type::Pointer(inner) => *inner.clone(),
                        Type::Array(elem) => {
                            let concrete = elem.substitute(&type_args);
                            concrete
                        }
                        Type::Primitive(Primitive::String) => Type::Primitive(Primitive::String),
                        _ => Type::Primitive(Primitive::Int),
                    };
                    return (Some(elem), byte);
                }
            }
        }
        if let Some((_, fields)) = self.unions.get(&sname) {
            for (fname, ftype) in fields {
                if Some(fname.as_str()) == field_name.as_deref() {
                    let byte = match ftype {
                        Type::Primitive(Primitive::String) => true,
                        Type::Array(_elem) => false,
                        Type::Pointer(inner) => Self::ptr_scale(&inner) == 1,
                        _ => false,
                    };
                    let elem = match ftype {
                        Type::Pointer(inner) => *inner.clone(),
                        Type::Array(elem) => {
                            let concrete = elem.substitute(&type_args);
                            concrete
                        }
                        Type::Primitive(Primitive::String) => Type::Primitive(Primitive::String),
                        _ => Type::Primitive(Primitive::Int),
                    };
                    return (Some(elem), byte);
                }
            }
        }
        (None, false)
    }
}
