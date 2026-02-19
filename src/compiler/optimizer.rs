use crate::compiler::ast::{Expr, Program};
use std::collections::{HashMap, HashSet};

pub struct Optimizer {
    lib_mode: bool,
}

impl Optimizer {
    pub fn new() -> Self {
        Self { lib_mode: false }
    }

    pub fn new_lib() -> Self {
        Self { lib_mode: true }
    }

    pub fn optimize(&self, program: &mut Program) {
        for expr in &mut program.body {
            self.propagate_constants(expr, &mut HashMap::new());
        }

        for expr in &mut program.body {
            self.optimize_expr(expr);
        }

        for expr in &mut program.body {
            self.dce(expr);
        }

        if !self.lib_mode {
            self.eliminate_unused(program);
        }
    }

    fn propagate_constants(&self, expr: &mut Expr, consts: &mut HashMap<String, Expr>) {
        match expr {
            Expr::Stmt(body) => {
                for e in body {
                    self.propagate_constants(e, consts);
                }
            }
            Expr::VarDecl(name, _, value) => {
                self.propagate_constants(value, consts);
                if self.can_propagate(value) {
                    consts.insert(name.clone(), *value.clone());
                }
            }
            Expr::VarAssign(name, value) => {
                self.propagate_constants(value, consts);
                if self.can_propagate(value) {
                    consts.insert(name.clone(), *value.clone());
                } else {
                    consts.remove(name);
                }
            }
            Expr::Var(name) => {
                if let Some(const_val) = consts.get(name) {
                    *expr = const_val.clone();
                }
            }
            Expr::If(cond, t, e) => {
                self.propagate_constants(cond, consts);

                let mut t_consts = consts.clone();
                self.propagate_constants(t, &mut t_consts);
                if let Some(e) = e {
                    let mut e_consts = consts.clone();
                    self.propagate_constants(e, &mut e_consts);
                }
            }
            Expr::While(cond, body) => {
                self.propagate_constants(cond, consts);
                let mut body_consts = consts.clone();
                self.propagate_constants(body, &mut body_consts);
            }
            Expr::For(_, start, end, body) => {
                self.propagate_constants(start, consts);
                self.propagate_constants(end, consts);
                let mut body_consts = consts.clone();
                self.propagate_constants(body, &mut body_consts);
            }
            Expr::FuncDecl(_, _params, _, body) => {
                let mut func_consts = HashMap::new();
                self.propagate_constants(body, &mut func_consts);
            }
            _ => match expr {
                Expr::Add(l, r)
                | Expr::Sub(l, r)
                | Expr::Mul(l, r)
                | Expr::Div(l, r)
                | Expr::Mod(l, r)
                | Expr::Eq(l, r)
                | Expr::Ne(l, r)
                | Expr::Lt(l, r)
                | Expr::Le(l, r)
                | Expr::Gt(l, r)
                | Expr::Ge(l, r) => {
                    self.propagate_constants(l, consts);
                    self.propagate_constants(r, consts);
                }
                Expr::FAdd(l, r)
                | Expr::FSub(l, r)
                | Expr::FMul(l, r)
                | Expr::FDiv(l, r)
                | Expr::FEq(l, r)
                | Expr::FNe(l, r)
                | Expr::FLt(l, r)
                | Expr::FLe(l, r)
                | Expr::FGt(l, r)
                | Expr::FGe(l, r) => {
                    self.propagate_constants(l, consts);
                    self.propagate_constants(r, consts);
                }
                Expr::And(l, r) | Expr::Or(l, r) | Expr::Index(l, r) => {
                    self.propagate_constants(l, consts);
                    self.propagate_constants(r, consts);
                }
                Expr::Not(e) | Expr::ArrayFill(_, e) => {
                    self.propagate_constants(e, consts);
                }
                Expr::Call(func, args) => {
                    self.propagate_constants(func, consts);
                    for arg in args {
                        self.propagate_constants(arg, consts);
                    }
                }
                Expr::Return(e) => {
                    self.propagate_constants(e, consts);
                }
                Expr::ArrayLiteral(elems) => {
                    for e in elems {
                        self.propagate_constants(e, consts);
                    }
                }
                Expr::IndexAssign(arr, v) => {
                    self.propagate_constants(arr, consts);
                    self.propagate_constants(v, consts);
                }
                Expr::StructLiteral(_, fields) => {
                    for (_, v) in fields {
                        self.propagate_constants(v, consts);
                    }
                }
                Expr::MemberAccess(obj, _) => {
                    self.propagate_constants(obj, consts);
                }
                _ => {}
            },
        }
    }

    fn can_propagate(&self, expr: &Expr) -> bool {
        match expr {
            Expr::Int(_) | Expr::Float(_) | Expr::Bool(_) | Expr::String(_) => true,
            Expr::Add(l, r) | Expr::Sub(l, r) | Expr::Mul(l, r) => {
                self.can_propagate(l) && self.can_propagate(r)
            }
            Expr::Div(l, r) => {
                self.can_propagate(l)
                    && self.can_propagate(r)
                    && !matches!(r.as_ref(), Expr::Int(0) | Expr::Float(0.0))
            }
            _ => false,
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
            Expr::Stmt(body) => {
                body.retain(|e| !self.is_pure_dead(e));
                for e in &mut *body {
                    self.dce(e);
                }
                body.retain(|e| !matches!(e, Expr::Stmt(b) if b.is_empty()));
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
                        *expr = Expr::Stmt(vec![]);
                    }
                }
            }
            Expr::While(cond, body) => {
                self.dce(cond);
                self.dce(body);
                if let Expr::Bool(false) = cond.as_ref() {
                    *expr = Expr::Stmt(vec![]);
                }
            }
            Expr::For(_, s, e, body) => {
                self.dce(s);
                self.dce(e);
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
                | Expr::Return(_)
                | Expr::Break
                | Expr::Continue
        )
    }

    fn eliminate_unused(&self, program: &mut Program) {
        let mut func_used: HashSet<String> = HashSet::new();

        for expr in &program.body {
            self.collect_func_usage(expr, &mut func_used);
        }

        func_used.insert("main".to_string());

        program.body.retain(|expr| match expr {
            Expr::FuncDecl(name, _, _, _) | Expr::Extern(name, _, _) => {
                func_used.contains(name) || name == "main"
            }
            _ => true,
        });

        for expr in &mut program.body {
            if let Expr::FuncDecl(_, _, _, body) = expr {
                self.eliminate_unused_vars(body);
            }
        }
    }

    fn collect_func_usage(&self, expr: &Expr, used: &mut HashSet<String>) {
        match expr {
            Expr::Call(callee, args) => {
                if let Expr::Var(name) = callee.as_ref() {
                    used.insert(name.clone());
                }
                for arg in args {
                    if let Expr::Var(func_name) = arg {
                        used.insert(func_name.clone());
                    }
                    self.collect_func_usage(arg, used);
                }
            }
            Expr::Stmt(body) => {
                for e in body {
                    self.collect_func_usage(e, used);
                }
            }
            Expr::FuncDecl(_, _, _, body) | Expr::For(_, _, _, body) | Expr::While(_, body) => {
                self.collect_func_usage(body, used);
            }
            Expr::If(cond, t, e) => {
                self.collect_func_usage(cond, used);
                self.collect_func_usage(t, used);
                if let Some(e) = e {
                    self.collect_func_usage(e, used);
                }
            }
            Expr::VarDecl(_, _, v) => {
                self.collect_func_usage(v, used);
                if let Expr::Var(func_name) = v.as_ref() {
                    used.insert(func_name.clone());
                }
            }
            Expr::VarAssign(_, v) | Expr::Return(v) | Expr::Not(v) | Expr::ArrayFill(_, v) => {
                self.collect_func_usage(v, used);
                if let Expr::Var(func_name) = v.as_ref() {
                    used.insert(func_name.clone());
                }
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
            | Expr::Index(l, r)
            | Expr::StrCat(l, r) => {
                self.collect_func_usage(l, used);
                self.collect_func_usage(r, used);
            }
            Expr::ArrayLiteral(elems) => {
                for e in elems {
                    self.collect_func_usage(e, used);
                }
            }
            Expr::IndexAssign(arr, v) => {
                self.collect_func_usage(arr, used);
                self.collect_func_usage(v, used);
            }
            Expr::StructLiteral(_, fields) => {
                for (_, v) in fields {
                    self.collect_func_usage(v, used);
                }
            }
            Expr::MemberAccess(obj, _) => {
                self.collect_func_usage(obj, used);
            }
            _ => {}
        }
    }

    fn eliminate_unused_vars(&self, expr: &mut Expr) {
        match expr {
            Expr::Stmt(body) => {
                let mut used: HashSet<String> = HashSet::new();
                for e in &*body {
                    self.collect_var_usage(e, &mut used);
                }

                body.retain(|e| {
                    if let Expr::VarDecl(name, _, _) = e {
                        used.contains(name)
                    } else {
                        true
                    }
                });

                for e in body {
                    self.eliminate_unused_vars(e);
                }
            }
            Expr::If(cond, t, e) => {
                self.eliminate_unused_vars(cond);
                self.eliminate_unused_vars(t);
                if let Some(e) = e {
                    self.eliminate_unused_vars(e);
                }
            }
            Expr::While(cond, body) => {
                self.eliminate_unused_vars(cond);
                self.eliminate_unused_vars(body);
            }
            Expr::For(var, s, e, body) => {
                let mut used: HashSet<String> = HashSet::new();
                used.insert(var.clone());
                self.collect_var_usage(s, &mut used);
                self.collect_var_usage(e, &mut used);
                self.collect_var_usage(body, &mut used);

                self.eliminate_unused_vars(s);
                self.eliminate_unused_vars(e);
                self.eliminate_unused_vars(body);
            }
            Expr::VarDecl(_, _, v) => self.eliminate_unused_vars(v),
            Expr::VarAssign(_, v) => self.eliminate_unused_vars(v),
            Expr::Return(v) => self.eliminate_unused_vars(v),
            Expr::Call(f, args) => {
                self.eliminate_unused_vars(f);
                for a in args {
                    self.eliminate_unused_vars(a);
                }
            }
            Expr::ArrayLiteral(elems) => {
                for e in elems {
                    self.eliminate_unused_vars(e);
                }
            }
            Expr::ArrayFill(_, len) => self.eliminate_unused_vars(len),
            Expr::Index(arr, idx) => {
                self.eliminate_unused_vars(arr);
                self.eliminate_unused_vars(idx);
            }
            Expr::IndexAssign(arr, v) => {
                self.eliminate_unused_vars(arr);
                self.eliminate_unused_vars(v);
            }
            Expr::StructLiteral(_, fields) => {
                for (_, v) in fields {
                    self.eliminate_unused_vars(v);
                }
            }
            Expr::MemberAccess(obj, _) => self.eliminate_unused_vars(obj),
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
                self.eliminate_unused_vars(l);
                self.eliminate_unused_vars(r);
            }
            Expr::Not(e) => self.eliminate_unused_vars(e),
            _ => {}
        }
    }

    fn collect_var_usage(&self, expr: &Expr, used: &mut HashSet<String>) {
        match expr {
            Expr::Var(name) => {
                used.insert(name.clone());
            }
            Expr::Stmt(body) => {
                for e in body {
                    self.collect_var_usage(e, used);
                }
            }
            Expr::If(cond, t, e) => {
                self.collect_var_usage(cond, used);
                self.collect_var_usage(t, used);
                if let Some(e) = e {
                    self.collect_var_usage(e, used);
                }
            }
            Expr::While(cond, body) => {
                self.collect_var_usage(cond, used);
                self.collect_var_usage(body, used);
            }
            Expr::For(var, s, e, body) => {
                used.insert(var.clone());
                self.collect_var_usage(s, used);
                self.collect_var_usage(e, used);
                self.collect_var_usage(body, used);
            }
            Expr::VarDecl(_, _, v) | Expr::VarAssign(_, v) | Expr::Return(v) => {
                self.collect_var_usage(v, used);
            }
            Expr::Call(f, args) => {
                self.collect_var_usage(f, used);
                for a in args {
                    self.collect_var_usage(a, used);
                }
            }
            Expr::ArrayLiteral(elems) => {
                for e in elems {
                    self.collect_var_usage(e, used);
                }
            }
            Expr::ArrayFill(_, len) => self.collect_var_usage(len, used),
            Expr::Index(arr, idx) => {
                self.collect_var_usage(arr, used);
                self.collect_var_usage(idx, used);
            }
            Expr::IndexAssign(arr, v) => {
                self.collect_var_usage(arr, used);
                self.collect_var_usage(v, used);
            }
            Expr::StructLiteral(_, fields) => {
                for (_, v) in fields {
                    self.collect_var_usage(v, used);
                }
            }
            Expr::MemberAccess(obj, _) => self.collect_var_usage(obj, used),
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
                self.collect_var_usage(l, used);
                self.collect_var_usage(r, used);
            }
            Expr::Not(e) => self.collect_var_usage(e, used),
            _ => {}
        }
    }
}
