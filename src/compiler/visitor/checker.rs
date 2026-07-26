use crate::compiler::Span;
use crate::compiler::parser::{Expr, Program, Type};
use std::collections::HashMap;

#[derive(Debug)]
pub enum CheckerError {
    TypeMismatch {
        expected: Type,
        found: Type,
        context: String,
        span: Span,
    },
    UndefinedVariable(String, Span),
    #[allow(dead_code)]
    UndefinedFunction(String, Span),
    UndefinedStruct(String, Span),
    UndefinedField {
        struct_name: String,
        field: String,
        span: Span,
    },
    ArgCountMismatch {
        expected: usize,
        found: usize,
        func: String,
        span: Span,
    },
    NonStructMemberAccess(String, Span),
    InvalidOperation {
        op: String,
        type_name: String,
        span: Span,
    },
}

impl std::fmt::Display for CheckerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CheckerError::TypeMismatch {
                expected,
                found,
                context,
                ..
            } => {
                write!(
                    f,
                    "Type mismatch in {}: expected {}, found {}",
                    context, expected, found
                )
            }
            CheckerError::UndefinedVariable(name, _) => {
                write!(f, "Undefined variable: {}", name)
            }
            CheckerError::UndefinedFunction(name, _) => {
                write!(f, "Undefined function: {}", name)
            }
            CheckerError::UndefinedStruct(name, _) => {
                write!(f, "Undefined struct: {}", name)
            }
            CheckerError::UndefinedField {
                struct_name, field, ..
            } => {
                write!(f, "Struct {} has no field {}", struct_name, field)
            }
            CheckerError::ArgCountMismatch {
                expected,
                found,
                func,
                ..
            } => {
                write!(
                    f,
                    "Function {} expects {} arguments, found {}",
                    func, expected, found
                )
            }
            CheckerError::NonStructMemberAccess(type_name, _) => {
                write!(f, "Cannot access member on non-struct type: {}", type_name)
            }
            CheckerError::InvalidOperation { op, type_name, .. } => {
                write!(f, "Invalid operation '{}' on type {}", op, type_name)
            }
        }
    }
}

impl std::error::Error for CheckerError {}

impl CheckerError {
    pub fn span(&self) -> crate::compiler::Span {
        match self {
            CheckerError::TypeMismatch { span, .. }
            | CheckerError::UndefinedField { span, .. }
            | CheckerError::ArgCountMismatch { span, .. }
            | CheckerError::InvalidOperation { span, .. } => *span,
            CheckerError::UndefinedVariable(_, s)
            | CheckerError::UndefinedFunction(_, s)
            | CheckerError::UndefinedStruct(_, s)
            | CheckerError::NonStructMemberAccess(_, s) => *s,
        }
    }
}

pub struct TypeChecker {
    type_stack: Vec<HashMap<String, Type>>,
    functions: HashMap<String, (Vec<Type>, Type)>,
    structs: HashMap<String, Vec<(String, Type)>>,
    typedefs: HashMap<String, Type>,
    type_var_counter: usize,
    type_bindings: HashMap<usize, Type>,
}

impl TypeChecker {
    pub fn new() -> Self {
        Self {
            type_stack: vec![HashMap::new()],
            functions: HashMap::new(),
            structs: HashMap::new(),
            typedefs: HashMap::new(),
            type_var_counter: 0,
            type_bindings: HashMap::new(),
        }
    }

    fn new_type_var(&mut self) -> Type {
        let id = self.type_var_counter;
        self.type_var_counter += 1;
        Type::TypeVar(id)
    }

    fn resolve_type_var(&self, ty: &Type) -> Type {
        match ty {
            Type::TypeVar(id) => {
                if let Some(bound_type) = self.type_bindings.get(id) {
                    self.resolve_type_var(bound_type)
                } else {
                    ty.clone()
                }
            }
            Type::Array(inner, len) => Type::Array(Box::new(self.resolve_type_var(inner)), *len),
            Type::Function(params, ret) => Type::Function(
                params
                    .iter()
                    .map(|p| Box::new(self.resolve_type_var(p)))
                    .collect(),
                Box::new(self.resolve_type_var(ret)),
            ),
            _ => ty.clone(),
        }
    }

    fn bind_type_var(&mut self, var_id: usize, ty: &Type) {
        let resolved_ty = self.resolve_type_var(ty);
        self.type_bindings.insert(var_id, resolved_ty);
    }

    fn unify_types(&mut self, t1: &Type, t2: &Type) -> Result<(), CheckerError> {
        let t1 = self.resolve_type_var(t1);
        let t2 = self.resolve_type_var(t2);

        match (&t1, &t2) {
            (Type::TypeVar(id), _) => {
                self.bind_type_var(*id, &t2);
                Ok(())
            }
            (_, Type::TypeVar(id)) => {
                self.bind_type_var(*id, &t1);
                Ok(())
            }
            (Type::Named(n1), Type::Named(n2)) if n1 == n2 => Ok(()),
            (Type::Array(a1, len1), Type::Array(a2, len2)) => {
                if *len1 != 0 && *len2 != 0 && len1 != len2 {
                    return Err(CheckerError::TypeMismatch {
                        expected: t1.clone(),
                        found: t2.clone(),
                        context: "array length mismatch".to_string(),
                        span: Span::new(0, 0),
                    });
                }
                self.unify_types(a1, a2)
            }
            (Type::Pointer(p1), Type::Pointer(p2)) => self.unify_types(p1, p2),
            (Type::Function(p1, r1), Type::Function(p2, r2)) => {
                if p1.len() != p2.len() {
                    return Err(CheckerError::TypeMismatch {
                        expected: t1.clone(),
                        found: t2.clone(),
                        context: "function type unification".to_string(),
                        span: Span::new(0, 0),
                    });
                }
                for (param1, param2) in p1.iter().zip(p2.iter()) {
                    self.unify_types(param1, param2)?;
                }
                self.unify_types(r1, r2)
            }
            (Type::Named(n), _) if n == "T" => Ok(()),
            (_, Type::Named(n)) if n == "T" => Ok(()),
            _ => Err(CheckerError::TypeMismatch {
                expected: t1,
                found: t2,
                context: "type unification".to_string(),
                span: Span::new(0, 0),
            }),
        }
    }

    pub fn check(mut self, program: &mut Program) -> Result<(), CheckerError> {
        for expr in &program.body {
            match expr {
                Expr::FuncDecl(name, params, ret_type, _, _) => {
                    let param_types: Vec<Type> = params.iter().map(|(_, t)| t.clone()).collect();
                    self.functions
                        .insert(name.clone(), (param_types, ret_type.clone()));
                }
                Expr::Extern(name, params, ret_type, _) => {
                    let param_types: Vec<Type> = params.iter().map(|(_, t)| t.clone()).collect();
                    self.functions
                        .insert(name.clone(), (param_types, ret_type.clone()));
                }
                Expr::Struct(name, fields, _) => {
                    self.structs.insert(name.clone(), fields.clone());
                }
                _ => {}
            }
        }

        for expr in &mut program.body {
            self.check_expr(expr)?;
        }

        Ok(())
    }

    fn push_scope(&mut self) {
        self.type_stack.push(HashMap::new());
    }

    fn pop_scope(&mut self) {
        self.type_stack.pop();
    }

    fn declare_var(&mut self, name: &str, ty: Type) {
        self.type_stack
            .last_mut()
            .unwrap()
            .insert(name.to_string(), ty);
    }

    fn lookup_var(&self, name: &str) -> Option<Type> {
        for scope in self.type_stack.iter().rev() {
            if let Some(ty) = scope.get(name) {
                return Some(ty.clone());
            }
        }
        None
    }

    fn resolve_type(&self, ty: &Type) -> Type {
        match ty {
            Type::Named(name) => {
                if let Some(resolved) = self.typedefs.get(name) {
                    self.resolve_type(resolved)
                } else {
                    ty.clone()
                }
            }
            Type::Array(inner, len) => Type::Array(Box::new(self.resolve_type(inner)), *len),
            Type::Pointer(inner) => Type::Pointer(Box::new(self.resolve_type(inner))),
            Type::Function(params, ret) => Type::Function(
                params
                    .iter()
                    .map(|p| Box::new(self.resolve_type(p)))
                    .collect(),
                Box::new(self.resolve_type(ret)),
            ),
            Type::TypeVar(_) => self.resolve_type_var(ty),
            Type::Auto => Type::Auto,
            Type::Gen => Type::Gen,
        }
    }

    fn resolve_gen_types(&mut self, ty: &Type) -> Type {
        match ty {
            Type::Named(n) if n == "gen" => self.new_type_var(),
            Type::Array(inner, len) => Type::Array(Box::new(self.resolve_gen_types(inner)), *len),
            Type::Pointer(inner) => Type::Pointer(Box::new(self.resolve_gen_types(inner))),
            Type::Function(params, ret) => Type::Function(
                params
                    .iter()
                    .map(|p| Box::new(self.resolve_gen_types(p)))
                    .collect(),
                Box::new(self.resolve_gen_types(ret)),
            ),
            _ => ty.clone(),
        }
    }

    fn get_expr_type(&self, expr: &Expr) -> Type {
        match expr {
            Expr::Int(_, _) => Type::Named("int".to_string()),
            Expr::Float(_, _) => Type::Named("float".to_string()),
            Expr::Bool(_, _) => Type::Named("bool".to_string()),
            Expr::String(_, _) => Type::Named("string".to_string()),
            Expr::Nil(_) => Type::Named("void".to_string()),
            Expr::Var(name, _) => self
                .lookup_var(name)
                .unwrap_or(Type::Named("int".to_string())),
            Expr::FAdd(_, _, _)
            | Expr::FSub(_, _, _)
            | Expr::FMul(_, _, _)
            | Expr::FDiv(_, _, _) => Type::Named("float".to_string()),
            Expr::Add(_, _, _)
            | Expr::Sub(_, _, _)
            | Expr::Mul(_, _, _)
            | Expr::Div(_, _, _)
            | Expr::Mod(_, _, _)
            | Expr::Neg(_, _)
            | Expr::Xor(_, _, _)
            | Expr::Inc(_, _)
            | Expr::Dec(_, _)
            | Expr::AddAssign(_, _, _)
            | Expr::SubAssign(_, _, _) => Type::Named("int".to_string()),
            Expr::FNeg(_, _) => Type::Named("float".to_string()),
            Expr::FEq(_, _, _)
            | Expr::FNe(_, _, _)
            | Expr::FLt(_, _, _)
            | Expr::FLe(_, _, _)
            | Expr::FGt(_, _, _)
            | Expr::FGe(_, _, _) => Type::Named("bool".to_string()),
            Expr::Eq(_, _, _)
            | Expr::Ne(_, _, _)
            | Expr::Lt(_, _, _)
            | Expr::Le(_, _, _)
            | Expr::Gt(_, _, _)
            | Expr::Ge(_, _, _) => Type::Named("bool".to_string()),
            Expr::And(_, _, _) | Expr::Or(_, _, _) | Expr::LAnd(_, _, _) | Expr::LOr(_, _, _) | Expr::Not(_, _) => {
                Type::Named("bool".to_string())
            }
            Expr::StrCat(_, _, _) => Type::Named("string".to_string()),
            Expr::Call(callee, _, _) => {
                if let Expr::Var(name, _) = callee.as_ref() {
                    if let Some((_, ret_type)) = self.functions.get(name) {
                        return ret_type.clone();
                    }
                }
                Type::Named("int".to_string())
            }
            Expr::Index(_, _, _) => Type::Named("int".to_string()),
            Expr::MemberAccess(obj, field_name, _) => {
                let obj_type = self.get_expr_type(obj);
                let inner_type = match obj_type {
                    Type::Pointer(inner) => *inner,
                    Type::Named(_) => obj_type,
                    _ => return Type::Named("int".to_string()),
                };
                if let Type::Named(struct_name) = inner_type {
                    if let Some(fields) = self.structs.get(&struct_name) {
                        for (name, ty) in fields {
                            if name == field_name {
                                return ty.clone();
                            }
                        }
                    }
                }
                Type::Named("int".to_string())
            }
            Expr::StructLiteral(name, _, _) => Type::Named(name.clone()),
            Expr::ArrayLiteral(_, _) | Expr::ArrayFill(_, _, _) => {
                Type::Array(Box::new(Type::Named("int".to_string())), 0)
            }
            Expr::AddressOf(expr, _) => {
                let inner_type = self.get_expr_type(expr);
                Type::Pointer(Box::new(inner_type))
            }
            Expr::Deref(expr, _) => {
                let ptr_type = self.get_expr_type(expr);
                match ptr_type {
                    Type::Pointer(inner) => *inner,
                    _ => Type::Named("int".to_string()),
                }
            }
            Expr::DerefAssign(_, _, _) => Type::Named("void".to_string()),
            _ => Type::Named("int".to_string()),
        }
    }

    fn is_numeric_type(ty: &Type) -> bool {
        matches!(ty, Type::Named(n) if n == "int" || n == "float") || matches!(ty, Type::TypeVar(_))
    }

    fn is_float_type(ty: &Type) -> bool {
        matches!(ty, Type::Named(n) if n == "float")
    }

    fn is_string_type(ty: &Type) -> bool {
        matches!(ty, Type::Named(n) if n == "string")
    }

    fn is_bool_type(ty: &Type) -> bool {
        matches!(ty, Type::Named(n) if n == "bool")
    }

    fn check_expr(&mut self, expr: &mut Expr) -> Result<Type, CheckerError> {
        let span = expr.span();
        match expr {
            Expr::Int(_, _) => Ok(Type::Named("int".to_string())),
            Expr::Float(_, _) => Ok(Type::Named("float".to_string())),
            Expr::Bool(_, _) => Ok(Type::Named("bool".to_string())),
            Expr::String(_, _) => Ok(Type::Named("string".to_string())),
            Expr::Nil(_) => Ok(Type::Named("void".to_string())),

            Expr::Var(name, _) => {
                if let Some(ty) = self.lookup_var(name) {
                    let resolved_ty = self.resolve_type(&ty);

                    if matches!(resolved_ty, Type::Named(n) if n == "gen") {
                        let type_var = self.new_type_var();

                        return Ok(type_var);
                    }
                    return Ok(ty);
                }

                if let Some((params, ret_type)) = self.functions.get(name) {
                    let params_cloned = params.clone();
                    let ret_type_cloned = ret_type.clone();
                    let resolved_params: Vec<Type> = params_cloned
                        .iter()
                        .map(|t| {
                            if matches!(t, Type::Named(n) if n == "gen") {
                                self.new_type_var()
                            } else {
                                t.clone()
                            }
                        })
                        .collect();
                    let resolved_ret = if matches!(ret_type_cloned, Type::Named(ref n) if n == "gen")
                    {
                        self.new_type_var()
                    } else {
                        ret_type_cloned.clone()
                    };
                    return Ok(Type::Function(
                        resolved_params
                            .iter()
                            .map(|t| Box::new(t.clone()))
                            .collect(),
                        Box::new(resolved_ret),
                    ));
                }

                Err(CheckerError::UndefinedVariable(name.clone(), span))
            }

            Expr::VarDecl(name, ty, value, _) => {
                let resolved_ty = self.resolve_type(ty);
                let value_type = self.check_expr(value)?;

                let actual_ty = match &resolved_ty {
                    Type::Auto => self.new_type_var(),
                    Type::Gen => self.new_type_var(),
                    _ => resolved_ty.clone(),
                };

                if !self.types_compatible(&actual_ty, &value_type) {
                    return Err(CheckerError::TypeMismatch {
                        expected: actual_ty.clone(),
                        found: value_type,
                        context: format!("variable declaration '{}'", name),
                        span: span,
                    });
                }

                self.declare_var(name, actual_ty.clone());
                Ok(actual_ty)
            }

            Expr::VarAssign(name, value, _) => {
                let var_type = self
                    .lookup_var(name)
                    .ok_or_else(|| CheckerError::UndefinedVariable(name.clone(), span))?;
                let value_type = self.check_expr(value)?;

                self.unify_types(&var_type, &value_type).map_err(|_| {
                    CheckerError::TypeMismatch {
                        expected: var_type.clone(),
                        found: value_type,
                        context: format!("assignment to '{}'", name),
                        span: span,
                    }
                })?;

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

                if Self::is_string_type(&lhs_type) || Self::is_string_type(&rhs_type) {
                    if has_type_var {
                        self.unify_types(&lhs_type, &rhs_type)?;

                        let string_type = Type::Named("string".to_string());
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
                    } else if !Self::is_string_type(&lhs_type) || !Self::is_string_type(&rhs_type) {
                        return Err(CheckerError::TypeMismatch {
                            expected: Type::Named("string".to_string()),
                            found: if !Self::is_string_type(&lhs_type) {
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
                    return Ok(Type::Named("string".to_string()));
                }

                if Self::is_float_type(&lhs_type) || Self::is_float_type(&rhs_type) {
                    if !Self::is_numeric_type(&lhs_type) || !Self::is_numeric_type(&rhs_type) {
                        return Err(CheckerError::InvalidOperation {
                            op: "arithmetic".to_string(),
                            type_name: format!("{:?} and {:?}", lhs_type, rhs_type),
                            span: span,
                        });
                    }

                    if has_type_var {
                        self.unify_types(&lhs_type, &rhs_type)?;
                        let float_type = Type::Named("float".to_string());
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
                    return Ok(Type::Named("float".to_string()));
                }

                if !Self::is_numeric_type(&lhs_type) || !Self::is_numeric_type(&rhs_type) {
                    return Err(CheckerError::InvalidOperation {
                        op: "arithmetic".to_string(),
                        type_name: format!("{:?} and {:?}", lhs_type, rhs_type),
                        span: span,
                    });
                }

                if has_type_var {
                    self.unify_types(&lhs_type, &rhs_type)?;
                    let int_type = Type::Named("int".to_string());
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

                Ok(Type::Named("int".to_string()))
            }

            Expr::Mod(lhs, rhs, _) => {
                let lhs_type = self.check_expr(lhs)?;
                let rhs_type = self.check_expr(rhs)?;

                if Self::is_float_type(&lhs_type) || Self::is_float_type(&rhs_type) {
                    return Err(CheckerError::InvalidOperation {
                        op: "modulo".to_string(),
                        type_name: "float".to_string(),
                        span: span,
                    });
                }

                if !Self::is_numeric_type(&lhs_type) || !Self::is_numeric_type(&rhs_type) {
                    return Err(CheckerError::InvalidOperation {
                        op: "modulo".to_string(),
                        type_name: format!("{:?} and {:?}", lhs_type, rhs_type),
                        span: span,
                    });
                }

                Ok(Type::Named("int".to_string()))
            }

            Expr::Neg(operand, _) => {
                let ty = self.check_expr(operand)?;
                if Self::is_float_type(&ty) {
                    *expr = Expr::FNeg(
                        operand.clone(),
                        Span::new(0, 0),
                    );
                    return self.check_expr(expr);
                }
                if !Self::is_numeric_type(&ty) {
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
                if !Self::is_numeric_type(&ty) {
                    return Err(CheckerError::InvalidOperation {
                        op: "negation".to_string(),
                        type_name: format!("{:?}", ty),
                        span: span,
                    });
                }
                Ok(Type::Named("float".to_string()))
            }

            Expr::Xor(lhs, rhs, _) => {
                let lhs_type = self.check_expr(lhs)?;
                let rhs_type = self.check_expr(rhs)?;
                if Self::is_float_type(&lhs_type) || Self::is_float_type(&rhs_type) {
                    return Err(CheckerError::InvalidOperation {
                        op: "xor".to_string(),
                        type_name: "float".to_string(),
                        span: span,
                    });
                }
                if !Self::is_numeric_type(&lhs_type) || !Self::is_numeric_type(&rhs_type) {
                    return Err(CheckerError::InvalidOperation {
                        op: "xor".to_string(),
                        type_name: format!("{:?} and {:?}", lhs_type, rhs_type),
                        span: span,
                    });
                }
                Ok(Type::Named("int".to_string()))
            }

            Expr::Inc(name, _) | Expr::Dec(name, _) => {
                let var_type = self
                    .lookup_var(name)
                    .ok_or_else(|| CheckerError::UndefinedVariable(name.clone(), span))?;
                if !Self::is_numeric_type(&var_type) {
                    return Err(CheckerError::InvalidOperation {
                        op: "increment/decrement".to_string(),
                        type_name: format!("{:?}", var_type),
                        span: span,
                    });
                }
                Ok(var_type)
            }

            Expr::AddAssign(name, value, _) | Expr::SubAssign(name, value, _) => {
                let var_type = self
                    .lookup_var(name)
                    .ok_or_else(|| CheckerError::UndefinedVariable(name.clone(), span))?;
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

            Expr::LAnd(lhs, rhs, _) | Expr::LOr(lhs, rhs, _) => {
                let lhs_type = self.check_expr(lhs)?;
                let rhs_type = self.check_expr(rhs)?;
                if !Self::is_bool_type(&lhs_type) || !Self::is_bool_type(&rhs_type) {
                    return Err(CheckerError::InvalidOperation {
                        op: "logical".to_string(),
                        type_name: format!("{:?} and {:?}", lhs_type, rhs_type),
                        span: span,
                    });
                }
                Ok(Type::Named("bool".to_string()))
            }

            Expr::Eq(lhs, rhs, _)
            | Expr::Ne(lhs, rhs, _)
            | Expr::Lt(lhs, rhs, _)
            | Expr::Le(lhs, rhs, _)
            | Expr::Gt(lhs, rhs, _)
            | Expr::Ge(lhs, rhs, _) => {
                let lhs_type = self.check_expr(lhs)?;
                let rhs_type = self.check_expr(rhs)?;

                if Self::is_float_type(&lhs_type) || Self::is_float_type(&rhs_type) {
                    if !Self::is_numeric_type(&lhs_type) || !Self::is_numeric_type(&rhs_type) {
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
                } else if !Self::is_numeric_type(&lhs_type) || !Self::is_numeric_type(&rhs_type) {
                    return Err(CheckerError::InvalidOperation {
                        op: "comparison".to_string(),
                        type_name: format!("{:?} and {:?}", lhs_type, rhs_type),
                        span: span,
                    });
                }

                Ok(Type::Named("bool".to_string()))
            }

            Expr::And(lhs, rhs, _) | Expr::Or(lhs, rhs, _) => {
                let lhs_type = self.check_expr(lhs)?;
                let rhs_type = self.check_expr(rhs)?;

                if !Self::is_bool_type(&lhs_type) || !Self::is_bool_type(&rhs_type) {
                    return Err(CheckerError::InvalidOperation {
                        op: "logical".to_string(),
                        type_name: format!("{:?} and {:?}", lhs_type, rhs_type),
                        span: span,
                    });
                }

                Ok(Type::Named("bool".to_string()))
            }

            Expr::Not(e, _) => {
                let ty = self.check_expr(e)?;
                if !Self::is_bool_type(&ty) {
                    return Err(CheckerError::InvalidOperation {
                        op: "not".to_string(),
                        type_name: format!("{:?}", ty),
                        span: span,
                    });
                }
                Ok(Type::Named("bool".to_string()))
            }

            Expr::Call(callee, args, _) => {
                let callee_type = self.check_expr(callee)?;
                let arg_types: Result<Vec<Type>, CheckerError> =
                    args.iter_mut().map(|arg| self.check_expr(arg)).collect();
                let arg_types = arg_types?;

                match &callee_type {
                    Type::Named(n) if n == "gen" => {
                        let ret_type_var = self.new_type_var();
                        Ok(ret_type_var)
                    }
                    Type::TypeVar(_) => {
                        let inferred_params: Vec<Type> = arg_types.clone();
                        let inferred_ret = self.new_type_var();

                        let inferred_func_type = Type::Function(
                            inferred_params
                                .iter()
                                .map(|t| Box::new(t.clone()))
                                .collect(),
                            Box::new(inferred_ret.clone()),
                        );
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
                            if !self.types_compatible(expected_ty, arg_type) {
                                return Err(CheckerError::TypeMismatch {
                                    expected: *expected_ty.clone(),
                                    found: arg_type.clone(),
                                    context: format!("argument {} of function pointer call", i + 1),
                                    span: span,
                                });
                            }
                        }

                        Ok(*ret_type.clone())
                    }
                    _ => Err(CheckerError::TypeMismatch {
                        expected: Type::Function(vec![], Box::new(Type::Named("void".to_string()))),
                        found: callee_type,
                        context: "callee is not a function type".to_string(),
                        span: span,
                    }),
                }
            }

            Expr::Return(value, _) => self.check_expr(value),

            Expr::If(cond, then_branch, else_branch, _) => {
                let cond_type = self.check_expr(cond)?;
                if !Self::is_bool_type(&cond_type) {
                    return Err(CheckerError::TypeMismatch {
                        expected: Type::Named("bool".to_string()),
                        found: cond_type,
                        context: "if condition".to_string(),
                        span: span,
                    });
                }

                self.push_scope();
                self.check_expr(then_branch)?;
                self.pop_scope();

                if let Some(else_expr) = else_branch {
                    self.push_scope();
                    self.check_expr(else_expr)?;
                    self.pop_scope();
                }

                Ok(Type::Named("void".to_string()))
            }

            Expr::While(cond, body, _) => {
                let cond_type = self.check_expr(cond)?;
                if !Self::is_bool_type(&cond_type) {
                    return Err(CheckerError::TypeMismatch {
                        expected: Type::Named("bool".to_string()),
                        found: cond_type,
                        context: "while condition".to_string(),
                        span: span,
                    });
                }

                self.push_scope();
                self.check_expr(body)?;
                self.pop_scope();

                Ok(Type::Named("void".to_string()))
            }

            Expr::For(var, array, body, _) => {
                let array_type = self.check_expr(array)?;

                let elem_type = match &array_type {
                    Type::Array(inner, _) => *inner.clone(),
                    Type::Named(n) if n == "string" => Type::Named("int".to_string()),
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

                Ok(Type::Named("void".to_string()))
            }

            Expr::Block(body, _) => {
                self.push_scope();
                for e in body {
                    self.check_expr(e)?;
                }
                self.pop_scope();
                Ok(Type::Named("void".to_string()))
            }

            Expr::Index(array, idx, _) => {
                let array_type = self.check_expr(array)?;
                let idx_type = self.check_expr(idx)?;

                if !Self::is_numeric_type(&idx_type) {
                    return Err(CheckerError::InvalidOperation {
                        op: "array index".to_string(),
                        type_name: format!("{:?}", idx_type),
                        span: span,
                    });
                }

                match array_type {
                    Type::Array(inner, _) => Ok(*inner),
                    Type::Named(n) if n == "string" => Ok(Type::Named("int".to_string())),
                    _ => Err(CheckerError::InvalidOperation {
                        op: "index".to_string(),
                        type_name: format!("{:?}", array_type),
                        span: span,
                    }),
                }
            }

            Expr::IndexAssign(array_idx, value, _) => {
                let value_type = self.check_expr(value)?;

                if let Expr::Index(array, idx, _) = array_idx.as_mut() {
                    let array_type = self.get_expr_type(array);
                    let idx_type = self.check_expr(idx)?;

                    if !Self::is_numeric_type(&idx_type) {
                        return Err(CheckerError::InvalidOperation {
                            op: "array index".to_string(),
                            type_name: format!("{:?}", idx_type),
                            span: span,
                        });
                    }

                    match array_type {
                        Type::Array(inner, _) => {
                            if !self.types_compatible(&inner, &value_type) {
                                return Err(CheckerError::TypeMismatch {
                                    expected: *inner,
                                    found: value_type,
                                    context: "array assignment".to_string(),
                                    span: span,
                                });
                            }
                        }
                        Type::Named(n) if n == "string" => {
                            if !self.types_compatible(&Type::Named("int".to_string()), &value_type)
                            {
                                return Err(CheckerError::TypeMismatch {
                                    expected: Type::Named("int".to_string()),
                                    found: value_type,
                                    context: "string assignment".to_string(),
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

                    Ok(Type::Named("void".to_string()))
                } else {
                    Err(CheckerError::InvalidOperation {
                        op: "index assignment".to_string(),
                        type_name: "non-index expression".to_string(),
                        span: span,
                    })
                }
            }

            Expr::ArrayLiteral(elements, _) => {
                let len = elements.len();
                let mut elem_type = Type::Named("int".to_string());
                for e in elements {
                    elem_type = self.check_expr(e)?;
                }
                Ok(Type::Array(Box::new(elem_type), len))
            }

            Expr::ArrayFill(elem_type, len, _) => {
                let len_type = self.check_expr(len)?;
                if !Self::is_numeric_type(&len_type) {
                    return Err(CheckerError::InvalidOperation {
                        op: "array fill".to_string(),
                        type_name: format!("{:?}", len_type),
                        span: span,
                    });
                }

                let len = if let Expr::Int(n, _) = len.as_ref() {
                    *n as usize
                } else {
                    0
                };
                Ok(Type::Array(Box::new(elem_type.clone()), len))
            }

            Expr::Range(start, end, _) => {
                let start_type = self.check_expr(start)?;
                let end_type = self.check_expr(end)?;

                if !Self::is_numeric_type(&start_type) || !Self::is_numeric_type(&end_type) {
                    return Err(CheckerError::InvalidOperation {
                        op: "range expression".to_string(),
                        type_name: format!("{:?} and {:?}", start_type, end_type),
                        span: span,
                    });
                }

                let len = if let (Expr::Int(s, _), Expr::Int(e, _)) = (start.as_ref(), end.as_ref())
                {
                    if *e > *s { (*e - *s) as usize } else { 0 }
                } else {
                    0
                };

                Ok(Type::Array(Box::new(Type::Named("int".to_string())), len))
            }

            Expr::FuncDecl(_name, params, _ret_type, body, _) => {
                self.push_scope();
                for (param_name, param_type) in params {
                    let actual_param_type = if matches!(param_type, Type::Named(n) if n == "gen") {
                        self.new_type_var()
                    } else {
                        param_type.clone()
                    };
                    self.declare_var(param_name, actual_param_type);
                }
                self.check_expr(body)?;
                self.pop_scope();

                Ok(Type::Named("void".to_string()))
            }

            Expr::Extern(_, _, _, _) => Ok(Type::Named("void".to_string())),

            Expr::Break(_) | Expr::Continue(_) => Ok(Type::Named("void".to_string())),

            Expr::TypeDef(_) => Ok(Type::Named("void".to_string())),

            Expr::Struct(_name, fields, _) => {
                for (_, field_ty) in fields {
                    self.validate_type(field_ty)?;
                }
                Ok(Type::Named("void".to_string()))
            }

            Expr::StructLiteral(name, field_values, _) => {
                let fields = self
                    .structs
                    .get(name)
                    .ok_or_else(|| CheckerError::UndefinedStruct(name.clone(), span))?
                    .clone();

                for (field_name, expected_ty) in &fields {
                    let mut found_idx = None;
                    for (idx, (n, _)) in field_values.iter().enumerate() {
                        if n == field_name {
                            found_idx = Some(idx);
                            break;
                        }
                    }
                    if let Some(idx) = found_idx {
                        let expr_type = self.check_expr(&mut field_values[idx].1)?;
                        if !self.types_compatible(expected_ty, &expr_type) {
                            return Err(CheckerError::TypeMismatch {
                                expected: expected_ty.clone(),
                                found: expr_type,
                                context: format!("struct '{}' field '{}'", name, field_name),
                                span: span,
                            });
                        }
                    }
                }

                Ok(Type::Named(name.clone()))
            }

            Expr::MemberAccess(obj, field_name, _) => {
                let obj_type = self.check_expr(obj)?;
                let struct_name = match &obj_type {
                    Type::Named(name) => name.clone(),
                    Type::Pointer(inner) => {
                        if let Type::Named(ref name) = **inner {
                            name.clone()
                        } else {
                            return Err(CheckerError::NonStructMemberAccess(
                                format!("{:?}", obj_type),
                                span,
                            ));
                        }
                    }
                    _ => {
                        return Err(CheckerError::NonStructMemberAccess(
                            format!("{:?}", obj_type),
                            span,
                        ));
                    }
                };
                let fields = self
                    .structs
                    .get(&struct_name)
                    .ok_or_else(|| CheckerError::UndefinedStruct(struct_name.clone(), span))?
                    .clone();

                for (name, ty) in &fields {
                    if name == field_name {
                        let resolved_ty = self.resolve_gen_types(ty);
                        return Ok(resolved_ty);
                    }
                }

                Err(CheckerError::UndefinedField {
                    struct_name,
                    field: field_name.clone(),
                    span: span,
                })
            }

            Expr::MemberAssign(obj, field_name, value, _) => {
                let obj_type = self.check_expr(obj)?;
                let value_type = self.check_expr(value)?;

                match &obj_type {
                    Type::Named(struct_name) => {
                        let struct_def = self.structs.get(struct_name).ok_or_else(|| {
                            CheckerError::UndefinedStruct(struct_name.clone(), span)
                        })?;

                        for (name, ty) in struct_def {
                            if name == field_name {
                                if !self.types_compatible(ty, &value_type) {
                                    return Err(CheckerError::TypeMismatch {
                                        expected: ty.clone(),
                                        found: value_type,
                                        context: format!(
                                            "struct '{}' field '{}' assignment",
                                            struct_name, field_name
                                        ),
                                        span: span,
                                    });
                                }
                                return Ok(Type::Named("void".to_string()));
                            }
                        }

                        Err(CheckerError::UndefinedField {
                            struct_name: struct_name.clone(),
                            field: field_name.clone(),
                            span: span,
                        })
                    }
                    Type::Pointer(inner) => {
                        if let Type::Named(struct_name) = &**inner {
                            let struct_def = self.structs.get(struct_name).ok_or_else(|| {
                                CheckerError::UndefinedStruct(struct_name.clone(), span)
                            })?;

                            for (name, ty) in struct_def {
                                if name == field_name {
                                    if !self.types_compatible(ty, &value_type) {
                                        return Err(CheckerError::TypeMismatch {
                                            expected: ty.clone(),
                                            found: value_type,
                                            context: format!(
                                                "struct '{}' field '{}' assignment",
                                                struct_name, field_name
                                            ),
                                            span: span,
                                        });
                                    }
                                    return Ok(Type::Named("void".to_string()));
                                }
                            }

                            Err(CheckerError::UndefinedField {
                                struct_name: struct_name.clone(),
                                field: field_name.clone(),
                                span: span,
                            })
                        } else {
                            Err(CheckerError::NonStructMemberAccess(
                                format!("{:?}", obj_type),
                                span,
                            ))
                        }
                    }
                    _ => Err(CheckerError::NonStructMemberAccess(
                        format!("{:?}", obj_type),
                        span,
                    )),
                }
            }

            Expr::FAdd(lhs, rhs, _)
            | Expr::FSub(lhs, rhs, _)
            | Expr::FMul(lhs, rhs, _)
            | Expr::FDiv(lhs, rhs, _) => {
                self.check_expr(lhs)?;
                self.check_expr(rhs)?;
                Ok(Type::Named("float".to_string()))
            }
            Expr::FEq(lhs, rhs, _)
            | Expr::FNe(lhs, rhs, _)
            | Expr::FLt(lhs, rhs, _)
            | Expr::FLe(lhs, rhs, _)
            | Expr::FGt(lhs, rhs, _)
            | Expr::FGe(lhs, rhs, _) => {
                self.check_expr(lhs)?;
                self.check_expr(rhs)?;
                Ok(Type::Named("bool".to_string()))
            }
            Expr::StrCat(lhs, rhs, _) => {
                self.check_expr(lhs)?;
                self.check_expr(rhs)?;
                Ok(Type::Named("string".to_string()))
            }
            Expr::Lambda(params, body, ret_type, _) => {
                self.push_scope();
                for (param_name, param_type) in params.iter() {
                    self.declare_var(param_name, param_type.clone());
                }
                self.check_expr(body)?;
                self.pop_scope();
                let param_types: Vec<Type> = params.iter().map(|(_, t)| t.clone()).collect();
                Ok(Type::Function(
                    param_types.iter().map(|t| Box::new(t.clone())).collect(),
                    Box::new(ret_type.clone()),
                ))
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
                        Ok(Type::Named("void".to_string()))
                    }
                    _ => Err(CheckerError::InvalidOperation {
                        op: "dereference assignment".to_string(),
                        type_name: format!("{:?}", ptr_type),
                        span: span,
                    }),
                }
            }
        }
    }

    fn types_compatible(&self, expected: &Type, found: &Type) -> bool {
        let expected = self.resolve_type(expected);
        let found = self.resolve_type(found);

        match (&expected, &found) {
            (Type::TypeVar(_), _) => true,
            (_, Type::TypeVar(_)) => true,
            (Type::Named(n), _) if n == "gen" => true,
            (_, Type::Named(n)) if n == "gen" => true,
            (Type::Named(n), _) if n == "void" => true,
            (_, Type::Named(n)) if n == "void" => true,
            (Type::Named(a), Type::Named(b)) => a == b,
            (Type::Array(a, len1), Type::Array(b, len2)) => {
                let len_compatible = *len1 == 0 || *len2 == 0 || len1 == len2;
                len_compatible && self.types_compatible(a, b)
            }

            (Type::Pointer(a), Type::Array(b, _)) => self.types_compatible(a, b),
            (Type::Pointer(a), Type::Pointer(b)) => self.types_compatible(a, b),
            (Type::Function(exp_params, exp_ret), Type::Function(found_params, found_ret)) => {
                if exp_params.len() != found_params.len() {
                    return false;
                }
                for (exp_p, found_p) in exp_params.iter().zip(found_params.iter()) {
                    if !self.types_compatible(exp_p, found_p) {
                        return false;
                    }
                }

                self.types_compatible(exp_ret, found_ret)
            }
            _ => false,
        }
    }

    fn validate_type(&self, ty: &Type) -> Result<(), CheckerError> {
        match ty {
            Type::Named(_) => Ok(()),
            Type::TypeVar(_) => Ok(()),
            Type::Array(inner, _) => self.validate_type(inner),
            Type::Pointer(inner) => self.validate_type(inner),
            Type::Function(params, ret) => {
                for param in params {
                    self.validate_type(param)?;
                }
                self.validate_type(ret)
            }
            Type::Auto => Ok(()),
            Type::Gen => Ok(()),
        }
    }
}
