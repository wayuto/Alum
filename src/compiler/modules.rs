use crate::compiler::parser::{Expr, Type};
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeclKind {
    Fn,
    Struct,
    Union,
    Enum,
    Const,
    GlobalVar,
    ExternVar,
    ExternFn,
}

#[derive(Debug, Clone, Default)]
pub struct LoadedModule {
    pub names: HashMap<String, String>,

    pub pub_names: std::collections::HashSet<String>,
    pub kinds: HashMap<String, DeclKind>,

    pub structs: HashMap<String, (Vec<String>, Vec<(String, Type)>)>,
    pub unions: HashMap<String, (Vec<String>, Vec<(String, Type)>)>,
    pub enums: HashMap<String, Vec<(String, isize)>>,
    pub typedefs: HashMap<String, Type>,
}

pub struct ModuleLoader {
    pub include_paths: Vec<String>,
    pub loading: Vec<String>,
    pub loaded: HashMap<String, LoadedModule>,
}

impl ModuleLoader {
    pub fn new(include_paths: Vec<String>) -> Self {
        Self {
            include_paths,
            loading: Vec::new(),
            loaded: HashMap::new(),
        }
    }

    fn search_dirs(&self, base_path: &str) -> Vec<String> {
        let mut dirs: Vec<String> = Vec::new();
        if !base_path.is_empty() {
            let dir = Path::new(base_path)
                .parent()
                .and_then(|p| p.to_str())
                .filter(|s| !s.is_empty())
                .unwrap_or(".");
            dirs.push(dir.to_string());
        }
        dirs.extend(self.include_paths.iter().cloned());
        dirs.push("/usr/local/include/alum".to_string());
        dirs.push("/usr/local/alum".to_string());
        dirs
    }

    pub fn find_file(&self, name: &str, base_path: &str) -> Option<String> {
        for dir in self.search_dirs(base_path) {
            for ext in [".al", ".ah"] {
                let p = format!("{}/{}{}", dir, name, ext);
                if Path::new(&p).exists() {
                    return Some(p);
                }
            }
        }
        None
    }

    pub fn dir_exists(&self, name: &str, base_path: &str) -> bool {
        self.search_dirs(base_path)
            .iter()
            .any(|d| Path::new(d).join(name).is_dir())
    }

    pub fn build_names_map(
        mod_name: &str,
        own_decls: &[(String, DeclKind, bool)],
    ) -> HashMap<String, String> {
        let sym_mod = mod_name.replace('/', "__");
        let mut map = HashMap::new();
        for (name, kind, _) in own_decls {
            let final_name = match kind {
                DeclKind::ExternVar => name.clone(),
                _ => format!("{}__{}", sym_mod, name),
            };
            map.insert(name.clone(), final_name);
        }
        map
    }

    pub fn rename_module(body: &mut Vec<Expr>, map: &HashMap<String, String>) {
        for expr in body.iter_mut() {
            rename_expr(expr, map, &mut Vec::new());
        }
    }

    pub fn strip_module_pub(body: &mut Vec<Expr>) {
        for expr in body.iter_mut() {
            match expr {
                Expr::FuncDecl { name: _, attrs, .. } => attrs.is_pub = false,
                Expr::ConstDecl {
                    name: _,
                    ty: _,
                    value: _,
                    is_pub,
                    ..
                } => *is_pub = false,
                Expr::GlobalVar {
                    name: _, is_pub, ..
                } => *is_pub = false,
                _ => {}
            }
        }
    }
}

fn rename_type(ty: &mut Type, map: &HashMap<String, String>) {
    match ty {
        Type::Pointer(inner) => rename_type(inner, map),
        Type::Array(inner) => rename_type(inner, map),
        Type::Function(params, ret) => {
            for p in params.iter_mut() {
                rename_type(p, map);
            }
            rename_type(ret, map);
        }
        Type::Struct(name, args) => {
            if let Some(n) = map.get(name) {
                *name = n.clone();
            }
            for a in args.iter_mut() {
                rename_type(a, map);
            }
        }
        Type::Union(name, args) => {
            if let Some(n) = map.get(name) {
                *name = n.clone();
            }
            for a in args.iter_mut() {
                rename_type(a, map);
            }
        }
        _ => {}
    }
}

fn rename_expr(e: &mut Expr, map: &HashMap<String, String>, locals: &mut Vec<String>) {
    fn ren(name: &mut String, map: &HashMap<String, String>, locals: &[String]) {
        if locals.iter().any(|l| l == name) {
            return;
        }
        if let Some(n) = map.get(name) {
            *name = n.clone();
        }
    }
    match e {
        Expr::Var { name, .. } => ren(name, map, locals),
        Expr::Inc { name, .. } | Expr::Dec { name, .. } => ren(name, map, locals),
        Expr::BAnd {
            left: l, right: r, ..
        }
        | Expr::BOr {
            left: l, right: r, ..
        } => {
            rename_expr(l, map, locals);
            rename_expr(r, map, locals);
        }
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
            ren(name, map, locals);
            rename_expr(v, map, locals);
        }
        Expr::VarDecl {
            name, ty, value: v, ..
        } => {
            rename_type(ty, map);
            rename_expr(v, map, locals);

            locals.push(name.clone());
        }
        Expr::ConstDecl {
            name, ty, value: v, ..
        } => {
            rename_type(ty, map);
            rename_expr(v, map, locals);
            if locals.is_empty() {
                ren(name, map, locals);
            } else {
                locals.push(name.clone());
            }
        }
        Expr::GlobalVar {
            name,
            is_pub: _,
            ty,
            value: v,
            ..
        } => {
            ren(name, map, locals);
            rename_type(ty, map);
            if let Some(v) = v {
                rename_expr(v, map, locals);
            }
        }
        Expr::FuncDecl {
            name,
            attrs: _,
            type_params: _,
            params,
            return_type: ret,
            body,
            ..
        } => {
            ren(name, map, locals);
            let mark = locals.len();
            for (p, t) in params.iter_mut() {
                rename_type(t, map);
                locals.push(p.clone());
            }
            rename_type(ret, map);
            rename_expr(body, map, locals);
            locals.truncate(mark);
        }
        Expr::ExternVar { name: _, ty, .. } => rename_type(ty, map),
        Expr::Struct {
            name,
            type_params: _,
            fields,
            ..
        } => {
            ren(name, map, locals);
            for (_, t) in fields {
                rename_type(t, map);
            }
        }
        Expr::Union {
            name,
            type_params: _,
            fields,
            ..
        } => {
            ren(name, map, locals);
            for (_, t) in fields {
                rename_type(t, map);
            }
        }
        Expr::Enum { name, .. } => ren(name, map, locals),
        Expr::StructLiteral {
            name,
            type_args: args,
            fields,
            ..
        }
        | Expr::UnionLiteral {
            name,
            type_args: args,
            fields,
            ..
        } => {
            ren(name, map, locals);
            for a in args {
                rename_type(a, map);
            }
            for (_, v) in fields {
                rename_expr(v, map, locals);
            }
        }
        Expr::Call {
            callee,
            type_args: targs,
            args,
            ..
        } => {
            rename_expr(callee, map, locals);
            for t in targs {
                rename_type(t, map);
            }
            for a in args {
                rename_expr(a, map, locals);
            }
        }
        Expr::Return { value: v, .. } => rename_expr(v, map, locals),
        Expr::If {
            cond: c,
            then_branch: t,
            else_branch: e,
            ..
        } => {
            rename_expr(c, map, locals);
            rename_expr(t, map, locals);
            if let Some(e) = e {
                rename_expr(e, map, locals);
            }
        }
        Expr::While {
            cond: c, body: b, ..
        } => {
            rename_expr(c, map, locals);
            rename_expr(b, map, locals);
        }
        Expr::Block { stmts: body, .. } => {
            let mark = locals.len();
            for b in body {
                rename_expr(b, map, locals);
            }
            locals.truncate(mark);
        }
        Expr::Index {
            array: a, index: i, ..
        }
        | Expr::IndexAssign {
            target: a,
            value: i,
            ..
        } => {
            rename_expr(a, map, locals);
            rename_expr(i, map, locals);
        }
        Expr::ArrayLiteral { elements: es, .. } => {
            for e in es {
                rename_expr(e, map, locals);
            }
        }
        Expr::ArrayFill {
            elem_type: ty, len, ..
        } => {
            rename_type(ty, map);
            rename_expr(len, map, locals);
        }
        Expr::Range {
            start: s, end: e, ..
        } => {
            rename_expr(s, map, locals);
            rename_expr(e, map, locals);
        }
        Expr::For {
            var,
            iterable: arr,
            body,
            ..
        } => {
            rename_expr(arr, map, locals);
            let mark = locals.len();
            locals.push(var.clone());
            rename_expr(body, map, locals);
            locals.truncate(mark);
        }
        Expr::TypeDef(_) => {}
        Expr::Match {
            target: t,
            branches: br,
            default,
            ..
        } => {
            rename_expr(t, map, locals);
            for (c, g, r) in br {
                rename_expr(c, map, locals);
                if let Some(g) = g {
                    rename_expr(g, map, locals);
                }
                rename_expr(r, map, locals);
            }
            if let Some(d) = default {
                rename_expr(d, map, locals);
            }
        }
        Expr::MemberAccess { obj: o, .. } => rename_expr(o, map, locals),
        Expr::MemberAssign {
            obj: o,
            field: _,
            value: v,
            ..
        } => {
            rename_expr(o, map, locals);
            rename_expr(v, map, locals);
        }
        Expr::Lambda {
            params,
            body,
            return_type: ret,
            ..
        } => {
            let mark = locals.len();
            for (p, t) in params.iter_mut() {
                rename_type(t, map);
                locals.push(p.clone());
            }
            rename_type(ret, map);
            rename_expr(body, map, locals);
            locals.truncate(mark);
        }
        Expr::AddressOf { expr: x, .. }
        | Expr::Deref { expr: x, .. }
        | Expr::BNot { expr: x, .. } => rename_expr(x, map, locals),
        Expr::DerefAssign {
            ptr: p, value: v, ..
        } => {
            rename_expr(p, map, locals);
            rename_expr(v, map, locals);
        }
        Expr::Cast { expr: x, ty, .. } => {
            rename_expr(x, map, locals);
            rename_type(ty, map);
        }
        Expr::FString { segs, .. } => {
            for s in segs {
                rename_expr(s, map, locals);
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
        | Expr::StrCat {
            left: l, right: r, ..
        } => {
            rename_expr(l, map, locals);
            rename_expr(r, map, locals);
        }
        Expr::Neg { expr: x, .. } | Expr::FNeg { expr: x, .. } | Expr::Not { expr: x, .. } => {
            rename_expr(x, map, locals)
        }
        Expr::Break { value: v, .. } => {
            if let Some(v) = v {
                rename_expr(v, map, locals);
            }
        }
        Expr::Int { .. }
        | Expr::Float { .. }
        | Expr::Char { .. }
        | Expr::Bool { .. }
        | Expr::String { .. }
        | Expr::Nil(_)
        | Expr::Continue(_) => {}
    }
}
