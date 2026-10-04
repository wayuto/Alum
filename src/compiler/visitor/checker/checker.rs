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
                Expr::FuncDecl {
                    name,
                    attrs,
                    type_params,
                    params,
                    return_type: ret_type,
                    ..
                } => {
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
                Expr::ExternVar { name, ty, .. } => {
                    self.extern_vars.insert(name.clone(), ty.clone());
                }
                Expr::GlobalVar {
                    name,
                    is_pub: _,
                    ty,
                    ..
                } => {
                    self.globals.insert(name.clone(), ty.clone());
                }
                Expr::ConstDecl { name, ty, .. } => {
                    if !matches!(ty, Type::Unknown) {
                        self.constants.insert(name.clone(), ty.clone());
                    }
                }
                Expr::Struct {
                    name,
                    type_params,
                    fields,
                    ..
                } => {
                    self.structs
                        .insert(name.clone(), (type_params.clone(), fields.clone()));
                }
                Expr::Union {
                    name,
                    type_params,
                    fields,
                    ..
                } => {
                    self.unions
                        .insert(name.clone(), (type_params.clone(), fields.clone()));
                }
                Expr::Enum { name, members, .. } => {
                    self.enums.insert(name.clone(), members.clone());
                }
                _ => {}
            }
        }
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

    pub(super) fn nearest_decl_is_const(&self, name: &str) -> bool {
        let n = self.type_stack.len();
        for i in (0..n).rev() {
            if let Some(scope) = self.const_stack.get(i) {
                if scope.contains(name) {
                    return true;
                }
            }
            if let Some(ts) = self.type_stack.get(i) {
                if ts.contains_key(name) {
                    return false;
                }
            }
        }
        self.constants.contains_key(name)
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
                Expr::Var { name, .. } => return self.is_constant(name).then(|| name.clone()),
                Expr::Index { array: base, .. } | Expr::MemberAccess { obj: base, .. } => {
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
            Expr::Var { name, .. } => match self.resolve_enum_member(name) {
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
            Expr::MemberAccess { obj, field, .. } => {
                if let Expr::Var {
                    name: enum_name, ..
                } = obj.as_ref()
                {
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
        branches: &[(Expr, Option<Box<Expr>>, Expr)],
        has_default: bool,
        span: Span,
    ) -> Result<(), CheckerError> {
        if has_default {
            return Ok(());
        }
        match self.resolve_type(target_ty) {
            Type::Primitive(Primitive::Boolean) => {
                let mut covered: HashSet<bool> = HashSet::new();
                for (case, guard, _) in branches {
                    if guard.is_some() {
                        continue;
                    }
                    if let Expr::Bool { value: b, .. } = case {
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
                for (case, guard, _) in branches {
                    if guard.is_some() {
                        continue;
                    }
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
            Type::TypeVar(id) => match self.type_bindings.get(id) {
                Some(bound_type) => self.resolve_type(bound_type),
                None => ty.clone(),
            },
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
    pub(super) fn normalize_type(&self, ty: &Type) -> Type {
        match self.resolve_type(ty) {
            Type::TypeVar(_) => Type::Primitive(Primitive::Int),
            t => t,
        }
    }
    pub(super) fn normalize_type_args(&self, args: &mut [Type]) {
        for ty in args.iter_mut() {
            *ty = self.normalize_type(ty);
        }
    }

    pub(super) fn resolve_call_type_args(&mut self, expr: &mut Expr) {
        match expr {
            Expr::Call {
                callee,
                type_args,
                args,
                ..
            } => {
                self.normalize_type_args(type_args);
                self.resolve_call_type_args(callee);
                for arg in args.iter_mut() {
                    self.resolve_call_type_args(arg);
                }
            }
            Expr::StructLiteral {
                name: _,
                type_args,
                fields,
                ..
            } => {
                self.normalize_type_args(type_args);
                for (_, value) in fields.iter_mut() {
                    self.resolve_call_type_args(value);
                }
            }
            Expr::UnionLiteral {
                name: _,
                type_args,
                fields,
                ..
            } => {
                self.normalize_type_args(type_args);
                for (_, value) in fields.iter_mut() {
                    self.resolve_call_type_args(value);
                }
            }
            Expr::Block { stmts: body, .. } => {
                for e in body.iter_mut() {
                    self.resolve_call_type_args(e);
                }
            }
            Expr::FuncDecl {
                name: _,
                attrs: _,
                type_params: _,
                params: _,
                return_type: _,
                body,
                ..
            } => self.resolve_call_type_args(body),
            Expr::Lambda {
                params: _, body, ..
            } => self.resolve_call_type_args(body),
            Expr::If {
                cond,
                then_branch,
                else_branch,
                ..
            } => {
                self.resolve_call_type_args(cond);
                self.resolve_call_type_args(then_branch);
                if let Some(e) = else_branch {
                    self.resolve_call_type_args(e);
                }
            }
            Expr::While { cond, body, .. } => {
                self.resolve_call_type_args(cond);
                self.resolve_call_type_args(body);
            }
            Expr::For {
                var: _,
                iterable: array,
                body,
                ..
            } => {
                self.resolve_call_type_args(array);
                self.resolve_call_type_args(body);
            }
            Expr::Match {
                target,
                branches,
                default,
                ..
            } => {
                self.resolve_call_type_args(target);
                for (case, guard, result) in branches.iter_mut() {
                    self.resolve_call_type_args(case);
                    if let Some(guard) = guard {
                        self.resolve_call_type_args(guard);
                    }
                    self.resolve_call_type_args(result);
                }
                if let Some(d) = default {
                    self.resolve_call_type_args(d);
                }
            }
            Expr::Range { start, end, .. } => {
                self.resolve_call_type_args(start);
                self.resolve_call_type_args(end);
            }
            Expr::VarDecl {
                name: _,
                ty: _,
                value,
                ..
            }
            | Expr::ConstDecl {
                name: _,
                ty: _,
                value,
                ..
            }
            | Expr::VarAssign { name: _, value, .. }
            | Expr::Return { value, .. }
            | Expr::AddAssign { name: _, value, .. }
            | Expr::SubAssign { name: _, value, .. } => self.resolve_call_type_args(value),
            Expr::GlobalVar {
                name: _,
                is_pub: _,
                ty: _,
                value,
                ..
            } => {
                if let Some(v) = value {
                    self.resolve_call_type_args(v);
                }
            }
            Expr::ArrayLiteral {
                elements: elems, ..
            } => {
                for e in elems.iter_mut() {
                    self.resolve_call_type_args(e);
                }
            }
            Expr::ArrayFill {
                elem_type: _, len, ..
            } => self.resolve_call_type_args(len),
            Expr::Index {
                array: arr,
                index: idx,
                ..
            } => {
                self.resolve_call_type_args(arr);
                self.resolve_call_type_args(idx);
            }
            Expr::IndexAssign {
                target: arr_idx, ..
            } => self.resolve_call_type_args(arr_idx),
            Expr::MemberAccess { obj, .. } => self.resolve_call_type_args(obj),
            Expr::MemberAssign {
                obj,
                field: _,
                value: val,
                ..
            } => {
                self.resolve_call_type_args(obj);
                self.resolve_call_type_args(val);
            }
            Expr::AddressOf { expr: inner, .. } => self.resolve_call_type_args(inner),
            Expr::Deref { expr: inner, .. } => self.resolve_call_type_args(inner),
            Expr::DerefAssign {
                ptr, value: val, ..
            } => {
                self.resolve_call_type_args(ptr);
                self.resolve_call_type_args(val);
            }
            Expr::Cast { expr: inner, .. } => self.resolve_call_type_args(inner),
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
            }
            | Expr::Xor {
                left: l, right: r, ..
            }
            | Expr::FAdd {
                left: l, right: r, ..
            }
            | Expr::FSub {
                left: l, right: r, ..
            }
            | Expr::FMul {
                left: l, right: r, ..
            }
            | Expr::FDiv {
                left: l, right: r, ..
            }
            | Expr::Eq {
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
            }
            | Expr::FEq {
                left: l, right: r, ..
            }
            | Expr::FNe {
                left: l, right: r, ..
            }
            | Expr::FLt {
                left: l, right: r, ..
            }
            | Expr::FLe {
                left: l, right: r, ..
            }
            | Expr::FGt {
                left: l, right: r, ..
            }
            | Expr::FGe {
                left: l, right: r, ..
            }
            | Expr::BAnd {
                left: l, right: r, ..
            }
            | Expr::BOr {
                left: l, right: r, ..
            }
            | Expr::LAnd {
                left: l, right: r, ..
            }
            | Expr::LOr {
                left: l, right: r, ..
            }
            | Expr::Shl {
                left: l, right: r, ..
            }
            | Expr::Shr {
                left: l, right: r, ..
            }
            | Expr::StrCat {
                left: l, right: r, ..
            } => {
                self.resolve_call_type_args(l);
                self.resolve_call_type_args(r);
            }
            Expr::BNot { expr: e, .. } => self.resolve_call_type_args(e),
            Expr::Inc { .. } | Expr::Dec { .. } => {}
            Expr::MulAssign {
                name: _, value: v, ..
            }
            | Expr::DivAssign {
                name: _, value: v, ..
            }
            | Expr::ModAssign {
                name: _, value: v, ..
            }
            | Expr::AndAssign {
                name: _, value: v, ..
            }
            | Expr::OrAssign {
                name: _, value: v, ..
            }
            | Expr::XorAssign {
                name: _, value: v, ..
            }
            | Expr::ShlAssign {
                name: _, value: v, ..
            }
            | Expr::ShrAssign {
                name: _, value: v, ..
            } => self.resolve_call_type_args(v),
            Expr::Not { expr: e, .. } | Expr::Neg { expr: e, .. } | Expr::FNeg { expr: e, .. } => {
                self.resolve_call_type_args(e)
            }
            Expr::FString { segs: parts, .. } => {
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
            (Type::Struct(s1, a1), Type::Pointer(inner))
                if **inner == Type::Struct(s1.clone(), a1.clone()) =>
            {
                true
            }
            (Type::Pointer(inner), Type::Struct(s1, a1))
                if **inner == Type::Struct(s1.clone(), a1.clone()) =>
            {
                true
            }
            (Type::Primitive(Primitive::Void), Type::Primitive(Primitive::Void)) => true,
            (Type::Pointer(_), Type::Primitive(Primitive::Void)) => true,
            (Type::TypeVar(_), Type::Primitive(Primitive::Void)) => true,
            (Type::Param(_), Type::Primitive(Primitive::Void)) => true,
            (Type::TypeVar(_), _) => true,
            (_, Type::TypeVar(_)) => true,

            (Type::Primitive(Primitive::Char), Type::Primitive(Primitive::Int))
            | (Type::Primitive(Primitive::Int), Type::Primitive(Primitive::Char)) => true,
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
