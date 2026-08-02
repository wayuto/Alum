use super::error::CheckerError;
use crate::compiler::{
    parser::{Expr, Primitive, Program, Type},
    visitor::TypeChecker,
};
use std::collections::HashMap;

impl TypeChecker {
    pub(super) fn new_type_var(&mut self) -> Type {
        let id = self.type_var_counter;
        self.type_var_counter += 1;
        Type::TypeVar(id)
    }

    pub(super) fn fresh_instantiate(
        &mut self,
        ty: &Type,
        subst: &mut HashMap<usize, Type>,
    ) -> Type {
        match ty {
            Type::Param(id) | Type::TypeVar(id) => subst
                .entry(*id)
                .or_insert_with(|| self.new_type_var())
                .clone(),
            Type::Array(inner) => Type::Array(Box::new(self.fresh_instantiate(inner, subst))),
            Type::Pointer(inner) => Type::Pointer(Box::new(self.fresh_instantiate(inner, subst))),
            Type::Function(params, ret) => Type::Function(
                params
                    .iter()
                    .map(|p| self.fresh_instantiate(p, subst))
                    .collect(),
                Box::new(self.fresh_instantiate(ret, subst)),
            ),
            Type::Struct(name, args) => Type::Struct(
                name.clone(),
                args.iter()
                    .map(|t| self.fresh_instantiate(t, subst))
                    .collect(),
            ),
            Type::Union(name, args) => Type::Union(
                name.clone(),
                args.iter()
                    .map(|t| self.fresh_instantiate(t, subst))
                    .collect(),
            ),
            _ => ty.clone(),
        }
    }

    pub(super) fn fresh_instantiate_signature(
        &mut self,
        params: &[Type],
        ret_type: &Type,
    ) -> (Vec<Type>, Type) {
        let mut subst = HashMap::new();
        let resolved_params: Vec<Type> = params
            .iter()
            .map(|t| self.fresh_instantiate(t, &mut subst))
            .collect();
        let resolved_ret = self.fresh_instantiate(ret_type, &mut subst);
        (resolved_params, resolved_ret)
    }

    pub(super) fn push_generic_params(&mut self, count: usize) {
        let mut scope = HashMap::new();
        for i in 0..count {
            scope.insert(i, self.new_type_var());
        }
        self.generic_params.push(scope);
    }

    pub(super) fn pop_generic_params(&mut self) {
        self.generic_params.pop();
    }

    pub(super) fn resolve_params(&self, ty: &Type) -> Type {
        match ty {
            Type::Param(id) => {
                if let Some(scope) = self.generic_params.last() {
                    if let Some(tv) = scope.get(id) {
                        return tv.clone();
                    }
                }
                ty.clone()
            }
            Type::Array(inner) => Type::Array(Box::new(self.resolve_params(inner))),
            Type::Pointer(inner) => Type::Pointer(Box::new(self.resolve_params(inner))),
            Type::Function(params, ret) => Type::Function(
                params.iter().map(|p| self.resolve_params(p)).collect(),
                Box::new(self.resolve_params(ret)),
            ),
            Type::Struct(name, args) => Type::Struct(
                name.clone(),
                args.iter().map(|t| self.resolve_params(t)).collect(),
            ),
            Type::Union(name, args) => Type::Union(
                name.clone(),
                args.iter().map(|t| self.resolve_params(t)).collect(),
            ),
            _ => ty.clone(),
        }
    }

    pub fn check(mut self, program: &mut Program) -> Result<(), CheckerError> {
        for expr in &program.body {
            match expr {
                Expr::FuncDecl(name, type_params, params, ret_type, _, _) => {
                    let param_types: Vec<Type> = params.iter().map(|(_, t)| t.clone()).collect();
                    self.functions.insert(
                        name.clone(),
                        (type_params.clone(), param_types, ret_type.clone()),
                    );
                }
                Expr::Extern(name, params, ret_type, _) => {
                    let param_types: Vec<Type> = params.iter().map(|(_, t)| t.clone()).collect();
                    self.functions
                        .insert(name.clone(), (Vec::new(), param_types, ret_type.clone()));
                }
                Expr::Struct(name, type_params, fields, _) => {
                    self.structs
                        .insert(name.clone(), (type_params.clone(), fields.clone()));
                }
                Expr::Union(name, type_params, fields, _) => {
                    self.unions
                        .insert(name.clone(), (type_params.clone(), fields.clone()));
                }
                Expr::Enum(name, members, _) => {
                    self.enums.insert(name.clone(), members.clone());
                }
                _ => {}
            }
        }

        for expr in &mut program.body {
            self.check_expr(expr)?;
        }

        for expr in &mut program.body {
            self.resolve_call_type_args(expr);
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

    pub(super) fn lookup_enum_member(&self, name: &str) -> Option<isize> {
        for members in self.enums.values() {
            for (member_name, value) in members {
                if member_name == name {
                    return Some(*value);
                }
            }
        }
        None
    }

    pub(super) fn resolve_type(&self, ty: &Type) -> Type {
        match ty {
            Type::TypeVar(_) => self.resolve_type_var(ty),
            Type::Array(inner) => Type::Array(Box::new(self.resolve_type(inner))),
            Type::Pointer(inner) => Type::Pointer(Box::new(self.resolve_type(inner))),
            Type::Function(params, ret) => Type::Function(
                params.iter().map(|p| self.resolve_type(p)).collect(),
                Box::new(self.resolve_type(ret)),
            ),
            Type::Struct(name, args) => Type::Struct(
                name.clone(),
                args.iter().map(|t| self.resolve_type(t)).collect(),
            ),
            Type::Union(name, args) => Type::Union(
                name.clone(),
                args.iter().map(|t| self.resolve_type(t)).collect(),
            ),
            _ => ty.clone(),
        }
    }

    pub(super) fn resolve_call_type_args(&mut self, expr: &mut Expr) {
        match expr {
            Expr::Call(callee, type_args, args, _) => {
                for ty in type_args.iter_mut() {
                    let resolved = self.resolve_type_var(ty);
                    *ty = match resolved {
                        Type::TypeVar(_) => Type::Primitive(Primitive::Int),
                        t => t,
                    };
                }
                self.resolve_call_type_args(callee);
                for arg in args.iter_mut() {
                    self.resolve_call_type_args(arg);
                }
            }
            Expr::StructLiteral(_, type_args, fields, _) => {
                for ty in type_args.iter_mut() {
                    let resolved = self.resolve_type_var(ty);
                    *ty = match resolved {
                        Type::TypeVar(_) => Type::Primitive(Primitive::Int),
                        t => t,
                    };
                }
                for (_, value) in fields.iter_mut() {
                    self.resolve_call_type_args(value);
                }
            }
            Expr::UnionLiteral(_, type_args, fields, _) => {
                for ty in type_args.iter_mut() {
                    let resolved = self.resolve_type_var(ty);
                    *ty = match resolved {
                        Type::TypeVar(_) => Type::Primitive(Primitive::Int),
                        t => t,
                    };
                }
                for (_, value) in fields.iter_mut() {
                    self.resolve_call_type_args(value);
                }
            }
            Expr::Block(body, _) => {
                for e in body.iter_mut() {
                    self.resolve_call_type_args(e);
                }
            }
            Expr::FuncDecl(_, _, _, _, body, _) => self.resolve_call_type_args(body),
            Expr::Lambda(_, body, _, _) => self.resolve_call_type_args(body),
            Expr::If(cond, then_branch, else_branch, _) => {
                self.resolve_call_type_args(cond);
                self.resolve_call_type_args(then_branch);
                if let Some(e) = else_branch {
                    self.resolve_call_type_args(e);
                }
            }
            Expr::While(cond, body, _) => {
                self.resolve_call_type_args(cond);
                self.resolve_call_type_args(body);
            }
            Expr::For(_, array, body, _) => {
                self.resolve_call_type_args(array);
                self.resolve_call_type_args(body);
            }
            Expr::VarDecl(_, _, value, _)
            | Expr::VarAssign(_, value, _)
            | Expr::Return(value, _)
            | Expr::AddAssign(_, value, _)
            | Expr::SubAssign(_, value, _) => self.resolve_call_type_args(value),
            Expr::ArrayLiteral(elems, _) => {
                for e in elems.iter_mut() {
                    self.resolve_call_type_args(e);
                }
            }
            Expr::ArrayFill(_, len, _) => self.resolve_call_type_args(len),
            Expr::Index(arr, idx, _) => {
                self.resolve_call_type_args(arr);
                self.resolve_call_type_args(idx);
            }
            Expr::IndexAssign(arr_idx, _, _) => self.resolve_call_type_args(arr_idx),
            Expr::MemberAccess(obj, _, _) => self.resolve_call_type_args(obj),
            Expr::MemberAssign(obj, _, val, _) => {
                self.resolve_call_type_args(obj);
                self.resolve_call_type_args(val);
            }
            Expr::AddressOf(inner, _) => self.resolve_call_type_args(inner),
            Expr::Deref(inner, _) => self.resolve_call_type_args(inner),
            Expr::DerefAssign(ptr, val, _) => {
                self.resolve_call_type_args(ptr);
                self.resolve_call_type_args(val);
            }
            Expr::Add(l, r, _)
            | Expr::Sub(l, r, _)
            | Expr::Mul(l, r, _)
            | Expr::Div(l, r, _)
            | Expr::Mod(l, r, _)
            | Expr::Xor(l, r, _)
            | Expr::FAdd(l, r, _)
            | Expr::FSub(l, r, _)
            | Expr::FMul(l, r, _)
            | Expr::FDiv(l, r, _)
            | Expr::Eq(l, r, _)
            | Expr::Ne(l, r, _)
            | Expr::Lt(l, r, _)
            | Expr::Le(l, r, _)
            | Expr::Gt(l, r, _)
            | Expr::Ge(l, r, _)
            | Expr::FEq(l, r, _)
            | Expr::FNe(l, r, _)
            | Expr::FLt(l, r, _)
            | Expr::FLe(l, r, _)
            | Expr::FGt(l, r, _)
            | Expr::FGe(l, r, _)
            | Expr::LAnd(l, r, _)
            | Expr::LOr(l, r, _)
            | Expr::StrCat(l, r, _) => {
                self.resolve_call_type_args(l);
                self.resolve_call_type_args(r);
            }
            Expr::Not(e, _) | Expr::Neg(e, _) | Expr::FNeg(e, _) => self.resolve_call_type_args(e),
            _ => {}
        }
    }

    pub(super) fn get_expr_type(&self, expr: &Expr) -> Type {
        match expr {
            Expr::Int(_, _) => Type::Primitive(Primitive::Int),
            Expr::Float(_, _) => Type::Primitive(Primitive::Float),
            Expr::Bool(_, _) => Type::Primitive(Primitive::Boolean),
            Expr::String(_, _) => Type::Primitive(Primitive::String),
            Expr::Nil(_) => Type::Primitive(Primitive::Void),
            Expr::Var(name, _) => self
                .lookup_var(name)
                .unwrap_or(Type::Primitive(Primitive::Int)),
            Expr::FAdd(_, _, _)
            | Expr::FSub(_, _, _)
            | Expr::FMul(_, _, _)
            | Expr::FDiv(_, _, _) => Type::Primitive(Primitive::Float),
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
            | Expr::SubAssign(_, _, _) => Type::Primitive(Primitive::Int),
            Expr::FNeg(_, _) => Type::Primitive(Primitive::Float),
            Expr::FEq(_, _, _)
            | Expr::FNe(_, _, _)
            | Expr::FLt(_, _, _)
            | Expr::FLe(_, _, _)
            | Expr::FGt(_, _, _)
            | Expr::FGe(_, _, _) => Type::Primitive(Primitive::Boolean),
            Expr::Eq(_, _, _)
            | Expr::Ne(_, _, _)
            | Expr::Lt(_, _, _)
            | Expr::Le(_, _, _)
            | Expr::Gt(_, _, _)
            | Expr::Ge(_, _, _) => Type::Primitive(Primitive::Boolean),
            Expr::LAnd(_, _, _) | Expr::LOr(_, _, _) | Expr::Not(_, _) => {
                Type::Primitive(Primitive::Boolean)
            }
            Expr::StrCat(_, _, _) => Type::Primitive(Primitive::String),
            Expr::Call(callee, _, _, _) => {
                if let Expr::Var(name, _) = callee.as_ref() {
                    if let Some((_, _, ret_type)) = self.functions.get(name) {
                        return ret_type.clone();
                    }
                }
                Type::Primitive(Primitive::Int)
            }
            Expr::Index(_, _, _) => Type::Primitive(Primitive::Int),
            Expr::MemberAccess(obj, field_name, _) => {
                if let Expr::Var(name, _) = obj.as_ref() {
                    if let Some(members) = self.enums.get(name) {
                        for (member_name, _) in members {
                            if member_name == field_name {
                                return Type::Primitive(Primitive::Int);
                            }
                        }
                        return Type::Primitive(Primitive::Int);
                    }
                }
                let obj_type = self.get_expr_type(obj);
                let inner_type = match obj_type {
                    Type::Pointer(inner) => *inner,
                    Type::Struct(_, _) | Type::Union(_, _) => obj_type,
                    _ => return Type::Primitive(Primitive::Int),
                };
                match inner_type {
                    Type::Struct(struct_name, args) => {
                        if let Some((_, fields)) = self.structs.get(&struct_name) {
                            for (name, ty) in fields {
                                if name == field_name {
                                    return self.resolve_type(&ty.substitute(&args));
                                }
                            }
                        }
                    }
                    Type::Union(union_name, args) => {
                        if let Some((_, fields)) = self.unions.get(&union_name) {
                            for (name, ty) in fields {
                                if name == field_name {
                                    return self.resolve_type(&ty.substitute(&args));
                                }
                            }
                        }
                    }
                    _ => {}
                }
                Type::Primitive(Primitive::Int)
            }
            Expr::StructLiteral(name, type_args, _, _) => {
                Type::Struct(name.clone(), type_args.clone())
            }
            Expr::UnionLiteral(name, type_args, _, _) => {
                Type::Union(name.clone(), type_args.clone())
            }
            Expr::ArrayLiteral(_, _) | Expr::ArrayFill(_, _, _) => {
                Type::Array(Box::new(Type::Primitive(Primitive::Int)))
            }
            Expr::AddressOf(expr, _) => {
                let inner_type = self.get_expr_type(expr);
                Type::Pointer(Box::new(inner_type))
            }
            Expr::Deref(expr, _) => {
                let ptr_type = self.get_expr_type(expr);
                match ptr_type {
                    Type::Pointer(inner) => *inner,
                    _ => Type::Primitive(Primitive::Int),
                }
            }
            Expr::DerefAssign(_, _, _) => Type::Primitive(Primitive::Void),
            _ => Type::Primitive(Primitive::Int),
        }
    }

    pub(super) fn types_compatible(&self, expected: &Type, found: &Type) -> bool {
        let expected = self.resolve_type(expected);
        let found = self.resolve_type(found);

        match (&expected, &found) {
            (Type::TypeVar(_), _) => true,
            (_, Type::TypeVar(_)) => true,
            (Type::Param(a), Type::Param(b)) => a == b,
            (Type::Primitive(a), Type::Primitive(b)) => a == b,
            (Type::Pointer(inner), Type::Primitive(Primitive::String))
                if matches!(inner.as_ref(), Type::Primitive(Primitive::Void)) =>
            {
                true
            }
            (Type::Primitive(Primitive::String), Type::Pointer(inner))
                if matches!(inner.as_ref(), Type::Primitive(Primitive::Void)) =>
            {
                true
            }
            (Type::Array(a), Type::Array(b)) => self.types_compatible(a, b),
            (Type::Pointer(a), Type::Array(b)) => self.types_compatible(a, b),
            (Type::Pointer(a), Type::Pointer(b)) => {
                if matches!(a.as_ref(), Type::Primitive(Primitive::Void))
                    || matches!(b.as_ref(), Type::Primitive(Primitive::Void))
                {
                    true
                } else {
                    self.types_compatible(a, b)
                }
            }
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
            (Type::Struct(n1, a1), Type::Struct(n2, a2)) => {
                n1 == n2
                    && a1.len() == a2.len()
                    && a1
                        .iter()
                        .zip(a2.iter())
                        .all(|(t1, t2)| self.types_compatible(t1, t2))
            }
            (Type::Union(n1, a1), Type::Union(n2, a2)) => {
                n1 == n2
                    && a1.len() == a2.len()
                    && a1
                        .iter()
                        .zip(a2.iter())
                        .all(|(t1, t2)| self.types_compatible(t1, t2))
            }
            _ => false,
        }
    }

    pub(super) fn validate_type(&self, ty: &Type) -> Result<(), CheckerError> {
        match ty {
            Type::Primitive(_) | Type::TypeVar(_) | Type::Param(_) | Type::Unknown => Ok(()),
            Type::Array(inner) => self.validate_type(inner),
            Type::Pointer(inner) => self.validate_type(inner),
            Type::Function(params, ret) => {
                for param in params {
                    self.validate_type(param)?;
                }
                self.validate_type(ret)
            }
            Type::Struct(_, args) => {
                for arg in args {
                    self.validate_type(arg)?;
                }
                Ok(())
            }
            Type::Union(_, args) => {
                for arg in args {
                    self.validate_type(arg)?;
                }
                Ok(())
            }
        }
    }
}
