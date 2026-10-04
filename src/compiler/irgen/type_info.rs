use super::context::Context;
use crate::compiler::{
    irgen::{IRGen, ir::IRType},
    parser::{Expr, Primitive, Type},
};

impl IRGen {
    pub(super) fn member_call_ret_type(&self, callee: &Expr, ctx: &Context) -> IRType {
        if let Expr::MemberAccess {
            obj,
            field: field_name,
            ..
        } = callee
        {
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
            Expr::Int { .. } => Some(Type::Primitive(Primitive::Int)),
            Expr::Float { .. } => Some(Type::Primitive(Primitive::Float)),
            Expr::Bool { .. } => Some(Type::Primitive(Primitive::Boolean)),
            Expr::String { .. } => Some(Type::Primitive(Primitive::String)),
            Expr::Nil(_) => Some(Type::Primitive(Primitive::Void)),
            Expr::Var { name, .. } => ctx
                .get_var_high_type(name.as_str())
                .cloned()
                .or_else(|| self.extern_vars.get(name).cloned()),
            Expr::AddressOf { expr: inner, .. } => match inner.as_ref() {
                Expr::Var { name: n, .. } => ctx
                    .get_var_high_type(n.as_str())
                    .cloned()
                    .map(|t| Type::Pointer(Box::new(t))),
                _ => self
                    .expr_high_type(inner, ctx)
                    .map(|t| Type::Pointer(Box::new(t))),
            },
            Expr::Deref { expr: inner, .. } => match inner.as_ref() {
                Expr::Var { name: n, .. } => match ctx.get_var_high_type(n.as_str()) {
                    Some(Type::Pointer(t)) => Some(*t.clone()),
                    _ => None,
                },
                _ => match &self.expr_high_type(inner, ctx) {
                    Some(Type::Pointer(t)) => Some(*t.clone()),
                    _ => None,
                },
            },
            Expr::Index { array: arr, .. } => match self.expr_high_type(arr, ctx) {
                Some(Type::Array(elem)) => Some(*elem),
                Some(Type::Pointer(elem)) => Some(*elem),
                Some(Type::Struct(sname, ta)) => self.struct_field_fn_ret(&sname, &ta, "nth"),
                _ => self.index_info(arr, ctx).0,
            },
            Expr::MemberAccess {
                obj,
                field: field_name,
                ..
            } => {
                let obj_ty = self.expr_high_type(obj, ctx)?;

                match self.member_field_type(&obj_ty, field_name)? {
                    Type::Function(_, ret) => Some(*ret),
                    other => Some(other),
                }
            }

            Expr::Call {
                callee,
                type_args: _,
                args,
                ..
            } if matches!(callee.as_ref(), Expr::Var { name: n, .. } if n == "_alum_copy")
                && args.len() == 1 =>
            {
                self.expr_high_type(&args[0], ctx)
            }
            Expr::Call {
                callee, type_args, ..
            } => self.call_ret_high_type(callee, type_args, ctx),
            Expr::StructLiteral {
                name, type_args, ..
            } => Some(Type::Struct(name.clone(), type_args.clone())),
            Expr::UnionLiteral {
                name, type_args, ..
            } => Some(Type::Union(name.clone(), type_args.clone())),
            Expr::ArrayLiteral {
                elements: items, ..
            } => items
                .first()
                .and_then(|i| self.expr_high_type(i, ctx))
                .map(|t| Type::Array(Box::new(t))),
            Expr::ArrayFill { elem_type: ty, .. } => Some(Type::Array(Box::new(ty.clone()))),
            Expr::StrCat { .. } => Some(Type::Primitive(Primitive::String)),
            Expr::Add { .. }
            | Expr::Sub { .. }
            | Expr::Mul { .. }
            | Expr::Div { .. }
            | Expr::Mod { .. } => {
                let (l, r) = match e {
                    Expr::Add {
                        left: l, right: r, ..
                    }
                    | Expr::Sub {
                        left: l, right: r, ..
                    }
                    | Expr::Mul {
                        left: l, right: r, ..
                    }
                    | Expr::Div {
                        left: l, right: r, ..
                    }
                    | Expr::Mod {
                        left: l, right: r, ..
                    } => (l, r),
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
            Expr::FAdd { .. } | Expr::FSub { .. } | Expr::FMul { .. } | Expr::FDiv { .. } => {
                Some(Type::Primitive(Primitive::Float))
            }
            Expr::Neg { expr: inner, .. } => self.expr_high_type(inner, ctx),
            Expr::FNeg { .. } => Some(Type::Primitive(Primitive::Float)),
            Expr::Not { .. }
            | Expr::Eq { .. }
            | Expr::Ne { .. }
            | Expr::Lt { .. }
            | Expr::Le { .. }
            | Expr::Gt { .. }
            | Expr::Ge { .. }
            | Expr::FEq { .. }
            | Expr::FNe { .. }
            | Expr::FLt { .. }
            | Expr::FLe { .. }
            | Expr::FGt { .. }
            | Expr::FGe { .. } => Some(Type::Primitive(Primitive::Boolean)),
            Expr::Xor { .. }
            | Expr::BAnd { .. }
            | Expr::BOr { .. }
            | Expr::LAnd { .. }
            | Expr::LOr { .. }
            | Expr::Inc { .. }
            | Expr::Dec { .. } => Some(Type::Primitive(Primitive::Int)),
            Expr::VarDecl {
                name: _,
                ty: _,
                value,
                ..
            }
            | Expr::VarAssign { name: _, value, .. } => self.expr_high_type(value, ctx),
            Expr::If {
                cond: _,
                then_branch,
                else_branch,
                ..
            } => self.expr_high_type(then_branch, ctx).or_else(|| {
                else_branch
                    .as_ref()
                    .and_then(|e| self.expr_high_type(e, ctx))
            }),
            Expr::Match {
                target: _,
                branches,
                default,
                ..
            } => branches
                .iter()
                .find_map(|(_, _, ret)| self.expr_high_type(ret, ctx))
                .or_else(|| default.as_ref().and_then(|e| self.expr_high_type(e, ctx))),
            Expr::Lambda {
                params,
                body: _,
                return_type: ret_type,
                ..
            } => {
                let param_types = params.iter().map(|(_, t)| t.clone()).collect();
                Some(Type::Function(param_types, Box::new(ret_type.clone())))
            }
            Expr::Return { value, .. } => self.expr_high_type(value, ctx),
            Expr::ExternVar { name: _, ty, .. } => Some(ty.clone()),
            Expr::Cast { expr: _, ty, .. } => Some(ty.clone()),
            Expr::GlobalVar {
                name: _,
                is_pub: _,
                ty,
                value,
                ..
            } => value
                .as_ref()
                .and_then(|v| self.expr_high_type(v, ctx))
                .or_else(|| {
                    if matches!(ty, Type::Unknown) {
                        None
                    } else {
                        Some(ty.clone())
                    }
                }),
            Expr::IndexAssign {
                target: arr, value, ..
            } => self
                .expr_high_type(value, ctx)
                .or_else(|| self.index_info(arr, ctx).0),
            Expr::MemberAssign {
                obj,
                field: field_name,
                value,
                ..
            } => self.expr_high_type(value, ctx).or_else(|| {
                let obj_ty = self.expr_high_type(obj, ctx)?;
                self.member_field_type(&obj_ty, field_name)
            }),
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
            Expr::Var { name: fname, .. } => {
                if let Some((_, _, ret, _)) = self.generic_funcs.get(fname) {
                    Some(ret.substitute(type_args))
                } else {
                    self.func_high_returns.get(fname).cloned()
                }
            }
            Expr::MemberAccess {
                obj,
                field: field_name,
                ..
            } => {
                let obj_ty = self.expr_high_type(obj, ctx)?;

                match self.member_field_type(&obj_ty, field_name)? {
                    Type::Function(_, ret) => Some(*ret),
                    other => Some(other),
                }
            }
            _ => None,
        }
    }

    pub(super) fn index_info(&self, arr: &Expr, ctx: &Context) -> (Option<Type>, bool) {
        if let Some(ty) = self.expr_high_type(arr, ctx) {
            match ty {
                Type::Array(elem) => return (Some(*elem), false),

                Type::Primitive(Primitive::String) => {
                    return (Some(Type::Primitive(Primitive::Char)), true);
                }
                Type::Pointer(inner) => {
                    let pointee = *inner;
                    return (Some(pointee.clone()), Self::ptr_scale(&pointee) == 1);
                }
                _ => {}
            }
        }

        let (sname, type_args, field_name) = match arr {
            Expr::Var { name, .. } => match ctx.get_var_high_type(name) {
                Some(Type::Array(elem)) => return (Some(elem.as_ref().clone()), false),
                Some(Type::Primitive(Primitive::String)) => {
                    return (Some(Type::Primitive(Primitive::Char)), true);
                }
                Some(Type::Pointer(inner)) => {
                    return (Some(*inner.clone()), Self::ptr_scale(inner) == 1);
                }
                Some(Type::Struct(sname, ta)) => (sname.clone(), ta.clone(), None),
                Some(Type::Union(sname, ta)) => (sname.clone(), ta.clone(), None),
                _ => return (None, false),
            },
            Expr::MemberAccess {
                obj,
                field: field_name,
                ..
            } => match &**obj {
                Expr::Var { name, .. } => match ctx.get_var_high_type(name) {
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

        if let Some((_, fields)) = self.structs.get(&sname).or_else(|| self.unions.get(&sname)) {
            for (fname, ftype) in fields {
                if Some(fname.as_str()) == field_name.as_deref() {
                    let byte = match ftype {
                        Type::Primitive(Primitive::String) => true,
                        Type::Array(_elem) => false,
                        Type::Pointer(inner) => Self::ptr_scale(inner) == 1,
                        _ => false,
                    };
                    let elem = match ftype {
                        Type::Pointer(inner) => *inner.clone(),
                        Type::Array(elem) => elem.substitute(&type_args),
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
