use super::error::CheckerError;
use crate::compiler::{
    Span,
    parser::{Expr, Primitive, Type},
    visitor::TypeChecker,
};
use std::collections::HashMap;

impl TypeChecker {
    pub(super) fn check_expr(&mut self, expr: &mut Expr) -> Result<Type, CheckerError> {
        let span = expr.span();
        match expr {
            Expr::Int(_, _) => Ok(Type::Primitive(Primitive::Int)),
            Expr::Float(_, _) => Ok(Type::Primitive(Primitive::Float)),
            Expr::Bool(_, _) => Ok(Type::Primitive(Primitive::Boolean)),
            Expr::String(_, _) => Ok(Type::Primitive(Primitive::String)),
            Expr::Nil(_) => Ok(Type::Primitive(Primitive::Void)),
            Expr::Var(name, _) => {
                if let Some(ty) = self.lookup_var(name) {
                    return Ok(self.resolve_type(&ty));
                }

                if let Some((_, params, ret_type)) = self.functions.get(name) {
                    let params = params.clone();
                    let ret_type = ret_type.clone();
                    let (resolved_params, resolved_ret) =
                        self.fresh_instantiate_signature(&params, &ret_type);
                    return Ok(Type::Function(resolved_params, Box::new(resolved_ret)));
                }

                if let Some(ty) = self.constants.get(name) {
                    return Ok(self.resolve_type(ty));
                }

                if let Some(ty) = self.extern_vars.get(name) {
                    return Ok(self.resolve_type(ty));
                }

                if let Some(ty) = self.globals.get(name) {
                    return Ok(self.resolve_type(ty));
                }

                match self.resolve_enum_member(name) {
                    Ok(Some(_)) => return Ok(Type::Primitive(Primitive::Int)),
                    Err(enums) => {
                        return Err(CheckerError::AmbiguousEnumMember {
                            member: name.clone(),
                            enums,
                            span,
                        });
                    }
                    Ok(None) => {}
                }

                Err(CheckerError::UndefinedVariable(name.clone(), span))
            }
            Expr::VarDecl(name, ty, value, _) => {
                let resolved_ty = self.resolve_type(ty);
                let value_type = self.check_expr(value)?;

                let actual_ty = match &resolved_ty {
                    Type::Unknown => self.new_type_var(),
                    _ => resolved_ty.clone(),
                };

                if !matches!(value.as_ref(), Expr::Nil(_)) {
                    self.unify_types(&actual_ty, &value_type).map_err(|_| {
                        CheckerError::TypeMismatch {
                            expected: self.resolve_type(&actual_ty),
                            found: self.resolve_type(&value_type),
                            context: format!("variable declaration '{}'", name),
                            span: span,
                        }
                    })?;
                }

                self.declare_var(name, actual_ty.clone());
                Ok(actual_ty)
            }
            Expr::ConstDecl(name, ty, value, _, _) => {
                let resolved_ty = self.resolve_type(ty);
                let value_type = self.check_expr(value)?;

                let actual_ty = match &resolved_ty {
                    Type::Unknown => value_type.clone(),
                    _ => resolved_ty.clone(),
                };

                if matches!(value.as_ref(), Expr::Nil(_)) {
                    return Err(CheckerError::TypeMismatch {
                        expected: resolved_ty.clone(),
                        found: Type::Primitive(Primitive::Void),
                        context: format!("constant declaration '{}'", name),
                        span: span,
                    });
                }

                self.unify_types(&actual_ty, &value_type).map_err(|_| {
                    CheckerError::TypeMismatch {
                        expected: self.resolve_type(&actual_ty),
                        found: self.resolve_type(&value_type),
                        context: format!("constant declaration '{}'", name),
                        span: span,
                    }
                })?;

                if self.is_global_scope() {
                    self.constants.insert(name.clone(), actual_ty.clone());
                } else {
                    self.declare_const(name);
                    self.declare_var(name, actual_ty.clone());
                }
                Ok(actual_ty)
            }
            Expr::GlobalVar(name, _, ty, value, _) => {
                if !self.is_global_scope() {
                    return Err(CheckerError::InvalidOperation {
                        op: "declaration".to_string(),
                        type_name: format!(
                            "global variable '{}' (only allowed at top level)",
                            name
                        ),
                        span: span,
                    });
                }
                let resolved_ty = self.resolve_type(ty);
                if let Some(value) = value {
                    let value_type = self.check_expr(value)?;
                    let actual_ty = match &resolved_ty {
                        Type::Unknown => value_type.clone(),
                        _ => resolved_ty.clone(),
                    };
                    self.unify_types(&actual_ty, &value_type).map_err(|_| {
                        CheckerError::TypeMismatch {
                            expected: self.resolve_type(&actual_ty),
                            found: self.resolve_type(&value_type),
                            context: format!("global variable declaration '{}'", name),
                            span: span,
                        }
                    })?;
                    self.globals.insert(name.clone(), actual_ty.clone());
                    Ok(actual_ty)
                } else {
                    if matches!(resolved_ty, Type::Unknown) {
                        return Err(CheckerError::TypeMismatch {
                            expected: resolved_ty.clone(),
                            found: Type::Primitive(Primitive::Void),
                            context: format!(
                                "global variable '{}' needs an explicit type or initializer",
                                name
                            ),
                            span: span,
                        });
                    }
                    self.globals.insert(name.clone(), resolved_ty.clone());
                    Ok(resolved_ty)
                }
            }
            Expr::ExternVar(name, ty, _) => {
                let resolved_ty = self.resolve_type(ty);
                if self.extern_vars.contains_key(name.as_str()) {
                    self.unify_types(&self.extern_vars.get(name).unwrap().clone(), &resolved_ty)
                        .map_err(|_| CheckerError::TypeMismatch {
                            expected: self.extern_vars.get(name).unwrap().clone(),
                            found: resolved_ty.clone(),
                            context: format!("extern variable '{}'", name),
                            span: span,
                        })?;
                } else {
                    self.extern_vars.insert(name.clone(), resolved_ty.clone());
                }
                Ok(resolved_ty)
            }
            Expr::VarAssign(name, value, _) => {
                if self.is_constant(name) {
                    return Err(CheckerError::InvalidOperation {
                        op: "assignment".to_string(),
                        type_name: format!("constant '{}'", name),
                        span: span,
                    });
                }
                let var_type = self
                    .lookup_var(name)
                    .or_else(|| self.extern_vars.get(name).cloned())
                    .or_else(|| self.globals.get(name).cloned())
                    .ok_or_else(|| CheckerError::UndefinedVariable(name.clone(), span))?;
                let value_type = self.check_expr(value)?;

                if !matches!(value.as_ref(), Expr::Nil(_)) {
                    self.unify_types(&var_type, &value_type).map_err(|_| {
                        CheckerError::TypeMismatch {
                            expected: self.resolve_type(&var_type),
                            found: self.resolve_type(&value_type),
                            context: format!("assignment to '{}'", name),
                            span: span,
                        }
                    })?;
                }

                Ok(var_type)
            }
            Expr::Add(lhs, rhs, _)
            | Expr::Sub(lhs, rhs, _)
            | Expr::Mul(lhs, rhs, _)
            | Expr::Div(lhs, rhs, _) => {
                let lhs_type = self.check_expr(lhs)?;
                let rhs_type = self.check_expr(rhs)?;

                let has_type_var =
                    matches!(&lhs_type, Type::TypeVar(_)) || matches!(&rhs_type, Type::TypeVar(_));

                if lhs_type.is_string() || rhs_type.is_string() {
                    if has_type_var {
                        self.unify_types(&lhs_type, &rhs_type)?;

                        let string_type = Type::Primitive(Primitive::String);
                        if matches!(&lhs_type, Type::TypeVar(_)) {
                            if let Type::TypeVar(id) = &lhs_type {
                                self.bind_type_var(*id, &string_type);
                            }
                        }
                        if matches!(&rhs_type, Type::TypeVar(_)) {
                            if let Type::TypeVar(id) = &rhs_type {
                                self.bind_type_var(*id, &string_type);
                            }
                        }
                    } else if !lhs_type.is_string() || !rhs_type.is_string() {
                        return Err(CheckerError::TypeMismatch {
                            expected: Type::Primitive(Primitive::String),
                            found: if !lhs_type.is_string() {
                                lhs_type.clone()
                            } else {
                                rhs_type.clone()
                            },
                            context: "string concatenation".to_string(),
                            span: span,
                        });
                    }

                    let (l, r) = match expr {
                        Expr::Add(l, r, _) => (l.clone(), r.clone()),
                        Expr::Sub(_, _, _) => unreachable!(),
                        Expr::Mul(_, _, _) => unreachable!(),
                        Expr::Div(_, _, _) => unreachable!(),
                        _ => unreachable!(),
                    };
                    *expr = Expr::StrCat(l, r, Span::new(0, 0));
                    return Ok(Type::Primitive(Primitive::String));
                }

                let is_int_like =
                    |t: &Type| matches!(t, Type::Primitive(Primitive::Int) | Type::TypeVar(_));
                if matches!(expr, Expr::Add(..) | Expr::Sub(..)) {
                    let ptr_type = if lhs_type.is_pointer() && is_int_like(&rhs_type) {
                        Some(lhs_type.clone())
                    } else if matches!(expr, Expr::Add(..))
                        && rhs_type.is_pointer()
                        && is_int_like(&lhs_type)
                    {
                        Some(rhs_type.clone())
                    } else {
                        None
                    };
                    if let Some(pt) = ptr_type {
                        return Ok(pt);
                    }
                }

                if lhs_type.is_float() || rhs_type.is_float() {
                    if !lhs_type.is_numeric() || !rhs_type.is_numeric() {
                        return Err(CheckerError::InvalidOperation {
                            op: "arithmetic".to_string(),
                            type_name: format!("{:?} and {:?}", lhs_type, rhs_type),
                            span: span,
                        });
                    }

                    if has_type_var {
                        self.unify_types(&lhs_type, &rhs_type)?;
                        let float_type = Type::Primitive(Primitive::Float);
                        if matches!(&lhs_type, Type::TypeVar(_)) {
                            if let Type::TypeVar(id) = &lhs_type {
                                self.bind_type_var(*id, &float_type);
                            }
                        }
                        if matches!(&rhs_type, Type::TypeVar(_)) {
                            if let Type::TypeVar(id) = &rhs_type {
                                self.bind_type_var(*id, &float_type);
                            }
                        }
                    }

                    let (l, r) = match expr {
                        Expr::Add(l, r, _) => (l.clone(), r.clone()),
                        Expr::Sub(l, r, _) => (l.clone(), r.clone()),
                        Expr::Mul(l, r, _) => (l.clone(), r.clone()),
                        Expr::Div(l, r, _) => (l.clone(), r.clone()),
                        _ => unreachable!(),
                    };
                    *expr = match expr {
                        Expr::Add(_, _, _) => Expr::FAdd(l, r, Span::new(0, 0)),
                        Expr::Sub(_, _, _) => Expr::FSub(l, r, Span::new(0, 0)),
                        Expr::Mul(_, _, _) => Expr::FMul(l, r, Span::new(0, 0)),
                        Expr::Div(_, _, _) => Expr::FDiv(l, r, Span::new(0, 0)),
                        _ => unreachable!(),
                    };
                    return Ok(Type::Primitive(Primitive::Float));
                }

                if !lhs_type.is_numeric() || !rhs_type.is_numeric() {
                    return Err(CheckerError::InvalidOperation {
                        op: "arithmetic".to_string(),
                        type_name: format!("{:?} and {:?}", lhs_type, rhs_type),
                        span: span,
                    });
                }

                if has_type_var {
                    self.unify_types(&lhs_type, &rhs_type)?;
                    let int_type = Type::Primitive(Primitive::Int);
                    if matches!(&lhs_type, Type::TypeVar(_)) {
                        if let Type::TypeVar(id) = &lhs_type {
                            self.bind_type_var(*id, &int_type);
                        }
                    }
                    if matches!(&rhs_type, Type::TypeVar(_)) {
                        if let Type::TypeVar(id) = &rhs_type {
                            self.bind_type_var(*id, &int_type);
                        }
                    }
                }

                Ok(Type::Primitive(Primitive::Int))
            }
            Expr::Mod(lhs, rhs, _) => {
                let lhs_type = self.check_expr(lhs)?;
                let rhs_type = self.check_expr(rhs)?;

                if lhs_type.is_float() || rhs_type.is_float() {
                    return Err(CheckerError::InvalidOperation {
                        op: "modulo".to_string(),
                        type_name: "float".to_string(),
                        span: span,
                    });
                }

                if !lhs_type.is_numeric() || !rhs_type.is_numeric() {
                    return Err(CheckerError::InvalidOperation {
                        op: "modulo".to_string(),
                        type_name: format!("{:?} and {:?}", lhs_type, rhs_type),
                        span: span,
                    });
                }

                Ok(Type::Primitive(Primitive::Int))
            }
            Expr::Neg(operand, _) => {
                let ty = self.check_expr(operand)?;
                if ty.is_float() {
                    *expr = Expr::FNeg(operand.clone(), Span::new(0, 0));
                    return self.check_expr(expr);
                }
                if !ty.is_numeric() {
                    return Err(CheckerError::InvalidOperation {
                        op: "negation".to_string(),
                        type_name: format!("{:?}", ty),
                        span: span,
                    });
                }
                Ok(ty)
            }
            Expr::FNeg(operand, _) => {
                let ty = self.check_expr(operand)?;
                if !ty.is_numeric() {
                    return Err(CheckerError::InvalidOperation {
                        op: "negation".to_string(),
                        type_name: format!("{:?}", ty),
                        span: span,
                    });
                }
                Ok(Type::Primitive(Primitive::Float))
            }
            Expr::Xor(lhs, rhs, _) => {
                let lhs_type = self.check_expr(lhs)?;
                let rhs_type = self.check_expr(rhs)?;
                if lhs_type.is_float() || rhs_type.is_float() {
                    return Err(CheckerError::InvalidOperation {
                        op: "xor".to_string(),
                        type_name: "float".to_string(),
                        span: span,
                    });
                }
                if !lhs_type.is_numeric() || !rhs_type.is_numeric() {
                    return Err(CheckerError::InvalidOperation {
                        op: "xor".to_string(),
                        type_name: format!("{:?} and {:?}", lhs_type, rhs_type),
                        span: span,
                    });
                }
                Ok(Type::Primitive(Primitive::Int))
            }
            Expr::Inc(name, _) | Expr::Dec(name, _) => {
                if self.is_constant(name) {
                    return Err(CheckerError::InvalidOperation {
                        op: "increment/decrement".to_string(),
                        type_name: format!("constant '{}'", name),
                        span: span,
                    });
                }
                let var_type = self
                    .lookup_var(name)
                    .or_else(|| self.extern_vars.get(name).cloned())
                    .or_else(|| self.globals.get(name).cloned())
                    .ok_or_else(|| CheckerError::UndefinedVariable(name.clone(), span))?;
                if !var_type.is_numeric() {
                    return Err(CheckerError::InvalidOperation {
                        op: "increment/decrement".to_string(),
                        type_name: format!("{:?}", var_type),
                        span: span,
                    });
                }
                Ok(var_type)
            }
            Expr::AddAssign(name, value, _) | Expr::SubAssign(name, value, _) => {
                if self.is_constant(name) {
                    return Err(CheckerError::InvalidOperation {
                        op: "compound assignment".to_string(),
                        type_name: format!("constant '{}'", name),
                        span: span,
                    });
                }
                let var_type = self
                    .lookup_var(name)
                    .or_else(|| self.extern_vars.get(name).cloned())
                    .or_else(|| self.globals.get(name).cloned())
                    .ok_or_else(|| CheckerError::UndefinedVariable(name.clone(), span))?;
                let value_type = self.check_expr(value)?;
                if var_type.is_pointer() {
                    if !matches!(
                        value_type,
                        Type::Primitive(Primitive::Int) | Type::TypeVar(_)
                    ) {
                        return Err(CheckerError::TypeMismatch {
                            expected: Type::Primitive(Primitive::Int),
                            found: value_type,
                            context: format!("compound assignment to '{}'", name),
                            span: span,
                        });
                    }
                } else {
                    self.unify_types(&var_type, &value_type).map_err(|_| {
                        CheckerError::TypeMismatch {
                            expected: var_type.clone(),
                            found: value_type,
                            context: format!("compound assignment to '{}'", name),
                            span: span,
                        }
                    })?;
                }
                Ok(var_type)
            }
            Expr::LAnd(lhs, rhs, _) | Expr::LOr(lhs, rhs, _) => {
                let lhs_type = self.check_expr(lhs)?;
                let rhs_type = self.check_expr(rhs)?;
                if !lhs_type.is_bool() || !rhs_type.is_bool() {
                    return Err(CheckerError::InvalidOperation {
                        op: "logical".to_string(),
                        type_name: format!("{:?} and {:?}", lhs_type, rhs_type),
                        span: span,
                    });
                }
                Ok(Type::Primitive(Primitive::Boolean))
            }
            Expr::Eq(lhs, rhs, _)
            | Expr::Ne(lhs, rhs, _)
            | Expr::Lt(lhs, rhs, _)
            | Expr::Le(lhs, rhs, _)
            | Expr::Gt(lhs, rhs, _)
            | Expr::Ge(lhs, rhs, _) => {
                let lhs_type = self.check_expr(lhs)?;
                let rhs_type = self.check_expr(rhs)?;

                if lhs_type.is_float() || rhs_type.is_float() {
                    if !lhs_type.is_numeric() || !rhs_type.is_numeric() {
                        return Err(CheckerError::InvalidOperation {
                            op: "comparison".to_string(),
                            type_name: format!("{:?} and {:?}", lhs_type, rhs_type),
                            span: span,
                        });
                    }

                    let (l, r) = match expr {
                        Expr::Eq(l, r, _) => (l.clone(), r.clone()),
                        Expr::Ne(l, r, _) => (l.clone(), r.clone()),
                        Expr::Lt(l, r, _) => (l.clone(), r.clone()),
                        Expr::Le(l, r, _) => (l.clone(), r.clone()),
                        Expr::Gt(l, r, _) => (l.clone(), r.clone()),
                        Expr::Ge(l, r, _) => (l.clone(), r.clone()),
                        _ => unreachable!(),
                    };
                    *expr = match expr {
                        Expr::Eq(_, _, _) => Expr::FEq(l, r, Span::new(0, 0)),
                        Expr::Ne(_, _, _) => Expr::FNe(l, r, Span::new(0, 0)),
                        Expr::Lt(_, _, _) => Expr::FLt(l, r, Span::new(0, 0)),
                        Expr::Le(_, _, _) => Expr::FLe(l, r, Span::new(0, 0)),
                        Expr::Gt(_, _, _) => Expr::FGt(l, r, Span::new(0, 0)),
                        Expr::Ge(_, _, _) => Expr::FGe(l, r, Span::new(0, 0)),
                        _ => unreachable!(),
                    };
                } else if lhs_type.is_string() || rhs_type.is_string() {
                    if !lhs_type.is_string() || !rhs_type.is_string() {
                        return Err(CheckerError::TypeMismatch {
                            expected: Type::Primitive(Primitive::String),
                            found: if !lhs_type.is_string() {
                                lhs_type.clone()
                            } else {
                                rhs_type.clone()
                            },
                            context: "string comparison".to_string(),
                            span: span,
                        });
                    }
                } else if lhs_type.is_pointer() || rhs_type.is_pointer() {
                    let void_like =
                        |t: &Type| t.is_pointer() || matches!(t, Type::Primitive(Primitive::Void));
                    if !void_like(&lhs_type) || !void_like(&rhs_type) {
                        return Err(CheckerError::InvalidOperation {
                            op: "comparison".to_string(),
                            type_name: format!("{:?} and {:?}", lhs_type, rhs_type),
                            span: span,
                        });
                    }
                } else if !lhs_type.is_numeric() || !rhs_type.is_numeric() {
                    return Err(CheckerError::InvalidOperation {
                        op: "comparison".to_string(),
                        type_name: format!("{:?} and {:?}", lhs_type, rhs_type),
                        span: span,
                    });
                }

                Ok(Type::Primitive(Primitive::Boolean))
            }
            Expr::Not(e, _) => {
                let ty = self.check_expr(e)?;
                if !ty.is_bool() {
                    return Err(CheckerError::InvalidOperation {
                        op: "not".to_string(),
                        type_name: format!("{:?}", ty),
                        span: span,
                    });
                }
                Ok(Type::Primitive(Primitive::Boolean))
            }
            Expr::Call(callee, type_args, args, _) => {
                let callee_type = self.check_expr(callee)?;
                let arg_types: Result<Vec<Type>, CheckerError> =
                    args.iter_mut().map(|arg| self.check_expr(arg)).collect();
                let arg_types = arg_types?;

                if let Expr::Var(name, _) = callee.as_ref() {
                    if let Some(sig) = self.functions.get(name).cloned() {
                        let (tp_names, params, ret_type) = sig;
                        if !tp_names.is_empty() {
                            let mut subst = HashMap::new();
                            let inst_params: Vec<Type> = params
                                .iter()
                                .map(|p| self.fresh_instantiate(p, &mut subst))
                                .collect();
                            let inst_ret = self.fresh_instantiate(&ret_type, &mut subst);

                            if args.len() != inst_params.len() {
                                return Err(CheckerError::ArgCountMismatch {
                                    expected: inst_params.len(),
                                    found: args.len(),
                                    func: name.clone(),
                                    span: span,
                                });
                            }

                            for (i, (arg_type, expected)) in
                                arg_types.iter().zip(inst_params.iter()).enumerate()
                            {
                                self.unify_types(expected, arg_type).map_err(|_| {
                                    CheckerError::TypeMismatch {
                                        expected: expected.clone(),
                                        found: arg_type.clone(),
                                        context: format!(
                                            "argument {} of generic function '{}'",
                                            i + 1,
                                            name
                                        ),
                                        span: span,
                                    }
                                })?;
                            }

                            let resolved_args: Vec<Type> = (0..tp_names.len())
                                .map(|i| {
                                    let tv = subst
                                        .get(&i)
                                        .cloned()
                                        .unwrap_or_else(|| Type::Primitive(Primitive::Int));
                                    self.resolve_type_var(&tv)
                                })
                                .collect();
                            *type_args = resolved_args.clone();

                            let ret = self.resolve_type_var(&inst_ret);
                            return Ok(ret);
                        }
                    }
                }

                match &callee_type {
                    Type::TypeVar(_) => {
                        let inferred_params: Vec<Type> = arg_types.clone();
                        let inferred_ret = self.new_type_var();

                        let inferred_func_type =
                            Type::Function(inferred_params, Box::new(inferred_ret.clone()));
                        self.unify_types(&callee_type, &inferred_func_type)?;
                        Ok(inferred_ret)
                    }
                    Type::Function(params, ret_type) => {
                        if args.len() != params.len() {
                            return Err(CheckerError::ArgCountMismatch {
                                expected: params.len(),
                                found: args.len(),
                                func: "function pointer".to_string(),
                                span: span,
                            });
                        }

                        for (i, (arg_type, expected_ty)) in
                            arg_types.iter().zip(params.iter()).enumerate()
                        {
                            self.unify_types(expected_ty, arg_type).map_err(|_| {
                                CheckerError::TypeMismatch {
                                    expected: expected_ty.clone(),
                                    found: arg_type.clone(),
                                    context: format!("argument {} of function pointer call", i + 1),
                                    span: span,
                                }
                            })?;
                        }

                        Ok(*ret_type.clone())
                    }
                    _ => Err(CheckerError::TypeMismatch {
                        expected: Type::Function(
                            vec![],
                            Box::new(Type::Primitive(Primitive::Void)),
                        ),
                        found: callee_type,
                        context: "callee is not a function type".to_string(),
                        span: span,
                    }),
                }
            }
            Expr::Return(value, _) => {
                let value_type = self.check_expr(value)?;
                if let Some(expected_ret) = self.return_types.last() {
                    let expected_ret = expected_ret.clone();
                    let resolved_ret = self.resolve_type(&expected_ret);
                    if matches!(value.as_ref(), Expr::Nil(_)) {
                        let is_loose_ret = matches!(resolved_ret, Type::TypeVar(_))
                            || matches!(resolved_ret, Type::Primitive(Primitive::Void));
                        if !is_loose_ret {
                            return Err(CheckerError::TypeMismatch {
                                expected: expected_ret.clone(),
                                found: value_type,
                                context: "return statement".to_string(),
                                span: span,
                            });
                        }
                        return Ok(expected_ret);
                    }
                    self.unify_types(&expected_ret, &value_type).map_err(|_| {
                        CheckerError::TypeMismatch {
                            expected: expected_ret.clone(),
                            found: value_type,
                            context: "return statement".to_string(),
                            span: span,
                        }
                    })?;
                    Ok(expected_ret)
                } else {
                    Ok(value_type)
                }
            }
            Expr::If(cond, then_branch, else_branch, _) => {
                let cond_type = self.check_expr(cond)?;
                if !cond_type.is_bool() {
                    return Err(CheckerError::TypeMismatch {
                        expected: Type::Primitive(Primitive::Boolean),
                        found: cond_type,
                        context: "if condition".to_string(),
                        span: span,
                    });
                }

                self.push_scope();
                let then_type = self.check_expr(then_branch)?;
                self.pop_scope();

                let else_type = if let Some(else_expr) = else_branch {
                    self.push_scope();
                    let t = self.check_expr(else_expr)?;
                    self.pop_scope();
                    Some(t)
                } else {
                    None
                };

                match else_type {
                    Some(else_type) => {
                        if self.unify_types(&then_type, &else_type).is_ok() {
                            Ok(self.resolve_type(&then_type))
                        } else {
                            Ok(Type::Primitive(Primitive::Void))
                        }
                    }
                    None => Ok(Type::Primitive(Primitive::Void)),
                }
            }
            Expr::While(cond, body, _) => {
                let cond_type = self.check_expr(cond)?;
                if !cond_type.is_bool() {
                    return Err(CheckerError::TypeMismatch {
                        expected: Type::Primitive(Primitive::Boolean),
                        found: cond_type,
                        context: "while condition".to_string(),
                        span: span,
                    });
                }

                self.push_scope();
                self.check_expr(body)?;
                self.pop_scope();

                Ok(Type::Primitive(Primitive::Void))
            }
            Expr::For(var, array, body, _) => {
                let array_type = self.check_expr(array)?;

                let elem_type = match &array_type {
                    Type::Array(inner) => *inner.clone(),
                    Type::Primitive(Primitive::String) => Type::Primitive(Primitive::String),
                    Type::Struct(struct_name, args) => {
                        let maybe = self
                            .struct_method_return(struct_name, args, "next")
                            .ok_or_else(|| CheckerError::InvalidOperation {
                                op: "for loop".to_string(),
                                type_name: format!("{:?}", array_type),
                                span: span,
                            })?;
                        match maybe {
                            Type::Struct(mname, margs) if mname == "Maybe" => margs
                                .into_iter()
                                .next()
                                .unwrap_or(Type::Primitive(Primitive::Int)),
                            _ => {
                                return Err(CheckerError::InvalidOperation {
                                    op: "for loop".to_string(),
                                    type_name: format!(
                                        "{:?} ('next' must return Maybe<T>)",
                                        array_type
                                    ),
                                    span: span,
                                });
                            }
                        }
                    }
                    _ => {
                        return Err(CheckerError::InvalidOperation {
                            op: "for loop".to_string(),
                            type_name: format!("{:?}", array_type),
                            span: span,
                        });
                    }
                };

                self.push_scope();
                self.declare_var(var, elem_type);
                self.check_expr(body)?;
                self.pop_scope();

                Ok(Type::Primitive(Primitive::Void))
            }
            Expr::Block(body, _) => {
                self.push_scope();
                let mut result = Type::Primitive(Primitive::Void);
                for e in body {
                    result = self.check_expr(e)?;
                }
                self.pop_scope();
                Ok(result)
            }
            Expr::Index(array, idx, _) => {
                let array_type = self.check_expr(array)?;
                let idx_type = self.check_expr(idx)?;

                if !idx_type.is_numeric() {
                    return Err(CheckerError::InvalidOperation {
                        op: "array index".to_string(),
                        type_name: format!("{:?}", idx_type),
                        span: span,
                    });
                }

                match array_type {
                    Type::Array(inner) => Ok(*inner),
                    Type::Primitive(Primitive::String) => Ok(Type::Primitive(Primitive::String)),
                    Type::Pointer(inner) => Ok(*inner),
                    Type::Struct(struct_name, args) => self
                        .struct_method_return(&struct_name, &args, "nth")
                        .ok_or(CheckerError::InvalidOperation {
                            op: "index".to_string(),
                            type_name: format!(
                                "{:?} (no 'nth' method)",
                                Type::Struct(struct_name, args)
                            ),
                            span: span,
                        }),
                    _ => Err(CheckerError::InvalidOperation {
                        op: "index".to_string(),
                        type_name: format!("{:?}", array_type),
                        span: span,
                    }),
                }
            }
            Expr::IndexAssign(array_idx, value, _) => {
                if let Some(name) = self.const_root_name(array_idx) {
                    return Err(CheckerError::InvalidOperation {
                        op: "assignment".to_string(),
                        type_name: format!("constant '{}'", name),
                        span: span,
                    });
                }
                let value_type = self.check_expr(value)?;

                if let Expr::Index(array, idx, _) = array_idx.as_mut() {
                    let array_type = self.get_expr_type(array);
                    let idx_type = self.check_expr(idx)?;

                    if !idx_type.is_numeric() {
                        return Err(CheckerError::InvalidOperation {
                            op: "array index".to_string(),
                            type_name: format!("{:?}", idx_type),
                            span: span,
                        });
                    }

                    match array_type {
                        Type::Array(inner) => {
                            if !self.types_compatible(&inner, &value_type) {
                                return Err(CheckerError::TypeMismatch {
                                    expected: *inner,
                                    found: value_type,
                                    context: "array assignment".to_string(),
                                    span: span,
                                });
                            }
                        }
                        Type::Primitive(Primitive::String) => {
                            if !self
                                .types_compatible(&Type::Primitive(Primitive::String), &value_type)
                            {
                                return Err(CheckerError::TypeMismatch {
                                    expected: Type::Primitive(Primitive::String),
                                    found: value_type,
                                    context: "string assignment".to_string(),
                                    span: span,
                                });
                            }
                        }
                        Type::Pointer(inner) => {
                            if !self.types_compatible(&inner, &value_type) {
                                return Err(CheckerError::TypeMismatch {
                                    expected: *inner,
                                    found: value_type,
                                    context: "pointer assignment".to_string(),
                                    span: span,
                                });
                            }
                        }
                        Type::Struct(struct_name, args) => {
                            let params = self
                                .struct_method_params(&struct_name, &args, "set_nth")
                                .ok_or(CheckerError::InvalidOperation {
                                    op: "index assignment".to_string(),
                                    type_name: format!(
                                        "{:?} (no 'set_nth' method)",
                                        Type::Struct(struct_name.clone(), args.clone())
                                    ),
                                    span: span,
                                })?;
                            let elem_ty = params.get(2).ok_or(CheckerError::InvalidOperation {
                                op: "index assignment".to_string(),
                                type_name: format!(
                                    "'set_nth' of '{:?}' must take 3 parameters",
                                    Type::Struct(struct_name.clone(), args.clone())
                                ),
                                span: span,
                            })?;
                            if !self.types_compatible(elem_ty, &value_type) {
                                return Err(CheckerError::TypeMismatch {
                                    expected: elem_ty.clone(),
                                    found: value_type,
                                    context: "array assignment".to_string(),
                                    span: span,
                                });
                            }
                        }
                        _ => {
                            return Err(CheckerError::InvalidOperation {
                                op: "index assignment".to_string(),
                                type_name: format!("{:?}", array_type),
                                span: span,
                            });
                        }
                    }

                    Ok(Type::Primitive(Primitive::Void))
                } else {
                    Err(CheckerError::InvalidOperation {
                        op: "index assignment".to_string(),
                        type_name: "non-index expression".to_string(),
                        span: span,
                    })
                }
            }
            Expr::ArrayLiteral(elements, _) => {
                let mut elem_type: Option<Type> = None;
                for e in elements {
                    let t = self.check_expr(e)?;
                    if let Some(first) = &elem_type {
                        self.unify_types(first, &t)
                            .map_err(|_| CheckerError::TypeMismatch {
                                expected: first.clone(),
                                found: t,
                                context: "array literal element".to_string(),
                                span: span,
                            })?;
                    } else {
                        elem_type = Some(t);
                    }
                }
                let elem_type = elem_type.unwrap_or(Type::Primitive(Primitive::Int));
                let elem_type = self.resolve_type(&elem_type);
                Ok(Type::Array(Box::new(elem_type)))
            }
            Expr::ArrayFill(elem_type, len, _) => {
                let len_type = self.check_expr(len)?;
                if !len_type.is_numeric() {
                    return Err(CheckerError::InvalidOperation {
                        op: "array fill".to_string(),
                        type_name: format!("{:?}", len_type),
                        span: span,
                    });
                }

                let resolved_elem = self.resolve_type(elem_type);
                Ok(Type::Array(Box::new(resolved_elem)))
            }
            Expr::Range(start, end, _) => {
                let start_type = self.check_expr(start)?;
                let end_type = self.check_expr(end)?;

                if !start_type.is_numeric() || !end_type.is_numeric() {
                    return Err(CheckerError::InvalidOperation {
                        op: "range expression".to_string(),
                        type_name: format!("{:?} and {:?}", start_type, end_type),
                        span: span,
                    });
                }

                Ok(Type::Array(Box::new(Type::Primitive(Primitive::Int))))
            }
            Expr::FuncDecl(name, attrs, type_params, params, ret_type, body, _) => {
                if attrs.is_external {
                    return Ok(Type::Primitive(Primitive::Void));
                }
                self.push_scope();
                if !type_params.is_empty() {
                    self.push_generic_params(type_params.len());
                }
                let mut param_vars = Vec::new();
                for (param_name, param_type) in params {
                    let actual_param_type = self.resolve_params(param_type);
                    self.declare_var(param_name, actual_param_type.clone());
                    param_vars.push(actual_param_type);
                }

                let ret_var = self.resolve_params(ret_type);
                self.return_types.push(ret_var.clone());
                self.check_expr(body)?;
                self.return_types.pop();
                if !type_params.is_empty() {
                    self.pop_generic_params();
                }
                self.pop_scope();

                if type_params.is_empty() {
                    let resolved_params: Vec<Type> =
                        param_vars.iter().map(|t| self.resolve_type(t)).collect();
                    let resolved_ret = self.resolve_type(&ret_var);
                    self.functions
                        .insert(name.clone(), (Vec::new(), resolved_params, resolved_ret));
                }

                Ok(Type::Primitive(Primitive::Void))
            }
            Expr::Break(_) | Expr::Continue(_) => Ok(Type::Primitive(Primitive::Void)),
            Expr::TypeDef(_) => Ok(Type::Primitive(Primitive::Void)),
            Expr::Match(target, branches, default, _) => {
                let target_type = self.check_expr(target)?;
                let has_default = default.is_some();
                self.check_match_exhaustiveness(&target_type, branches, has_default, span)?;
                let mut case_types: Vec<Type> = Vec::new();
                let mut ret_types: Vec<Type> = Vec::new();
                for (case_type, ret_type) in branches {
                    case_types.push(self.check_expr(case_type)?);
                    ret_types.push(self.check_expr(ret_type)?);
                }
                if let Some(d) = default {
                    ret_types.push(self.check_expr(d)?)
                }
                for case_type in case_types {
                    if case_type != target_type {
                        return Err(CheckerError::TypeMismatch {
                            expected: target_type,
                            found: case_type,
                            context: "case".to_string(),
                            span,
                        });
                    }
                }
                if !ret_types.clone().is_empty() {
                    let expected_ret_type = ret_types.first().cloned().unwrap();
                    for ret_type in ret_types {
                        if ret_type != expected_ret_type.clone() {
                            return Err(CheckerError::TypeMismatch {
                                expected: expected_ret_type.clone(),
                                found: ret_type,
                                context: "case".to_string(),
                                span,
                            });
                        }
                    }
                    Ok(expected_ret_type.to_owned())
                } else {
                    Ok(Type::Primitive(Primitive::Void))
                }
            }
            Expr::Struct(_name, type_params, fields, _) => {
                self.push_generic_params(type_params.len());
                for (_, field_ty) in fields {
                    self.validate_type(field_ty)?;
                }
                self.pop_generic_params();
                Ok(Type::Primitive(Primitive::Void))
            }
            Expr::Union(_name, type_params, fields, _) => {
                self.push_generic_params(type_params.len());
                for (_, field_ty) in fields {
                    self.validate_type(field_ty)?;
                }
                self.pop_generic_params();
                Ok(Type::Primitive(Primitive::Void))
            }
            Expr::Enum(_name, members, _) => {
                let mut seen = std::collections::HashSet::new();
                for (member_name, _) in members {
                    if !seen.insert(member_name.clone()) {
                        return Err(CheckerError::InvalidOperation {
                            op: "redeclared enum member".to_string(),
                            type_name: member_name.clone(),
                            span: span,
                        });
                    }
                }
                Ok(Type::Primitive(Primitive::Void))
            }
            Expr::StructLiteral(name, type_args, field_values, _) => {
                let (tp_names, fields) = self
                    .structs
                    .get(name)
                    .ok_or_else(|| CheckerError::UndefinedStruct(name.clone(), span))?
                    .clone();

                let inferred = type_args.is_empty() && !tp_names.is_empty();
                let resolved_args: Vec<Type> = if inferred {
                    let mut subst = HashMap::new();
                    let args: Vec<Type> = (0..tp_names.len())
                        .map(|i| self.fresh_instantiate(&Type::Param(i), &mut subst))
                        .collect();

                    for (field_name, expected_ty) in &fields {
                        let expected = expected_ty.substitute(&args);
                        if let Some((idx, _)) = field_values
                            .iter()
                            .enumerate()
                            .find(|(_, (n, _))| n == field_name)
                        {
                            let expr_type = self.check_expr(&mut field_values[idx].1)?;
                            if let Err(e) = self.unify_types(&expected, &expr_type) {
                                let _ = e;
                                return Err(CheckerError::TypeMismatch {
                                    expected: expected.clone(),
                                    found: expr_type,
                                    context: format!("struct '{}' field '{}'", name, field_name),
                                    span: span,
                                });
                            }
                        }
                    }

                    args.iter().map(|t| self.resolve_type(t)).collect()
                } else {
                    type_args.clone()
                };

                let resolved_args: Vec<Type> = resolved_args
                    .iter()
                    .map(|t| match self.resolve_type(t) {
                        Type::TypeVar(_) => Type::Primitive(Primitive::Int),
                        t => t,
                    })
                    .collect();
                *type_args = resolved_args.clone();

                for (field_name, expected_ty) in &fields {
                    let expected = expected_ty.substitute(&resolved_args);
                    if let Some((idx, _)) = field_values
                        .iter()
                        .enumerate()
                        .find(|(_, (n, _))| n == field_name)
                    {
                        let expr_type = self.check_expr(&mut field_values[idx].1)?;
                        if !self.types_compatible(&expected, &expr_type) {
                            return Err(CheckerError::TypeMismatch {
                                expected: expected.clone(),
                                found: expr_type,
                                context: format!("struct '{}' field '{}'", name, field_name),
                                span: span,
                            });
                        }
                    }
                }

                Ok(Type::Struct(name.clone(), resolved_args))
            }
            Expr::UnionLiteral(name, type_args, field_values, _) => {
                let (tp_names, fields) = self
                    .unions
                    .get(name)
                    .ok_or_else(|| CheckerError::UndefinedUnion(name.clone(), span))?
                    .clone();

                let inferred = type_args.is_empty() && !tp_names.is_empty();
                let resolved_args: Vec<Type> = if inferred {
                    let mut subst = HashMap::new();
                    let args: Vec<Type> = (0..tp_names.len())
                        .map(|i| self.fresh_instantiate(&Type::Param(i), &mut subst))
                        .collect();

                    for (field_name, expected_ty) in &fields {
                        let expected = expected_ty.substitute(&args);
                        if let Some((idx, _)) = field_values
                            .iter()
                            .enumerate()
                            .find(|(_, (n, _))| n == field_name)
                        {
                            let expr_type = self.check_expr(&mut field_values[idx].1)?;
                            if let Err(e) = self.unify_types(&expected, &expr_type) {
                                let _ = e;
                                return Err(CheckerError::TypeMismatch {
                                    expected: expected.clone(),
                                    found: expr_type,
                                    context: format!("union '{}' field '{}'", name, field_name),
                                    span: span,
                                });
                            }
                        }
                    }

                    args.iter().map(|t| self.resolve_type(t)).collect()
                } else {
                    type_args.clone()
                };

                let resolved_args: Vec<Type> = resolved_args
                    .iter()
                    .map(|t| match self.resolve_type(t) {
                        Type::TypeVar(_) => Type::Primitive(Primitive::Int),
                        t => t,
                    })
                    .collect();
                *type_args = resolved_args.clone();

                for (field_name, expected_ty) in &fields {
                    let expected = expected_ty.substitute(&resolved_args);
                    if let Some((idx, _)) = field_values
                        .iter()
                        .enumerate()
                        .find(|(_, (n, _))| n == field_name)
                    {
                        let expr_type = self.check_expr(&mut field_values[idx].1)?;
                        if !self.types_compatible(&expected, &expr_type) {
                            return Err(CheckerError::TypeMismatch {
                                expected: expected.clone(),
                                found: expr_type,
                                context: format!("union '{}' field '{}'", name, field_name),
                                span: span,
                            });
                        }
                    }
                }

                Ok(Type::Union(name.clone(), resolved_args))
            }
            Expr::MemberAccess(obj, field_name, _) => {
                if let Expr::Var(name, _) = obj.as_ref() {
                    if let Some(members) = self.enums.get(name) {
                        for (member_name, _) in members {
                            if member_name == field_name {
                                return Ok(Type::Primitive(Primitive::Int));
                            }
                        }
                        return Err(CheckerError::UndefinedEnumMember {
                            enum_name: name.clone(),
                            member: field_name.clone(),
                            span: span,
                        });
                    }
                }
                let obj_type = self.check_expr(obj)?;
                let (type_name, type_args) = match &obj_type {
                    Type::Struct(name, args) => (name.clone(), args.clone()),
                    Type::Union(name, args) => (name.clone(), args.clone()),
                    Type::Pointer(inner) => match **inner {
                        Type::Struct(ref name, ref args) => (name.clone(), args.clone()),
                        Type::Union(ref name, ref args) => (name.clone(), args.clone()),
                        _ => {
                            return Err(CheckerError::NonStructMemberAccess(
                                format!("{:?}", obj_type),
                                span,
                            ));
                        }
                    },
                    _ => {
                        return Err(CheckerError::NonStructMemberAccess(
                            format!("{:?}", obj_type),
                            span,
                        ));
                    }
                };

                let fields = match self.structs.get(&type_name) {
                    Some((_, fields)) => fields.clone(),
                    None => match self.unions.get(&type_name) {
                        Some((_, fields)) => fields.clone(),
                        None => return Err(CheckerError::UndefinedStruct(type_name.clone(), span)),
                    },
                };

                for (name, ty) in &fields {
                    if name == field_name {
                        let substituted = ty.substitute(&type_args);
                        let resolved_ty = self.resolve_type(&substituted);
                        return Ok(match resolved_ty {
                            Type::TypeVar(_) => Type::Primitive(Primitive::Int),
                            t => t,
                        });
                    }
                }

                Err(CheckerError::UndefinedField {
                    struct_name: type_name,
                    field: field_name.clone(),
                    span: span,
                })
            }
            Expr::MemberAssign(obj, field_name, value, _) => {
                if let Some(name) = self.const_root_name(obj) {
                    return Err(CheckerError::InvalidOperation {
                        op: "assignment".to_string(),
                        type_name: format!("constant '{}'", name),
                        span: span,
                    });
                }
                if let Expr::Var(name, _) = obj.as_ref() {
                    if self.enums.contains_key(name) {
                        return Err(CheckerError::InvalidOperation {
                            op: "assignment to enum member".to_string(),
                            type_name: format!("{}.{}", name, field_name),
                            span: span,
                        });
                    }
                }
                let obj_type = self.check_expr(obj)?;
                let value_type = self.check_expr(value)?;

                let (type_name, type_args) = match &obj_type {
                    Type::Struct(name, args) => (name.clone(), args.clone()),
                    Type::Union(name, args) => (name.clone(), args.clone()),
                    Type::Pointer(inner) => match **inner {
                        Type::Struct(ref name, ref args) => (name.clone(), args.clone()),
                        Type::Union(ref name, ref args) => (name.clone(), args.clone()),
                        _ => {
                            return Err(CheckerError::NonStructMemberAccess(
                                format!("{:?}", obj_type),
                                span,
                            ));
                        }
                    },
                    _ => {
                        return Err(CheckerError::NonStructMemberAccess(
                            format!("{:?}", obj_type),
                            span,
                        ));
                    }
                };

                let fields = match self.structs.get(&type_name) {
                    Some((_, fields)) => fields.clone(),
                    None => match self.unions.get(&type_name) {
                        Some((_, fields)) => fields.clone(),
                        None => return Err(CheckerError::UndefinedStruct(type_name.clone(), span)),
                    },
                };

                for (name, ty) in &fields {
                    if name == field_name {
                        let expected = ty.substitute(&type_args);
                        if !self.types_compatible(&expected, &value_type) {
                            return Err(CheckerError::TypeMismatch {
                                expected: expected.clone(),
                                found: value_type,
                                context: format!(
                                    "struct '{}' field '{}' assignment",
                                    type_name, field_name
                                ),
                                span: span,
                            });
                        }
                        return Ok(Type::Primitive(Primitive::Void));
                    }
                }

                Err(CheckerError::UndefinedField {
                    struct_name: type_name,
                    field: field_name.clone(),
                    span: span,
                })
            }
            Expr::FAdd(lhs, rhs, _)
            | Expr::FSub(lhs, rhs, _)
            | Expr::FMul(lhs, rhs, _)
            | Expr::FDiv(lhs, rhs, _) => {
                self.check_expr(lhs)?;
                self.check_expr(rhs)?;
                Ok(Type::Primitive(Primitive::Float))
            }
            Expr::FEq(lhs, rhs, _)
            | Expr::FNe(lhs, rhs, _)
            | Expr::FLt(lhs, rhs, _)
            | Expr::FLe(lhs, rhs, _)
            | Expr::FGt(lhs, rhs, _)
            | Expr::FGe(lhs, rhs, _) => {
                self.check_expr(lhs)?;
                self.check_expr(rhs)?;
                Ok(Type::Primitive(Primitive::Boolean))
            }
            Expr::StrCat(lhs, rhs, _) => {
                self.check_expr(lhs)?;
                self.check_expr(rhs)?;
                Ok(Type::Primitive(Primitive::String))
            }
            Expr::FString(parts, span) => {
                let mut strings: Vec<Expr> = Vec::new();
                for part in parts.iter_mut() {
                    let ty = self.check_expr(part)?;
                    if ty.is_string() {
                        strings.push(part.clone());
                    } else {
                        strings.push(self.fstring_to_string(part, &ty, *span)?);
                    }
                }
                *expr = if strings.is_empty() {
                    Expr::String(String::new(), *span)
                } else {
                    let mut acc = strings.remove(0);
                    for s in strings {
                        acc = Expr::StrCat(Box::new(acc), Box::new(s), *span);
                    }
                    acc
                };
                Ok(Type::Primitive(Primitive::String))
            }
            Expr::Lambda(params, body, ret_type, _) => {
                self.push_scope();
                let mut param_types = Vec::new();
                for (param_name, param_type) in params.iter() {
                    let actual_param_type = self.resolve_params(param_type);
                    self.declare_var(param_name, actual_param_type.clone());
                    param_types.push(actual_param_type);
                }
                let ret_var = self.resolve_params(ret_type);
                self.return_types.push(ret_var.clone());
                self.check_expr(body)?;
                self.return_types.pop();
                self.pop_scope();
                Ok(Type::Function(param_types, Box::new(ret_var)))
            }
            Expr::AddressOf(expr, _) => {
                let inner_type = self.check_expr(expr)?;
                Ok(Type::Pointer(Box::new(inner_type)))
            }
            Expr::Deref(expr, _) => {
                let ptr_type = self.check_expr(expr)?;
                match ptr_type {
                    Type::Pointer(inner) => Ok(*inner),
                    _ => Err(CheckerError::InvalidOperation {
                        op: "dereference".to_string(),
                        type_name: format!("{:?}", ptr_type),
                        span: span,
                    }),
                }
            }
            Expr::DerefAssign(ptr, val, _) => {
                let ptr_type = self.check_expr(ptr)?;
                let val_type = self.check_expr(val)?;
                match ptr_type {
                    Type::Pointer(inner) => {
                        if !self.types_compatible(&inner, &val_type) {
                            return Err(CheckerError::TypeMismatch {
                                expected: *inner,
                                found: val_type,
                                context: "dereference assignment".to_string(),
                                span: span,
                            });
                        }
                        Ok(Type::Primitive(Primitive::Void))
                    }
                    _ => Err(CheckerError::InvalidOperation {
                        op: "dereference assignment".to_string(),
                        type_name: format!("{:?}", ptr_type),
                        span: span,
                    }),
                }
            }
            Expr::Cast(inner, target_ty, _) => {
                let src_type = self.check_expr(inner)?;
                let resolved_target = self.resolve_type(target_ty);
                match (&src_type, &resolved_target) {
                    (Type::Primitive(Primitive::Int), Type::Primitive(Primitive::Float))
                    | (Type::Primitive(Primitive::Float), Type::Primitive(Primitive::Int))
                    | (Type::Primitive(Primitive::Int), Type::Primitive(Primitive::Int))
                    | (Type::Primitive(Primitive::Float), Type::Primitive(Primitive::Float))
                    | (Type::Primitive(Primitive::Int), Type::Primitive(Primitive::Boolean))
                    | (Type::Primitive(Primitive::Boolean), Type::Primitive(Primitive::Int))
                    | (Type::Primitive(Primitive::Boolean), Type::Primitive(Primitive::Boolean)) => {
                        Ok(resolved_target)
                    }
                    _ => Err(CheckerError::InvalidOperation {
                        op: "cast".to_string(),
                        type_name: format!("{:?} to {:?}", src_type, resolved_target),
                        span: span,
                    }),
                }
            }
        }
    }

    fn struct_method_return(
        &self,
        struct_name: &str,
        type_args: &[Type],
        method: &str,
    ) -> Option<Type> {
        let (_, fields) = self.structs.get(struct_name)?;
        for (fname, fty) in fields {
            if fname == method {
                let substituted = fty.substitute(type_args);
                let resolved = self.resolve_type(&substituted);
                if let Type::Function(_, ret) = resolved {
                    return Some(*ret);
                }
            }
        }
        None
    }

    fn struct_method_params(
        &self,
        struct_name: &str,
        type_args: &[Type],
        method: &str,
    ) -> Option<Vec<Type>> {
        let (_, fields) = self.structs.get(struct_name)?;
        for (fname, fty) in fields {
            if fname == method {
                let substituted = fty.substitute(type_args);
                let resolved = self.resolve_type(&substituted);
                if let Type::Function(params, _) = resolved {
                    return Some(params);
                }
            }
        }
        None
    }

    fn fstring_to_string(&self, part: &Expr, ty: &Type, span: Span) -> Result<Expr, CheckerError> {
        match ty {
            Type::Primitive(Primitive::Int) => Ok(Expr::Call(
                Box::new(Expr::Var("itoa".to_string(), span)),
                Vec::new(),
                vec![part.clone()],
                span,
            )),
            Type::Primitive(Primitive::Float) => Ok(Expr::Call(
                Box::new(Expr::Var("ftoa".to_string(), span)),
                Vec::new(),
                vec![part.clone()],
                span,
            )),
            Type::Primitive(Primitive::Boolean) => Ok(Expr::If(
                Box::new(part.clone()),
                Box::new(Expr::String("true".to_string(), span)),
                Some(Box::new(Expr::String("false".to_string(), span))),
                span,
            )),
            _ => Err(CheckerError::InvalidOperation {
                op: "f-string interpolation".to_string(),
                type_name: ty.to_string(),
                span,
            }),
        }
    }
}
