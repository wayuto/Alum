use crate::compiler::ast::{Expr, Program};

pub struct Optimizer {}

impl Optimizer {
    pub fn new() -> Self {
        Self {}
    }

    pub fn optimize(&self, program: &mut Program) {
        for expr in &mut program.body {
            self.optimize_expr(expr);
        }

        for expr in &mut program.body {
            self.dce(expr);
        }
    }

    fn optimize_expr(&self, expr: &mut Expr) {
        self.visit(expr);
        if let Some(folded) = self.fold(expr) {
            *expr = folded;
        }
    }

    fn visit(&self, expr: &mut Expr) {
        match expr {
            Expr::Block(body) => body.iter_mut().for_each(|e| self.optimize_expr(e)),
            Expr::FuncDecl(_, _, _, body) => self.optimize_expr(body),
            Expr::If(cond, t, e) => {
                self.optimize_expr(cond);
                self.optimize_expr(t);
                if let Some(e) = e {
                    self.optimize_expr(e);
                }
            }
            Expr::While(cond, body) => {
                self.optimize_expr(cond);
                self.optimize_expr(body);
            }
            Expr::For(_, array, body) => {
                self.optimize_expr(array);
                self.optimize_expr(body);
            }
            Expr::VarDecl(_, _, v) | Expr::VarAssign(_, v) | Expr::Return(v) => {
                self.optimize_expr(v)
            }
            Expr::Call(f, args) => {
                self.optimize_expr(f);
                args.iter_mut().for_each(|a| self.optimize_expr(a));
            }
            Expr::ArrayLiteral(elems) => elems.iter_mut().for_each(|e| self.optimize_expr(e)),
            Expr::ArrayFill(_, len) => self.optimize_expr(len),
            Expr::Index(arr, idx) => {
                self.optimize_expr(arr);
                self.optimize_expr(idx);
            }
            Expr::IndexAssign(arr_idx, _) => self.optimize_expr(arr_idx),
            Expr::StructLiteral(_, fields) => {
                fields.iter_mut().for_each(|(_, v)| self.optimize_expr(v));
            }
            Expr::MemberAccess(obj, _) => self.optimize_expr(obj),
            Expr::MemberAssign(obj, _, val) => {
                self.optimize_expr(obj);
                self.optimize_expr(val);
            }
            Expr::AddressOf(expr) => self.optimize_expr(expr),
            Expr::Deref(expr) => self.optimize_expr(expr),
            Expr::DerefAssign(ptr, val) => {
                self.optimize_expr(ptr);
                self.optimize_expr(val);
            }
            Expr::Add(l, r)
            | Expr::Sub(l, r)
            | Expr::Mul(l, r)
            | Expr::Div(l, r)
            | Expr::Mod(l, r)
            | Expr::FAdd(l, r)
            | Expr::FSub(l, r)
            | Expr::FMul(l, r)
            | Expr::FDiv(l, r)
            | Expr::Eq(l, r)
            | Expr::Ne(l, r)
            | Expr::Lt(l, r)
            | Expr::Le(l, r)
            | Expr::Gt(l, r)
            | Expr::Ge(l, r)
            | Expr::FEq(l, r)
            | Expr::FNe(l, r)
            | Expr::FLt(l, r)
            | Expr::FLe(l, r)
            | Expr::FGt(l, r)
            | Expr::FGe(l, r)
            | Expr::And(l, r)
            | Expr::Or(l, r)
            | Expr::StrCat(l, r) => {
                self.optimize_expr(l);
                self.optimize_expr(r);
            }
            Expr::Not(e) => self.optimize_expr(e),
            _ => {}
        }
    }

    fn fold(&self, expr: &Expr) -> Option<Expr> {
        match expr {
            Expr::Add(l, r) => match (l.as_ref(), r.as_ref()) {
                (Expr::Int(a), Expr::Int(b)) => Some(Expr::Int(a + b)),
                (Expr::Float(a), Expr::Float(b)) => Some(Expr::Float(a + b)),

                (Expr::Int(0), _) => Some(*r.clone()),
                (_, Expr::Int(0)) => Some(*l.clone()),
                (Expr::Float(0.0), _) => Some(*r.clone()),
                (_, Expr::Float(0.0)) => Some(*l.clone()),
                _ => None,
            },
            Expr::Sub(l, r) => match (l.as_ref(), r.as_ref()) {
                (Expr::Int(a), Expr::Int(b)) => Some(Expr::Int(a - b)),
                (Expr::Float(a), Expr::Float(b)) => Some(Expr::Float(a - b)),

                (_, Expr::Int(0)) => Some(*l.clone()),
                (_, Expr::Float(0.0)) => Some(*l.clone()),
                _ => None,
            },
            Expr::Mul(l, r) => match (l.as_ref(), r.as_ref()) {
                (Expr::Int(a), Expr::Int(b)) => Some(Expr::Int(a * b)),
                (Expr::Float(a), Expr::Float(b)) => Some(Expr::Float(a * b)),

                (Expr::Int(0), _) => Some(Expr::Int(0)),
                (_, Expr::Int(0)) => Some(Expr::Int(0)),
                (Expr::Float(0.0), _) => Some(Expr::Float(0.0)),
                (_, Expr::Float(0.0)) => Some(Expr::Float(0.0)),
                (Expr::Int(1), _) => Some(*r.clone()),
                (_, Expr::Int(1)) => Some(*l.clone()),
                (Expr::Float(1.0), _) => Some(*r.clone()),
                (_, Expr::Float(1.0)) => Some(*l.clone()),
                _ => None,
            },
            Expr::Div(l, r) => match (l.as_ref(), r.as_ref()) {
                (Expr::Int(a), Expr::Int(b)) => Some(Expr::Int(a / b)),
                (Expr::Float(a), Expr::Float(b)) => Some(Expr::Float(a / b)),

                (_, Expr::Int(1)) => Some(*l.clone()),
                (_, Expr::Float(1.0)) => Some(*l.clone()),
                _ => None,
            },
            Expr::Mod(l, r) => match (l.as_ref(), r.as_ref()) {
                (Expr::Int(a), Expr::Int(b)) => Some(Expr::Int(a % b)),
                _ => None,
            },
            Expr::FAdd(l, r) => match (l.as_ref(), r.as_ref()) {
                (Expr::Float(a), Expr::Float(b)) => Some(Expr::Float(a + b)),
                (Expr::Float(0.0), _) => Some(*r.clone()),
                (_, Expr::Float(0.0)) => Some(*l.clone()),
                _ => None,
            },
            Expr::FSub(l, r) => match (l.as_ref(), r.as_ref()) {
                (Expr::Float(a), Expr::Float(b)) => Some(Expr::Float(a - b)),
                (_, Expr::Float(0.0)) => Some(*l.clone()),
                _ => None,
            },
            Expr::FMul(l, r) => match (l.as_ref(), r.as_ref()) {
                (Expr::Float(a), Expr::Float(b)) => Some(Expr::Float(a * b)),
                (Expr::Float(0.0), _) => Some(Expr::Float(0.0)),
                (_, Expr::Float(0.0)) => Some(Expr::Float(0.0)),
                (Expr::Float(1.0), _) => Some(*r.clone()),
                (_, Expr::Float(1.0)) => Some(*l.clone()),
                _ => None,
            },
            Expr::FDiv(l, r) => match (l.as_ref(), r.as_ref()) {
                (Expr::Float(a), Expr::Float(b)) => Some(Expr::Float(a / b)),
                (_, Expr::Float(1.0)) => Some(*l.clone()),
                _ => None,
            },
            Expr::Eq(l, r) => match (l.as_ref(), r.as_ref()) {
                (Expr::Int(a), Expr::Int(b)) => Some(Expr::Bool(a == b)),
                (Expr::Bool(a), Expr::Bool(b)) => Some(Expr::Bool(a == b)),
                _ => None,
            },
            Expr::Ne(l, r) => match (l.as_ref(), r.as_ref()) {
                (Expr::Int(a), Expr::Int(b)) => Some(Expr::Bool(a != b)),
                (Expr::Bool(a), Expr::Bool(b)) => Some(Expr::Bool(a != b)),
                _ => None,
            },
            Expr::Lt(l, r) => match (l.as_ref(), r.as_ref()) {
                (Expr::Int(a), Expr::Int(b)) => Some(Expr::Bool(a < b)),
                _ => None,
            },
            Expr::Le(l, r) => match (l.as_ref(), r.as_ref()) {
                (Expr::Int(a), Expr::Int(b)) => Some(Expr::Bool(a <= b)),
                _ => None,
            },
            Expr::Gt(l, r) => match (l.as_ref(), r.as_ref()) {
                (Expr::Int(a), Expr::Int(b)) => Some(Expr::Bool(a > b)),
                _ => None,
            },
            Expr::Ge(l, r) => match (l.as_ref(), r.as_ref()) {
                (Expr::Int(a), Expr::Int(b)) => Some(Expr::Bool(a >= b)),
                _ => None,
            },
            Expr::FEq(l, r) => match (l.as_ref(), r.as_ref()) {
                (Expr::Float(a), Expr::Float(b)) => Some(Expr::Bool(a == b)),
                _ => None,
            },
            Expr::FNe(l, r) => match (l.as_ref(), r.as_ref()) {
                (Expr::Float(a), Expr::Float(b)) => Some(Expr::Bool(a != b)),
                _ => None,
            },
            Expr::FLt(l, r) => match (l.as_ref(), r.as_ref()) {
                (Expr::Float(a), Expr::Float(b)) => Some(Expr::Bool(a < b)),
                _ => None,
            },
            Expr::FLe(l, r) => match (l.as_ref(), r.as_ref()) {
                (Expr::Float(a), Expr::Float(b)) => Some(Expr::Bool(a <= b)),
                _ => None,
            },
            Expr::FGt(l, r) => match (l.as_ref(), r.as_ref()) {
                (Expr::Float(a), Expr::Float(b)) => Some(Expr::Bool(a > b)),
                _ => None,
            },
            Expr::FGe(l, r) => match (l.as_ref(), r.as_ref()) {
                (Expr::Float(a), Expr::Float(b)) => Some(Expr::Bool(a >= b)),
                _ => None,
            },
            Expr::And(l, r) => match (l.as_ref(), r.as_ref()) {
                (Expr::Bool(true), e) => Some(e.clone()),
                (Expr::Bool(false), _) => Some(Expr::Bool(false)),
                (e, Expr::Bool(true)) => Some(e.clone()),
                (_, Expr::Bool(false)) => Some(Expr::Bool(false)),
                _ => None,
            },
            Expr::Or(l, r) => match (l.as_ref(), r.as_ref()) {
                (Expr::Bool(true), _) => Some(Expr::Bool(true)),
                (Expr::Bool(false), e) => Some(e.clone()),
                (_, Expr::Bool(true)) => Some(Expr::Bool(true)),
                (e, Expr::Bool(false)) => Some(e.clone()),
                _ => None,
            },
            Expr::Not(e) => match e.as_ref() {
                Expr::Bool(b) => Some(Expr::Bool(!b)),
                _ => None,
            },
            _ => None,
        }
    }

    fn dce(&self, expr: &mut Expr) {
        match expr {
            Expr::Block(body) => {
                body.retain(|e| !self.is_pure_dead(e));
                for e in &mut *body {
                    self.dce(e);
                }
                body.retain(|e| !matches!(e, Expr::Block(b) if b.is_empty()));
            }
            Expr::If(cond, t, e) => {
                self.dce(cond);
                self.dce(t);
                if let Some(e) = e {
                    self.dce(e);
                }
                if let Expr::Bool(true) = cond.as_ref() {
                    *expr = *t.clone();
                } else if let Expr::Bool(false) = cond.as_ref() {
                    if let Some(else_expr) = e {
                        *expr = *else_expr.clone();
                    } else {
                        *expr = Expr::Block(vec![]);
                    }
                }
            }
            Expr::While(cond, body) => {
                self.dce(cond);
                self.dce(body);
                if let Expr::Bool(false) = cond.as_ref() {
                    *expr = Expr::Block(vec![]);
                }
            }
            Expr::For(_, array, body) => {
                self.dce(array);
                self.dce(body);
            }
            Expr::FuncDecl(_, _, _, body) => self.dce(body),
            Expr::VarDecl(_, _, v) => self.dce(v),
            Expr::VarAssign(_, v) => self.dce(v),
            Expr::Return(v) => self.dce(v),
            Expr::Call(f, args) => {
                self.dce(f);
                for a in args {
                    self.dce(a);
                }
            }
            Expr::ArrayLiteral(elems) => {
                for e in elems {
                    self.dce(e);
                }
            }
            Expr::ArrayFill(_, len) => self.dce(len),
            Expr::Index(arr, idx) => {
                self.dce(arr);
                self.dce(idx);
            }
            Expr::IndexAssign(arr_idx, v) => {
                self.dce(arr_idx);
                self.dce(v);
            }
            Expr::StructLiteral(_, fields) => {
                for (_, v) in fields {
                    self.dce(v);
                }
            }
            Expr::MemberAccess(obj, _) => self.dce(obj),
            Expr::MemberAssign(obj, _, val) => {
                self.dce(obj);
                self.dce(val);
            }
            Expr::AddressOf(expr) => self.dce(expr),
            Expr::Deref(expr) => self.dce(expr),
            Expr::DerefAssign(ptr, val) => {
                self.dce(ptr);
                self.dce(val);
            }
            Expr::Add(l, r)
            | Expr::Sub(l, r)
            | Expr::Mul(l, r)
            | Expr::Div(l, r)
            | Expr::Mod(l, r)
            | Expr::FAdd(l, r)
            | Expr::FSub(l, r)
            | Expr::FMul(l, r)
            | Expr::FDiv(l, r)
            | Expr::Eq(l, r)
            | Expr::Ne(l, r)
            | Expr::Lt(l, r)
            | Expr::Le(l, r)
            | Expr::Gt(l, r)
            | Expr::Ge(l, r)
            | Expr::FEq(l, r)
            | Expr::FNe(l, r)
            | Expr::FLt(l, r)
            | Expr::FLe(l, r)
            | Expr::FGt(l, r)
            | Expr::FGe(l, r)
            | Expr::And(l, r)
            | Expr::Or(l, r)
            | Expr::StrCat(l, r) => {
                self.dce(l);
                self.dce(r);
            }
            Expr::Not(e) => self.dce(e),
            _ => {}
        }
    }

    fn is_pure_dead(&self, expr: &Expr) -> bool {
        self.is_pure(expr) && !self.has_side_effect(expr)
    }

    fn is_pure(&self, expr: &Expr) -> bool {
        match expr {
            Expr::Int(_) | Expr::Float(_) | Expr::Bool(_) | Expr::String(_) | Expr::Nil => true,
            Expr::Add(l, r)
            | Expr::Sub(l, r)
            | Expr::Mul(l, r)
            | Expr::Div(l, r)
            | Expr::Mod(l, r)
            | Expr::FAdd(l, r)
            | Expr::FSub(l, r)
            | Expr::FMul(l, r)
            | Expr::FDiv(l, r)
            | Expr::Eq(l, r)
            | Expr::Ne(l, r)
            | Expr::Lt(l, r)
            | Expr::Le(l, r)
            | Expr::Gt(l, r)
            | Expr::Ge(l, r)
            | Expr::FEq(l, r)
            | Expr::FNe(l, r)
            | Expr::FLt(l, r)
            | Expr::FLe(l, r)
            | Expr::FGt(l, r)
            | Expr::FGe(l, r)
            | Expr::And(l, r)
            | Expr::Or(l, r) => self.is_pure(l) && self.is_pure(r),
            Expr::Not(e) => self.is_pure(e),
            Expr::Index(l, r) => self.is_pure(l) && self.is_pure(r),
            Expr::ArrayLiteral(elems) => elems.iter().all(|e| self.is_pure(e)),
            Expr::ArrayFill(_, len) => self.is_pure(len),
            Expr::StructLiteral(_, fields) => fields.iter().all(|(_, v)| self.is_pure(v)),
            Expr::MemberAccess(obj, _) => self.is_pure(obj),
            Expr::AddressOf(expr) => self.is_pure(expr),
            Expr::Deref(expr) => self.is_pure(expr),
            Expr::DerefAssign(ptr, val) => self.is_pure(ptr) && self.is_pure(val),
            _ => false,
        }
    }

    fn has_side_effect(&self, expr: &Expr) -> bool {
        matches!(
            expr,
            Expr::VarDecl(_, _, _)
                | Expr::VarAssign(_, _)
                | Expr::Call(_, _)
                | Expr::IndexAssign(_, _)
                | Expr::MemberAssign(_, _, _)
                | Expr::DerefAssign(_, _)
                | Expr::Return(_)
                | Expr::Break
                | Expr::Continue
        )
    }
}
