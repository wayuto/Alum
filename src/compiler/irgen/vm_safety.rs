use crate::compiler::parser::Expr;
use std::collections::{HashMap, HashSet};

const VM_LAMBDA_MARKER: &str = "\u{03bb}";

pub(super) struct VmSafety<'a> {
    program_body: &'a [Expr],
    pure_fns: &'a HashSet<String>,
    lambda_memo: HashMap<String, bool>,
    lambda_in_progress: HashSet<String>,
    bound: Vec<HashMap<String, String>>,
}

impl<'a> VmSafety<'a> {
    pub(super) fn new(program_body: &'a [Expr], pure_fns: &'a HashSet<String>) -> Self {
        VmSafety {
            program_body,
            pure_fns,
            lambda_memo: HashMap::new(),
            lambda_in_progress: HashSet::new(),
            bound: vec![HashMap::new()],
        }
    }

    fn enter_scope(&mut self) {
        self.bound.push(HashMap::new());
    }

    fn leave_scope(&mut self) {
        self.bound.pop();
    }

    fn lookup_bound(&self, name: &str) -> Option<&String> {
        self.bound.iter().rev().find_map(|scope| scope.get(name))
    }

    fn unbind(&mut self, name: &str) {
        for scope in &mut self.bound {
            scope.remove(name);
        }
    }

    fn bind(&mut self, name: &str, value: &Expr) {
        match value {
            Expr::Var { name: v, .. } => {
                let target = if v.starts_with("_lambda_") || self.pure_fns.contains(v) {
                    Some(v.clone())
                } else {
                    self.lookup_bound(v).cloned()
                };
                if let Some(t) = target {
                    if let Some(scope) = self.bound.last_mut() {
                        scope.insert(name.to_string(), t);
                    }
                }
            }
            Expr::Lambda {
                params: _, body, ..
            } => {
                if self.safe(body) {
                    if let Some(scope) = self.bound.last_mut() {
                        scope.insert(name.to_string(), VM_LAMBDA_MARKER.to_string());
                    }
                }
            }
            _ => {}
        }
    }

    fn callee_pure(&mut self, name: &str) -> bool {
        if self.pure_fns.contains(name) || name == VM_LAMBDA_MARKER {
            return true;
        }
        if name.starts_with("_lambda_") {
            return self.lambda_is_pure(name);
        }
        false
    }

    fn lambda_is_pure(&mut self, name: &str) -> bool {
        if let Some(&r) = self.lambda_memo.get(name) {
            return r;
        }
        if self.lambda_in_progress.contains(name) {
            return true;
        }
        let Some(body) = self.find_lambda_body(name).cloned() else {
            self.lambda_memo.insert(name.to_string(), false);
            return false;
        };
        self.lambda_in_progress.insert(name.to_string());
        let saved = std::mem::take(&mut self.bound);
        let r = self.safe(&body);
        self.bound = saved;
        self.lambda_in_progress.remove(name);
        self.lambda_memo.insert(name.to_string(), r);
        r
    }

    fn find_lambda_body(&self, name: &str) -> Option<&Expr> {
        self.program_body.iter().find_map(|e| match e {
            Expr::FuncDecl {
                name: n,
                attrs: _,
                type_params: _,
                params: _,
                return_type: _,
                body: b,
                ..
            } if n == name => Some(b.as_ref()),
            _ => None,
        })
    }

    pub(super) fn safe(&mut self, expr: &Expr) -> bool {
        use Expr::*;
        match expr {
            Int { .. }
            | Float { .. }
            | Char { .. }
            | Bool { .. }
            | String { .. }
            | Nil(_)
            | Var { .. }
            | Continue(_)
            | TypeDef(_)
            | Struct { .. }
            | Union { .. }
            | Enum { .. }
            | GlobalVar { .. }
            | ExternVar { .. }
            | FuncDecl { .. } => true,
            Break { value: v, .. } => v.as_ref().map(|v| self.safe(v)).unwrap_or(true),

            Call {
                callee,
                type_args: _,
                args,
                ..
            } => {
                match callee.as_ref() {
                    Var { name, .. } => {
                        let bound_target = self.lookup_bound(name).cloned();
                        let pure = match &bound_target {
                            Some(t) => self.callee_pure(t),
                            None => self.callee_pure(name),
                        };
                        if !pure {
                            return false;
                        }
                    }

                    _ => return false,
                }
                args.iter().all(|a| self.safe(a))
            }

            Block { stmts, .. } => {
                self.enter_scope();
                let r = stmts.iter().all(|s| self.safe(s));
                self.leave_scope();
                r
            }
            If {
                cond: c,
                then_branch: t,
                else_branch: e,
                ..
            } => {
                if !self.safe(c) {
                    return false;
                }
                self.enter_scope();
                let r_t = self.safe(t);
                self.leave_scope();
                if !r_t {
                    return false;
                }
                match e {
                    Some(x) => {
                        self.enter_scope();
                        let r = self.safe(x);
                        self.leave_scope();
                        r
                    }
                    None => true,
                }
            }
            While {
                cond: c, body: b, ..
            } => {
                if !self.safe(c) {
                    return false;
                }
                self.enter_scope();
                let r = self.safe(b);
                self.leave_scope();
                r
            }
            For {
                var,
                iterable,
                body,
                ..
            } => {
                if !self.safe(iterable) {
                    return false;
                }
                self.enter_scope();
                self.unbind(var);
                let r = self.safe(body);
                self.leave_scope();
                r
            }
            Range { start, end, .. } => self.safe(start) && self.safe(end),
            Match {
                target: s,
                branches: arms,
                default: d,
                ..
            } => {
                if !self.safe(s) {
                    return false;
                }
                for (pat, guard, arm) in arms {
                    if !self.safe(pat) {
                        return false;
                    }
                    if let Some(guard) = guard {
                        if !self.safe(guard) {
                            return false;
                        }
                    }
                    self.enter_scope();
                    let r = self.safe(arm);
                    self.leave_scope();
                    if !r {
                        return false;
                    }
                }
                match d {
                    Some(x) => {
                        self.enter_scope();
                        let r = self.safe(x);
                        self.leave_scope();
                        r
                    }
                    None => true,
                }
            }
            Return { value: v, .. } => self.safe(v),
            Lambda {
                params: _, body: b, ..
            } => {
                let saved = std::mem::take(&mut self.bound);
                self.bound = vec![HashMap::new()];
                let r = self.safe(b);
                self.bound = saved;
                r
            }
            VarDecl {
                name,
                ty: _,
                value: v,
                ..
            }
            | ConstDecl {
                name,
                ty: _,
                value: v,
                ..
            } => {
                let r = self.safe(v);
                if r {
                    self.bind(name, v);
                }
                r
            }
            Not { expr: v, .. }
            | BNot { expr: v, .. }
            | Neg { expr: v, .. }
            | FNeg { expr: v, .. } => self.safe(v),
            AddressOf { .. } => false,
            Deref { .. } => false,
            VarAssign { name, value: v, .. }
            | AddAssign { name, value: v, .. }
            | SubAssign { name, value: v, .. }
            | MulAssign { name, value: v, .. }
            | DivAssign { name, value: v, .. }
            | ModAssign { name, value: v, .. }
            | AndAssign { name, value: v, .. }
            | OrAssign { name, value: v, .. }
            | XorAssign { name, value: v, .. }
            | ShlAssign { name, value: v, .. }
            | ShrAssign { name, value: v, .. } => {
                let r = self.safe(v);
                if r {
                    match v.as_ref() {
                        Var { .. } | Lambda { .. } => self.bind(name, v),
                        _ => self.unbind(name),
                    }
                }
                r
            }
            Inc { .. } | Dec { .. } => true,
            IndexAssign { .. } => false,
            Add {
                left: l, right: r, ..
            }
            | Sub {
                left: l, right: r, ..
            }
            | Mul {
                left: l, right: r, ..
            }
            | Div {
                left: l, right: r, ..
            }
            | Mod {
                left: l, right: r, ..
            }
            | FAdd {
                left: l, right: r, ..
            }
            | FSub {
                left: l, right: r, ..
            }
            | FMul {
                left: l, right: r, ..
            }
            | FDiv {
                left: l, right: r, ..
            }
            | Eq {
                left: l, right: r, ..
            }
            | Ne {
                left: l, right: r, ..
            }
            | Lt {
                left: l, right: r, ..
            }
            | Le {
                left: l, right: r, ..
            }
            | Gt {
                left: l, right: r, ..
            }
            | Ge {
                left: l, right: r, ..
            }
            | FEq {
                left: l, right: r, ..
            }
            | FNe {
                left: l, right: r, ..
            }
            | FLt {
                left: l, right: r, ..
            }
            | FLe {
                left: l, right: r, ..
            }
            | FGt {
                left: l, right: r, ..
            }
            | FGe {
                left: l, right: r, ..
            }
            | Xor {
                left: l, right: r, ..
            }
            | BAnd {
                left: l, right: r, ..
            }
            | BOr {
                left: l, right: r, ..
            }
            | LAnd {
                left: l, right: r, ..
            }
            | LOr {
                left: l, right: r, ..
            }
            | Shl {
                left: l, right: r, ..
            }
            | Shr {
                left: l, right: r, ..
            }
            | StrCat {
                left: l, right: r, ..
            } => self.safe(l) && self.safe(r),
            DerefAssign { .. } => false,
            Index {
                array: l, index: r, ..
            } => self.safe(l) && self.safe(r),
            ArrayLiteral {
                elements: items, ..
            } => items.iter().all(|it| self.safe(it)),
            ArrayFill {
                elem_type: _, len, ..
            } => self.safe(len),
            StructLiteral {
                name: _,
                type_args: _,
                fields,
                ..
            } => fields.iter().all(|(_, v)| self.safe(v)),
            UnionLiteral {
                name: _,
                type_args: _,
                fields,
                ..
            } => fields.iter().all(|(_, v)| self.safe(v)),
            MemberAccess { obj, .. } => self.safe(obj),
            MemberAssign {
                obj,
                field: _,
                value: val,
                ..
            } => self.safe(obj) && self.safe(val),
            FString { segs: parts, .. } => parts.iter().all(|p| self.safe(p)),
            Cast { expr: inner, .. } => self.safe(inner),
        }
    }
}
