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
                Expr::FuncDecl { name, attrs, .. } if attrs.is_pure => Some(name.clone()),
                _ => None,
            })
            .collect();
        let fn_names: HashSet<String> = program
            .body
            .iter()
            .filter_map(|e| match e {
                Expr::FuncDecl { name, .. } => Some(name.clone()),
                _ => None,
            })
            .collect();
        let const_names: HashSet<String> = program
            .body
            .iter()
            .filter_map(|e| match e {
                Expr::ConstDecl { name, .. } => Some(name.clone()),
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
            Expr::FuncDecl { name, attrs, .. } => {
                if attrs.is_external || attrs.is_pub || name == "main" {
                    true
                } else {
                    fn_used.contains(name)
                }
            }
            Expr::ConstDecl {
                name,
                ty: _,
                value: init,
                is_pub,
                ..
            } => {
                *is_pub
                    || const_used.contains(name)
                    || matches!(init.as_ref(), Expr::FuncDecl { .. })
            }
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
            Expr::Block { stmts: body, .. } => {
                let mut kept = Vec::new();
                for stmt in body.drain(..) {
                    let drop = match &stmt {
                        Expr::VarDecl {
                            name,
                            ty: _,
                            value: init,
                            ..
                        }
                        | Expr::ConstDecl {
                            name,
                            ty: _,
                            value: init,
                            ..
                        } => !used.contains(name) && self.discardable(init, pure_fns),
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
            Expr::If {
                cond,
                then_branch: t,
                else_branch: e,
                ..
            } => {
                self.prune_locals(cond, used, pure_fns);
                self.prune_locals(t, used, pure_fns);
                if let Some(e) = e {
                    self.prune_locals(e, used, pure_fns);
                }
            }
            Expr::While { cond, body, .. } => {
                self.prune_locals(cond, used, pure_fns);
                self.prune_locals(body, used, pure_fns);
            }
            Expr::For {
                var: _,
                iterable: array,
                body,
                ..
            } => {
                self.prune_locals(array, used, pure_fns);
                self.prune_locals(body, used, pure_fns);
            }
            Expr::Lambda {
                params: _, body, ..
            } => self.prune_locals(body, used, pure_fns),
            Expr::FuncDecl {
                name: _,
                attrs: _,
                type_params: _,
                params: _,
                return_type: _,
                body,
                ..
            } => self.prune_locals(body, used, pure_fns),
            Expr::VarDecl {
                name: _,
                ty: _,
                value: v,
                ..
            }
            | Expr::ConstDecl {
                name: _,
                ty: _,
                value: v,
                ..
            }
            | Expr::VarAssign {
                name: _, value: v, ..
            }
            | Expr::AddAssign {
                name: _, value: v, ..
            }
            | Expr::SubAssign {
                name: _, value: v, ..
            }
            | Expr::MulAssign {
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
            }
            | Expr::Return { value: v, .. } => self.prune_locals(v, used, pure_fns),
            Expr::Call {
                callee: f,
                type_args: _,
                args,
                ..
            } => {
                self.prune_locals(f, used, pure_fns);
                for a in args {
                    self.prune_locals(a, used, pure_fns);
                }
            }
            Expr::IndexAssign {
                target: a,
                value: v,
                ..
            } => {
                self.prune_locals(a, used, pure_fns);
                self.prune_locals(v, used, pure_fns);
            }
            Expr::MemberAssign {
                obj: o,
                field: _,
                value: v,
                ..
            } => {
                self.prune_locals(o, used, pure_fns);
                self.prune_locals(v, used, pure_fns);
            }
            Expr::DerefAssign {
                ptr: p, value: v, ..
            } => {
                self.prune_locals(p, used, pure_fns);
                self.prune_locals(v, used, pure_fns);
            }
            Expr::Cast { expr: inner, .. } => self.prune_locals(inner, used, pure_fns),
            Expr::ArrayLiteral { elements: es, .. } => {
                for e in es {
                    self.prune_locals(e, used, pure_fns);
                }
            }
            Expr::ArrayFill {
                elem_type: _, len, ..
            } => self.prune_locals(len, used, pure_fns),
            Expr::Index {
                array: arr,
                index: idx,
                ..
            } => {
                self.prune_locals(arr, used, pure_fns);
                self.prune_locals(idx, used, pure_fns);
            }
            Expr::StructLiteral {
                name: _,
                type_args: _,
                fields: fs,
                ..
            }
            | Expr::UnionLiteral {
                name: _,
                type_args: _,
                fields: fs,
                ..
            } => {
                for (_, v) in fs {
                    self.prune_locals(v, used, pure_fns);
                }
            }
            Expr::MemberAccess { obj: o, .. } => self.prune_locals(o, used, pure_fns),
            Expr::AddressOf { expr: e, .. }
            | Expr::Deref { expr: e, .. }
            | Expr::Not { expr: e, .. }
            | Expr::Neg { expr: e, .. }
            | Expr::FNeg { expr: e, .. } => self.prune_locals(e, used, pure_fns),
            Expr::Match {
                target: t,
                branches: br,
                default,
                ..
            } => {
                self.prune_locals(t, used, pure_fns);
                for (c, g, r) in br {
                    self.prune_locals(c, used, pure_fns);
                    if let Some(g) = g {
                        self.prune_locals(g, used, pure_fns);
                    }
                    self.prune_locals(r, used, pure_fns);
                }
                if let Some(d) = default {
                    self.prune_locals(d, used, pure_fns);
                }
            }
            Expr::Range {
                start: s, end: e, ..
            } => {
                self.prune_locals(s, used, pure_fns);
                self.prune_locals(e, used, pure_fns);
            }
            Expr::FString { segs, .. } => {
                for seg in segs {
                    self.prune_locals(seg, used, pure_fns);
                }
            }
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
                self.prune_locals(l, used, pure_fns);
                self.prune_locals(r, used, pure_fns);
            }
            Expr::BNot { expr: e, .. } => self.prune_locals(e, used, pure_fns),
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
            Expr::FuncDecl {
                name: _,
                attrs: _,
                type_params: _,
                params: _,
                return_type: _,
                body,
                ..
            } => self.for_each_name(body, f),
            Expr::Var { name, .. } | Expr::Inc { name, .. } | Expr::Dec { name, .. } => f(name),
            Expr::VarAssign { name, value: v, .. }
            | Expr::AddAssign { name, value: v, .. }
            | Expr::SubAssign { name, value: v, .. }
            | Expr::MulAssign { name, value: v, .. }
            | Expr::DivAssign { name, value: v, .. }
            | Expr::ModAssign { name, value: v, .. }
            | Expr::AndAssign { name, value: v, .. }
            | Expr::OrAssign { name, value: v, .. }
            | Expr::XorAssign { name, value: v, .. }
            | Expr::ShlAssign { name, value: v, .. }
            | Expr::ShrAssign { name, value: v, .. } => {
                f(name);
                self.for_each_name(v, f);
            }
            Expr::VarDecl {
                name: _,
                ty: _,
                value: v,
                ..
            }
            | Expr::ConstDecl {
                name: _,
                ty: _,
                value: v,
                ..
            }
            | Expr::Return { value: v, .. } => self.for_each_name(v, f),
            Expr::GlobalVar {
                name: _,
                is_pub: _,
                ty: _,
                value: v,
                ..
            } => {
                if let Some(v) = v {
                    self.for_each_name(v, f);
                }
            }
            Expr::Call {
                callee,
                type_args: _,
                args,
                ..
            } => {
                self.for_each_name(callee, f);
                for a in args {
                    self.for_each_name(a, f);
                }
            }
            Expr::Block { stmts: body, .. } => {
                for e in body {
                    self.for_each_name(e, f);
                }
            }
            Expr::If {
                cond,
                then_branch: t,
                else_branch: e,
                ..
            } => {
                self.for_each_name(cond, f);
                self.for_each_name(t, f);
                if let Some(e) = e {
                    self.for_each_name(e, f);
                }
            }
            Expr::While { cond, body, .. } => {
                self.for_each_name(cond, f);
                self.for_each_name(body, f);
            }
            Expr::For {
                var: _,
                iterable: array,
                body,
                ..
            } => {
                self.for_each_name(array, f);
                self.for_each_name(body, f);
            }
            Expr::Lambda {
                params: _, body, ..
            } => self.for_each_name(body, f),
            Expr::Index {
                array: a, index: i, ..
            } => {
                self.for_each_name(a, f);
                self.for_each_name(i, f);
            }
            Expr::IndexAssign {
                target: a,
                value: v,
                ..
            } => {
                self.for_each_name(a, f);
                self.for_each_name(v, f);
            }
            Expr::ArrayLiteral { elements: es, .. } => {
                for e in es {
                    self.for_each_name(e, f);
                }
            }
            Expr::ArrayFill {
                elem_type: _, len, ..
            } => self.for_each_name(len, f),
            Expr::Range {
                start: s, end: e, ..
            } => {
                self.for_each_name(s, f);
                self.for_each_name(e, f);
            }
            Expr::Match {
                target: t,
                branches: br,
                default,
                ..
            } => {
                self.for_each_name(t, f);
                for (c, g, r) in br {
                    self.for_each_name(c, f);
                    if let Some(g) = g {
                        self.for_each_name(g, f);
                    }
                    self.for_each_name(r, f);
                }
                if let Some(d) = default {
                    self.for_each_name(d, f);
                }
            }
            Expr::StructLiteral {
                name: _,
                type_args: _,
                fields: fs,
                ..
            }
            | Expr::UnionLiteral {
                name: _,
                type_args: _,
                fields: fs,
                ..
            } => {
                for (_, v) in fs {
                    self.for_each_name(v, f);
                }
            }
            Expr::MemberAccess { obj: o, .. } => self.for_each_name(o, f),
            Expr::MemberAssign {
                obj: o,
                field: _,
                value: v,
                ..
            } => {
                self.for_each_name(o, f);
                self.for_each_name(v, f);
            }
            Expr::AddressOf { expr: e, .. }
            | Expr::Deref { expr: e, .. }
            | Expr::Not { expr: e, .. }
            | Expr::Neg { expr: e, .. }
            | Expr::FNeg { expr: e, .. }
            | Expr::Cast { expr: e, .. } => self.for_each_name(e, f),
            Expr::DerefAssign {
                ptr: p, value: v, ..
            } => {
                self.for_each_name(p, f);
                self.for_each_name(v, f);
            }
            Expr::FString { segs, .. } => {
                for seg in segs {
                    self.for_each_name(seg, f);
                }
            }
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
                self.for_each_name(l, f);
                self.for_each_name(r, f);
            }
            Expr::BNot { expr: e, .. } => self.for_each_name(e, f),
            _ => {}
        }
    }

    fn discardable(&self, expr: &Expr, pure_fns: &HashSet<String>) -> bool {
        match expr {
            Expr::Int { .. }
            | Expr::Float { .. }
            | Expr::Char { .. }
            | Expr::Bool { .. }
            | Expr::String { .. }
            | Expr::Nil(_)
            | Expr::Var { .. } => true,
            Expr::Call {
                callee,
                type_args: _,
                args,
                ..
            } => match callee.as_ref() {
                Expr::Var { name, .. } => {
                    pure_fns.contains(name) && args.iter().all(|a| self.discardable(a, pure_fns))
                }
                _ => false,
            },
            Expr::Index {
                array: l, index: r, ..
            } => self.discardable(l, pure_fns) && self.discardable(r, pure_fns),
            Expr::ArrayLiteral { elements: es, .. } => {
                es.iter().all(|e| self.discardable(e, pure_fns))
            }
            Expr::ArrayFill {
                elem_type: _, len, ..
            } => self.discardable(len, pure_fns),
            Expr::StructLiteral {
                name: _,
                type_args: _,
                fields: fs,
                ..
            }
            | Expr::UnionLiteral {
                name: _,
                type_args: _,
                fields: fs,
                ..
            } => fs.iter().all(|(_, v)| self.discardable(v, pure_fns)),
            Expr::MemberAccess { obj: o, .. } => self.discardable(o, pure_fns),
            Expr::AddressOf { expr: e, .. }
            | Expr::Deref { expr: e, .. }
            | Expr::Not { expr: e, .. }
            | Expr::Neg { expr: e, .. }
            | Expr::FNeg { expr: e, .. }
            | Expr::Cast { expr: e, .. } => self.discardable(e, pure_fns),
            Expr::Range {
                start: s, end: e, ..
            } => self.discardable(s, pure_fns) && self.discardable(e, pure_fns),
            Expr::FString { segs, .. } => segs.iter().all(|s| self.discardable(s, pure_fns)),
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
            | Expr::LAnd {
                left: l, right: r, ..
            }
            | Expr::LOr {
                left: l, right: r, ..
            }
            | Expr::StrCat {
                left: l, right: r, ..
            } => self.discardable(l, pure_fns) && self.discardable(r, pure_fns),
            _ => false,
        }
    }

    fn visit(&self, expr: &mut Expr) {
        match expr {
            Expr::Block { stmts: body, .. } => body.iter_mut().for_each(|e| self.optimize_expr(e)),
            Expr::FuncDecl {
                name: _,
                attrs: _,
                type_params: _,
                params: _,
                return_type: _,
                body,
                ..
            } => self.optimize_expr(body),
            Expr::Lambda {
                params: _, body, ..
            } => self.optimize_expr(body),
            Expr::If {
                cond,
                then_branch: t,
                else_branch: e,
                ..
            } => {
                self.optimize_expr(cond);
                self.optimize_expr(t);
                if let Some(e) = e {
                    self.optimize_expr(e);
                }
            }
            Expr::While { cond, body, .. } => {
                self.optimize_expr(cond);
                self.optimize_expr(body);
            }
            Expr::For {
                var: _,
                iterable: array,
                body,
                ..
            } => {
                self.optimize_expr(array);
                self.optimize_expr(body);
            }
            Expr::VarDecl {
                name: _,
                ty: _,
                value: v,
                ..
            }
            | Expr::ConstDecl {
                name: _,
                ty: _,
                value: v,
                ..
            }
            | Expr::VarAssign {
                name: _, value: v, ..
            }
            | Expr::Return { value: v, .. }
            | Expr::AddAssign {
                name: _, value: v, ..
            }
            | Expr::SubAssign {
                name: _, value: v, ..
            }
            | Expr::MulAssign {
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
            } => self.optimize_expr(v),
            Expr::GlobalVar {
                name: _,
                is_pub: _,
                ty: _,
                value: v,
                ..
            } => {
                if let Some(v) = v {
                    self.optimize_expr(v);
                }
            }
            Expr::Inc { .. } | Expr::Dec { .. } => {}
            Expr::Call {
                callee: f,
                type_args: _,
                args,
                ..
            } => {
                self.optimize_expr(f);
                args.iter_mut().for_each(|a| self.optimize_expr(a));
            }
            Expr::ArrayLiteral {
                elements: elems, ..
            } => elems.iter_mut().for_each(|e| self.optimize_expr(e)),
            Expr::ArrayFill {
                elem_type: _, len, ..
            } => self.optimize_expr(len),
            Expr::Index {
                array: arr,
                index: idx,
                ..
            } => {
                self.optimize_expr(arr);
                self.optimize_expr(idx);
            }
            Expr::IndexAssign {
                target: arr_idx, ..
            } => self.optimize_expr(arr_idx),
            Expr::StructLiteral {
                name: _,
                type_args: _,
                fields,
                ..
            } => {
                fields.iter_mut().for_each(|(_, v)| self.optimize_expr(v));
            }
            Expr::UnionLiteral {
                name: _,
                type_args: _,
                fields,
                ..
            } => {
                fields.iter_mut().for_each(|(_, v)| self.optimize_expr(v));
            }
            Expr::MemberAccess { obj, .. } => self.optimize_expr(obj),
            Expr::MemberAssign {
                obj,
                field: _,
                value: val,
                ..
            } => {
                self.optimize_expr(obj);
                self.optimize_expr(val);
            }
            Expr::AddressOf { expr, .. } => self.optimize_expr(expr),
            Expr::Deref { expr, .. } => self.optimize_expr(expr),
            Expr::Cast { expr, .. } => self.optimize_expr(expr),
            Expr::DerefAssign {
                ptr, value: val, ..
            } => {
                self.optimize_expr(ptr);
                self.optimize_expr(val);
            }
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
                self.optimize_expr(l);
                self.optimize_expr(r);
            }
            Expr::BNot { expr: e, .. } => self.optimize_expr(e),
            Expr::FString { segs, .. } => segs.iter_mut().for_each(|s| self.optimize_expr(s)),
            Expr::Match {
                target,
                branches,
                default,
                ..
            } => {
                self.optimize_expr(target);
                for (c, g, v) in branches.iter_mut() {
                    self.optimize_expr(c);
                    if let Some(g) = g {
                        self.optimize_expr(g);
                    }
                    self.optimize_expr(v);
                }
                if let Some(d) = default {
                    self.optimize_expr(d);
                }
            }
            Expr::Not { expr: e, .. } | Expr::Neg { expr: e, .. } | Expr::FNeg { expr: e, .. } => {
                self.optimize_expr(e)
            }
            _ => {}
        }
    }

    fn is_effect_free(&self, expr: &Expr) -> bool {
        match expr {
            Expr::Call { .. }
            | Expr::VarAssign { .. }
            | Expr::AddAssign { .. }
            | Expr::SubAssign { .. }
            | Expr::MulAssign { .. }
            | Expr::DivAssign { .. }
            | Expr::ModAssign { .. }
            | Expr::AndAssign { .. }
            | Expr::OrAssign { .. }
            | Expr::XorAssign { .. }
            | Expr::ShlAssign { .. }
            | Expr::ShrAssign { .. }
            | Expr::IndexAssign { .. }
            | Expr::MemberAssign { .. }
            | Expr::DerefAssign { .. }
            | Expr::Inc { .. }
            | Expr::Dec { .. }
            | Expr::While { .. }
            | Expr::For { .. }
            | Expr::FuncDecl { .. }
            | Expr::GlobalVar { .. }
            | Expr::ExternVar { .. }
            | Expr::Continue(_) => false,
            Expr::Break { value: v, .. } => {
                v.as_ref().map(|v| self.is_effect_free(v)).unwrap_or(true)
            }
            Expr::Int { .. }
            | Expr::Float { .. }
            | Expr::Bool { .. }
            | Expr::String { .. }
            | Expr::Nil(_)
            | Expr::Var { .. } => true,
            Expr::Block { stmts: body, .. } => body.iter().all(|e| self.is_effect_free(e)),
            Expr::If {
                cond: c,
                then_branch: t,
                else_branch: e,
                ..
            } => {
                self.is_effect_free(c)
                    && self.is_effect_free(t)
                    && e.as_ref().map(|e| self.is_effect_free(e)).unwrap_or(true)
            }
            Expr::Lambda {
                params: _, body: b, ..
            } => self.is_effect_free(b),
            Expr::VarDecl {
                name: _,
                ty: _,
                value: v,
                ..
            }
            | Expr::ConstDecl {
                name: _,
                ty: _,
                value: v,
                ..
            }
            | Expr::Return { value: v, .. } => self.is_effect_free(v),
            Expr::ArrayLiteral { elements: es, .. } => es.iter().all(|e| self.is_effect_free(e)),
            Expr::ArrayFill {
                elem_type: _, len, ..
            } => self.is_effect_free(len),
            Expr::Range {
                start: s, end: e, ..
            } => self.is_effect_free(s) && self.is_effect_free(e),
            Expr::Match {
                target: t,
                branches: br,
                default: d,
                ..
            } => {
                self.is_effect_free(t)
                    && br.iter().all(|(c, g, r)| {
                        self.is_effect_free(c)
                            && g.as_ref().map(|g| self.is_effect_free(g)).unwrap_or(true)
                            && self.is_effect_free(r)
                    })
                    && d.as_ref().map(|d| self.is_effect_free(d)).unwrap_or(true)
            }
            Expr::StructLiteral {
                name: _,
                type_args: _,
                fields: fs,
                ..
            }
            | Expr::UnionLiteral {
                name: _,
                type_args: _,
                fields: fs,
                ..
            } => fs.iter().all(|(_, v)| self.is_effect_free(v)),
            Expr::FString { segs, .. } => segs.iter().all(|s| self.is_effect_free(s)),
            Expr::Index {
                array: a, index: i, ..
            } => self.is_effect_free(a) && self.is_effect_free(i),
            Expr::MemberAccess { obj: a, .. }
            | Expr::AddressOf { expr: a, .. }
            | Expr::Deref { expr: a, .. }
            | Expr::Cast { expr: a, .. }
            | Expr::Not { expr: a, .. }
            | Expr::Neg { expr: a, .. }
            | Expr::FNeg { expr: a, .. } => self.is_effect_free(a),
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
            | Expr::Shl {
                left: l, right: r, ..
            }
            | Expr::Shr {
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
            | Expr::LAnd {
                left: l, right: r, ..
            }
            | Expr::LOr {
                left: l, right: r, ..
            }
            | Expr::StrCat {
                left: l, right: r, ..
            } => self.is_effect_free(l) && self.is_effect_free(r),
            _ => false,
        }
    }

    fn fold(&self, expr: &Expr) -> Option<Expr> {
        match expr {
            Expr::Add {
                left: l, right: r, ..
            } => match (l.as_ref(), r.as_ref()) {
                (Expr::Int { value: a, .. }, Expr::Int { value: b, .. }) => Some(Expr::Int {
                    value: a.wrapping_add(*b),
                    span: Span::new(0, 0),
                }),
                (Expr::Float { value: a, .. }, Expr::Float { value: b, .. }) => Some(Expr::Float {
                    value: a + b,
                    span: Span::new(0, 0),
                }),

                (Expr::Int { value: 0, .. }, _) => Some(*r.clone()),
                (_, Expr::Int { value: 0, .. }) => Some(*l.clone()),
                _ => None,
            },
            Expr::Sub {
                left: l, right: r, ..
            } => match (l.as_ref(), r.as_ref()) {
                (Expr::Int { value: a, .. }, Expr::Int { value: b, .. }) => Some(Expr::Int {
                    value: a.wrapping_sub(*b),
                    span: Span::new(0, 0),
                }),
                (Expr::Float { value: a, .. }, Expr::Float { value: b, .. }) => Some(Expr::Float {
                    value: a - b,
                    span: Span::new(0, 0),
                }),

                (_, Expr::Int { value: 0, .. }) => Some(*l.clone()),
                (_, Expr::Float { value: 0.0, .. }) => Some(*l.clone()),
                _ => None,
            },
            Expr::Mul {
                left: l, right: r, ..
            } => match (l.as_ref(), r.as_ref()) {
                (Expr::Int { value: a, .. }, Expr::Int { value: b, .. }) => Some(Expr::Int {
                    value: a.wrapping_mul(*b),
                    span: Span::new(0, 0),
                }),
                (Expr::Float { value: a, .. }, Expr::Float { value: b, .. }) => Some(Expr::Float {
                    value: a * b,
                    span: Span::new(0, 0),
                }),

                (Expr::Int { value: 1, .. }, _) => Some(*r.clone()),
                (_, Expr::Int { value: 1, .. }) => Some(*l.clone()),
                (Expr::Float { value: 1.0, .. }, _) => Some(*r.clone()),
                (_, Expr::Float { value: 1.0, .. }) => Some(*l.clone()),
                _ => None,
            },
            Expr::Div {
                left: l, right: r, ..
            } => match (l.as_ref(), r.as_ref()) {
                (Expr::Int { value: a, .. }, Expr::Int { value: b, .. }) => {
                    if *b == 0 || (*a == isize::MIN && *b == -1) {
                        None
                    } else {
                        Some(Expr::Int {
                            value: a.wrapping_div(*b),
                            span: Span::new(0, 0),
                        })
                    }
                }
                (Expr::Float { value: a, .. }, Expr::Float { value: b, .. }) => Some(Expr::Float {
                    value: a / b,
                    span: Span::new(0, 0),
                }),

                (_, Expr::Int { value: 1, .. }) => Some(*l.clone()),
                (_, Expr::Float { value: 1.0, .. }) => Some(*l.clone()),
                _ => None,
            },
            Expr::Mod {
                left: l, right: r, ..
            } => match (l.as_ref(), r.as_ref()) {
                (Expr::Int { value: a, .. }, Expr::Int { value: b, .. }) => {
                    if *b == 0 || (*a == isize::MIN && *b == -1) {
                        None
                    } else {
                        Some(Expr::Int {
                            value: a.wrapping_rem(*b),
                            span: Span::new(0, 0),
                        })
                    }
                }
                _ => None,
            },
            Expr::Xor {
                left: l, right: r, ..
            } => match (l.as_ref(), r.as_ref()) {
                (Expr::Int { value: a, .. }, Expr::Int { value: b, .. }) => Some(Expr::Int {
                    value: a ^ b,
                    span: Span::new(0, 0),
                }),
                (Expr::Int { value: 0, .. }, _) => Some(*r.clone()),
                (_, Expr::Int { value: 0, .. }) => Some(*l.clone()),
                _ => None,
            },
            Expr::FAdd {
                left: l, right: r, ..
            } => match (l.as_ref(), r.as_ref()) {
                (Expr::Float { value: a, .. }, Expr::Float { value: b, .. }) => Some(Expr::Float {
                    value: a + b,
                    span: Span::new(0, 0),
                }),
                _ => None,
            },
            Expr::FSub {
                left: l, right: r, ..
            } => match (l.as_ref(), r.as_ref()) {
                (Expr::Float { value: a, .. }, Expr::Float { value: b, .. }) => Some(Expr::Float {
                    value: a - b,
                    span: Span::new(0, 0),
                }),
                (_, Expr::Float { value: 0.0, .. }) => Some(*l.clone()),
                _ => None,
            },
            Expr::FMul {
                left: l, right: r, ..
            } => match (l.as_ref(), r.as_ref()) {
                (Expr::Float { value: a, .. }, Expr::Float { value: b, .. }) => Some(Expr::Float {
                    value: a * b,
                    span: Span::new(0, 0),
                }),

                (Expr::Float { value: 1.0, .. }, _) => Some(*r.clone()),
                (_, Expr::Float { value: 1.0, .. }) => Some(*l.clone()),
                _ => None,
            },
            Expr::FDiv {
                left: l, right: r, ..
            } => match (l.as_ref(), r.as_ref()) {
                (Expr::Float { value: a, .. }, Expr::Float { value: b, .. }) => Some(Expr::Float {
                    value: a / b,
                    span: Span::new(0, 0),
                }),
                (_, Expr::Float { value: 1.0, .. }) => Some(*l.clone()),
                _ => None,
            },
            Expr::Eq {
                left: l, right: r, ..
            } => match (l.as_ref(), r.as_ref()) {
                (Expr::Int { value: a, .. }, Expr::Int { value: b, .. }) => Some(Expr::Bool {
                    value: a == b,
                    span: Span::new(0, 0),
                }),
                (Expr::Bool { value: a, .. }, Expr::Bool { value: b, .. }) => Some(Expr::Bool {
                    value: a == b,
                    span: Span::new(0, 0),
                }),
                _ => None,
            },
            Expr::Ne {
                left: l, right: r, ..
            } => match (l.as_ref(), r.as_ref()) {
                (Expr::Int { value: a, .. }, Expr::Int { value: b, .. }) => Some(Expr::Bool {
                    value: a != b,
                    span: Span::new(0, 0),
                }),
                (Expr::Bool { value: a, .. }, Expr::Bool { value: b, .. }) => Some(Expr::Bool {
                    value: a != b,
                    span: Span::new(0, 0),
                }),
                _ => None,
            },
            Expr::Lt {
                left: l, right: r, ..
            } => match (l.as_ref(), r.as_ref()) {
                (Expr::Int { value: a, .. }, Expr::Int { value: b, .. }) => Some(Expr::Bool {
                    value: a < b,
                    span: Span::new(0, 0),
                }),
                _ => None,
            },
            Expr::Le {
                left: l, right: r, ..
            } => match (l.as_ref(), r.as_ref()) {
                (Expr::Int { value: a, .. }, Expr::Int { value: b, .. }) => Some(Expr::Bool {
                    value: a <= b,
                    span: Span::new(0, 0),
                }),
                _ => None,
            },
            Expr::Gt {
                left: l, right: r, ..
            } => match (l.as_ref(), r.as_ref()) {
                (Expr::Int { value: a, .. }, Expr::Int { value: b, .. }) => Some(Expr::Bool {
                    value: a > b,
                    span: Span::new(0, 0),
                }),
                _ => None,
            },
            Expr::Ge {
                left: l, right: r, ..
            } => match (l.as_ref(), r.as_ref()) {
                (Expr::Int { value: a, .. }, Expr::Int { value: b, .. }) => Some(Expr::Bool {
                    value: a >= b,
                    span: Span::new(0, 0),
                }),
                _ => None,
            },
            Expr::FEq {
                left: l, right: r, ..
            } => match (l.as_ref(), r.as_ref()) {
                (Expr::Float { value: a, .. }, Expr::Float { value: b, .. }) => Some(Expr::Bool {
                    value: a == b,
                    span: Span::new(0, 0),
                }),
                _ => None,
            },
            Expr::FNe {
                left: l, right: r, ..
            } => match (l.as_ref(), r.as_ref()) {
                (Expr::Float { value: a, .. }, Expr::Float { value: b, .. }) => Some(Expr::Bool {
                    value: a != b,
                    span: Span::new(0, 0),
                }),
                _ => None,
            },
            Expr::FLt {
                left: l, right: r, ..
            } => match (l.as_ref(), r.as_ref()) {
                (Expr::Float { value: a, .. }, Expr::Float { value: b, .. }) => Some(Expr::Bool {
                    value: a < b,
                    span: Span::new(0, 0),
                }),
                _ => None,
            },
            Expr::FLe {
                left: l, right: r, ..
            } => match (l.as_ref(), r.as_ref()) {
                (Expr::Float { value: a, .. }, Expr::Float { value: b, .. }) => Some(Expr::Bool {
                    value: a <= b,
                    span: Span::new(0, 0),
                }),
                _ => None,
            },
            Expr::FGt {
                left: l, right: r, ..
            } => match (l.as_ref(), r.as_ref()) {
                (Expr::Float { value: a, .. }, Expr::Float { value: b, .. }) => Some(Expr::Bool {
                    value: a > b,
                    span: Span::new(0, 0),
                }),
                _ => None,
            },
            Expr::FGe {
                left: l, right: r, ..
            } => match (l.as_ref(), r.as_ref()) {
                (Expr::Float { value: a, .. }, Expr::Float { value: b, .. }) => Some(Expr::Bool {
                    value: a >= b,
                    span: Span::new(0, 0),
                }),
                _ => None,
            },
            Expr::LAnd {
                left: l, right: r, ..
            } => match (l.as_ref(), r.as_ref()) {
                (Expr::Bool { value: true, .. }, e) => Some(e.clone()),
                (Expr::Bool { value: false, .. }, e) if self.is_effect_free(e) => {
                    Some(Expr::Bool {
                        value: false,
                        span: Span::new(0, 0),
                    })
                }
                (e, Expr::Bool { value: true, .. }) => Some(e.clone()),
                (e, Expr::Bool { value: false, .. }) if self.is_effect_free(e) => {
                    Some(Expr::Bool {
                        value: false,
                        span: Span::new(0, 0),
                    })
                }
                _ => None,
            },
            Expr::LOr {
                left: l, right: r, ..
            } => match (l.as_ref(), r.as_ref()) {
                (Expr::Bool { value: true, .. }, e) if self.is_effect_free(e) => Some(Expr::Bool {
                    value: true,
                    span: Span::new(0, 0),
                }),
                (Expr::Bool { value: false, .. }, e) => Some(e.clone()),
                (e, Expr::Bool { value: true, .. }) if self.is_effect_free(e) => Some(Expr::Bool {
                    value: true,
                    span: Span::new(0, 0),
                }),
                (e, Expr::Bool { value: false, .. }) => Some(e.clone()),
                _ => None,
            },
            Expr::Not { expr: e, .. } => match e.as_ref() {
                Expr::Bool { value: b, .. } => Some(Expr::Bool {
                    value: !b,
                    span: Span::new(0, 0),
                }),
                _ => None,
            },
            Expr::Shl {
                left: l, right: r, ..
            } => match (l.as_ref(), r.as_ref()) {
                (Expr::Int { value: a, .. }, Expr::Int { value: b, .. }) if *b >= 0 && *b < 64 => {
                    Some(Expr::Int {
                        value: a.wrapping_shl(*b as u32),
                        span: Span::new(0, 0),
                    })
                }
                _ => None,
            },
            Expr::Shr {
                left: l, right: r, ..
            } => match (l.as_ref(), r.as_ref()) {
                (Expr::Int { value: a, .. }, Expr::Int { value: b, .. }) if *b >= 0 && *b < 64 => {
                    Some(Expr::Int {
                        value: a.wrapping_shr(*b as u32),
                        span: Span::new(0, 0),
                    })
                }
                _ => None,
            },
            Expr::BNot { expr: e, .. } => match e.as_ref() {
                Expr::Int { value: n, .. } => Some(Expr::Int {
                    value: !*n,
                    span: Span::new(0, 0),
                }),
                _ => None,
            },
            Expr::Neg { expr: e, .. } => match e.as_ref() {
                Expr::Int { value: n, .. } => Some(Expr::Int {
                    value: n.wrapping_neg(),
                    span: Span::new(0, 0),
                }),
                _ => None,
            },
            Expr::FNeg { expr: e, .. } => match e.as_ref() {
                Expr::Float { value: n, .. } => Some(Expr::Float {
                    value: -n,
                    span: Span::new(0, 0),
                }),
                _ => None,
            },
            _ => None,
        }
    }

    fn dce(&self, expr: &mut Expr, pure_fns: &HashSet<String>) {
        match expr {
            Expr::Block { stmts: body, .. } => {
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

                let last_idx = body.len().saturating_sub(1);
                let mut i = 0usize;
                body.retain(|e| {
                    let keep =
                        i == last_idx || !matches!(e, Expr::Block { stmts: b, .. } if b.is_empty());
                    i += 1;
                    keep
                });
            }
            Expr::If {
                cond,
                then_branch: t,
                else_branch: e,
                ..
            } => {
                self.dce(cond, pure_fns);
                self.dce(t, pure_fns);
                if let Some(e) = e {
                    self.dce(e, pure_fns);
                }
                if let Expr::Bool { value: true, .. } = cond.as_ref() {
                    *expr = *t.clone();
                } else if let Expr::Bool { value: false, .. } = cond.as_ref() {
                    if let Some(else_expr) = e {
                        *expr = *else_expr.clone();
                    } else {
                        *expr = Expr::Block {
                            stmts: vec![],
                            span: Span::new(0, 0),
                        };
                    }
                }
            }
            Expr::While { cond, body, .. } => {
                self.dce(cond, pure_fns);
                self.dce(body, pure_fns);
                if let Expr::Bool { value: false, .. } = cond.as_ref() {
                    *expr = Expr::Block {
                        stmts: vec![],
                        span: Span::new(0, 0),
                    };
                }
            }
            Expr::For {
                var: _,
                iterable: array,
                body,
                ..
            } => {
                self.dce(array, pure_fns);
                self.dce(body, pure_fns);
            }
            Expr::FuncDecl {
                name: _,
                attrs: _,
                type_params: _,
                params: _,
                return_type: _,
                body,
                ..
            } => self.dce(body, pure_fns),
            Expr::Lambda {
                params: _, body, ..
            } => self.dce(body, pure_fns),
            Expr::VarDecl {
                name: _,
                ty: _,
                value: v,
                ..
            } => self.dce(v, pure_fns),
            Expr::ConstDecl {
                name: _,
                ty: _,
                value: v,
                ..
            } => self.dce(v, pure_fns),
            Expr::GlobalVar {
                name: _,
                is_pub: _,
                ty: _,
                value: v,
                ..
            } => {
                if let Some(v) = v {
                    self.dce(v, pure_fns);
                }
            }
            Expr::VarAssign {
                name: _, value: v, ..
            }
            | Expr::AddAssign {
                name: _, value: v, ..
            }
            | Expr::SubAssign {
                name: _, value: v, ..
            }
            | Expr::MulAssign {
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
            } => self.dce(v, pure_fns),
            Expr::Return { value: v, .. } => self.dce(v, pure_fns),
            Expr::Inc { .. } | Expr::Dec { .. } => {}
            Expr::Call {
                callee: f,
                type_args: _,
                args,
                ..
            } => {
                self.dce(f, pure_fns);
                for a in args {
                    self.dce(a, pure_fns);
                }
            }
            Expr::ArrayLiteral {
                elements: elems, ..
            } => {
                for e in elems {
                    self.dce(e, pure_fns);
                }
            }
            Expr::ArrayFill {
                elem_type: _, len, ..
            } => self.dce(len, pure_fns),
            Expr::Index {
                array: arr,
                index: idx,
                ..
            } => {
                self.dce(arr, pure_fns);
                self.dce(idx, pure_fns);
            }
            Expr::IndexAssign {
                target: arr_idx,
                value: v,
                ..
            } => {
                self.dce(arr_idx, pure_fns);
                self.dce(v, pure_fns);
            }
            Expr::StructLiteral {
                name: _,
                type_args: _,
                fields,
                ..
            } => {
                for (_, v) in fields {
                    self.dce(v, pure_fns);
                }
            }
            Expr::UnionLiteral {
                name: _,
                type_args: _,
                fields,
                ..
            } => {
                for (_, v) in fields {
                    self.dce(v, pure_fns);
                }
            }
            Expr::MemberAccess { obj, .. } => self.dce(obj, pure_fns),
            Expr::MemberAssign {
                obj,
                field: _,
                value: val,
                ..
            } => {
                self.dce(obj, pure_fns);
                self.dce(val, pure_fns);
            }
            Expr::AddressOf { expr, .. } => self.dce(expr, pure_fns),
            Expr::Deref { expr, .. } => self.dce(expr, pure_fns),
            Expr::Cast { expr, .. } => self.dce(expr, pure_fns),
            Expr::DerefAssign {
                ptr, value: val, ..
            } => {
                self.dce(ptr, pure_fns);
                self.dce(val, pure_fns);
            }
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
                self.dce(l, pure_fns);
                self.dce(r, pure_fns);
            }
            Expr::BNot { expr: e, .. } => self.dce(e, pure_fns),
            Expr::Not { expr: e, .. } | Expr::Neg { expr: e, .. } | Expr::FNeg { expr: e, .. } => {
                self.dce(e, pure_fns)
            }
            _ => {}
        }
    }
    fn is_pure_dead(&self, expr: &Expr, pure_fns: &HashSet<String>) -> bool {
        self.is_pure(expr, pure_fns)
    }

    fn is_pure(&self, expr: &Expr, pure_fns: &HashSet<String>) -> bool {
        match expr {
            Expr::Int { .. }
            | Expr::Float { .. }
            | Expr::Bool { .. }
            | Expr::String { .. }
            | Expr::Nil(_)
            | Expr::Var { .. } => true,
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
            | Expr::LAnd {
                left: l, right: r, ..
            }
            | Expr::LOr {
                left: l, right: r, ..
            } => self.is_pure(l, pure_fns) && self.is_pure(r, pure_fns),
            Expr::Not { expr: e, .. } | Expr::Neg { expr: e, .. } | Expr::FNeg { expr: e, .. } => {
                self.is_pure(e, pure_fns)
            }
            Expr::Index {
                array: l, index: r, ..
            } => self.is_pure(l, pure_fns) && self.is_pure(r, pure_fns),
            Expr::ArrayLiteral {
                elements: elems, ..
            } => elems.iter().all(|e| self.is_pure(e, pure_fns)),
            Expr::ArrayFill {
                elem_type: _, len, ..
            } => self.is_pure(len, pure_fns),
            Expr::StructLiteral {
                name: _,
                type_args: _,
                fields,
                ..
            } => fields.iter().all(|(_, v)| self.is_pure(v, pure_fns)),
            Expr::UnionLiteral {
                name: _,
                type_args: _,
                fields,
                ..
            } => fields.iter().all(|(_, v)| self.is_pure(v, pure_fns)),
            Expr::MemberAccess { obj, .. } => self.is_pure(obj, pure_fns),
            Expr::AddressOf { expr, .. } => self.is_pure(expr, pure_fns),
            Expr::Deref { expr, .. } => self.is_pure(expr, pure_fns),
            Expr::Cast { expr, .. } => self.is_pure(expr, pure_fns),
            Expr::Call {
                callee,
                type_args: _,
                args,
                ..
            } => match callee.as_ref() {
                Expr::Var { name, .. } => {
                    pure_fns.contains(name) && args.iter().all(|a| self.is_pure(a, pure_fns))
                }
                _ => false,
            },
            _ => false,
        }
    }
}
