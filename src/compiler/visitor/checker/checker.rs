use super::error::CheckerError;
use crate::compiler::{
    Span,
    parser::{Expr, Primitive, Program, Type},
    visitor::TypeChecker,
};
use std::collections::{HashMap, HashSet};

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

    fn collect_declarations(&mut self, program: &Program) {
        for expr in &program.body {
            match expr {
                Expr::FuncDecl(name, attrs, type_params, params, ret_type, _, _) => {
                    let param_types: Vec<Type> = params.iter().map(|(_, t)| t.clone()).collect();
                    if attrs.is_external {
                        self.functions
                            .insert(name.clone(), (Vec::new(), param_types, ret_type.clone()));
                    } else {
                        self.functions.insert(
                            name.clone(),
                            (type_params.clone(), param_types, ret_type.clone()),
                        );
                    }
                }
                Expr::ExternVar(name, ty, _) => {
                    self.extern_vars.insert(name.clone(), ty.clone());
                }
                Expr::GlobalVar(name, _, ty, _, _) => {
                    self.globals.insert(name.clone(), ty.clone());
                }
                Expr::ConstDecl(name, ty, _, _, _) => {
                    if !matches!(ty, Type::Unknown) {
                        self.constants.insert(name.clone(), ty.clone());
                    }
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
    }

    pub fn check(mut self, program: &mut Program) -> Result<(), CheckerError> {
        self.collect_declarations(program);

        for expr in &mut program.body {
            match self.check_expr(expr) {
                Ok(_) => {}
                Err(e) => self.errors.push(e),
            }

            if self.errors.first().is_some() {
                return Err(self.errors.remove(0));
            }
        }

        for expr in &mut program.body {
            self.resolve_call_type_args(expr);
        }

        Ok(())
    }

    pub fn check_collect(mut self, program: &mut Program) -> Vec<CheckerError> {
        self.collect_declarations(program);

        for expr in &mut program.body {
            let type_stack_len = self.type_stack.len();
            let const_stack_len = self.const_stack.len();
            let generic_params_len = self.generic_params.len();
            let return_types_len = self.return_types.len();

            if let Err(e) = self.check_expr(expr) {
                self.errors.push(e);
                self.type_stack.truncate(type_stack_len);
                self.const_stack.truncate(const_stack_len);
                self.generic_params.truncate(generic_params_len);
                self.return_types.truncate(return_types_len);
            }
        }

        for expr in &mut program.body {
            self.resolve_call_type_args(expr);
        }

        self.errors
    }

    pub(super) fn push_scope(&mut self) {
        self.type_stack.push(HashMap::new());
        self.const_stack.push(std::collections::HashSet::new());
    }

    pub(super) fn pop_scope(&mut self) {
        self.type_stack.pop();
        self.const_stack.pop();
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

    pub(super) fn declare_const(&mut self, name: &str) {
        self.const_stack
            .last_mut()
            .unwrap()
            .insert(name.to_string());
    }

    pub(super) fn is_constant(&self, name: &str) -> bool {
        if self.constants.contains_key(name) {
            return true;
        }
        for scope in self.const_stack.iter().rev() {
            if scope.contains(name) {
                return true;
            }
        }
        false
    }

    pub(super) fn const_root_name(&self, expr: &Expr) -> Option<String> {
        let mut e = expr;
        loop {
            match e {
                Expr::Var(name, _) => return self.is_constant(name).then(|| name.clone()),
                Expr::Index(base, _, _) | Expr::MemberAccess(base, _, _) => {
                    e = base;
                }
                _ => return None,
            }
        }
    }

    pub(super) fn is_global_scope(&self) -> bool {
        self.type_stack.len() == 1 && self.return_types.is_empty() && self.generic_params.is_empty()
    }

    pub(super) fn resolve_enum_member(&self, name: &str) -> Result<Option<isize>, Vec<String>> {
        let mut found: Vec<(&str, isize)> = Vec::new();
        for (enum_name, members) in &self.enums {
            for (member_name, value) in members {
                if member_name == name {
                    found.push((enum_name.as_str(), *value));
                }
            }
        }
        if found.len() > 1 {
            let mut names: Vec<String> = found.iter().map(|(n, _)| n.to_string()).collect();
            names.sort();
            Err(names)
        } else {
            Ok(found.first().map(|(_, v)| *v))
        }
    }

    fn enum_member_of(&self, expr: &Expr) -> Option<(String, isize)> {
        match expr {
            Expr::Var(name, _) => match self.resolve_enum_member(name) {
                Ok(Some(value)) => {
                    let owners: Vec<String> = self
                        .enums
                        .iter()
                        .filter(|(_, members)| members.iter().any(|(m, _)| m == name))
                        .map(|(e, _)| e.clone())
                        .collect();
                    if owners.len() == 1 {
                        Some((owners[0].clone(), value))
                    } else {
                        None
                    }
                }
                _ => None,
            },
            Expr::MemberAccess(obj, field, _) => {
                if let Expr::Var(enum_name, _) = obj.as_ref() {
                    if let Some(members) = self.enums.get(enum_name) {
                        for (m, v) in members {
                            if m == field {
                                return Some((enum_name.clone(), *v));
                            }
                        }
                    }
                }
                None
            }
            _ => None,
        }
    }

    pub(super) fn check_match_exhaustiveness(
        &self,
        target_ty: &Type,
        branches: &[(Expr, Expr)],
        has_default: bool,
        span: Span,
    ) -> Result<(), CheckerError> {
        if has_default {
            return Ok(());
        }
        match self.resolve_type(target_ty) {
            Type::Primitive(Primitive::Boolean) => {
                let mut covered: HashSet<bool> = HashSet::new();
                for (case, _) in branches {
                    if let Expr::Bool(b, _) = case {
                        covered.insert(*b);
                    } else {
                        return Ok(());
                    }
                }
                let missing: Vec<String> = [true, false]
                    .iter()
                    .filter(|b| !covered.contains(b))
                    .map(|b| b.to_string())
                    .collect();
                if missing.is_empty() {
                    Ok(())
                } else {
                    Err(CheckerError::NonExhaustiveMatch {
                        missing: missing.join(", "),
                        span,
                    })
                }
            }
            Type::Primitive(Primitive::Int) => {
                let mut enum_name: Option<String> = None;
                let mut covered: HashSet<isize> = HashSet::new();
                for (case, _) in branches {
                    match self.enum_member_of(case) {
                        Some((en, value)) => {
                            if let Some(cur) = &enum_name {
                                if *cur != en {
                                    return Err(CheckerError::NonExhaustiveMatch {
                                        missing: "an else (default) branch".to_string(),
                                        span,
                                    });
                                }
                            } else {
                                enum_name = Some(en.clone());
                            }
                            covered.insert(value);
                        }
                        None => {
                            return Err(CheckerError::NonExhaustiveMatch {
                                missing: "an else (default) branch".to_string(),
                                span,
                            });
                        }
                    }
                }
                match enum_name {
                    Some(en) => {
                        let members = &self.enums[&en];
                        let missing: Vec<String> = members
                            .iter()
                            .filter(|(_, v)| !covered.contains(v))
                            .map(|(n, _)| n.clone())
                            .collect();
                        if missing.is_empty() {
                            Ok(())
                        } else {
                            Err(CheckerError::NonExhaustiveMatch {
                                missing: format!("{} from enum '{}'", missing.join(", "), en),
                                span,
                            })
                        }
                    }
                    None => Err(CheckerError::NonExhaustiveMatch {
                        missing: "an else (default) branch".to_string(),
                        span,
                    }),
                }
            }
            _ => Err(CheckerError::NonExhaustiveMatch {
                missing: "an else (default) branch".to_string(),
                span,
            }),
        }
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
            Expr::FuncDecl(_, _, _, _, _, body, _) => self.resolve_call_type_args(body),
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
            Expr::Match(target, branches, default, _) => {
                self.resolve_call_type_args(target);
                for (case, result) in branches.iter_mut() {
                    self.resolve_call_type_args(case);
                    self.resolve_call_type_args(result);
                }
                if let Some(d) = default {
                    self.resolve_call_type_args(d);
                }
            }
            Expr::Range(start, end, _) => {
                self.resolve_call_type_args(start);
                self.resolve_call_type_args(end);
            }
            Expr::VarDecl(_, _, value, _)
            | Expr::ConstDecl(_, _, value, _, _)
            | Expr::VarAssign(_, value, _)
            | Expr::Return(value, _)
            | Expr::AddAssign(_, value, _)
            | Expr::SubAssign(_, value, _) => self.resolve_call_type_args(value),
            Expr::GlobalVar(_, _, _, value, _) => {
                if let Some(v) = value {
                    self.resolve_call_type_args(v);
                }
            }
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
            Expr::Cast(inner, _, _) => self.resolve_call_type_args(inner),
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
            | Expr::Shl(l, r, _)
            | Expr::Shr(l, r, _)
            | Expr::StrCat(l, r, _) => {
                self.resolve_call_type_args(l);
                self.resolve_call_type_args(r);
            }
            Expr::BNot(e, _) => self.resolve_call_type_args(e),
            Expr::Inc(_, _) | Expr::Dec(_, _) => {}
            Expr::MulAssign(_, v, _)
            | Expr::DivAssign(_, v, _)
            | Expr::ModAssign(_, v, _)
            | Expr::AndAssign(_, v, _)
            | Expr::OrAssign(_, v, _)
            | Expr::XorAssign(_, v, _)
            | Expr::ShlAssign(_, v, _)
            | Expr::ShrAssign(_, v, _) => self.resolve_call_type_args(v),
            Expr::Not(e, _) | Expr::Neg(e, _) | Expr::FNeg(e, _) => self.resolve_call_type_args(e),
            Expr::FString(parts, _) => {
                for p in parts {
                    self.resolve_call_type_args(p);
                }
            }
            _ => {}
        }
    }

    pub(super) fn types_compatible(&self, expected: &Type, found: &Type) -> bool {
        let expected = self.resolve_type(expected);
        let found = self.resolve_type(found);

        match (&expected, &found) {
            (Type::Primitive(Primitive::Void), Type::Primitive(Primitive::Void)) => true,
            (Type::Pointer(_), Type::Primitive(Primitive::Void)) => true,
            (Type::TypeVar(_), Type::Primitive(Primitive::Void)) => true,
            (Type::Param(_), Type::Primitive(Primitive::Void)) => true,
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
