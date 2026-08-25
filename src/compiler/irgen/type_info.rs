use super::context::Context;
use crate::compiler::{
    irgen::{IRGen, ir::IRType},
    parser::{Expr, Primitive, Type},
};

impl IRGen {
    pub(super) fn member_call_ret_type(&self, callee: &Expr, ctx: &Context) -> IRType {
        if let Expr::MemberAccess(obj, field_name, _) = callee {
            if let Some(obj_ty) = self.expr_high_type(obj, ctx) {
                if let Some(ftype) = self.member_field_type(&obj_ty, field_name) {
                    if let Type::Function(_, ret) = ftype {
                        return Context::type_to_ir_type(&ret);
                    }
                }
            }
        }
        IRType::Int
    }

    pub(super) fn ptr_scale(ty: &Type) -> usize {
        match ty {
            Type::Primitive(Primitive::Void) => 1,
            _ => 8,
        }
    }

    pub(super) fn expr_high_type(&self, e: &Expr, ctx: &Context) -> Option<Type> {
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

            Expr::Call(callee, _, args, _)
                if matches!(callee.as_ref(), Expr::Var(n, _) if n == "_alum_copy")
                    && args.len() == 1 =>
            {
                self.expr_high_type(&args[0], ctx)
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
            Expr::Cast(_, ty, _) => Some(ty.clone()),
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

    pub(super) fn member_field_type(&self, obj_type: &Type, field: &str) -> Option<Type> {
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

    pub(super) fn call_ret_high_type(
        &self,
        callee: &Expr,
        type_args: &[Type],
        ctx: &Context,
    ) -> Option<Type> {
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

    pub(super) fn index_info(&self, arr: &Expr, ctx: &Context) -> (Option<Type>, bool) {
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
