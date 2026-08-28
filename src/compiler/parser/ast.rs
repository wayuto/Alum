use crate::compiler::Span;
use std::fmt;

#[derive(Debug, Clone)]
pub struct Program {
    pub body: Vec<Expr>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Primitive {
    Int,
    Float,
    String,
    Boolean,
    Void,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Type {
    Primitive(Primitive),
    Pointer(Box<Type>),
    Array(Box<Type>),
    Function(Vec<Type>, Box<Type>),
    Struct(String, Vec<Type>),
    Union(String, Vec<Type>),
    Param(usize),
    TypeVar(usize),
    Unknown,
}

impl Type {
    pub fn is_float(&self) -> bool {
        matches!(self, Type::Primitive(Primitive::Float))
    }

    pub fn is_string(&self) -> bool {
        matches!(self, Type::Primitive(Primitive::String))
    }

    pub fn is_bool(&self) -> bool {
        matches!(self, Type::Primitive(Primitive::Boolean))
    }

    pub fn is_pointer(&self) -> bool {
        matches!(self, Type::Pointer(_))
    }

    pub fn pointee(&self) -> Option<&Type> {
        match self {
            Type::Pointer(inner) => Some(inner.as_ref()),
            _ => None,
        }
    }

    pub fn is_numeric(&self) -> bool {
        matches!(
            self,
            Type::Primitive(Primitive::Int) | Type::Primitive(Primitive::Float) | Type::TypeVar(_)
        )
    }

    pub fn substitute(&self, args: &[Type]) -> Type {
        match self {
            Type::Param(id) => args.get(*id).cloned().unwrap_or_else(|| self.clone()),
            Type::Pointer(inner) => Type::Pointer(Box::new(inner.substitute(args))),
            Type::Array(inner) => Type::Array(Box::new(inner.substitute(args))),
            Type::Function(params, ret) => Type::Function(
                params.iter().map(|p| p.substitute(args)).collect(),
                Box::new(ret.substitute(args)),
            ),
            Type::Struct(name, type_args) => Type::Struct(
                name.clone(),
                type_args.iter().map(|t| t.substitute(args)).collect(),
            ),
            Type::Union(name, type_args) => Type::Union(
                name.clone(),
                type_args.iter().map(|t| t.substitute(args)).collect(),
            ),
            _ => self.clone(),
        }
    }

    pub fn mangle(&self) -> String {
        match self {
            Type::Primitive(p) => match p {
                Primitive::Int => "int".to_string(),
                Primitive::Float => "float".to_string(),
                Primitive::String => "str".to_string(),
                Primitive::Boolean => "bool".to_string(),
                Primitive::Void => "void".to_string(),
            },
            Type::Pointer(inner) => format!("ptr_{}", inner.mangle()),
            Type::Array(inner) => format!("arr_{}", inner.mangle()),
            Type::Function(params, ret) => {
                let param_str: Vec<String> = params.iter().map(|p| p.mangle()).collect();
                format!("fn_{}_{}", param_str.join("_"), ret.mangle())
            }
            Type::Struct(name, args) => {
                let arg_str: Vec<String> = args.iter().map(|t| t.mangle()).collect();
                format!("{}_{}", name, arg_str.join("_"))
            }
            Type::Union(name, args) => {
                let arg_str: Vec<String> = args.iter().map(|t| t.mangle()).collect();
                format!("{}_{}", name, arg_str.join("_"))
            }
            Type::Param(id) => format!("param_{}", id),
            Type::TypeVar(id) => format!("tv_{}", id),
            Type::Unknown => "unknown".to_string(),
        }
    }
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Type::Primitive(p) => match p {
                Primitive::Int => write!(f, "int"),
                Primitive::Float => write!(f, "float"),
                Primitive::String => write!(f, "string"),
                Primitive::Boolean => write!(f, "bool"),
                Primitive::Void => write!(f, "void"),
            },
            Type::Array(inner) => write!(f, "{}[]", inner),
            Type::Pointer(inner) => write!(f, "*{}", inner),
            Type::Function(params, ret) => {
                let param_str: Vec<String> = params.iter().map(|p| p.to_string()).collect();
                write!(f, "{}({})", ret, param_str.join(", "))
            }
            Type::Struct(name, args) => {
                if args.is_empty() {
                    write!(f, "{}", name)
                } else {
                    let arg_str: Vec<String> = args.iter().map(|t| t.to_string()).collect();
                    write!(f, "{}<{}>", name, arg_str.join(", "))
                }
            }
            Type::Union(name, args) => {
                if args.is_empty() {
                    write!(f, "{}", name)
                } else {
                    let arg_str: Vec<String> = args.iter().map(|t| t.to_string()).collect();
                    write!(f, "{}<{}>", name, arg_str.join(", "))
                }
            }
            Type::Param(id) => write!(f, "P{}", id),
            Type::TypeVar(id) => write!(f, "T{}", id),
            Type::Unknown => write!(f, "auto"),
        }
    }
}

impl Expr {
    pub fn span(&self) -> Span {
        match self {
            Expr::Int(_, s)
            | Expr::Float(_, s)
            | Expr::Bool(_, s)
            | Expr::String(_, s)
            | Expr::Nil(s)
            | Expr::Add(_, _, s)
            | Expr::Sub(_, _, s)
            | Expr::Mul(_, _, s)
            | Expr::Div(_, _, s)
            | Expr::Mod(_, _, s)
            | Expr::Shl(_, _, s)
            | Expr::Shr(_, _, s)
            | Expr::FAdd(_, _, s)
            | Expr::FSub(_, _, s)
            | Expr::FMul(_, _, s)
            | Expr::FDiv(_, _, s)
            | Expr::Eq(_, _, s)
            | Expr::Ne(_, _, s)
            | Expr::Lt(_, _, s)
            | Expr::Le(_, _, s)
            | Expr::Gt(_, _, s)
            | Expr::Ge(_, _, s)
            | Expr::FEq(_, _, s)
            | Expr::FNe(_, _, s)
            | Expr::FLt(_, _, s)
            | Expr::FLe(_, _, s)
            | Expr::FGt(_, _, s)
            | Expr::FGe(_, _, s)
            | Expr::Not(_, s)
            | Expr::StrCat(_, _, s)
            | Expr::Var(_, s)
            | Expr::VarDecl(_, _, _, s)
            | Expr::ConstDecl(_, _, _, _, s)
            | Expr::GlobalVar(_, _, _, _, s)
            | Expr::ExternVar(_, _, s)
            | Expr::VarAssign(_, _, s)
            | Expr::FuncDecl(_, _, _, _, _, _, s)
            | Expr::Call(_, _, _, s)
            | Expr::Return(_, s)
            | Expr::If(_, _, _, s)
            | Expr::While(_, _, s)
            | Expr::Break(_, s)
            | Expr::Continue(s)
            | Expr::Block(_, s)
            | Expr::Index(_, _, s)
            | Expr::IndexAssign(_, _, s)
            | Expr::ArrayLiteral(_, s)
            | Expr::ArrayFill(_, _, s)
            | Expr::Range(_, _, s)
            | Expr::For(_, _, _, s)
            | Expr::TypeDef(s)
            | Expr::Match(_, _, _, s)
            | Expr::Struct(_, _, _, s)
            | Expr::StructLiteral(_, _, _, s)
            | Expr::Union(_, _, _, s)
            | Expr::UnionLiteral(_, _, _, s)
            | Expr::Enum(_, _, s)
            | Expr::MemberAccess(_, _, s)
            | Expr::MemberAssign(_, _, _, s)
            | Expr::Lambda(_, _, _, s)
            | Expr::Neg(_, s)
            | Expr::FNeg(_, s)
            | Expr::BNot(_, s)
            | Expr::Inc(_, s)
            | Expr::Dec(_, s)
            | Expr::Xor(_, _, s)
            | Expr::BAnd(_, _, s)
            | Expr::BOr(_, _, s)
            | Expr::LAnd(_, _, s)
            | Expr::LOr(_, _, s)
            | Expr::AddAssign(_, _, s)
            | Expr::SubAssign(_, _, s)
            | Expr::MulAssign(_, _, s)
            | Expr::DivAssign(_, _, s)
            | Expr::ModAssign(_, _, s)
            | Expr::AndAssign(_, _, s)
            | Expr::OrAssign(_, _, s)
            | Expr::XorAssign(_, _, s)
            | Expr::ShlAssign(_, _, s)
            | Expr::ShrAssign(_, _, s)
            | Expr::AddressOf(_, s)
            | Expr::Deref(_, s)
            | Expr::DerefAssign(_, _, s)
            | Expr::Cast(_, _, s)
            | Expr::FString(_, s) => *s,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct FuncAttrs {
    pub is_pub: bool,
    pub is_external: bool,
    pub is_pure: bool,
    pub link_name: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Expr {
    Int(isize, Span),
    Float(f64, Span),
    Bool(bool, Span),
    String(String, Span),
    Nil(Span),
    Add(Box<Expr>, Box<Expr>, Span),
    Sub(Box<Expr>, Box<Expr>, Span),
    Mul(Box<Expr>, Box<Expr>, Span),
    Div(Box<Expr>, Box<Expr>, Span),
    Mod(Box<Expr>, Box<Expr>, Span),
    FAdd(Box<Expr>, Box<Expr>, Span),
    FSub(Box<Expr>, Box<Expr>, Span),
    FMul(Box<Expr>, Box<Expr>, Span),
    FDiv(Box<Expr>, Box<Expr>, Span),
    Eq(Box<Expr>, Box<Expr>, Span),
    Ne(Box<Expr>, Box<Expr>, Span),
    Lt(Box<Expr>, Box<Expr>, Span),
    Le(Box<Expr>, Box<Expr>, Span),
    Gt(Box<Expr>, Box<Expr>, Span),
    Ge(Box<Expr>, Box<Expr>, Span),
    FEq(Box<Expr>, Box<Expr>, Span),
    FNe(Box<Expr>, Box<Expr>, Span),
    FLt(Box<Expr>, Box<Expr>, Span),
    FLe(Box<Expr>, Box<Expr>, Span),
    FGt(Box<Expr>, Box<Expr>, Span),
    FGe(Box<Expr>, Box<Expr>, Span),
    Neg(Box<Expr>, Span),
    FNeg(Box<Expr>, Span),
    Not(Box<Expr>, Span),
    Inc(String, Span),
    Dec(String, Span),
    Xor(Box<Expr>, Box<Expr>, Span),
    BAnd(Box<Expr>, Box<Expr>, Span),
    BOr(Box<Expr>, Box<Expr>, Span),
    LAnd(Box<Expr>, Box<Expr>, Span),
    LOr(Box<Expr>, Box<Expr>, Span),
    Shl(Box<Expr>, Box<Expr>, Span),
    Shr(Box<Expr>, Box<Expr>, Span),
    BNot(Box<Expr>, Span),
    AddAssign(String, Box<Expr>, Span),
    SubAssign(String, Box<Expr>, Span),
    MulAssign(String, Box<Expr>, Span),
    DivAssign(String, Box<Expr>, Span),
    ModAssign(String, Box<Expr>, Span),
    AndAssign(String, Box<Expr>, Span),
    OrAssign(String, Box<Expr>, Span),
    XorAssign(String, Box<Expr>, Span),
    ShlAssign(String, Box<Expr>, Span),
    ShrAssign(String, Box<Expr>, Span),
    StrCat(Box<Expr>, Box<Expr>, Span),
    Var(String, Span),
    VarDecl(String, Type, Box<Expr>, Span),
    ConstDecl(String, Type, Box<Expr>, bool, Span),
    GlobalVar(String, bool, Type, Option<Box<Expr>>, Span),
    VarAssign(String, Box<Expr>, Span),
    FuncDecl(
        String,
        FuncAttrs,
        Vec<String>,
        Vec<(String, Type)>,
        Type,
        Box<Expr>,
        Span,
    ),
    ExternVar(String, Type, Span),
    Call(Box<Expr>, Vec<Type>, Vec<Expr>, Span),
    Return(Box<Expr>, Span),
    If(Box<Expr>, Box<Expr>, Option<Box<Expr>>, Span),
    While(Box<Expr>, Box<Expr>, Span),
    Break(Option<Box<Expr>>, Span),
    Continue(Span),
    Block(Vec<Expr>, Span),
    Index(Box<Expr>, Box<Expr>, Span),
    IndexAssign(Box<Expr>, Box<Expr>, Span),
    ArrayLiteral(Vec<Expr>, Span),
    ArrayFill(Type, Box<Expr>, Span),
    Range(Box<Expr>, Box<Expr>, Span),
    For(String, Box<Expr>, Box<Expr>, Span),
    TypeDef(Span),
    Match(
        Box<Expr>,
        Vec<(Expr, Option<Box<Expr>>, Expr)>,
        Option<Box<Expr>>,
        Span,
    ),
    Struct(String, Vec<String>, Vec<(String, Type)>, Span),
    StructLiteral(String, Vec<Type>, Vec<(String, Expr)>, Span),
    Union(String, Vec<String>, Vec<(String, Type)>, Span),
    UnionLiteral(String, Vec<Type>, Vec<(String, Expr)>, Span),
    Enum(String, Vec<(String, isize)>, Span),
    MemberAccess(Box<Expr>, String, Span),
    MemberAssign(Box<Expr>, String, Box<Expr>, Span),
    Lambda(Vec<(String, Type)>, Box<Expr>, Type, Span),
    AddressOf(Box<Expr>, Span),
    Deref(Box<Expr>, Span),
    DerefAssign(Box<Expr>, Box<Expr>, Span),
    Cast(Box<Expr>, Type, Span),
    FString(Vec<Expr>, Span),
}
