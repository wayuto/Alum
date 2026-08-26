use super::error::CheckerError;
use crate::compiler::{
    Span,
    parser::{Primitive, Type},
    visitor::TypeChecker,
};

impl TypeChecker {
    pub(super) fn occurs_check(&self, var_id: usize, ty: &Type) -> bool {
        match ty {
            Type::TypeVar(id) => {
                if *id == var_id {
                    return true;
                }
                match self.type_bindings.get(id) {
                    Some(bound) => self.occurs_check(var_id, bound),
                    None => false,
                }
            }
            Type::Array(inner) => self.occurs_check(var_id, inner),
            Type::Pointer(inner) => self.occurs_check(var_id, inner),
            Type::Function(params, ret) => {
                params.iter().any(|p| self.occurs_check(var_id, p))
                    || self.occurs_check(var_id, ret)
            }
            Type::Struct(_, args) => args.iter().any(|t| self.occurs_check(var_id, t)),
            Type::Union(_, args) => args.iter().any(|t| self.occurs_check(var_id, t)),
            _ => false,
        }
    }

    pub(super) fn bind_type_var(&mut self, var_id: usize, ty: &Type) {
        let resolved_ty = self.resolve_type(ty);
        if self.occurs_check(var_id, &resolved_ty) {
            return;
        }
        self.type_bindings.insert(var_id, resolved_ty);
    }
    pub(super) fn unify_types(&mut self, t1: &Type, t2: &Type) -> Result<(), CheckerError> {
        let t1 = self.resolve_type(t1);
        let t2 = self.resolve_type(t2);

        match (&t1, &t2) {
            (Type::Primitive(Primitive::Void), Type::Primitive(Primitive::Void)) => Ok(()),
            (Type::Pointer(_), Type::Primitive(Primitive::Void)) => Ok(()),

            (Type::TypeVar(id), Type::Primitive(Primitive::Void)) => {
                self.bind_type_var(*id, &t2);
                Ok(())
            }
            (Type::Primitive(Primitive::Void), Type::TypeVar(id)) => {
                self.bind_type_var(*id, &t1);
                Ok(())
            }
            (Type::TypeVar(id), _) => {
                self.bind_type_var(*id, &t2);
                Ok(())
            }
            (_, Type::TypeVar(id)) => {
                self.bind_type_var(*id, &t1);
                Ok(())
            }
            (Type::Param(p1), Type::Param(p2)) if p1 == p2 => Ok(()),
            (Type::Primitive(p1), Type::Primitive(p2)) if p1 == p2 => Ok(()),

            (Type::Pointer(p), Type::Array(a)) => {
                if matches!(p.as_ref(), Type::Primitive(Primitive::Void)) {
                    Ok(())
                } else {
                    self.unify_types(p, a)
                }
            }
            (Type::Array(a), Type::Pointer(p)) => {
                if matches!(p.as_ref(), Type::Primitive(Primitive::Void)) {
                    Ok(())
                } else {
                    self.unify_types(p, a)
                }
            }
            (Type::Pointer(inner), Type::Primitive(Primitive::String))
                if matches!(inner.as_ref(), Type::Primitive(Primitive::Void)) =>
            {
                Ok(())
            }
            (Type::Primitive(Primitive::String), Type::Pointer(inner))
                if matches!(inner.as_ref(), Type::Primitive(Primitive::Void)) =>
            {
                Ok(())
            }
            (Type::Array(a1), Type::Array(a2)) => self.unify_types(a1, a2),
            (Type::Pointer(p1), Type::Pointer(p2)) => {
                if matches!(p1.as_ref(), Type::Primitive(Primitive::Void))
                    || matches!(p2.as_ref(), Type::Primitive(Primitive::Void))
                {
                    Ok(())
                } else {
                    self.unify_types(p1, p2)
                }
            }
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
            (Type::Struct(n1, a1), Type::Struct(n2, a2)) if n1 == n2 && a1.len() == a2.len() => {
                for (t1, t2) in a1.iter().zip(a2.iter()) {
                    self.unify_types(t1, t2)?;
                }
                Ok(())
            }
            (Type::Union(n1, a1), Type::Union(n2, a2)) if n1 == n2 && a1.len() == a2.len() => {
                for (t1, t2) in a1.iter().zip(a2.iter()) {
                    self.unify_types(t1, t2)?;
                }
                Ok(())
            }
            _ => Err(CheckerError::TypeMismatch {
                expected: t1,
                found: t2,
                context: "type unification".to_string(),
                span: Span::new(0, 0),
            }),
        }
    }
}
