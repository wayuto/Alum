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
                Expr::FuncDecl(_, attrs, _, _, _, _, _) => attrs.is_pub = false,
                Expr::ConstDecl(_, _, _, is_pub, _) => *is_pub = false,
                Expr::GlobalVar(_, is_pub, _, _, _) => *is_pub = false,
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
        Expr::Var(name, _) => ren(name, map, locals),
        Expr::Inc(name, _) | Expr::Dec(name, _) => ren(name, map, locals),
        Expr::VarAssign(name, v, _)
        | Expr::AddAssign(name, v, _)
        | Expr::SubAssign(name, v, _)
        | Expr::MulAssign(name, v, _)
        | Expr::DivAssign(name, v, _)
        | Expr::ModAssign(name, v, _)
        | Expr::AndAssign(name, v, _)
        | Expr::OrAssign(name, v, _)
        | Expr::XorAssign(name, v, _)
        | Expr::ShlAssign(name, v, _)
        | Expr::ShrAssign(name, v, _) => {
            ren(name, map, locals);
            rename_expr(v, map, locals);
        }
        Expr::VarDecl(name, ty, v, _) => {
            rename_type(ty, map);
            rename_expr(v, map, locals);

            locals.push(name.clone());
        }
        Expr::ConstDecl(name, ty, v, _, _) => {
            rename_type(ty, map);
            rename_expr(v, map, locals);
            if locals.is_empty() {
                ren(name, map, locals);
            } else {
                locals.push(name.clone());
            }
        }
        Expr::GlobalVar(name, _, ty, v, _) => {
            ren(name, map, locals);
            rename_type(ty, map);
            if let Some(v) = v {
                rename_expr(v, map, locals);
            }
        }
        Expr::FuncDecl(name, _, _, params, ret, body, _) => {
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
        Expr::ExternVar(_, ty, _) => rename_type(ty, map),
        Expr::Struct(name, _, fields, _) => {
            ren(name, map, locals);
            for (_, t) in fields {
                rename_type(t, map);
            }
        }
        Expr::Union(name, _, fields, _) => {
            ren(name, map, locals);
            for (_, t) in fields {
                rename_type(t, map);
            }
        }
        Expr::Enum(name, _, _) => ren(name, map, locals),
        Expr::StructLiteral(name, args, fields, _) | Expr::UnionLiteral(name, args, fields, _) => {
            ren(name, map, locals);
            for a in args {
                rename_type(a, map);
            }
            for (_, v) in fields {
                rename_expr(v, map, locals);
            }
        }
        Expr::Call(callee, targs, args, _) => {
            rename_expr(callee, map, locals);
            for t in targs {
                rename_type(t, map);
            }
            for a in args {
                rename_expr(a, map, locals);
            }
        }
        Expr::Return(v, _) => rename_expr(v, map, locals),
        Expr::If(c, t, e, _) => {
            rename_expr(c, map, locals);
            rename_expr(t, map, locals);
            if let Some(e) = e {
                rename_expr(e, map, locals);
            }
        }
        Expr::While(c, b, _) => {
            rename_expr(c, map, locals);
            rename_expr(b, map, locals);
        }
        Expr::Block(body, _) => {
            let mark = locals.len();
            for b in body {
                rename_expr(b, map, locals);
            }
            locals.truncate(mark);
        }
        Expr::Index(a, i, _) | Expr::IndexAssign(a, i, _) => {
            rename_expr(a, map, locals);
            rename_expr(i, map, locals);
        }
        Expr::ArrayLiteral(es, _) => {
            for e in es {
                rename_expr(e, map, locals);
            }
        }
        Expr::ArrayFill(ty, len, _) => {
            rename_type(ty, map);
            rename_expr(len, map, locals);
        }
        Expr::Range(s, e, _) => {
            rename_expr(s, map, locals);
            rename_expr(e, map, locals);
        }
        Expr::For(var, arr, body, _) => {
            rename_expr(arr, map, locals);
            let mark = locals.len();
            locals.push(var.clone());
            rename_expr(body, map, locals);
            locals.truncate(mark);
        }
        Expr::TypeDef(_) => {}
        Expr::Match(t, br, default, _) => {
            rename_expr(t, map, locals);
            for (c, r) in br {
                rename_expr(c, map, locals);
                rename_expr(r, map, locals);
            }
            if let Some(d) = default {
                rename_expr(d, map, locals);
            }
        }
        Expr::MemberAccess(o, _, _) => rename_expr(o, map, locals),
        Expr::MemberAssign(o, _, v, _) => {
            rename_expr(o, map, locals);
            rename_expr(v, map, locals);
        }
        Expr::Lambda(params, body, ret, _) => {
            let mark = locals.len();
            for (p, t) in params.iter_mut() {
                rename_type(t, map);
                locals.push(p.clone());
            }
            rename_type(ret, map);
            rename_expr(body, map, locals);
            locals.truncate(mark);
        }
        Expr::AddressOf(x, _) | Expr::Deref(x, _) | Expr::BNot(x, _) => rename_expr(x, map, locals),
        Expr::DerefAssign(p, v, _) => {
            rename_expr(p, map, locals);
            rename_expr(v, map, locals);
        }
        Expr::Cast(x, ty, _) => {
            rename_expr(x, map, locals);
            rename_type(ty, map);
        }
        Expr::FString(segs, _) => {
            for s in segs {
                rename_expr(s, map, locals);
            }
        }
        Expr::Add(l, r, _)
        | Expr::Sub(l, r, _)
        | Expr::Mul(l, r, _)
        | Expr::Div(l, r, _)
        | Expr::Mod(l, r, _)
        | Expr::Xor(l, r, _)
        | Expr::LAnd(l, r, _)
        | Expr::LOr(l, r, _)
        | Expr::Shl(l, r, _)
        | Expr::Shr(l, r, _)
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
        | Expr::StrCat(l, r, _) => {
            rename_expr(l, map, locals);
            rename_expr(r, map, locals);
        }
        Expr::Neg(x, _) | Expr::FNeg(x, _) | Expr::Not(x, _) => rename_expr(x, map, locals),
        Expr::Int(_, _)
        | Expr::Float(_, _)
        | Expr::Bool(_, _)
        | Expr::String(_, _)
        | Expr::Nil(_)
        | Expr::Break(_)
        | Expr::Continue(_) => {}
    }
}
