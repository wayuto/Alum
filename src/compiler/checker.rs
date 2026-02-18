use crate::compiler::ast::{Expr, Program, Type};
use std::collections::HashMap;

#[derive(Debug)]
pub enum CheckerError {
    TypeMismatch {
        expected: Type,
        found: Type,
        context: String,
    },
    UndefinedVariable(String),
    UndefinedFunction(String),
    UndefinedStruct(String),
    UndefinedField {
        struct_name: String,
        field: String,
    },
    ArgCountMismatch {
        expected: usize,
        found: usize,
        func: String,
    },
    NonStructMemberAccess(String),
    InvalidOperation {
        op: String,
        type_name: String,
    },
}

impl std::fmt::Display for CheckerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CheckerError::TypeMismatch {
                expected,
                found,
                context,
            } => {
                write!(
                    f,
                    "Type mismatch in {}: expected {:?}, found {:?}",
                    context, expected, found
                )
            }
            CheckerError::UndefinedVariable(name) => {
                write!(f, "Undefined variable: {}", name)
            }
            CheckerError::UndefinedFunction(name) => {
                write!(f, "Undefined function: {}", name)
            }
            CheckerError::UndefinedStruct(name) => {
                write!(f, "Undefined struct: {}", name)
            }
            CheckerError::UndefinedField { struct_name, field } => {
                write!(f, "Struct {} has no field {}", struct_name, field)
            }
            CheckerError::ArgCountMismatch {
                expected,
                found,
                func,
            } => {
                write!(
                    f,
                    "Function {} expects {} arguments, found {}",
                    func, expected, found
                )
            }
            CheckerError::NonStructMemberAccess(type_name) => {
                write!(f, "Cannot access member on non-struct type: {}", type_name)
            }
            CheckerError::InvalidOperation { op, type_name } => {
                write!(f, "Invalid operation '{}' on type {}", op, type_name)
            }
        }
    }
}

impl std::error::Error for CheckerError {}

#[derive(Debug, Clone)]
struct StructDef {
    fields: Vec<(String, Type)>,
}

pub struct TypeChecker {
    type_stack: Vec<HashMap<String, Type>>,

    functions: HashMap<String, (Vec<Type>, Type)>,

    structs: HashMap<String, StructDef>,

    typedefs: HashMap<String, Type>,
}

impl TypeChecker {
    pub fn new() -> Self {
        Self {
            type_stack: vec![HashMap::new()],
            functions: HashMap::new(),
            structs: HashMap::new(),
            typedefs: HashMap::new(),
        }
    }

    pub fn check(mut self, program: &mut Program) -> Result<(), CheckerError> {
        for expr in &program.body {
            match expr {
                Expr::FuncDecl(name, params, ret_type, _) => {
                    let param_types: Vec<Type> = params.iter().map(|(_, t)| t.clone()).collect();
                    self.functions
                        .insert(name.clone(), (param_types, ret_type.clone()));
                }
                Expr::Extern(name, params, ret_type) => {
                    let param_types: Vec<Type> = params.iter().map(|(_, t)| t.clone()).collect();
                    self.functions
                        .insert(name.clone(), (param_types, ret_type.clone()));
                }
                Expr::Struct(name, fields) => {
                    self.structs.insert(
                        name.clone(),
                        StructDef {
                            fields: fields.clone(),
                        },
                    );
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
            Type::Array(inner) => Type::Array(Box::new(self.resolve_type(inner))),
        }
    }

    fn get_expr_type(&self, expr: &Expr) -> Type {
        match expr {
            Expr::Int(_) => Type::Named("int".to_string()),
            Expr::Float(_) => Type::Named("float".to_string()),
            Expr::Bool(_) => Type::Named("bool".to_string()),
            Expr::String(_) => Type::Named("string".to_string()),
            Expr::Nil => Type::Named("void".to_string()),
            Expr::Var(name) => self
                .lookup_var(name)
                .unwrap_or(Type::Named("int".to_string())),
            Expr::FAdd(_, _) | Expr::FSub(_, _) | Expr::FMul(_, _) | Expr::FDiv(_, _) => {
                Type::Named("float".to_string())
            }
            Expr::Add(_, _)
            | Expr::Sub(_, _)
            | Expr::Mul(_, _)
            | Expr::Div(_, _)
            | Expr::Mod(_, _) => Type::Named("int".to_string()),
            Expr::FEq(_, _)
            | Expr::FNe(_, _)
            | Expr::FLt(_, _)
            | Expr::FLe(_, _)
            | Expr::FGt(_, _)
            | Expr::FGe(_, _) => Type::Named("bool".to_string()),
            Expr::Eq(_, _)
            | Expr::Ne(_, _)
            | Expr::Lt(_, _)
            | Expr::Le(_, _)
            | Expr::Gt(_, _)
            | Expr::Ge(_, _) => Type::Named("bool".to_string()),
            Expr::And(_, _) | Expr::Or(_, _) | Expr::Not(_) => Type::Named("bool".to_string()),
            Expr::StrConcat(_, _) => Type::Named("string".to_string()),
            Expr::Call(callee, _) => {
                if let Expr::Var(name) = callee.as_ref() {
                    if let Some((_, ret_type)) = self.functions.get(name) {
                        return ret_type.clone();
                    }
                }
                Type::Named("int".to_string())
            }
            Expr::Index(_, _) => Type::Named("int".to_string()),
            Expr::MemberAccess(obj, field_name) => {
                let obj_type = self.get_expr_type(obj);
                if let Type::Named(struct_name) = obj_type {
                    if let Some(struct_def) = self.structs.get(&struct_name) {
                        for (name, ty) in &struct_def.fields {
                            if name == field_name {
                                return ty.clone();
                            }
                        }
                    }
                }
                Type::Named("int".to_string())
            }
            Expr::StructLiteral(name, _) => Type::Named(name.clone()),
            Expr::ArrayLiteral(_) | Expr::ArrayFill(_, _) => {
                Type::Array(Box::new(Type::Named("int".to_string())))
            }
            _ => Type::Named("int".to_string()),
        }
    }

    fn is_numeric_type(ty: &Type) -> bool {
        matches!(ty, Type::Named(n) if n == "int" || n == "float")
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
        match expr {
            Expr::Int(_) => Ok(Type::Named("int".to_string())),
            Expr::Float(_) => Ok(Type::Named("float".to_string())),
            Expr::Bool(_) => Ok(Type::Named("bool".to_string())),
            Expr::String(_) => Ok(Type::Named("string".to_string())),
            Expr::Nil => Ok(Type::Named("void".to_string())),

            Expr::Var(name) => self
                .lookup_var(name)
                .ok_or_else(|| CheckerError::UndefinedVariable(name.clone())),

            Expr::VarDecl(name, ty, value) => {
                let ty = self.resolve_type(ty);
                let value_type = self.check_expr(value)?;

                if !self.types_compatible(&ty, &value_type) {
                    return Err(CheckerError::TypeMismatch {
                        expected: ty.clone(),
                        found: value_type,
                        context: format!("variable declaration '{}'", name),
                    });
                }

                self.declare_var(name, ty.clone());
                Ok(ty)
            }

            Expr::VarAssign(name, value) => {
                let var_type = self
                    .lookup_var(name)
                    .ok_or_else(|| CheckerError::UndefinedVariable(name.clone()))?;
                let value_type = self.check_expr(value)?;

                if !self.types_compatible(&var_type, &value_type) {
                    return Err(CheckerError::TypeMismatch {
                        expected: var_type.clone(),
                        found: value_type,
                        context: format!("assignment to '{}'", name),
                    });
                }

                Ok(var_type)
            }

            Expr::Add(lhs, rhs)
            | Expr::Sub(lhs, rhs)
            | Expr::Mul(lhs, rhs)
            | Expr::Div(lhs, rhs) => {
                let lhs_type = self.check_expr(lhs)?;
                let rhs_type = self.check_expr(rhs)?;

                if Self::is_string_type(&lhs_type) || Self::is_string_type(&rhs_type) {
                    if !Self::is_string_type(&lhs_type) || !Self::is_string_type(&rhs_type) {
                        return Err(CheckerError::TypeMismatch {
                            expected: Type::Named("string".to_string()),
                            found: if !Self::is_string_type(&lhs_type) {
                                lhs_type
                            } else {
                                rhs_type
                            },
                            context: "string concatenation".to_string(),
                        });
                    }

                    let (l, r) = match expr {
                        Expr::Add(l, r) => (l.clone(), r.clone()),
                        Expr::Sub(_, _) => unreachable!(),
                        Expr::Mul(_, _) => unreachable!(),
                        Expr::Div(_, _) => unreachable!(),
                        _ => unreachable!(),
                    };
                    *expr = Expr::StrConcat(l, r);
                    return Ok(Type::Named("string".to_string()));
                }

                if Self::is_float_type(&lhs_type) || Self::is_float_type(&rhs_type) {
                    if !Self::is_numeric_type(&lhs_type) || !Self::is_numeric_type(&rhs_type) {
                        return Err(CheckerError::InvalidOperation {
                            op: "arithmetic".to_string(),
                            type_name: format!("{:?} and {:?}", lhs_type, rhs_type),
                        });
                    }

                    let (l, r) = match expr {
                        Expr::Add(l, r) => (l.clone(), r.clone()),
                        Expr::Sub(l, r) => (l.clone(), r.clone()),
                        Expr::Mul(l, r) => (l.clone(), r.clone()),
                        Expr::Div(l, r) => (l.clone(), r.clone()),
                        _ => unreachable!(),
                    };
                    *expr = match expr {
                        Expr::Add(_, _) => Expr::FAdd(l, r),
                        Expr::Sub(_, _) => Expr::FSub(l, r),
                        Expr::Mul(_, _) => Expr::FMul(l, r),
                        Expr::Div(_, _) => Expr::FDiv(l, r),
                        _ => unreachable!(),
                    };
                    return Ok(Type::Named("float".to_string()));
                }

                if !Self::is_numeric_type(&lhs_type) || !Self::is_numeric_type(&rhs_type) {
                    return Err(CheckerError::InvalidOperation {
                        op: "arithmetic".to_string(),
                        type_name: format!("{:?} and {:?}", lhs_type, rhs_type),
                    });
                }

                Ok(Type::Named("int".to_string()))
            }

            Expr::Mod(lhs, rhs) => {
                let lhs_type = self.check_expr(lhs)?;
                let rhs_type = self.check_expr(rhs)?;

                if Self::is_float_type(&lhs_type) || Self::is_float_type(&rhs_type) {
                    return Err(CheckerError::InvalidOperation {
                        op: "modulo".to_string(),
                        type_name: "float".to_string(),
                    });
                }

                if !Self::is_numeric_type(&lhs_type) || !Self::is_numeric_type(&rhs_type) {
                    return Err(CheckerError::InvalidOperation {
                        op: "modulo".to_string(),
                        type_name: format!("{:?} and {:?}", lhs_type, rhs_type),
                    });
                }

                Ok(Type::Named("int".to_string()))
            }

            Expr::Eq(lhs, rhs)
            | Expr::Ne(lhs, rhs)
            | Expr::Lt(lhs, rhs)
            | Expr::Le(lhs, rhs)
            | Expr::Gt(lhs, rhs)
            | Expr::Ge(lhs, rhs) => {
                let lhs_type = self.check_expr(lhs)?;
                let rhs_type = self.check_expr(rhs)?;

                if Self::is_float_type(&lhs_type) || Self::is_float_type(&rhs_type) {
                    if !Self::is_numeric_type(&lhs_type) || !Self::is_numeric_type(&rhs_type) {
                        return Err(CheckerError::InvalidOperation {
                            op: "comparison".to_string(),
                            type_name: format!("{:?} and {:?}", lhs_type, rhs_type),
                        });
                    }

                    let (l, r) = match expr {
                        Expr::Eq(l, r) => (l.clone(), r.clone()),
                        Expr::Ne(l, r) => (l.clone(), r.clone()),
                        Expr::Lt(l, r) => (l.clone(), r.clone()),
                        Expr::Le(l, r) => (l.clone(), r.clone()),
                        Expr::Gt(l, r) => (l.clone(), r.clone()),
                        Expr::Ge(l, r) => (l.clone(), r.clone()),
                        _ => unreachable!(),
                    };
                    *expr = match expr {
                        Expr::Eq(_, _) => Expr::FEq(l, r),
                        Expr::Ne(_, _) => Expr::FNe(l, r),
                        Expr::Lt(_, _) => Expr::FLt(l, r),
                        Expr::Le(_, _) => Expr::FLe(l, r),
                        Expr::Gt(_, _) => Expr::FGt(l, r),
                        Expr::Ge(_, _) => Expr::FGe(l, r),
                        _ => unreachable!(),
                    };
                } else if !Self::is_numeric_type(&lhs_type) || !Self::is_numeric_type(&rhs_type) {
                    return Err(CheckerError::InvalidOperation {
                        op: "comparison".to_string(),
                        type_name: format!("{:?} and {:?}", lhs_type, rhs_type),
                    });
                }

                Ok(Type::Named("bool".to_string()))
            }

            Expr::And(lhs, rhs) | Expr::Or(lhs, rhs) => {
                let lhs_type = self.check_expr(lhs)?;
                let rhs_type = self.check_expr(rhs)?;

                if !Self::is_bool_type(&lhs_type) || !Self::is_bool_type(&rhs_type) {
                    return Err(CheckerError::InvalidOperation {
                        op: "logical".to_string(),
                        type_name: format!("{:?} and {:?}", lhs_type, rhs_type),
                    });
                }

                Ok(Type::Named("bool".to_string()))
            }

            Expr::Not(e) => {
                let ty = self.check_expr(e)?;
                if !Self::is_bool_type(&ty) {
                    return Err(CheckerError::InvalidOperation {
                        op: "not".to_string(),
                        type_name: format!("{:?}", ty),
                    });
                }
                Ok(Type::Named("bool".to_string()))
            }

            Expr::Call(callee, args) => {
                let func_name = match callee.as_ref() {
                    Expr::Var(name) => name.clone(),
                    _ => return Ok(Type::Named("int".to_string())),
                };

                let func_info = self
                    .functions
                    .get(&func_name)
                    .ok_or_else(|| CheckerError::UndefinedFunction(func_name.clone()))?
                    .clone();

                if args.len() != func_info.0.len() {
                    return Err(CheckerError::ArgCountMismatch {
                        expected: func_info.0.len(),
                        found: args.len(),
                        func: func_name,
                    });
                }

                for (i, (arg, expected_ty)) in args.iter_mut().zip(func_info.0.iter()).enumerate() {
                    let arg_type = self.check_expr(arg)?;
                    if !self.types_compatible(expected_ty, &arg_type) {
                        return Err(CheckerError::TypeMismatch {
                            expected: expected_ty.clone(),
                            found: arg_type,
                            context: format!("argument {} of function '{}'", i + 1, func_name),
                        });
                    }
                }

                Ok(func_info.1)
            }

            Expr::Return(value) => self.check_expr(value),

            Expr::If(cond, then_branch, else_branch) => {
                let cond_type = self.check_expr(cond)?;
                if !Self::is_bool_type(&cond_type) {
                    return Err(CheckerError::TypeMismatch {
                        expected: Type::Named("bool".to_string()),
                        found: cond_type,
                        context: "if condition".to_string(),
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

            Expr::While(cond, body) => {
                let cond_type = self.check_expr(cond)?;
                if !Self::is_bool_type(&cond_type) {
                    return Err(CheckerError::TypeMismatch {
                        expected: Type::Named("bool".to_string()),
                        found: cond_type,
                        context: "while condition".to_string(),
                    });
                }

                self.push_scope();
                self.check_expr(body)?;
                self.pop_scope();

                Ok(Type::Named("void".to_string()))
            }

            Expr::For(var, start, end, body) => {
                let start_type = self.check_expr(start)?;
                let end_type = self.check_expr(end)?;

                if !Self::is_numeric_type(&start_type) || !Self::is_numeric_type(&end_type) {
                    return Err(CheckerError::InvalidOperation {
                        op: "for loop range".to_string(),
                        type_name: format!("{:?} and {:?}", start_type, end_type),
                    });
                }

                self.push_scope();
                self.declare_var(var, Type::Named("int".to_string()));
                self.check_expr(body)?;
                self.pop_scope();

                Ok(Type::Named("void".to_string()))
            }

            Expr::Stmt(body) => {
                self.push_scope();
                for e in body {
                    self.check_expr(e)?;
                }
                self.pop_scope();
                Ok(Type::Named("void".to_string()))
            }

            Expr::Index(array, index) => {
                let array_type = self.check_expr(array)?;
                let index_type = self.check_expr(index)?;

                if !Self::is_numeric_type(&index_type) {
                    return Err(CheckerError::InvalidOperation {
                        op: "array index".to_string(),
                        type_name: format!("{:?}", index_type),
                    });
                }

                match array_type {
                    Type::Array(inner) => Ok(*inner),
                    _ => Err(CheckerError::InvalidOperation {
                        op: "index".to_string(),
                        type_name: format!("{:?}", array_type),
                    }),
                }
            }

            Expr::IndexAssign(array_index, value) => {
                let value_type = self.check_expr(value)?;

                if let Expr::Index(array, index) = array_index.as_mut() {
                    let array_type = self.get_expr_type(array);
                    let index_type = self.check_expr(index)?;

                    if !Self::is_numeric_type(&index_type) {
                        return Err(CheckerError::InvalidOperation {
                            op: "array index".to_string(),
                            type_name: format!("{:?}", index_type),
                        });
                    }

                    match array_type {
                        Type::Array(inner) => {
                            if !self.types_compatible(&inner, &value_type) {
                                return Err(CheckerError::TypeMismatch {
                                    expected: *inner,
                                    found: value_type,
                                    context: "array assignment".to_string(),
                                });
                            }
                        }
                        _ => {
                            return Err(CheckerError::InvalidOperation {
                                op: "index assignment".to_string(),
                                type_name: format!("{:?}", array_type),
                            });
                        }
                    }

                    Ok(Type::Named("void".to_string()))
                } else {
                    Err(CheckerError::InvalidOperation {
                        op: "index assignment".to_string(),
                        type_name: "non-index expression".to_string(),
                    })
                }
            }

            Expr::ArrayLiteral(elements) => {
                let mut elem_type = Type::Named("int".to_string());
                for e in elements {
                    elem_type = self.check_expr(e)?;
                }
                Ok(Type::Array(Box::new(elem_type)))
            }

            Expr::ArrayFill(elem_type, length) => {
                let len_type = self.check_expr(length)?;
                if !Self::is_numeric_type(&len_type) {
                    return Err(CheckerError::InvalidOperation {
                        op: "array fill".to_string(),
                        type_name: format!("{:?}", len_type),
                    });
                }
                Ok(Type::Array(Box::new(elem_type.clone())))
            }

            Expr::FuncDecl(_name, params, _ret_type, body) => {
                self.push_scope();
                for (param_name, param_type) in params {
                    self.declare_var(param_name, param_type.clone());
                }
                self.check_expr(body)?;
                self.pop_scope();

                Ok(Type::Named("void".to_string()))
            }

            Expr::Extern(_, _, _) => Ok(Type::Named("void".to_string())),

            Expr::Break | Expr::Continue => Ok(Type::Named("void".to_string())),

            Expr::TypeDef => Ok(Type::Named("void".to_string())),

            Expr::Struct(_name, fields) => {
                for (_, field_ty) in fields {
                    self.validate_type(field_ty)?;
                }
                Ok(Type::Named("void".to_string()))
            }

            Expr::StructLiteral(name, field_values) => {
                let struct_def = self
                    .structs
                    .get(name)
                    .ok_or_else(|| CheckerError::UndefinedStruct(name.clone()))?
                    .clone();

                for (field_name, expected_ty) in &struct_def.fields {
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
                            });
                        }
                    }
                }

                Ok(Type::Named(name.clone()))
            }

            Expr::MemberAccess(obj, field_name) => {
                let obj_type = self.check_expr(obj)?;
                match obj_type {
                    Type::Named(struct_name) => {
                        let struct_def = self
                            .structs
                            .get(&struct_name)
                            .ok_or_else(|| CheckerError::UndefinedStruct(struct_name.clone()))?;

                        for (name, ty) in &struct_def.fields {
                            if name == field_name {
                                return Ok(ty.clone());
                            }
                        }

                        Err(CheckerError::UndefinedField {
                            struct_name,
                            field: field_name.clone(),
                        })
                    }
                    _ => Err(CheckerError::NonStructMemberAccess(format!(
                        "{:?}",
                        obj_type
                    ))),
                }
            }

            Expr::FAdd(lhs, rhs)
            | Expr::FSub(lhs, rhs)
            | Expr::FMul(lhs, rhs)
            | Expr::FDiv(lhs, rhs) => {
                self.check_expr(lhs)?;
                self.check_expr(rhs)?;
                Ok(Type::Named("float".to_string()))
            }
            Expr::FEq(lhs, rhs)
            | Expr::FNe(lhs, rhs)
            | Expr::FLt(lhs, rhs)
            | Expr::FLe(lhs, rhs)
            | Expr::FGt(lhs, rhs)
            | Expr::FGe(lhs, rhs) => {
                self.check_expr(lhs)?;
                self.check_expr(rhs)?;
                Ok(Type::Named("bool".to_string()))
            }
            Expr::StrConcat(lhs, rhs) => {
                self.check_expr(lhs)?;
                self.check_expr(rhs)?;
                Ok(Type::Named("string".to_string()))
            }
        }
    }

    fn types_compatible(&self, expected: &Type, found: &Type) -> bool {
        let expected = self.resolve_type(expected);
        let found = self.resolve_type(found);

        match (&expected, &found) {
            (Type::Named(n), _) if n == "void" => true,
            (_, Type::Named(n)) if n == "void" => true,
            (Type::Named(a), Type::Named(b)) => a == b,
            (Type::Array(a), Type::Array(b)) => self.types_compatible(a, b),
            _ => false,
        }
    }

    fn validate_type(&self, ty: &Type) -> Result<(), CheckerError> {
        match ty {
            Type::Named(_) => Ok(()),
            Type::Array(inner) => self.validate_type(inner),
        }
    }
}
