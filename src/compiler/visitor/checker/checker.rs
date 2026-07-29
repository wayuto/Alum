use super::error::CheckerError;
use crate::compiler::{
    parser::{Expr, Program, Type},
    visitor::TypeChecker,
};
use std::collections::HashMap;

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

    pub(super) fn new_type_var(&mut self) -> Type {
        let id = self.type_var_counter;
        self.type_var_counter += 1;
        Type::TypeVar(id)
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

    pub(super) fn push_scope(&mut self) {
        self.type_stack.push(HashMap::new());
    }

    pub(super) fn pop_scope(&mut self) {
        self.type_stack.pop();
    }

    pub(super) fn declare_var(&mut self, name: &str, ty: Type) {
        self.type_stack
            .last_mut()
            .unwrap()
            .insert(name.to_string(), ty);
    }

    pub(super) fn lookup_var(&self, name: &str) -> Option<Type> {
        for scope in self.type_stack.iter().rev() {
            if let Some(ty) = scope.get(name) {
                return Some(ty.clone());
            }
        }
        None
    }

    pub(super) fn resolve_type(&self, ty: &Type) -> Type {
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

    pub(super) fn resolve_gen_types(&mut self, ty: &Type) -> Type {
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

    pub(super) fn get_expr_type(&self, expr: &Expr) -> Type {
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
            Expr::LAnd(_, _, _) | Expr::LOr(_, _, _) | Expr::Not(_, _) => {
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

    pub(super) fn is_numeric_type(ty: &Type) -> bool {
        matches!(ty, Type::Named(n) if n == "int" || n == "float") || matches!(ty, Type::TypeVar(_))
    }

    pub(super) fn is_float_type(ty: &Type) -> bool {
        matches!(ty, Type::Named(n) if n == "float")
    }

    pub(super) fn is_string_type(ty: &Type) -> bool {
        matches!(ty, Type::Named(n) if n == "string")
    }

    pub(super) fn is_bool_type(ty: &Type) -> bool {
        matches!(ty, Type::Named(n) if n == "bool")
    }

    pub(super) fn types_compatible(&self, expected: &Type, found: &Type) -> bool {
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

    pub(super) fn validate_type(&self, ty: &Type) -> Result<(), CheckerError> {
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
