use crate::compiler::{
    Span,
    parser::{Expr, Program},
};
use std::collections::HashSet;

pub struct Optimizer {}

impl Optimizer {
    pub fn new() -> Self {
        Self {}
    }

    pub fn optimize(&self, program: &mut Program) {
        let pure_fns: HashSet<String> = program
            .body
            .iter()
            .filter_map(|e| match e {
                Expr::FuncDecl(name, attrs, _, _, _, _, _) if attrs.is_pure => Some(name.clone()),
                _ => None,
            })
            .collect();
        let fn_names: HashSet<String> = program
            .body
            .iter()
            .filter_map(|e| match e {
                Expr::FuncDecl(name, _, _, _, _, _, _) => Some(name.clone()),
                _ => None,
            })
            .collect();
        let const_names: HashSet<String> = program
            .body
            .iter()
            .filter_map(|e| match e {
                Expr::ConstDecl(name, _, _, _, _) => Some(name.clone()),
                _ => None,
            })
            .collect();

        for expr in &mut program.body {
            self.optimize_expr(expr);
        }

        loop {
            let removed = self.remove_unused_top(program, &fn_names, &const_names);
            if removed == 0 {
                break;
            }
        }

        for expr in &mut program.body {
            self.dce(expr, &pure_fns);
            self.remove_unused_locals(expr, &pure_fns);
        }
    }

    fn optimize_expr(&self, expr: &mut Expr) {
        self.visit(expr);
        if let Some(folded) = self.fold(expr) {
            *expr = folded;
        }
    }

    fn remove_unused_top(
        &self,
        program: &mut Program,
        fn_names: &HashSet<String>,
        const_names: &HashSet<String>,
    ) -> usize {
        let mut fn_used: HashSet<String> = HashSet::new();
        let mut const_used: HashSet<String> = HashSet::new();
        for expr in &program.body {
            self.collect_refs(expr, fn_names, const_names, &mut fn_used, &mut const_used);
        }
        let before = program.body.len();
        program.body.retain(|expr| match expr {
            Expr::FuncDecl(name, attrs, ..) => {
                if attrs.is_external || attrs.is_pub || name == "main" {
                    true
                } else {
                    fn_used.contains(name)
                }
            }
            Expr::ConstDecl(name, _, _, is_pub, _) => *is_pub || const_used.contains(name),
            Expr::GlobalVar(_, _, _, _, _) => true,
            _ => true,
        });
        before - program.body.len()
    }

    fn remove_unused_locals(&self, expr: &mut Expr, pure_fns: &HashSet<String>) {
        let mut used: HashSet<String> = HashSet::new();
        self.for_each_name(expr, &mut |n: &str| {
            used.insert(n.to_string());
        });
        self.prune_locals(expr, &used, pure_fns);
    }

    fn prune_locals(&self, expr: &mut Expr, used: &HashSet<String>, pure_fns: &HashSet<String>) {
        match expr {
            Expr::Block(body, _) => {
                let mut kept = Vec::new();
                for stmt in body.drain(..) {
                    let drop = match &stmt {
                        Expr::VarDecl(name, _, init, _) | Expr::ConstDecl(name, _, init, _, _) => {
                            !used.contains(name) && self.discardable(init, pure_fns)
                        }
                        _ => false,
                    };
                    if !drop {
                        kept.push(stmt);
                    }
                }
                *body = kept;
                for e in &mut *body {
                    self.prune_locals(e, used, pure_fns);
                }
            }
            Expr::If(cond, t, e, _) => {
                self.prune_locals(cond, used, pure_fns);
                self.prune_locals(t, used, pure_fns);
                if let Some(e) = e {
                    self.prune_locals(e, used, pure_fns);
                }
            }
            Expr::While(cond, body, _) => {
                self.prune_locals(cond, used, pure_fns);
                self.prune_locals(body, used, pure_fns);
            }
            Expr::For(_, array, body, _) => {
                self.prune_locals(array, used, pure_fns);
                self.prune_locals(body, used, pure_fns);
            }
            Expr::Lambda(_, body, _, _) => self.prune_locals(body, used, pure_fns),
            Expr::VarDecl(_, _, v, _)
            | Expr::ConstDecl(_, _, v, _, _)
            | Expr::VarAssign(_, v, _)
            | Expr::AddAssign(_, v, _)
            | Expr::SubAssign(_, v, _)
            | Expr::Return(v, _) => self.prune_locals(v, used, pure_fns),
            Expr::Call(f, _, args, _) => {
                self.prune_locals(f, used, pure_fns);
                for a in args {
                    self.prune_locals(a, used, pure_fns);
                }
            }
            Expr::IndexAssign(a, v, _) => {
                self.prune_locals(a, used, pure_fns);
                self.prune_locals(v, used, pure_fns);
            }
            Expr::MemberAssign(o, _, v, _) => {
                self.prune_locals(o, used, pure_fns);
                self.prune_locals(v, used, pure_fns);
            }
            Expr::DerefAssign(p, v, _) => {
                self.prune_locals(p, used, pure_fns);
                self.prune_locals(v, used, pure_fns);
            }
            Expr::ArrayLiteral(es, _) => {
                for e in es {
                    self.prune_locals(e, used, pure_fns);
                }
            }
            Expr::ArrayFill(_, len, _) => self.prune_locals(len, used, pure_fns),
            Expr::Index(arr, idx, _) => {
                self.prune_locals(arr, used, pure_fns);
                self.prune_locals(idx, used, pure_fns);
            }
            Expr::StructLiteral(_, _, fs, _) | Expr::UnionLiteral(_, _, fs, _) => {
                for (_, v) in fs {
                    self.prune_locals(v, used, pure_fns);
                }
            }
            Expr::MemberAccess(o, _, _) => self.prune_locals(o, used, pure_fns),
            Expr::AddressOf(e, _)
            | Expr::Deref(e, _)
            | Expr::Not(e, _)
            | Expr::Neg(e, _)
            | Expr::FNeg(e, _) => self.prune_locals(e, used, pure_fns),
            Expr::Match(t, br, default, _) => {
                self.prune_locals(t, used, pure_fns);
                for (c, r) in br {
                    self.prune_locals(c, used, pure_fns);
                    self.prune_locals(r, used, pure_fns);
                }
                if let Some(d) = default {
                    self.prune_locals(d, used, pure_fns);
                }
            }
            Expr::Range(s, e, _) => {
                self.prune_locals(s, used, pure_fns);
                self.prune_locals(e, used, pure_fns);
            }
            Expr::FString(segs, _) => {
                for seg in segs {
                    self.prune_locals(seg, used, pure_fns);
                }
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
                self.prune_locals(l, used, pure_fns);
                self.prune_locals(r, used, pure_fns);
            }
            _ => {}
        }
    }

    fn collect_refs(
        &self,
        expr: &Expr,
        fn_names: &HashSet<String>,
        const_names: &HashSet<String>,
        fn_used: &mut HashSet<String>,
        const_used: &mut HashSet<String>,
    ) {
        self.for_each_name(expr, &mut |n: &str| {
            if fn_names.contains(n) {
                fn_used.insert(n.to_string());
            }
            if const_names.contains(n) {
                const_used.insert(n.to_string());
            }
        });
    }

    fn for_each_name(&self, expr: &Expr, f: &mut dyn FnMut(&str)) {
        match expr {
            Expr::FuncDecl(_, _, _, _, _, body, _) => self.for_each_name(body, f),
            Expr::Var(name, _) | Expr::Inc(name, _) | Expr::Dec(name, _) => f(name),
            Expr::VarAssign(name, v, _)
            | Expr::AddAssign(name, v, _)
            | Expr::SubAssign(name, v, _) => {
                f(name);
                self.for_each_name(v, f);
            }
            Expr::VarDecl(_, _, v, _) | Expr::ConstDecl(_, _, v, _, _) | Expr::Return(v, _) => {
                self.for_each_name(v, f)
            }
            Expr::GlobalVar(_, _, _, v, _) => {
                if let Some(v) = v {
                    self.for_each_name(v, f);
                }
            }
            Expr::Call(callee, _, args, _) => {
                self.for_each_name(callee, f);
                for a in args {
                    self.for_each_name(a, f);
                }
            }
            Expr::Block(body, _) => {
                for e in body {
                    self.for_each_name(e, f);
                }
            }
            Expr::If(cond, t, e, _) => {
                self.for_each_name(cond, f);
                self.for_each_name(t, f);
                if let Some(e) = e {
                    self.for_each_name(e, f);
                }
            }
            Expr::While(cond, body, _) => {
                self.for_each_name(cond, f);
                self.for_each_name(body, f);
            }
            Expr::For(_, array, body, _) => {
                self.for_each_name(array, f);
                self.for_each_name(body, f);
            }
            Expr::Lambda(_, body, _, _) => self.for_each_name(body, f),
            Expr::Index(a, i, _) => {
                self.for_each_name(a, f);
                self.for_each_name(i, f);
            }
            Expr::IndexAssign(a, v, _) => {
                self.for_each_name(a, f);
                self.for_each_name(v, f);
            }
            Expr::ArrayLiteral(es, _) => {
                for e in es {
                    self.for_each_name(e, f);
                }
            }
            Expr::ArrayFill(_, len, _) => self.for_each_name(len, f),
            Expr::Range(s, e, _) => {
                self.for_each_name(s, f);
                self.for_each_name(e, f);
            }
            Expr::Match(t, br, default, _) => {
                self.for_each_name(t, f);
                for (c, r) in br {
                    self.for_each_name(c, f);
                    self.for_each_name(r, f);
                }
                if let Some(d) = default {
                    self.for_each_name(d, f);
                }
            }
            Expr::StructLiteral(_, _, fs, _) | Expr::UnionLiteral(_, _, fs, _) => {
                for (_, v) in fs {
                    self.for_each_name(v, f);
                }
            }
            Expr::MemberAccess(o, _, _) => self.for_each_name(o, f),
            Expr::MemberAssign(o, _, v, _) => {
                self.for_each_name(o, f);
                self.for_each_name(v, f);
            }
            Expr::AddressOf(e, _)
            | Expr::Deref(e, _)
            | Expr::Not(e, _)
            | Expr::Neg(e, _)
            | Expr::FNeg(e, _) => self.for_each_name(e, f),
            Expr::DerefAssign(p, v, _) => {
                self.for_each_name(p, f);
                self.for_each_name(v, f);
            }
            Expr::FString(segs, _) => {
                for seg in segs {
                    self.for_each_name(seg, f);
                }
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
                self.for_each_name(l, f);
                self.for_each_name(r, f);
            }
            _ => {}
        }
    }

    fn discardable(&self, expr: &Expr, pure_fns: &HashSet<String>) -> bool {
        match expr {
            Expr::Int(_, _)
            | Expr::Float(_, _)
            | Expr::Bool(_, _)
            | Expr::String(_, _)
            | Expr::Nil(_)
            | Expr::Var(_, _) => true,
            Expr::Call(callee, _, args, _) => match callee.as_ref() {
                Expr::Var(name, _) => {
                    pure_fns.contains(name) && args.iter().all(|a| self.discardable(a, pure_fns))
                }
                _ => false,
            },
            Expr::Index(l, r, _) => self.discardable(l, pure_fns) && self.discardable(r, pure_fns),
            Expr::ArrayLiteral(es, _) => es.iter().all(|e| self.discardable(e, pure_fns)),
            Expr::ArrayFill(_, len, _) => self.discardable(len, pure_fns),
            Expr::StructLiteral(_, _, fs, _) | Expr::UnionLiteral(_, _, fs, _) => {
                fs.iter().all(|(_, v)| self.discardable(v, pure_fns))
            }
            Expr::MemberAccess(o, _, _) => self.discardable(o, pure_fns),
            Expr::AddressOf(e, _)
            | Expr::Deref(e, _)
            | Expr::Not(e, _)
            | Expr::Neg(e, _)
            | Expr::FNeg(e, _) => self.discardable(e, pure_fns),
            Expr::Range(s, e, _) => self.discardable(s, pure_fns) && self.discardable(e, pure_fns),
            Expr::FString(segs, _) => segs.iter().all(|s| self.discardable(s, pure_fns)),
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
                self.discardable(l, pure_fns) && self.discardable(r, pure_fns)
            }
            _ => false,
        }
    }

    fn visit(&self, expr: &mut Expr) {
        match expr {
            Expr::Block(body, _) => body.iter_mut().for_each(|e| self.optimize_expr(e)),
            Expr::FuncDecl(_, _, _, _, _, body, _) => self.optimize_expr(body),
            Expr::Lambda(_, body, _, _) => self.optimize_expr(body),
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
            | Expr::ConstDecl(_, _, v, _, _)
            | Expr::VarAssign(_, v, _)
            | Expr::Return(v, _)
            | Expr::AddAssign(_, v, _)
            | Expr::SubAssign(_, v, _) => self.optimize_expr(v),
            Expr::GlobalVar(_, _, _, v, _) => {
                if let Some(v) = v {
                    self.optimize_expr(v);
                }
            }
            Expr::Inc(_, _) | Expr::Dec(_, _) => {}
            Expr::Call(f, _, args, _) => {
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
            Expr::StructLiteral(_, _, fields, _) => {
                fields.iter_mut().for_each(|(_, v)| self.optimize_expr(v));
            }
            Expr::UnionLiteral(_, _, fields, _) => {
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

    fn dce(&self, expr: &mut Expr, pure_fns: &HashSet<String>) {
        match expr {
            Expr::Block(body, _) => {
                let last = body.len().saturating_sub(1);
                *body = body
                    .drain(..)
                    .enumerate()
                    .filter(|(i, e)| *i == last || !self.is_pure_dead(e, pure_fns))
                    .map(|(_, e)| e)
                    .collect();
                for e in &mut *body {
                    self.dce(e, pure_fns);
                }
                body.retain(|e| !matches!(e, Expr::Block(b, _) if b.is_empty()));
            }
            Expr::If(cond, t, e, _) => {
                self.dce(cond, pure_fns);
                self.dce(t, pure_fns);
                if let Some(e) = e {
                    self.dce(e, pure_fns);
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
                self.dce(cond, pure_fns);
                self.dce(body, pure_fns);
                if let Expr::Bool(false, _) = cond.as_ref() {
                    *expr = Expr::Block(vec![], Span::new(0, 0));
                }
            }
            Expr::For(_, array, body, _) => {
                self.dce(array, pure_fns);
                self.dce(body, pure_fns);
            }
            Expr::FuncDecl(_, _, _, _, _, body, _) => self.dce(body, pure_fns),
            Expr::Lambda(_, body, _, _) => self.dce(body, pure_fns),
            Expr::VarDecl(_, _, v, _) => self.dce(v, pure_fns),
            Expr::ConstDecl(_, _, v, _, _) => self.dce(v, pure_fns),
            Expr::GlobalVar(_, _, _, v, _) => {
                if let Some(v) = v {
                    self.dce(v, pure_fns);
                }
            }
            Expr::VarAssign(_, v, _) | Expr::AddAssign(_, v, _) | Expr::SubAssign(_, v, _) => {
                self.dce(v, pure_fns)
            }
            Expr::Return(v, _) => self.dce(v, pure_fns),
            Expr::Inc(_, _) | Expr::Dec(_, _) => {}
            Expr::Call(f, _, args, _) => {
                self.dce(f, pure_fns);
                for a in args {
                    self.dce(a, pure_fns);
                }
            }
            Expr::ArrayLiteral(elems, _) => {
                for e in elems {
                    self.dce(e, pure_fns);
                }
            }
            Expr::ArrayFill(_, len, _) => self.dce(len, pure_fns),
            Expr::Index(arr, idx, _) => {
                self.dce(arr, pure_fns);
                self.dce(idx, pure_fns);
            }
            Expr::IndexAssign(arr_idx, v, _) => {
                self.dce(arr_idx, pure_fns);
                self.dce(v, pure_fns);
            }
            Expr::StructLiteral(_, _, fields, _) => {
                for (_, v) in fields {
                    self.dce(v, pure_fns);
                }
            }
            Expr::UnionLiteral(_, _, fields, _) => {
                for (_, v) in fields {
                    self.dce(v, pure_fns);
                }
            }
            Expr::MemberAccess(obj, _, _) => self.dce(obj, pure_fns),
            Expr::MemberAssign(obj, _, val, _) => {
                self.dce(obj, pure_fns);
                self.dce(val, pure_fns);
            }
            Expr::AddressOf(expr, _) => self.dce(expr, pure_fns),
            Expr::Deref(expr, _) => self.dce(expr, pure_fns),
            Expr::DerefAssign(ptr, val, _) => {
                self.dce(ptr, pure_fns);
                self.dce(val, pure_fns);
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
                self.dce(l, pure_fns);
                self.dce(r, pure_fns);
            }
            Expr::Not(e, _) | Expr::Neg(e, _) | Expr::FNeg(e, _) => self.dce(e, pure_fns),
            _ => {}
        }
    }

    fn is_pure_dead(&self, expr: &Expr, pure_fns: &HashSet<String>) -> bool {
        self.is_pure(expr, pure_fns) && !self.has_side_effect(expr, pure_fns)
    }

    fn is_pure(&self, expr: &Expr, pure_fns: &HashSet<String>) -> bool {
        match expr {
            Expr::Int(_, _)
            | Expr::Float(_, _)
            | Expr::Bool(_, _)
            | Expr::String(_, _)
            | Expr::Nil(_)
            | Expr::Var(_, _) => true,
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
            | Expr::LOr(l, r, _) => self.is_pure(l, pure_fns) && self.is_pure(r, pure_fns),
            Expr::Not(e, _) | Expr::Neg(e, _) | Expr::FNeg(e, _) => self.is_pure(e, pure_fns),
            Expr::Index(l, r, _) => self.is_pure(l, pure_fns) && self.is_pure(r, pure_fns),
            Expr::ArrayLiteral(elems, _) => elems.iter().all(|e| self.is_pure(e, pure_fns)),
            Expr::ArrayFill(_, len, _) => self.is_pure(len, pure_fns),
            Expr::StructLiteral(_, _, fields, _) => {
                fields.iter().all(|(_, v)| self.is_pure(v, pure_fns))
            }
            Expr::UnionLiteral(_, _, fields, _) => {
                fields.iter().all(|(_, v)| self.is_pure(v, pure_fns))
            }
            Expr::MemberAccess(obj, _, _) => self.is_pure(obj, pure_fns),
            Expr::AddressOf(expr, _) => self.is_pure(expr, pure_fns),
            Expr::Deref(expr, _) => self.is_pure(expr, pure_fns),
            Expr::DerefAssign(ptr, val, _) => {
                self.is_pure(ptr, pure_fns) && self.is_pure(val, pure_fns)
            }
            Expr::ConstDecl(_, _, v, _, _) => self.is_pure(v, pure_fns),
            Expr::GlobalVar(_, _, _, v, _) => match v {
                Some(v) => self.is_pure(v, pure_fns),
                None => true,
            },
            Expr::Call(callee, _, args, _) => match callee.as_ref() {
                Expr::Var(name, _) => {
                    pure_fns.contains(name) && args.iter().all(|a| self.is_pure(a, pure_fns))
                }
                _ => false,
            },
            _ => false,
        }
    }

    fn has_side_effect(&self, expr: &Expr, pure_fns: &HashSet<String>) -> bool {
        match expr {
            Expr::Call(callee, _, _, _) => match callee.as_ref() {
                Expr::Var(name, _) => !pure_fns.contains(name),
                _ => true,
            },
            _ => matches!(
                expr,
                Expr::VarDecl(_, _, _, _)
                    | Expr::VarAssign(_, _, _)
                    | Expr::ConstDecl(_, _, _, _, _)
                    | Expr::GlobalVar(_, _, _, _, _)
                    | Expr::AddAssign(_, _, _)
                    | Expr::SubAssign(_, _, _)
                    | Expr::Inc(_, _)
                    | Expr::Dec(_, _)
                    | Expr::IndexAssign(_, _, _)
                    | Expr::MemberAssign(_, _, _, _)
                    | Expr::DerefAssign(_, _, _)
                    | Expr::Return(_, _)
                    | Expr::Break(_)
                    | Expr::Continue(_)
            ),
        }
    }
}
