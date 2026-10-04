use super::error::CheckerError;
use crate::compiler::{
    Span,
    parser::{Expr, Primitive, Type},
    visitor::TypeChecker,
};
use std::collections::HashMap;

impl TypeChecker {
    fn check_composite_literal(
        &mut self,
        name: &str,
        type_args: &mut Vec<Type>,
        field_values: &mut [(String, Expr)],
        span: Span,
        is_union: bool,
    ) -> Result<Type, CheckerError> {
        let tag = if is_union { "union" } else { "struct" };
        let (tp_names, fields) = if is_union {
            self.unions.get(name)
        } else {
            self.structs.get(name)
        }
        .ok_or_else(|| {
            if is_union {
                CheckerError::UndefinedUnion(name.to_string(), span)
            } else {
                CheckerError::UndefinedStruct(name.to_string(), span)
            }
        })?
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
                            context: format!("{tag} '{name}' field '{field_name}'"),
                            span,
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
            .map(|t| self.normalize_type(t))
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
                        context: format!("{tag} '{name}' field '{field_name}'"),
                        span,
                    });
                }
            }
        }

        {
            let mut seen: Vec<&str> = Vec::new();
            for (n, _) in field_values.iter() {
                if !fields.iter().any(|(fname, _)| fname == n) {
                    return Err(CheckerError::UndefinedField {
                        struct_name: name.to_string(),
                        field: n.clone(),
                        span,
                    });
                }
                if seen.contains(&n.as_str()) {
                    return Err(CheckerError::InvalidOperation {
                        op: format!("duplicate field '{n}' in {tag} literal"),
                        type_name: name.to_string(),
                        span,
                    });
                }
                seen.push(n.as_str());
            }
            if is_union {
                if seen.len() != 1 && !fields.is_empty() {
                    return Err(CheckerError::InvalidOperation {
                        op: format!(
                            "union literal must specify exactly one field, got {}",
                            seen.len()
                        ),
                        type_name: name.to_string(),
                        span,
                    });
                }
            } else {
                for (fname, _) in &fields {
                    if !seen.contains(&fname.as_str()) {
                        return Err(CheckerError::InvalidOperation {
                            op: format!("missing field '{fname}' in struct literal"),
                            type_name: name.to_string(),
                            span,
                        });
                    }
                }
            }
        }

        Ok(if is_union {
            Type::Union(name.to_string(), resolved_args)
        } else {
            Type::Struct(name.to_string(), resolved_args)
        })
    }
    fn lookup_assignable(&self, name: &str, op: &str, span: Span) -> Result<Type, CheckerError> {
        match self
            .lookup_var(name)
            .or_else(|| self.extern_vars.get(name).cloned())
            .or_else(|| self.globals.get(name).cloned())
        {
            Some(t) if !self.nearest_decl_is_const(name) => Ok(t),
            Some(_) => Err(CheckerError::InvalidOperation {
                op: op.to_string(),
                type_name: format!("constant '{name}'"),
                span,
            }),
            None if self.is_constant(name) => Err(CheckerError::InvalidOperation {
                op: op.to_string(),
                type_name: format!("constant '{name}'"),
                span,
            }),
            None => Err(CheckerError::UndefinedVariable(name.to_string(), span)),
        }
    }

    pub(super) fn check_expr(&mut self, expr: &mut Expr) -> Result<Type, CheckerError> {
        const MAX_DEPTH: usize = 8000;
        if self.expr_depth > MAX_DEPTH {
            return Err(CheckerError::InvalidOperation {
                op: format!("expression nesting exceeds {} levels", MAX_DEPTH),
                type_name: String::new(),
                span: Span::new(0, 0),
            });
        }
        self.expr_depth += 1;
        let r = self.check_expr_inner(expr);
        self.expr_depth -= 1;
        r
    }

    fn check_expr_inner(&mut self, expr: &mut Expr) -> Result<Type, CheckerError> {
        let span = expr.span();
        match expr {
            Expr::Int { .. } => Ok(Type::Primitive(Primitive::Int)),
            Expr::Char { .. } => Ok(Type::Primitive(Primitive::Char)),
            Expr::Float { .. } => Ok(Type::Primitive(Primitive::Float)),
            Expr::Bool { .. } => Ok(Type::Primitive(Primitive::Boolean)),
            Expr::String { .. } => Ok(Type::Primitive(Primitive::String)),
            Expr::Nil(_) => Ok(Type::Primitive(Primitive::Void)),
            Expr::Var { name, .. } => {
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
            Expr::VarDecl {
                name, ty, value, ..
            } => {
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
            Expr::ConstDecl {
                name, ty, value, ..
            } => {
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
            Expr::GlobalVar {
                name,
                is_pub: _,
                ty,
                value,
                ..
            } => {
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
            Expr::ExternVar { name, ty, .. } => {
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
            Expr::VarAssign { name, value, .. } => {
                let var_type = self.lookup_assignable(name, "assignment", span)?;
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
            Expr::Add {
                left: lhs,
                right: rhs,
                ..
            }
            | Expr::Sub {
                left: lhs,
                right: rhs,
                ..
            }
            | Expr::Mul {
                left: lhs,
                right: rhs,
                ..
            }
            | Expr::Div {
                left: lhs,
                right: rhs,
                ..
            } => {
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

                    if !matches!(expr, Expr::Add { .. }) {
                        return Err(CheckerError::InvalidOperation {
                            op: "arithmetic".to_string(),
                            type_name: format!(
                                "{:?} and {:?} (only '+' is valid for strings)",
                                lhs_type, rhs_type
                            ),
                            span: span,
                        });
                    }

                    let nil = Expr::Nil(span);
                    let (l, r) = match expr {
                        Expr::Add {
                            left: l, right: r, ..
                        } => (
                            std::mem::replace(l.as_mut(), nil.clone()),
                            std::mem::replace(r.as_mut(), nil),
                        ),
                        _ => unreachable!(),
                    };
                    *expr = Expr::StrCat {
                        left: Box::new(l),
                        right: Box::new(r),
                        span,
                    };
                    return Ok(Type::Primitive(Primitive::String));
                }

                let is_int_like =
                    |t: &Type| matches!(t, Type::Primitive(Primitive::Int) | Type::TypeVar(_));
                if matches!(expr, Expr::Add { .. } | Expr::Sub { .. }) {
                    let ptr_type = if lhs_type.is_pointer() && is_int_like(&rhs_type) {
                        Some(lhs_type.clone())
                    } else if matches!(expr, Expr::Add { .. })
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

                if (lhs_type.is_float() && matches!(rhs_type, Type::Primitive(Primitive::Int)))
                    || (rhs_type.is_float() && matches!(lhs_type, Type::Primitive(Primitive::Int)))
                {
                    return Err(CheckerError::TypeMismatch {
                        expected: Type::Primitive(Primitive::Float),
                        found: if lhs_type.is_float() {
                            rhs_type.clone()
                        } else {
                            lhs_type.clone()
                        },
                        context: String::from(
                            "mixed int/float arithmetic (add an explicit '@float' cast)",
                        ),
                        span,
                    });
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

                    let nil = Expr::Nil(span);
                    let (l, r) = match expr {
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
                        } => (
                            std::mem::replace(l.as_mut(), nil.clone()),
                            std::mem::replace(r.as_mut(), nil),
                        ),
                        _ => unreachable!(),
                    };

                    let l = if matches!(lhs_type, Type::Primitive(Primitive::Int)) {
                        Expr::Cast {
                            expr: Box::new(l),
                            ty: Type::Primitive(Primitive::Float),
                            span,
                        }
                    } else {
                        l
                    };
                    let r = if matches!(rhs_type, Type::Primitive(Primitive::Int)) {
                        Expr::Cast {
                            expr: Box::new(r),
                            ty: Type::Primitive(Primitive::Float),
                            span,
                        }
                    } else {
                        r
                    };
                    *expr = match expr {
                        Expr::Add { .. } => Expr::FAdd {
                            left: Box::new(l),
                            right: Box::new(r),
                            span,
                        },
                        Expr::Sub { .. } => Expr::FSub {
                            left: Box::new(l),
                            right: Box::new(r),
                            span,
                        },
                        Expr::Mul { .. } => Expr::FMul {
                            left: Box::new(l),
                            right: Box::new(r),
                            span,
                        },
                        Expr::Div { .. } => Expr::FDiv {
                            left: Box::new(l),
                            right: Box::new(r),
                            span,
                        },
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
            Expr::Mod {
                left: lhs,
                right: rhs,
                ..
            } => {
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
            Expr::Neg { expr: operand, .. } => {
                let ty = self.check_expr(operand)?;
                if ty.is_float() {
                    let nil = Expr::Nil(span);
                    let inner = std::mem::replace(operand.as_mut(), nil);
                    *expr = Expr::FNeg {
                        expr: Box::new(inner),
                        span,
                    };
                    return Ok(Type::Primitive(Primitive::Float));
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
            Expr::FNeg { expr: operand, .. } => {
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
            Expr::Xor {
                left: lhs,
                right: rhs,
                ..
            } => {
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
            Expr::Shl {
                left: lhs,
                right: rhs,
                ..
            }
            | Expr::Shr {
                left: lhs,
                right: rhs,
                ..
            } => {
                let lhs_type = self.check_expr(lhs)?;
                let rhs_type = self.check_expr(rhs)?;
                if lhs_type.is_float() || rhs_type.is_float() {
                    return Err(CheckerError::InvalidOperation {
                        op: "shift".to_string(),
                        type_name: "float".to_string(),
                        span: span,
                    });
                }
                if !lhs_type.is_numeric() || !rhs_type.is_numeric() {
                    return Err(CheckerError::InvalidOperation {
                        op: "shift".to_string(),
                        type_name: format!("{:?} and {:?}", lhs_type, rhs_type),
                        span: span,
                    });
                }
                Ok(Type::Primitive(Primitive::Int))
            }
            Expr::BNot { expr: operand, .. } => {
                let ty = self.check_expr(operand)?;
                if ty.is_float() {
                    return Err(CheckerError::InvalidOperation {
                        op: "bitwise not".to_string(),
                        type_name: "float".to_string(),
                        span: span,
                    });
                }
                if !ty.is_numeric() {
                    return Err(CheckerError::InvalidOperation {
                        op: "bitwise not".to_string(),
                        type_name: format!("{:?}", ty),
                        span: span,
                    });
                }
                Ok(Type::Primitive(Primitive::Int))
            }
            Expr::Inc { name, .. } | Expr::Dec { name, .. } => {
                let var_type = self.lookup_assignable(name, "increment/decrement", span)?;
                if !var_type.is_numeric() {
                    return Err(CheckerError::InvalidOperation {
                        op: "increment/decrement".to_string(),
                        type_name: format!("{:?}", var_type),
                        span: span,
                    });
                }
                Ok(var_type)
            }
            Expr::AddAssign { name, value, .. } | Expr::SubAssign { name, value, .. } => {
                let var_type = self.lookup_assignable(name, "compound assignment", span)?;
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
            Expr::MulAssign { name, value, .. }
            | Expr::DivAssign { name, value, .. }
            | Expr::ModAssign { name, value, .. }
            | Expr::AndAssign { name, value, .. }
            | Expr::OrAssign { name, value, .. }
            | Expr::XorAssign { name, value, .. }
            | Expr::ShlAssign { name, value, .. }
            | Expr::ShrAssign { name, value, .. } => {
                let var_type = self.lookup_assignable(name, "compound assignment", span)?;
                if var_type.is_pointer() {
                    return Err(CheckerError::InvalidOperation {
                        op: "compound assignment".to_string(),
                        type_name: format!("pointer '{}'", name),
                        span: span,
                    });
                }
                let value_type = self.check_expr(value)?;
                self.unify_types(&var_type, &value_type).map_err(|_| {
                    CheckerError::TypeMismatch {
                        expected: var_type.clone(),
                        found: value_type,
                        context: format!("compound assignment to '{}'", name),
                        span: span,
                    }
                })?;
                Ok(var_type)
            }
            Expr::BAnd {
                left: lhs,
                right: rhs,
                ..
            }
            | Expr::BOr {
                left: lhs,
                right: rhs,
                ..
            } => {
                let lhs_type = self.check_expr(lhs)?;
                let rhs_type = self.check_expr(rhs)?;
                if lhs_type.is_float() || rhs_type.is_float() {
                    return Err(CheckerError::InvalidOperation {
                        op: "bitwise".to_string(),
                        type_name: format!("{:?} and {:?}", lhs_type, rhs_type),
                        span: span,
                    });
                }
                if lhs_type.is_numeric() && rhs_type.is_numeric() {
                    return Ok(Type::Primitive(Primitive::Int));
                }
                return Err(CheckerError::InvalidOperation {
                    op: "bitwise".to_string(),
                    type_name: format!("{:?} and {:?}", lhs_type, rhs_type),
                    span: span,
                });
            }
            Expr::LAnd {
                left: lhs,
                right: rhs,
                ..
            }
            | Expr::LOr {
                left: lhs,
                right: rhs,
                ..
            } => {
                let lhs_type = self.check_expr(lhs)?;
                let rhs_type = self.check_expr(rhs)?;
                if lhs_type.is_bool() && rhs_type.is_bool() {
                    return Ok(Type::Primitive(Primitive::Boolean));
                }
                if lhs_type.is_numeric() && rhs_type.is_numeric() {
                    return Ok(Type::Primitive(Primitive::Int));
                }
                return Err(CheckerError::InvalidOperation {
                    op: "logical/bitwise".to_string(),
                    type_name: format!("{:?} and {:?}", lhs_type, rhs_type),
                    span: span,
                });
            }
            Expr::Eq {
                left: lhs,
                right: rhs,
                ..
            }
            | Expr::Ne {
                left: lhs,
                right: rhs,
                ..
            }
            | Expr::Lt {
                left: lhs,
                right: rhs,
                ..
            }
            | Expr::Le {
                left: lhs,
                right: rhs,
                ..
            }
            | Expr::Gt {
                left: lhs,
                right: rhs,
                ..
            }
            | Expr::Ge {
                left: lhs,
                right: rhs,
                ..
            } => {
                let lhs_type = self.check_expr(lhs)?;
                let rhs_type = self.check_expr(rhs)?;

                if (lhs_type.is_float() && matches!(rhs_type, Type::Primitive(Primitive::Int)))
                    || (rhs_type.is_float() && matches!(lhs_type, Type::Primitive(Primitive::Int)))
                {
                    return Err(CheckerError::TypeMismatch {
                        expected: Type::Primitive(Primitive::Float),
                        found: if lhs_type.is_float() {
                            rhs_type.clone()
                        } else {
                            lhs_type.clone()
                        },
                        context: String::from(
                            "mixed int/float comparison (add an explicit '@float' cast)",
                        ),
                        span,
                    });
                }

                if lhs_type.is_float() || rhs_type.is_float() {
                    if !lhs_type.is_numeric() || !rhs_type.is_numeric() {
                        return Err(CheckerError::InvalidOperation {
                            op: "comparison".to_string(),
                            type_name: format!("{:?} and {:?}", lhs_type, rhs_type),
                            span: span,
                        });
                    }

                    let nil = Expr::Nil(span);
                    let (l, r) = match expr {
                        Expr::Eq {
                            left: l, right: r, ..
                        }
                        | Expr::Ne {
                            left: l, right: r, ..
                        }
                        | Expr::Lt {
                            left: l, right: r, ..
                        }
                        | Expr::Le {
                            left: l, right: r, ..
                        }
                        | Expr::Gt {
                            left: l, right: r, ..
                        }
                        | Expr::Ge {
                            left: l, right: r, ..
                        } => (
                            std::mem::replace(l.as_mut(), nil.clone()),
                            std::mem::replace(r.as_mut(), nil),
                        ),
                        _ => unreachable!(),
                    };

                    let l = if matches!(lhs_type, Type::Primitive(Primitive::Int)) {
                        Expr::Cast {
                            expr: Box::new(l),
                            ty: Type::Primitive(Primitive::Float),
                            span,
                        }
                    } else {
                        l
                    };
                    let r = if matches!(rhs_type, Type::Primitive(Primitive::Int)) {
                        Expr::Cast {
                            expr: Box::new(r),
                            ty: Type::Primitive(Primitive::Float),
                            span,
                        }
                    } else {
                        r
                    };
                    *expr = match expr {
                        Expr::Eq { .. } => Expr::FEq {
                            left: Box::new(l),
                            right: Box::new(r),
                            span,
                        },
                        Expr::Ne { .. } => Expr::FNe {
                            left: Box::new(l),
                            right: Box::new(r),
                            span,
                        },
                        Expr::Lt { .. } => Expr::FLt {
                            left: Box::new(l),
                            right: Box::new(r),
                            span,
                        },
                        Expr::Le { .. } => Expr::FLe {
                            left: Box::new(l),
                            right: Box::new(r),
                            span,
                        },
                        Expr::Gt { .. } => Expr::FGt {
                            left: Box::new(l),
                            right: Box::new(r),
                            span,
                        },
                        Expr::Ge { .. } => Expr::FGe {
                            left: Box::new(l),
                            right: Box::new(r),
                            span,
                        },
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
            Expr::Not { expr: e, .. } => {
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
            Expr::Call {
                callee,
                type_args: _,
                args,
                ..
            } if matches!(callee.as_ref(), Expr::Var { name: n, .. } if n == "_alum_copy") => {
                if args.len() != 1 {
                    return Err(CheckerError::ArgCountMismatch {
                        expected: 1,
                        found: args.len(),
                        func: "copy".to_string(),
                        span,
                    });
                }
                self.check_expr(&mut args[0])
            }
            Expr::Call {
                callee,
                type_args,
                args,
                ..
            } => {
                let callee_type = self.check_expr(callee)?;
                let arg_types: Result<Vec<Type>, CheckerError> =
                    args.iter_mut().map(|arg| self.check_expr(arg)).collect();
                let arg_types = arg_types?;

                if let Expr::Var { name, .. } = callee.as_ref() {
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
                                    self.resolve_type(&tv)
                                })
                                .collect();
                            *type_args = resolved_args.clone();

                            let ret = self.resolve_type(&inst_ret);
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
            Expr::Return { value, .. } => {
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
            Expr::If {
                cond,
                then_branch,
                else_branch,
                ..
            } => {
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
            Expr::While { cond, body, .. } => {
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
                self.loop_break_types.push(Vec::new());
                self.check_expr(body)?;
                let break_types = self.loop_break_types.pop().unwrap_or_default();
                self.pop_scope();

                Ok(unify_break_types(break_types, span)?)
            }
            Expr::For {
                var,
                iterable: array,
                body,
                ..
            } => {
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
                            Type::Struct(mname, margs)
                                if crate::compiler::is_maybe_type_name(&mname) =>
                            {
                                { margs }
                                    .into_iter()
                                    .next()
                                    .unwrap_or(Type::Primitive(Primitive::Int))
                            }
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
                self.loop_break_types.push(Vec::new());
                self.check_expr(body)?;
                let break_types = self.loop_break_types.pop().unwrap_or_default();
                self.pop_scope();

                Ok(unify_break_types(break_types, span)?)
            }
            Expr::Block { stmts: body, .. } => {
                self.push_scope();
                let base = self.type_stack.len();
                let ret_base = self.return_types.len();
                let gen_base = self.generic_params.len();
                let mut result = Type::Primitive(Primitive::Void);
                for e in body {
                    match self.check_expr(e) {
                        Ok(t) => result = t,
                        Err(err) => self.errors.push(err),
                    }
                    self.type_stack.truncate(base);
                    self.const_stack.truncate(base);
                    self.return_types.truncate(ret_base);
                    self.generic_params.truncate(gen_base);
                }
                self.pop_scope();
                Ok(result)
            }
            Expr::Index {
                array, index: idx, ..
            } => {
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
                    Type::Primitive(Primitive::String) => Ok(Type::Primitive(Primitive::Char)),
                    Type::Pointer(inner)
                        if matches!(inner.as_ref(), Type::Primitive(Primitive::Void)) =>
                    {
                        Ok(self.new_type_var())
                    }
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
            Expr::IndexAssign {
                target: array_idx,
                value,
                ..
            } => {
                if let Some(name) = self.const_root_name(array_idx) {
                    return Err(CheckerError::InvalidOperation {
                        op: "assignment".to_string(),
                        type_name: format!("constant '{}'", name),
                        span: span,
                    });
                }
                let value_type = self.check_expr(value)?;

                if let Expr::Index {
                    array, index: idx, ..
                } = array_idx.as_mut()
                {
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
                            let byte_store = matches!(
                                value_type,
                                Type::Primitive(Primitive::Int) | Type::Primitive(Primitive::Char)
                            );
                            if !byte_store
                                && !self.types_compatible(
                                    &Type::Primitive(Primitive::String),
                                    &value_type,
                                )
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
                            let void_byte_store =
                                matches!(inner.as_ref(), Type::Primitive(Primitive::Void))
                                    && matches!(value_type, Type::Primitive(Primitive::Int));
                            if !void_byte_store && !self.types_compatible(&inner, &value_type) {
                                return Err(CheckerError::TypeMismatch {
                                    expected: *inner,
                                    found: value_type,
                                    context: "pointer assignment".to_string(),
                                    span,
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

                    Ok(value_type)
                } else {
                    Err(CheckerError::InvalidOperation {
                        op: "index assignment".to_string(),
                        type_name: "non-index expression".to_string(),
                        span: span,
                    })
                }
            }
            Expr::ArrayLiteral { elements, .. } => {
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
            Expr::ArrayFill { elem_type, len, .. } => {
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
            Expr::Range { start, end, .. } => {
                let start_type = self.check_expr(start)?;
                let end_type = self.check_expr(end)?;

                if !start_type.is_numeric() || !end_type.is_numeric() {
                    return Err(CheckerError::InvalidOperation {
                        op: "range expression".to_string(),
                        type_name: format!("{:?} and {:?}", start_type, end_type),
                        span: span,
                    });
                }

                let elem = if matches!(start_type, Type::Primitive(Primitive::Char)) {
                    Type::Primitive(Primitive::Char)
                } else {
                    Type::Primitive(Primitive::Int)
                };
                Ok(Type::Array(Box::new(elem)))
            }
            Expr::FuncDecl {
                name,
                attrs,
                type_params,
                params,
                return_type: ret_type,
                body,
                ..
            } => {
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
                let body_type = self.check_expr(body)?;

                {
                    let resolved_ret = self.resolve_type(&ret_var);
                    let resolved_body = self.resolve_type(&body_type);

                    let ret_is_void = matches!(resolved_ret, Type::Primitive(Primitive::Void));
                    let body_unresolved = matches!(resolved_body, Type::TypeVar(_));
                    if ret_is_void
                        && !body_unresolved
                        && !matches!(resolved_body, Type::Primitive(Primitive::Void))
                    {
                        self.return_types.pop();
                        if !type_params.is_empty() {
                            self.pop_generic_params();
                        }
                        self.pop_scope();
                        return Err(CheckerError::TypeMismatch {
                            expected: ret_var.clone(),
                            found: resolved_body.clone(),
                            context: "function return type".to_string(),
                            span,
                        });
                    }
                    if !ret_is_void && resolved_ret != resolved_body {
                        self.return_types.pop();
                        if !type_params.is_empty() {
                            self.pop_generic_params();
                        }
                        self.pop_scope();
                        return Err(CheckerError::TypeMismatch {
                            expected: ret_var.clone(),
                            found: body_type.clone(),
                            context: "function return type".to_string(),
                            span,
                        });
                    }
                }
                self.return_types.pop();
                if !type_params.is_empty() {
                    self.pop_generic_params();
                }
                self.pop_scope();

                if type_params.is_empty() {
                    let resolved_params: Vec<Type> =
                        param_vars.iter().map(|t| self.resolve_type(t)).collect();
                    let resolved_ret = self.resolve_type(&ret_var);
                    self.functions.insert(
                        name.clone(),
                        (Vec::new(), resolved_params.clone(), resolved_ret.clone()),
                    );
                    Ok(Type::Function(resolved_params, Box::new(resolved_ret)))
                } else {
                    Ok(Type::Primitive(Primitive::Void))
                }
            }
            Expr::Break { value, span: bspan } => {
                if self.loop_break_types.is_empty() {
                    return Err(CheckerError::InvalidOperation {
                        op: "break".to_string(),
                        type_name: "outside of a loop".to_string(),
                        span: *bspan,
                    });
                }
                if let Some(v) = value {
                    let t = self.check_expr(v)?;
                    if let Some(types) = self.loop_break_types.last_mut() {
                        types.push(t);
                    }
                }
                Ok(Type::Primitive(Primitive::Void))
            }
            Expr::Continue(_) => Ok(Type::Primitive(Primitive::Void)),
            Expr::TypeDef(_) => Ok(Type::Primitive(Primitive::Void)),
            Expr::Match {
                target,
                branches,
                default,
                ..
            } => {
                let target_type = self.check_expr(target)?;
                let has_default = default.is_some();
                self.check_match_exhaustiveness(&target_type, branches, has_default, span)?;
                let mut case_types: Vec<Type> = Vec::new();
                let mut ret_types: Vec<Type> = Vec::new();
                for (case_type, guard, ret_type) in branches {
                    if let Some(guard) = guard {
                        let guard_type = self.check_expr(guard)?;
                        if !guard_type.is_bool() {
                            return Err(CheckerError::TypeMismatch {
                                expected: Type::Primitive(Primitive::Boolean),
                                found: guard_type,
                                context: "match guard".to_string(),
                                span: span,
                            });
                        }
                    }
                    if let Expr::Range {
                        start: lo,
                        end: hi,
                        inclusive: _,
                        span: rspan,
                    } = case_type
                    {
                        let lo_type = self.check_expr(lo)?;
                        let hi_type = self.check_expr(hi)?;
                        if !lo_type.is_numeric() || !hi_type.is_numeric() {
                            return Err(CheckerError::InvalidOperation {
                                op: "range pattern".to_string(),
                                type_name: format!("{:?} and {:?}", lo_type, hi_type),
                                span: *rspan,
                            });
                        }
                        let resolved_target = self.resolve_type(&target_type);
                        if !matches!(
                            resolved_target,
                            Type::Primitive(Primitive::Int) | Type::Primitive(Primitive::Char)
                        ) {
                            return Err(CheckerError::TypeMismatch {
                                expected: Type::Primitive(Primitive::Int),
                                found: resolved_target,
                                context: "range pattern".to_string(),
                                span: *rspan,
                            });
                        }
                        ret_types.push(self.check_expr(ret_type)?);
                        continue;
                    }
                    case_types.push(self.check_expr(case_type)?);
                    ret_types.push(self.check_expr(ret_type)?);
                }
                if let Some(d) = default {
                    ret_types.push(self.check_expr(d)?)
                }
                for case_type in case_types {
                    let both_numeric = case_type.is_numeric() && target_type.is_numeric();
                    if case_type != target_type && !both_numeric {
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
            Expr::Struct {
                name: _name,
                type_params,
                fields,
                ..
            } => {
                self.push_generic_params(type_params.len());
                let mut seen = std::collections::HashSet::new();
                for (field_name, field_ty) in fields {
                    self.validate_type(field_ty)?;
                    if !seen.insert(field_name.clone()) {
                        return Err(CheckerError::InvalidOperation {
                            op: "redeclared struct field".to_string(),
                            type_name: field_name.clone(),
                            span: span,
                        });
                    }
                }
                self.pop_generic_params();
                Ok(Type::Primitive(Primitive::Void))
            }
            Expr::Union {
                name: _name,
                type_params,
                fields,
                ..
            } => {
                self.push_generic_params(type_params.len());
                let mut seen = std::collections::HashSet::new();
                for (field_name, field_ty) in fields {
                    self.validate_type(field_ty)?;
                    if !seen.insert(field_name.clone()) {
                        return Err(CheckerError::InvalidOperation {
                            op: "redeclared union field".to_string(),
                            type_name: field_name.clone(),
                            span: span,
                        });
                    }
                }
                self.pop_generic_params();
                Ok(Type::Primitive(Primitive::Void))
            }
            Expr::Enum {
                name: _name,
                members,
                ..
            } => {
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
            Expr::StructLiteral {
                name,
                type_args,
                fields: field_values,
                ..
            } => self.check_composite_literal(name, type_args, field_values, span, false),
            Expr::UnionLiteral {
                name,
                type_args,
                fields: field_values,
                ..
            } => self.check_composite_literal(name, type_args, field_values, span, true),
            Expr::MemberAccess {
                obj,
                field: field_name,
                ..
            } => {
                if let Expr::Var { name, .. } = obj.as_ref() {
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
                        return Ok(self.normalize_type(&substituted));
                    }
                }

                Err(CheckerError::UndefinedField {
                    struct_name: type_name,
                    field: field_name.clone(),
                    span: span,
                })
            }
            Expr::MemberAssign {
                obj,
                field: field_name,
                value,
                ..
            } => {
                if let Some(name) = self.const_root_name(obj) {
                    return Err(CheckerError::InvalidOperation {
                        op: "assignment".to_string(),
                        type_name: format!("constant '{}'", name),
                        span: span,
                    });
                }
                if let Expr::Var { name, .. } = obj.as_ref() {
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
                        return Ok(expected);
                    }
                }

                Err(CheckerError::UndefinedField {
                    struct_name: type_name,
                    field: field_name.clone(),
                    span: span,
                })
            }
            Expr::FAdd {
                left: lhs,
                right: rhs,
                ..
            }
            | Expr::FSub {
                left: lhs,
                right: rhs,
                ..
            }
            | Expr::FMul {
                left: lhs,
                right: rhs,
                ..
            }
            | Expr::FDiv {
                left: lhs,
                right: rhs,
                ..
            } => {
                self.check_expr(lhs)?;
                self.check_expr(rhs)?;
                Ok(Type::Primitive(Primitive::Float))
            }
            Expr::FEq {
                left: lhs,
                right: rhs,
                ..
            }
            | Expr::FNe {
                left: lhs,
                right: rhs,
                ..
            }
            | Expr::FLt {
                left: lhs,
                right: rhs,
                ..
            }
            | Expr::FLe {
                left: lhs,
                right: rhs,
                ..
            }
            | Expr::FGt {
                left: lhs,
                right: rhs,
                ..
            }
            | Expr::FGe {
                left: lhs,
                right: rhs,
                ..
            } => {
                self.check_expr(lhs)?;
                self.check_expr(rhs)?;
                Ok(Type::Primitive(Primitive::Boolean))
            }
            Expr::StrCat {
                left: lhs,
                right: rhs,
                ..
            } => {
                self.check_expr(lhs)?;
                self.check_expr(rhs)?;
                Ok(Type::Primitive(Primitive::String))
            }
            Expr::FString { segs: parts, span } => {
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
                    Expr::String {
                        value: String::new(),
                        span: *span,
                    }
                } else if strings.len() == 1 {
                    Expr::Call {
                        callee: Box::new(Expr::Var {
                            name: "_alum_copy".to_string(),
                            span: *span,
                        }),
                        type_args: Vec::new(),
                        args: strings,
                        span: *span,
                    }
                } else {
                    let mut acc = strings.remove(0);
                    for s in strings {
                        acc = Expr::StrCat {
                            left: Box::new(acc),
                            right: Box::new(s),
                            span: *span,
                        };
                    }
                    acc
                };
                Ok(Type::Primitive(Primitive::String))
            }
            Expr::Lambda {
                params,
                body,
                return_type: ret_type,
                span: lambda_span,
            } => {
                self.push_scope();
                let mut param_types = Vec::new();
                for (param_name, param_type) in params.iter() {
                    let actual_param_type = self.resolve_params(param_type);
                    self.declare_var(param_name, actual_param_type.clone());
                    param_types.push(actual_param_type);
                }
                let ret_var = self.resolve_params(ret_type);
                self.return_types.push(ret_var.clone());
                let body_type = self.check_expr(body)?;
                {
                    let resolved_ret = self.resolve_type(&ret_var);
                    let resolved_body = self.resolve_type(&body_type);
                    let ret_is_void = matches!(resolved_ret, Type::Primitive(Primitive::Void));
                    if !ret_is_void && resolved_ret != resolved_body {
                        self.return_types.pop();
                        self.pop_scope();
                        return Err(CheckerError::TypeMismatch {
                            expected: ret_var.clone(),
                            found: body_type.clone(),
                            context: "lambda return type".to_string(),
                            span: *lambda_span,
                        });
                    }
                }
                self.return_types.pop();
                self.pop_scope();
                Ok(Type::Function(param_types, Box::new(ret_var)))
            }
            Expr::AddressOf { expr, .. } => {
                let inner_type = self.check_expr(expr)?;
                Ok(Type::Pointer(Box::new(inner_type)))
            }
            Expr::Deref { expr, .. } => {
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
            Expr::DerefAssign {
                ptr, value: val, ..
            } => {
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
                        Ok(*inner)
                    }
                    _ => Err(CheckerError::InvalidOperation {
                        op: "dereference assignment".to_string(),
                        type_name: format!("{:?}", ptr_type),
                        span: span,
                    }),
                }
            }
            Expr::Cast {
                expr: inner,
                ty: target_ty,
                ..
            } => {
                let src_type = self.check_expr(inner)?;
                let resolved_target = self.resolve_type(target_ty);
                match (&src_type, &resolved_target) {
                    (Type::Primitive(Primitive::Int), Type::Primitive(Primitive::Float))
                    | (Type::Primitive(Primitive::Float), Type::Primitive(Primitive::Int))
                    | (Type::Primitive(Primitive::Int), Type::Primitive(Primitive::Int))
                    | (Type::Primitive(Primitive::Float), Type::Primitive(Primitive::Float))
                    | (Type::Primitive(Primitive::Int), Type::Primitive(Primitive::Boolean))
                    | (Type::Primitive(Primitive::Boolean), Type::Primitive(Primitive::Int))
                    | (Type::Primitive(Primitive::Boolean), Type::Primitive(Primitive::Boolean))
                    | (_, Type::Primitive(Primitive::Void)) => Ok(resolved_target),
                    (Type::Primitive(Primitive::Void), _) => Ok(resolved_target),
                    (Type::Primitive(Primitive::Char), Type::Primitive(Primitive::Int))
                    | (Type::Primitive(Primitive::Int), Type::Primitive(Primitive::Char))
                    | (Type::Primitive(Primitive::Char), Type::Primitive(Primitive::Char))
                    | (Type::Primitive(Primitive::Char), Type::Primitive(Primitive::Float))
                    | (Type::Primitive(Primitive::Float), Type::Primitive(Primitive::Char)) => {
                        Ok(resolved_target)
                    }
                    (Type::Pointer(_), Type::Pointer(_)) => Ok(resolved_target),
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
            Type::Primitive(Primitive::Int) => Ok(Expr::Call {
                callee: Box::new(Expr::Var {
                    name: "itoa".to_string(),
                    span,
                }),
                type_args: Vec::new(),
                args: vec![part.clone()],
                span,
            }),
            Type::Primitive(Primitive::Float) => Ok(Expr::Call {
                callee: Box::new(Expr::Var {
                    name: "ftoa".to_string(),
                    span,
                }),
                type_args: Vec::new(),
                args: vec![part.clone()],
                span,
            }),
            Type::Primitive(Primitive::Boolean) => Ok(Expr::If {
                cond: Box::new(part.clone()),
                then_branch: Box::new(Expr::String {
                    value: "true".to_string(),
                    span,
                }),
                else_branch: Some(Box::new(Expr::String {
                    value: "false".to_string(),
                    span,
                })),
                span,
            }),
            Type::Primitive(Primitive::Void) => Ok(Expr::String {
                value: "nil".to_string(),
                span,
            }),
            _ => Err(CheckerError::InvalidOperation {
                op: "f-string interpolation".to_string(),
                type_name: ty.to_string(),
                span,
            }),
        }
    }
}

fn unify_break_types(
    types: Vec<Type>,
    span: crate::compiler::span::Span,
) -> Result<Type, CheckerError> {
    let mut iter = types.into_iter();
    let Some(first) = iter.next() else {
        return Ok(Type::Primitive(Primitive::Void));
    };
    for t in iter {
        if t != first {
            return Err(CheckerError::TypeMismatch {
                expected: first.clone(),
                found: t,
                context: "break value".to_string(),
                span,
            });
        }
    }
    Ok(first)
}
