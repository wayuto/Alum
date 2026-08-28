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
            Expr::Var(v, _) => {
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
            Expr::Lambda(_, body, _, _) => {
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
            Expr::FuncDecl(n, _, _, _, _, b, _) if n == name => Some(b.as_ref()),
            _ => None,
        })
    }

    pub(super) fn safe(&mut self, expr: &Expr) -> bool {
        use Expr::*;
        match expr {
            Int(..) | Float(..) | Bool(..) | String(..) | Nil(_) | Var(..) | Continue(_)
            | TypeDef(_) | Struct(..) | Union(..) | Enum(..) | GlobalVar(..) | ExternVar(..)
            | FuncDecl(..) => true,
            Break(v, _) => v.as_ref().map(|v| self.safe(v)).unwrap_or(true),

            Call(callee, _, args, _) => {
                match callee.as_ref() {
                    Var(name, _) => {
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

            Block(stmts, _) => {
                self.enter_scope();
                let r = stmts.iter().all(|s| self.safe(s));
                self.leave_scope();
                r
            }
            If(c, t, e, _) => {
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
            While(c, b, _) => {
                if !self.safe(c) {
                    return false;
                }
                self.enter_scope();
                let r = self.safe(b);
                self.leave_scope();
                r
            }
            For(var, iterable, body, _) => {
                if !self.safe(iterable) {
                    return false;
                }
                self.enter_scope();
                self.unbind(var);
                let r = self.safe(body);
                self.leave_scope();
                r
            }
            Range(start, end, _) => self.safe(start) && self.safe(end),
            Match(s, arms, d, _) => {
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
            Return(v, _) => self.safe(v),
            Lambda(_, b, _, _) => {
                let saved = std::mem::take(&mut self.bound);
                self.bound = vec![HashMap::new()];
                let r = self.safe(b);
                self.bound = saved;
                r
            }
            VarDecl(name, _, v, _) | ConstDecl(name, _, v, _, _) => {
                let r = self.safe(v);
                if r {
                    self.bind(name, v);
                }
                r
            }
            Not(v, _) | BNot(v, _) | Neg(v, _) | FNeg(v, _) => self.safe(v),
            AddressOf(..) => false,
            Deref(..) => false,
            VarAssign(name, v, _)
            | AddAssign(name, v, _)
            | SubAssign(name, v, _)
            | MulAssign(name, v, _)
            | DivAssign(name, v, _)
            | ModAssign(name, v, _)
            | AndAssign(name, v, _)
            | OrAssign(name, v, _)
            | XorAssign(name, v, _)
            | ShlAssign(name, v, _)
            | ShrAssign(name, v, _) => {
                let r = self.safe(v);
                if r {
                    match v.as_ref() {
                        Var(..) | Lambda(..) => self.bind(name, v),
                        _ => self.unbind(name),
                    }
                }
                r
            }
            Inc(..) | Dec(..) => true,
            IndexAssign(..) => false,
            Add(l, r, _)
            | Sub(l, r, _)
            | Mul(l, r, _)
            | Div(l, r, _)
            | Mod(l, r, _)
            | FAdd(l, r, _)
            | FSub(l, r, _)
            | FMul(l, r, _)
            | FDiv(l, r, _)
            | Eq(l, r, _)
            | Ne(l, r, _)
            | Lt(l, r, _)
            | Le(l, r, _)
            | Gt(l, r, _)
            | Ge(l, r, _)
            | FEq(l, r, _)
            | FNe(l, r, _)
            | FLt(l, r, _)
            | FLe(l, r, _)
            | FGt(l, r, _)
            | FGe(l, r, _)
            | Xor(l, r, _)
            | BAnd(l, r, _)
            | BOr(l, r, _)
            | LAnd(l, r, _)
            | LOr(l, r, _)
            | Shl(l, r, _)
            | Shr(l, r, _)
            | StrCat(l, r, _) => self.safe(l) && self.safe(r),
            DerefAssign(..) => false,
            Index(l, r, _) => self.safe(l) && self.safe(r),
            ArrayLiteral(items, _) => items.iter().all(|it| self.safe(it)),
            ArrayFill(_, len, _) => self.safe(len),
            StructLiteral(_, _, fields, _) => fields.iter().all(|(_, v)| self.safe(v)),
            UnionLiteral(_, _, fields, _) => fields.iter().all(|(_, v)| self.safe(v)),
            MemberAccess(obj, _, _) => self.safe(obj),
            MemberAssign(obj, _, val, _) => self.safe(obj) && self.safe(val),
            FString(parts, _) => parts.iter().all(|p| self.safe(p)),
            Cast(inner, _, _) => self.safe(inner),
        }
    }
}
