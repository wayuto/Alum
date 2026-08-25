use super::context::Context;
use super::ir::{IRConst, IRType, Instruction, Op, Operand};
use crate::compiler::{
    codegen::CodeGenError,
    irgen::IRGen,
    parser::{Expr, Primitive, Type},
};
use ordered_float::OrderedFloat;

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

    pub(super) fn member_addr(
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

    pub(super) fn member_offset_and_type(
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

    pub(super) fn extern_op(&self, name: &str) -> Option<Operand> {
        if self.extern_vars.contains_key(name) || self.global_storage.contains_key(name) {
            Some(Operand::Global(name.to_string()))
        } else {
            None
        }
    }

    pub(super) fn glob_ir_type(&self, name: &str) -> Option<IRType> {
        if let Some(t) = self.extern_vars.get(name) {
            Some(Context::type_to_ir_type(t))
        } else {
            self.global_storage.get(name).map(|(t, _, _)| t.clone())
        }
    }

    pub(super) fn extern_load_op(&self, name: &str) -> Option<Op> {
        self.glob_ir_type(name).map(|t| match t {
            IRType::Float => Op::FGlobLoad,
            _ => Op::GlobLoad,
        })
    }

    pub(super) fn extern_store_op(&self, name: &str) -> Option<Op> {
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
}
