use super::error::CheckerError;
use crate::compiler::{Span, parser::Type, visitor::TypeChecker};

impl TypeChecker {
    pub(super) fn resolve_type_var(&self, ty: &Type) -> Type {
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

    pub(super) fn bind_type_var(&mut self, var_id: usize, ty: &Type) {
        let resolved_ty = self.resolve_type_var(ty);
        self.type_bindings.insert(var_id, resolved_ty);
    }

    pub(super) fn unify_types(&mut self, t1: &Type, t2: &Type) -> Result<(), CheckerError> {
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
}
