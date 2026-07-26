use crate::compiler::Span;
use crate::compiler::parser::{Expr, Program};

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
            Expr::Block(body, _) => body.iter_mut().for_each(|e| self.optimize_expr(e)),
            Expr::FuncDecl(_, _, _, body, _) => self.optimize_expr(body),
            Expr::If(cond, t, e, _) => {
                self.optimize_expr(cond);
                self.optimize_expr(t);
                if let Some(e) = e {
                    self.optimize_expr(e);
                }
            }
            Expr::While(cond, body, _) => {
                self.optimize_expr(cond);
                self.optimize_expr(body);
            }
            Expr::For(_, array, body, _) => {
                self.optimize_expr(array);
                self.optimize_expr(body);
            }
            Expr::VarDecl(_, _, v, _)
            | Expr::VarAssign(_, v, _)
            | Expr::Return(v, _)
            | Expr::AddAssign(_, v, _)
            | Expr::SubAssign(_, v, _) => self.optimize_expr(v),
            Expr::Inc(_, _) | Expr::Dec(_, _) => {}
            Expr::Call(f, args, _) => {
                self.optimize_expr(f);
                args.iter_mut().for_each(|a| self.optimize_expr(a));
            }
            Expr::ArrayLiteral(elems, _) => elems.iter_mut().for_each(|e| self.optimize_expr(e)),
            Expr::ArrayFill(_, len, _) => self.optimize_expr(len),
            Expr::Index(arr, idx, _) => {
                self.optimize_expr(arr);
                self.optimize_expr(idx);
            }
            Expr::IndexAssign(arr_idx, _, _) => self.optimize_expr(arr_idx),
            Expr::StructLiteral(_, fields, _) => {
                fields.iter_mut().for_each(|(_, v)| self.optimize_expr(v));
            }
            Expr::MemberAccess(obj, _, _) => self.optimize_expr(obj),
            Expr::MemberAssign(obj, _, val, _) => {
                self.optimize_expr(obj);
                self.optimize_expr(val);
            }
            Expr::AddressOf(expr, _) => self.optimize_expr(expr),
            Expr::Deref(expr, _) => self.optimize_expr(expr),
            Expr::DerefAssign(ptr, val, _) => {
                self.optimize_expr(ptr);
                self.optimize_expr(val);
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
                self.optimize_expr(l);
                self.optimize_expr(r);
            }
            Expr::Not(e, _) | Expr::Neg(e, _) | Expr::FNeg(e, _) => self.optimize_expr(e),
            _ => {}
        }
    }

    fn fold(&self, expr: &Expr) -> Option<Expr> {
        match expr {
            Expr::Add(l, r, _) => match (l.as_ref(), r.as_ref()) {
                (Expr::Int(a, _), Expr::Int(b, _)) => Some(Expr::Int(a + b, Span::new(0, 0))),
                (Expr::Float(a, _), Expr::Float(b, _)) => Some(Expr::Float(a + b, Span::new(0, 0))),

                (Expr::Int(0, _), _) => Some(*r.clone()),
                (_, Expr::Int(0, _)) => Some(*l.clone()),
                (Expr::Float(0.0, _), _) => Some(*r.clone()),
                (_, Expr::Float(0.0, _)) => Some(*l.clone()),
                _ => None,
            },
            Expr::Sub(l, r, _) => match (l.as_ref(), r.as_ref()) {
                (Expr::Int(a, _), Expr::Int(b, _)) => Some(Expr::Int(a - b, Span::new(0, 0))),
                (Expr::Float(a, _), Expr::Float(b, _)) => Some(Expr::Float(a - b, Span::new(0, 0))),

                (_, Expr::Int(0, _)) => Some(*l.clone()),
                (_, Expr::Float(0.0, _)) => Some(*l.clone()),
                _ => None,
            },
            Expr::Mul(l, r, _) => match (l.as_ref(), r.as_ref()) {
                (Expr::Int(a, _), Expr::Int(b, _)) => Some(Expr::Int(a * b, Span::new(0, 0))),
                (Expr::Float(a, _), Expr::Float(b, _)) => Some(Expr::Float(a * b, Span::new(0, 0))),

                (Expr::Int(0, _), _) => Some(Expr::Int(0, Span::new(0, 0))),
                (_, Expr::Int(0, _)) => Some(Expr::Int(0, Span::new(0, 0))),
                (Expr::Float(0.0, _), _) => Some(Expr::Float(0.0, Span::new(0, 0))),
                (_, Expr::Float(0.0, _)) => Some(Expr::Float(0.0, Span::new(0, 0))),
                (Expr::Int(1, _), _) => Some(*r.clone()),
                (_, Expr::Int(1, _)) => Some(*l.clone()),
                (Expr::Float(1.0, _), _) => Some(*r.clone()),
                (_, Expr::Float(1.0, _)) => Some(*l.clone()),
                _ => None,
            },
            Expr::Div(l, r, _) => match (l.as_ref(), r.as_ref()) {
                (Expr::Int(a, _), Expr::Int(b, _)) => Some(Expr::Int(a / b, Span::new(0, 0))),
                (Expr::Float(a, _), Expr::Float(b, _)) => Some(Expr::Float(a / b, Span::new(0, 0))),

                (_, Expr::Int(1, _)) => Some(*l.clone()),
                (_, Expr::Float(1.0, _)) => Some(*l.clone()),
                _ => None,
            },
            Expr::Mod(l, r, _) => match (l.as_ref(), r.as_ref()) {
                (Expr::Int(a, _), Expr::Int(b, _)) => Some(Expr::Int(a % b, Span::new(0, 0))),
                _ => None,
            },
            Expr::Xor(l, r, _) => match (l.as_ref(), r.as_ref()) {
                (Expr::Int(a, _), Expr::Int(b, _)) => Some(Expr::Int(a ^ b, Span::new(0, 0))),
                (Expr::Int(0, _), _) => Some(*r.clone()),
                (_, Expr::Int(0, _)) => Some(*l.clone()),
                _ => None,
            },
            Expr::FAdd(l, r, _) => match (l.as_ref(), r.as_ref()) {
                (Expr::Float(a, _), Expr::Float(b, _)) => Some(Expr::Float(a + b, Span::new(0, 0))),
                (Expr::Float(0.0, _), _) => Some(*r.clone()),
                (_, Expr::Float(0.0, _)) => Some(*l.clone()),
                _ => None,
            },
            Expr::FSub(l, r, _) => match (l.as_ref(), r.as_ref()) {
                (Expr::Float(a, _), Expr::Float(b, _)) => Some(Expr::Float(a - b, Span::new(0, 0))),
                (_, Expr::Float(0.0, _)) => Some(*l.clone()),
                _ => None,
            },
            Expr::FMul(l, r, _) => match (l.as_ref(), r.as_ref()) {
                (Expr::Float(a, _), Expr::Float(b, _)) => Some(Expr::Float(a * b, Span::new(0, 0))),
                (Expr::Float(0.0, _), _) => Some(Expr::Float(0.0, Span::new(0, 0))),
                (_, Expr::Float(0.0, _)) => Some(Expr::Float(0.0, Span::new(0, 0))),
                (Expr::Float(1.0, _), _) => Some(*r.clone()),
                (_, Expr::Float(1.0, _)) => Some(*l.clone()),
                _ => None,
            },
            Expr::FDiv(l, r, _) => match (l.as_ref(), r.as_ref()) {
                (Expr::Float(a, _), Expr::Float(b, _)) => Some(Expr::Float(a / b, Span::new(0, 0))),
                (_, Expr::Float(1.0, _)) => Some(*l.clone()),
                _ => None,
            },
            Expr::Eq(l, r, _) => match (l.as_ref(), r.as_ref()) {
                (Expr::Int(a, _), Expr::Int(b, _)) => Some(Expr::Bool(a == b, Span::new(0, 0))),
                (Expr::Bool(a, _), Expr::Bool(b, _)) => Some(Expr::Bool(a == b, Span::new(0, 0))),
                _ => None,
            },
            Expr::Ne(l, r, _) => match (l.as_ref(), r.as_ref()) {
                (Expr::Int(a, _), Expr::Int(b, _)) => Some(Expr::Bool(a != b, Span::new(0, 0))),
                (Expr::Bool(a, _), Expr::Bool(b, _)) => Some(Expr::Bool(a != b, Span::new(0, 0))),
                _ => None,
            },
            Expr::Lt(l, r, _) => match (l.as_ref(), r.as_ref()) {
                (Expr::Int(a, _), Expr::Int(b, _)) => Some(Expr::Bool(a < b, Span::new(0, 0))),
                _ => None,
            },
            Expr::Le(l, r, _) => match (l.as_ref(), r.as_ref()) {
                (Expr::Int(a, _), Expr::Int(b, _)) => Some(Expr::Bool(a <= b, Span::new(0, 0))),
                _ => None,
            },
            Expr::Gt(l, r, _) => match (l.as_ref(), r.as_ref()) {
                (Expr::Int(a, _), Expr::Int(b, _)) => Some(Expr::Bool(a > b, Span::new(0, 0))),
                _ => None,
            },
            Expr::Ge(l, r, _) => match (l.as_ref(), r.as_ref()) {
                (Expr::Int(a, _), Expr::Int(b, _)) => Some(Expr::Bool(a >= b, Span::new(0, 0))),
                _ => None,
            },
            Expr::FEq(l, r, _) => match (l.as_ref(), r.as_ref()) {
                (Expr::Float(a, _), Expr::Float(b, _)) => Some(Expr::Bool(a == b, Span::new(0, 0))),
                _ => None,
            },
            Expr::FNe(l, r, _) => match (l.as_ref(), r.as_ref()) {
                (Expr::Float(a, _), Expr::Float(b, _)) => Some(Expr::Bool(a != b, Span::new(0, 0))),
                _ => None,
            },
            Expr::FLt(l, r, _) => match (l.as_ref(), r.as_ref()) {
                (Expr::Float(a, _), Expr::Float(b, _)) => Some(Expr::Bool(a < b, Span::new(0, 0))),
                _ => None,
            },
            Expr::FLe(l, r, _) => match (l.as_ref(), r.as_ref()) {
                (Expr::Float(a, _), Expr::Float(b, _)) => Some(Expr::Bool(a <= b, Span::new(0, 0))),
                _ => None,
            },
            Expr::FGt(l, r, _) => match (l.as_ref(), r.as_ref()) {
                (Expr::Float(a, _), Expr::Float(b, _)) => Some(Expr::Bool(a > b, Span::new(0, 0))),
                _ => None,
            },
            Expr::FGe(l, r, _) => match (l.as_ref(), r.as_ref()) {
                (Expr::Float(a, _), Expr::Float(b, _)) => Some(Expr::Bool(a >= b, Span::new(0, 0))),
                _ => None,
            },
            Expr::LAnd(l, r, _) => match (l.as_ref(), r.as_ref()) {
                (Expr::Bool(true, _), e) => Some(e.clone()),
                (Expr::Bool(false, _), _) => Some(Expr::Bool(false, Span::new(0, 0))),
                (e, Expr::Bool(true, _)) => Some(e.clone()),
                (_, Expr::Bool(false, _)) => Some(Expr::Bool(false, Span::new(0, 0))),
                _ => None,
            },
            Expr::LOr(l, r, _) => match (l.as_ref(), r.as_ref()) {
                (Expr::Bool(true, _), _) => Some(Expr::Bool(true, Span::new(0, 0))),
                (Expr::Bool(false, _), e) => Some(e.clone()),
                (_, Expr::Bool(true, _)) => Some(Expr::Bool(true, Span::new(0, 0))),
                (e, Expr::Bool(false, _)) => Some(e.clone()),
                _ => None,
            },
            Expr::Not(e, _) => match e.as_ref() {
                Expr::Bool(b, _) => Some(Expr::Bool(!b, Span::new(0, 0))),
                _ => None,
            },
            Expr::Neg(e, _) => match e.as_ref() {
                Expr::Int(n, _) => Some(Expr::Int(-n, Span::new(0, 0))),
                _ => None,
            },
            Expr::FNeg(e, _) => match e.as_ref() {
                Expr::Float(n, _) => Some(Expr::Float(-n, Span::new(0, 0))),
                _ => None,
            },
            _ => None,
        }
    }

    fn dce(&self, expr: &mut Expr) {
        match expr {
            Expr::Block(body, _) => {
                body.retain(|e| !self.is_pure_dead(e));
                for e in &mut *body {
                    self.dce(e);
                }
                body.retain(|e| !matches!(e, Expr::Block(b, _) if b.is_empty()));
            }
            Expr::If(cond, t, e, _) => {
                self.dce(cond);
                self.dce(t);
                if let Some(e) = e {
                    self.dce(e);
                }
                if let Expr::Bool(true, _) = cond.as_ref() {
                    *expr = *t.clone();
                } else if let Expr::Bool(false, _) = cond.as_ref() {
                    if let Some(else_expr) = e {
                        *expr = *else_expr.clone();
                    } else {
                        *expr = Expr::Block(vec![], Span::new(0, 0));
                    }
                }
            }
            Expr::While(cond, body, _) => {
                self.dce(cond);
                self.dce(body);
                if let Expr::Bool(false, _) = cond.as_ref() {
                    *expr = Expr::Block(vec![], Span::new(0, 0));
                }
            }
            Expr::For(_, array, body, _) => {
                self.dce(array);
                self.dce(body);
            }
            Expr::FuncDecl(_, _, _, body, _) => self.dce(body),
            Expr::VarDecl(_, _, v, _) => self.dce(v),
            Expr::VarAssign(_, v, _) | Expr::AddAssign(_, v, _) | Expr::SubAssign(_, v, _) => {
                self.dce(v)
            }
            Expr::Return(v, _) => self.dce(v),
            Expr::Inc(_, _) | Expr::Dec(_, _) => {}
            Expr::Call(f, args, _) => {
                self.dce(f);
                for a in args {
                    self.dce(a);
                }
            }
            Expr::ArrayLiteral(elems, _) => {
                for e in elems {
                    self.dce(e);
                }
            }
            Expr::ArrayFill(_, len, _) => self.dce(len),
            Expr::Index(arr, idx, _) => {
                self.dce(arr);
                self.dce(idx);
            }
            Expr::IndexAssign(arr_idx, v, _) => {
                self.dce(arr_idx);
                self.dce(v);
            }
            Expr::StructLiteral(_, fields, _) => {
                for (_, v) in fields {
                    self.dce(v);
                }
            }
            Expr::MemberAccess(obj, _, _) => self.dce(obj),
            Expr::MemberAssign(obj, _, val, _) => {
                self.dce(obj);
                self.dce(val);
            }
            Expr::AddressOf(expr, _) => self.dce(expr),
            Expr::Deref(expr, _) => self.dce(expr),
            Expr::DerefAssign(ptr, val, _) => {
                self.dce(ptr);
                self.dce(val);
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
                self.dce(l);
                self.dce(r);
            }
            Expr::Not(e, _) | Expr::Neg(e, _) | Expr::FNeg(e, _) => self.dce(e),
            _ => {}
        }
    }

    fn is_pure_dead(&self, expr: &Expr) -> bool {
        self.is_pure(expr) && !self.has_side_effect(expr)
    }

    fn is_pure(&self, expr: &Expr) -> bool {
        match expr {
            Expr::Int(_, _)
            | Expr::Float(_, _)
            | Expr::Bool(_, _)
            | Expr::String(_, _)
            | Expr::Nil(_) => true,
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
            | Expr::LOr(l, r, _) => self.is_pure(l) && self.is_pure(r),
            Expr::Not(e, _) | Expr::Neg(e, _) | Expr::FNeg(e, _) => self.is_pure(e),
            Expr::Index(l, r, _) => self.is_pure(l) && self.is_pure(r),
            Expr::ArrayLiteral(elems, _) => elems.iter().all(|e| self.is_pure(e)),
            Expr::ArrayFill(_, len, _) => self.is_pure(len),
            Expr::StructLiteral(_, fields, _) => fields.iter().all(|(_, v)| self.is_pure(v)),
            Expr::MemberAccess(obj, _, _) => self.is_pure(obj),
            Expr::AddressOf(expr, _) => self.is_pure(expr),
            Expr::Deref(expr, _) => self.is_pure(expr),
            Expr::DerefAssign(ptr, val, _) => self.is_pure(ptr) && self.is_pure(val),
            _ => false,
        }
    }

    fn has_side_effect(&self, expr: &Expr) -> bool {
        matches!(
            expr,
            Expr::VarDecl(_, _, _, _)
                | Expr::VarAssign(_, _, _)
                | Expr::AddAssign(_, _, _)
                | Expr::SubAssign(_, _, _)
                | Expr::Inc(_, _)
                | Expr::Dec(_, _)
                | Expr::Call(_, _, _)
                | Expr::IndexAssign(_, _, _)
                | Expr::MemberAssign(_, _, _, _)
                | Expr::DerefAssign(_, _, _)
                | Expr::Return(_, _)
                | Expr::Break(_)
                | Expr::Continue(_)
        )
    }
}
