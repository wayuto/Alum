use super::context::Context;
use super::ir::{IRConst, IRType, Operand};
use super::vm_safety::VmSafety;
use crate::compiler::{
    bytecode::{Compiler, GVM, NativeKind, NativeSig, Value},
    irgen::IRGen,
    parser::{Expr, Primitive, Program, Type},
};
use ordered_float::OrderedFloat;
use std::collections::{BTreeMap, BTreeSet, HashSet};

impl IRGen {
    pub(super) fn eval_const(
        &mut self,
        expr: &Expr,
        ctx: Option<&Context>,
    ) -> Option<(IRConst, IRType)> {
        match expr {
            Expr::Int { value: n, .. } => Some((IRConst::Int(*n as i64), IRType::Int)),
            Expr::Float { value: f, .. } => Some((IRConst::Float(OrderedFloat(*f)), IRType::Float)),
            Expr::String { value: s, .. } => Some((IRConst::Str(s.clone()), IRType::String)),
            Expr::Bool { value: b, .. } => {
                Some((IRConst::Int(if *b { 1 } else { 0 }), IRType::Bool))
            }
            Expr::Nil(_) => Some((IRConst::Int(0), IRType::Int)),
            Expr::Var { name, .. } => {
                if ctx.map(|c| c.get_var_type(name).is_ok()).unwrap_or(false) {
                    return None;
                }
                self.globals.get(name).cloned()
            }
            Expr::Neg { expr: e, .. } => match self.eval_const(e, ctx)? {
                (IRConst::Int(v), IRType::Int) => {
                    Some((IRConst::Int(v.wrapping_neg()), IRType::Int))
                }
                _ => None,
            },
            Expr::FNeg { expr: e, .. } => match self.eval_const(e, ctx)? {
                (IRConst::Float(v), IRType::Float) => Some((IRConst::Float(-v), IRType::Float)),
                _ => None,
            },
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
            } => {
                let (lc, lt) = self.eval_const(l, ctx)?;
                let (rc, rt) = self.eval_const(r, ctx)?;
                if matches!(lt, IRType::Float) || matches!(rt, IRType::Float) {
                    let (a, b) = match (lc, rc) {
                        (IRConst::Float(a), IRConst::Float(b)) => (a.into_inner(), b.into_inner()),
                        _ => return None,
                    };
                    let v = match expr {
                        Expr::FAdd { .. } | Expr::Add { .. } => a + b,
                        Expr::FSub { .. } | Expr::Sub { .. } => a - b,
                        Expr::FMul { .. } | Expr::Mul { .. } => a * b,
                        Expr::FDiv { .. } | Expr::Div { .. } => a / b,
                        _ => return None,
                    };
                    Some((IRConst::Float(OrderedFloat(v)), IRType::Float))
                } else {
                    let (a, b) = match (lc, rc) {
                        (IRConst::Int(a), IRConst::Int(b)) => (a, b),
                        _ => return None,
                    };
                    let v = match expr {
                        Expr::Add { .. } => a.wrapping_add(b),
                        Expr::Sub { .. } => a.wrapping_sub(b),
                        Expr::Mul { .. } => a.wrapping_mul(b),
                        Expr::Div { .. } => {
                            if b == 0 {
                                return None;
                            }
                            a.wrapping_div(b)
                        }
                        Expr::Mod { .. } => {
                            if b == 0 {
                                return None;
                            }
                            a.wrapping_rem(b)
                        }
                        _ => return None,
                    };
                    Some((IRConst::Int(v), IRType::Int))
                }
            }
            _ => self.eval_const_vm(expr),
        }
    }

    pub(super) fn eval_const_vm(&mut self, expr: &Expr) -> Option<(IRConst, IRType)> {
        if self.expr_has_var(expr) {
            return None;
        }

        let pure_fns: HashSet<String> = self
            .program_body
            .iter()
            .filter_map(|e| match e {
                Expr::FuncDecl { name, attrs, .. }
                    if attrs.is_pure && (!attrs.is_external || self.native_resolved(name)) =>
                {
                    Some(name.clone())
                }
                _ => None,
            })
            .collect();
        let mut safety = VmSafety::new(&self.program_body, &pure_fns);
        if !safety.safe(expr) {
            return None;
        }

        let mut unsafe_fns: HashSet<String> = HashSet::new();
        for decl in self.program_body.iter() {
            if let Expr::FuncDecl {
                name,
                attrs,
                type_params: _,
                params: _,
                return_type: _,
                body,
                ..
            } = decl
            {
                if (attrs.is_pure || name.starts_with("_lambda_"))
                    && (!attrs.is_external || self.native_resolved(name))
                {
                    let mut fn_safety = VmSafety::new(&self.program_body, &pure_fns);
                    if !fn_safety.safe(body) {
                        unsafe_fns.insert(name.clone());
                    }
                }
            }
        }

        let mut selected: Vec<(String, Expr)> = self
            .program_body
            .iter()
            .filter_map(|e| match e {
                Expr::FuncDecl { name, attrs, .. }
                    if ((attrs.is_pure || name.starts_with("_lambda_"))
                        && (!attrs.is_external || self.native_resolved(name))
                        && !unsafe_fns.contains(name)) =>
                {
                    Some((name.clone(), e.clone()))
                }
                _ => None,
            })
            .collect();

        let mut body = order_vm_functions(&mut selected);

        for e in self.program_body.iter() {
            if matches!(
                e,
                Expr::Struct { .. } | Expr::Union { .. } | Expr::Enum { .. } | Expr::TypeDef(_)
            ) {
                body.push(e.clone());
            }
        }

        body.push(expr.clone());
        let program = Program { body };

        let natives = self
            .natives
            .as_ref()
            .map(|t| t.entries.clone())
            .unwrap_or_default();

        let prev_hook = std::panic::take_hook();
        std::panic::set_hook(std::boxed::Box::new(|_| {}));

        let mut global_consts: std::collections::HashMap<String, Value> =
            std::collections::HashMap::new();
        for (name, (c, _)) in self.globals.iter() {
            let v = match c {
                IRConst::Int(i) => Some(Value::Int(*i)),
                IRConst::Float(f) => Some(Value::Float(f.into_inner())),
                IRConst::Str(s) => Some(Value::Str(s.clone())),
                _ => None,
            };
            if let Some(v) = v {
                global_consts.insert(name.clone(), v);
            }
        }
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let bc = Compiler::with_global_consts(global_consts).compile(program);
            let mut vm = GVM::new(bc, natives);
            vm.run();
            vm.result()
        }));
        std::panic::set_hook(prev_hook);

        let mut fail_reason = String::new();
        let result = match result {
            Ok(r) => r,
            Err(payload) => {
                fail_reason = payload
                    .downcast_ref::<String>()
                    .cloned()
                    .or_else(|| payload.downcast_ref::<&str>().map(|s| s.to_string()))
                    .unwrap_or_default();
                None
            }
        };
        if !fail_reason.is_empty() {
            eprintln!("warning: compile-time evaluation failed: {}", fail_reason);
        }
        let result = result?;

        match result {
            Value::Int(i) => Some((IRConst::Int(i), IRType::Int)),
            Value::Float(f) => Some((IRConst::Float(OrderedFloat(f)), IRType::Float)),
            Value::Str(s) => Some((IRConst::Str(s), IRType::String)),
            Value::Bool(b) => Some((IRConst::Int(if b { 1 } else { 0 }), IRType::Bool)),
            Value::Array(elems) => {
                let mut operands = Vec::with_capacity(elems.len());
                for e in elems {
                    operands.push(self.vm_value_to_const(&e)?);
                }
                Some((IRConst::Array(operands), IRType::Array))
            }
            Value::Void => None,
            Value::Fn(..) => None,
        }
    }

    fn vm_value_to_const(&mut self, value: &Value) -> Option<Operand> {
        match value {
            Value::Int(i) => Some(Operand::ConstIdx(self.get_const_index(IRConst::Int(*i)))),
            Value::Float(f) => Some(Operand::ConstIdx(
                self.get_const_index(IRConst::Float(OrderedFloat(*f))),
            )),
            Value::Str(s) => Some(Operand::ConstIdx(
                self.get_const_index(IRConst::Str(s.clone())),
            )),
            Value::Bool(b) => Some(Operand::ConstIdx(
                self.get_const_index(IRConst::Int(if *b { 1 } else { 0 })),
            )),
            Value::Array(elems) => {
                let mut operands = Vec::with_capacity(elems.len());
                for e in elems {
                    operands.push(self.vm_value_to_const(e)?);
                }
                Some(Operand::ConstIdx(
                    self.get_const_index(IRConst::Array(operands)),
                ))
            }
            Value::Void => None,
            Value::Fn(..) => None,
        }
    }

    fn expr_has_var(&self, expr: &Expr) -> bool {
        use Expr::*;
        match expr {
            Int { .. }
            | Float { .. }
            | Char { .. }
            | Bool { .. }
            | String { .. }
            | Nil(_)
            | Continue(_)
            | TypeDef(_)
            | Struct { .. }
            | Union { .. }
            | Enum { .. } => false,
            Break { value: v, .. } => v.as_ref().map(|v| self.expr_has_var(v)).unwrap_or(false),
            Var { .. } => true,
            Call {
                callee: f,
                type_args: _,
                args,
                ..
            } => {
                if !matches!(f.as_ref(), Var { .. }) && self.expr_has_var(f) {
                    return true;
                }
                args.iter().any(|a| self.expr_has_var(a))
            }
            Block { stmts, .. } => stmts.iter().any(|s| self.expr_has_var(s)),
            If {
                cond: c,
                then_branch: t,
                else_branch: e,
                ..
            } => {
                self.expr_has_var(c)
                    || self.expr_has_var(t)
                    || e.as_ref().map(|x| self.expr_has_var(x)).unwrap_or(false)
            }
            While {
                cond: c, body: b, ..
            } => self.expr_has_var(c) || self.expr_has_var(b),
            For {
                var: _,
                iterable: i,
                body: b,
                ..
            } => self.expr_has_var(i) || self.expr_has_var(b),
            Range {
                start: l, end: r, ..
            } => self.expr_has_var(l) || self.expr_has_var(r),
            Match {
                target: s,
                branches: arms,
                default: d,
                ..
            } => {
                self.expr_has_var(s)
                    || arms.iter().any(|(p, g, a)| {
                        self.expr_has_var(p)
                            || g.as_ref().map(|g| self.expr_has_var(g)).unwrap_or(false)
                            || self.expr_has_var(a)
                    })
                    || d.as_ref().map(|x| self.expr_has_var(x)).unwrap_or(false)
            }
            Return { value: v, .. } => self.expr_has_var(v),
            Lambda {
                params: _, body: b, ..
            } => self.expr_has_var(b),
            FuncDecl {
                name: _,
                attrs: _,
                type_params: _,
                params: _,
                return_type: _,
                body: b,
                ..
            } => self.expr_has_var(b),
            GlobalVar { .. } | ExternVar { .. } => false,
            VarDecl {
                name: _,
                ty: _,
                value: v,
                ..
            }
            | ConstDecl {
                name: _,
                ty: _,
                value: v,
                ..
            } => self.expr_has_var(v),
            Not { expr: e, .. }
            | BNot { expr: e, .. }
            | Neg { expr: e, .. }
            | FNeg { expr: e, .. }
            | AddressOf { expr: e, .. }
            | Deref { expr: e, .. } => self.expr_has_var(e),
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
            }
            | Index {
                array: l, index: r, ..
            }
            | DerefAssign {
                ptr: l, value: r, ..
            } => self.expr_has_var(l) || self.expr_has_var(r),
            IndexAssign {
                target: o,
                value: v,
                ..
            }
            | MemberAssign {
                obj: o,
                field: _,
                value: v,
                ..
            } => self.expr_has_var(o) || self.expr_has_var(v),
            ArrayLiteral {
                elements: items, ..
            } => items.iter().any(|it| self.expr_has_var(it)),
            ArrayFill {
                elem_type: _, len, ..
            } => self.expr_has_var(len),
            StructLiteral {
                name: _,
                type_args: _,
                fields,
                ..
            }
            | UnionLiteral {
                name: _,
                type_args: _,
                fields,
                ..
            } => fields.iter().any(|(_, v)| self.expr_has_var(v)),
            MemberAccess { obj: o, .. } => self.expr_has_var(o),
            Inc { .. } | Dec { .. } => false,
            VarAssign {
                name: _, value: v, ..
            }
            | AddAssign {
                name: _, value: v, ..
            }
            | SubAssign {
                name: _, value: v, ..
            }
            | MulAssign {
                name: _, value: v, ..
            }
            | DivAssign {
                name: _, value: v, ..
            }
            | ModAssign {
                name: _, value: v, ..
            }
            | AndAssign {
                name: _, value: v, ..
            }
            | OrAssign {
                name: _, value: v, ..
            }
            | XorAssign {
                name: _, value: v, ..
            }
            | ShlAssign {
                name: _, value: v, ..
            }
            | ShrAssign {
                name: _, value: v, ..
            } => self.expr_has_var(v),
            FString { segs: parts, .. } => parts.iter().any(|p| self.expr_has_var(p)),
            Cast { expr: inner, .. } => self.expr_has_var(inner),
        }
    }
}

fn order_vm_functions(selected: &mut Vec<(String, Expr)>) -> Vec<Expr> {
    if selected.is_empty() {
        return Vec::new();
    }
    selected.sort_by(|a, b| a.0.cmp(&b.0));
    let decls: BTreeMap<String, Expr> = selected.drain(..).collect();
    let mut ordered: Vec<Expr> = Vec::new();
    let mut done: HashSet<String> = HashSet::new();
    let mut in_progress: HashSet<String> = HashSet::new();

    fn emit(
        name: &str,
        decls: &BTreeMap<String, Expr>,
        ordered: &mut Vec<Expr>,
        done: &mut HashSet<String>,
        in_progress: &mut HashSet<String>,
    ) {
        if done.contains(name) || in_progress.contains(name) {
            return;
        }
        let Some(decl) = decls.get(name) else {
            return;
        };
        in_progress.insert(name.to_string());
        let mut deps: BTreeSet<String> = BTreeSet::new();
        collect_var_refs(decl, &mut deps);
        for dep in deps {
            emit(&dep, decls, ordered, done, in_progress);
        }
        ordered.push(decl.clone());
        in_progress.remove(name);
        done.insert(name.to_string());
    }

    let names: Vec<String> = decls.keys().cloned().collect();
    for name in names {
        emit(&name, &decls, &mut ordered, &mut done, &mut in_progress);
    }
    ordered
}

fn collect_var_refs(expr: &Expr, out: &mut BTreeSet<String>) {
    use Expr::*;
    match expr {
        Int { .. }
        | Float { .. }
        | Char { .. }
        | Bool { .. }
        | String { .. }
        | Nil(_)
        | Continue(_)
        | TypeDef(_)
        | Struct { .. }
        | Union { .. }
        | Enum { .. }
        | GlobalVar { .. }
        | ExternVar { .. } => {}
        Break { value: v, .. } => {
            if let Some(v) = v {
                collect_var_refs(v, out);
            }
        }
        Var { name, .. } => {
            out.insert(name.clone());
        }
        FuncDecl {
            name: _,
            attrs: _,
            type_params: _,
            params: _,
            return_type: _,
            body,
            ..
        } => {
            collect_var_refs(body, out);
        }
        Call {
            callee: f,
            type_args: _,
            args,
            ..
        } => {
            collect_var_refs(f, out);
            for a in args {
                collect_var_refs(a, out);
            }
        }
        Block { stmts, .. } => {
            for s in stmts {
                collect_var_refs(s, out);
            }
        }
        If {
            cond: c,
            then_branch: t,
            else_branch: e,
            ..
        } => {
            collect_var_refs(c, out);
            collect_var_refs(t, out);
            if let Some(x) = e {
                collect_var_refs(x, out);
            }
        }
        While {
            cond: c, body: b, ..
        }
        | Range {
            start: c, end: b, ..
        } => {
            collect_var_refs(c, out);
            collect_var_refs(b, out);
        }
        For {
            var: _,
            iterable: i,
            body: b,
            ..
        } => {
            collect_var_refs(i, out);
            collect_var_refs(b, out);
        }
        Match {
            target: s,
            branches: arms,
            default: d,
            ..
        } => {
            collect_var_refs(s, out);
            for (p, g, a) in arms {
                collect_var_refs(p, out);
                if let Some(g) = g {
                    collect_var_refs(g, out);
                }
                collect_var_refs(a, out);
            }
            if let Some(x) = d {
                collect_var_refs(x, out);
            }
        }
        Return { value: v, .. }
        | Not { expr: v, .. }
        | BNot { expr: v, .. }
        | Neg { expr: v, .. }
        | FNeg { expr: v, .. }
        | AddressOf { expr: v, .. }
        | Deref { expr: v, .. } => collect_var_refs(v, out),
        Lambda {
            params: _, body: b, ..
        } => collect_var_refs(b, out),
        VarDecl {
            name: _,
            ty: _,
            value: v,
            ..
        }
        | ConstDecl {
            name: _,
            ty: _,
            value: v,
            ..
        } => collect_var_refs(v, out),
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
        }
        | Index {
            array: l, index: r, ..
        }
        | DerefAssign {
            ptr: l, value: r, ..
        } => {
            collect_var_refs(l, out);
            collect_var_refs(r, out);
        }
        IndexAssign {
            target: o,
            value: v,
            ..
        }
        | MemberAssign {
            obj: o,
            field: _,
            value: v,
            ..
        } => {
            collect_var_refs(o, out);
            collect_var_refs(v, out);
        }
        ArrayLiteral {
            elements: items, ..
        } => {
            for it in items {
                collect_var_refs(it, out);
            }
        }
        ArrayFill {
            elem_type: _, len, ..
        } => collect_var_refs(len, out),
        StructLiteral {
            name: _,
            type_args: _,
            fields,
            ..
        }
        | UnionLiteral {
            name: _,
            type_args: _,
            fields,
            ..
        } => {
            for (_, v) in fields {
                collect_var_refs(v, out);
            }
        }
        MemberAccess { obj: o, .. } => collect_var_refs(o, out),
        Inc { .. } | Dec { .. } => {}
        VarAssign {
            name: _, value: v, ..
        }
        | AddAssign {
            name: _, value: v, ..
        }
        | SubAssign {
            name: _, value: v, ..
        }
        | MulAssign {
            name: _, value: v, ..
        }
        | DivAssign {
            name: _, value: v, ..
        }
        | ModAssign {
            name: _, value: v, ..
        }
        | AndAssign {
            name: _, value: v, ..
        }
        | OrAssign {
            name: _, value: v, ..
        }
        | XorAssign {
            name: _, value: v, ..
        }
        | ShlAssign {
            name: _, value: v, ..
        }
        | ShrAssign {
            name: _, value: v, ..
        } => collect_var_refs(v, out),
        FString { segs: parts, .. } => {
            for p in parts {
                collect_var_refs(p, out);
            }
        }
        Cast { expr: inner, .. } => collect_var_refs(inner, out),
    }
}

pub(super) fn native_sig(params: &[(String, Type)], ret_type: &Type) -> Option<NativeSig> {
    let f = |ty: &Type| -> Option<NativeKind> {
        match ty {
            Type::Primitive(Primitive::Int) => Some(NativeKind::Int),
            Type::Primitive(Primitive::Float) => Some(NativeKind::Float),
            Type::Primitive(Primitive::Boolean) => Some(NativeKind::Bool),
            Type::Primitive(Primitive::String) => Some(NativeKind::Str),
            _ => None,
        }
    };
    let mut kinds: Vec<NativeKind> = Vec::new();
    for (_, pty) in params {
        kinds.push(f(pty)?);
    }
    let ret = f(ret_type)?;
    Some(NativeSig {
        params: Box::leak(kinds.into_boxed_slice()),
        ret,
    })
}
