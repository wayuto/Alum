use crate::compiler::ast::{Expr, Program};

pub struct Optimizer;

impl Optimizer {
    pub fn new() -> Self {
        Self
    }

    pub fn optimize(&self, program: &mut Program) {
        for expr in &mut program.body {
            self.optimize_expr(expr);
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
            Expr::Stmt(body) => body.iter_mut().for_each(|e| self.optimize_expr(e)),
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
            Expr::For(_, s, e, body) => {
                self.optimize_expr(s);
                self.optimize_expr(e);
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
                _ => None,
            },
            Expr::Sub(l, r) => match (l.as_ref(), r.as_ref()) {
                (Expr::Int(a), Expr::Int(b)) => Some(Expr::Int(a - b)),
                (Expr::Float(a), Expr::Float(b)) => Some(Expr::Float(a - b)),
                _ => None,
            },
            Expr::Mul(l, r) => match (l.as_ref(), r.as_ref()) {
                (Expr::Int(a), Expr::Int(b)) => Some(Expr::Int(a * b)),
                (Expr::Float(a), Expr::Float(b)) => Some(Expr::Float(a * b)),
                _ => None,
            },
            Expr::Div(l, r) => match (l.as_ref(), r.as_ref()) {
                (Expr::Int(a), Expr::Int(b)) => Some(Expr::Int(a / b)),
                (Expr::Float(a), Expr::Float(b)) => Some(Expr::Float(a / b)),
                _ => None,
            },
            Expr::Mod(l, r) => match (l.as_ref(), r.as_ref()) {
                (Expr::Int(a), Expr::Int(b)) => Some(Expr::Int(a % b)),
                _ => None,
            },
            Expr::FAdd(l, r) => match (l.as_ref(), r.as_ref()) {
                (Expr::Float(a), Expr::Float(b)) => Some(Expr::Float(a + b)),
                _ => None,
            },
            Expr::FSub(l, r) => match (l.as_ref(), r.as_ref()) {
                (Expr::Float(a), Expr::Float(b)) => Some(Expr::Float(a - b)),
                _ => None,
            },
            Expr::FMul(l, r) => match (l.as_ref(), r.as_ref()) {
                (Expr::Float(a), Expr::Float(b)) => Some(Expr::Float(a * b)),
                _ => None,
            },
            Expr::FDiv(l, r) => match (l.as_ref(), r.as_ref()) {
                (Expr::Float(a), Expr::Float(b)) => Some(Expr::Float(a / b)),
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
}
