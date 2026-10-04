use crate::compiler::Span;
use std::fmt;

#[derive(Debug, Clone)]
pub struct Program {
    pub body: Vec<Expr>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Primitive {
    Int,
    Char,
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
            Type::Primitive(Primitive::Int)
                | Type::Primitive(Primitive::Char)
                | Type::Primitive(Primitive::Float)
                | Type::TypeVar(_)
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
                Primitive::Char => "char".to_string(),
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
                Primitive::Char => write!(f, "char"),
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
            Expr::Int { value: _, span: s }
            | Expr::Char { value: _, span: s }
            | Expr::Float { value: _, span: s }
            | Expr::Bool { value: _, span: s }
            | Expr::String { value: _, span: s }
            | Expr::Nil(s)
            | Expr::Add {
                left: _,
                right: _,
                span: s,
            }
            | Expr::Sub {
                left: _,
                right: _,
                span: s,
            }
            | Expr::Mul {
                left: _,
                right: _,
                span: s,
            }
            | Expr::Div {
                left: _,
                right: _,
                span: s,
            }
            | Expr::Mod {
                left: _,
                right: _,
                span: s,
            }
            | Expr::Shl {
                left: _,
                right: _,
                span: s,
            }
            | Expr::Shr {
                left: _,
                right: _,
                span: s,
            }
            | Expr::FAdd {
                left: _,
                right: _,
                span: s,
            }
            | Expr::FSub {
                left: _,
                right: _,
                span: s,
            }
            | Expr::FMul {
                left: _,
                right: _,
                span: s,
            }
            | Expr::FDiv {
                left: _,
                right: _,
                span: s,
            }
            | Expr::Eq {
                left: _,
                right: _,
                span: s,
            }
            | Expr::Ne {
                left: _,
                right: _,
                span: s,
            }
            | Expr::Lt {
                left: _,
                right: _,
                span: s,
            }
            | Expr::Le {
                left: _,
                right: _,
                span: s,
            }
            | Expr::Gt {
                left: _,
                right: _,
                span: s,
            }
            | Expr::Ge {
                left: _,
                right: _,
                span: s,
            }
            | Expr::FEq {
                left: _,
                right: _,
                span: s,
            }
            | Expr::FNe {
                left: _,
                right: _,
                span: s,
            }
            | Expr::FLt {
                left: _,
                right: _,
                span: s,
            }
            | Expr::FLe {
                left: _,
                right: _,
                span: s,
            }
            | Expr::FGt {
                left: _,
                right: _,
                span: s,
            }
            | Expr::FGe {
                left: _,
                right: _,
                span: s,
            }
            | Expr::Not { expr: _, span: s }
            | Expr::StrCat {
                left: _,
                right: _,
                span: s,
            }
            | Expr::Var { name: _, span: s }
            | Expr::VarDecl {
                name: _,
                ty: _,
                value: _,
                span: s,
            }
            | Expr::ConstDecl {
                name: _,
                ty: _,
                value: _,
                is_pub: _,
                span: s,
            }
            | Expr::GlobalVar {
                name: _,
                is_pub: _,
                ty: _,
                value: _,
                span: s,
            }
            | Expr::ExternVar {
                name: _,
                ty: _,
                span: s,
            }
            | Expr::VarAssign {
                name: _,
                value: _,
                span: s,
            }
            | Expr::FuncDecl {
                name: _,
                attrs: _,
                type_params: _,
                params: _,
                return_type: _,
                body: _,
                span: s,
            }
            | Expr::Call {
                callee: _,
                type_args: _,
                args: _,
                span: s,
            }
            | Expr::Return { value: _, span: s }
            | Expr::If {
                cond: _,
                then_branch: _,
                else_branch: _,
                span: s,
            }
            | Expr::While {
                cond: _,
                body: _,
                span: s,
            }
            | Expr::Break { value: _, span: s }
            | Expr::Continue(s)
            | Expr::Block { stmts: _, span: s }
            | Expr::Index {
                array: _,
                index: _,
                span: s,
            }
            | Expr::IndexAssign {
                target: _,
                value: _,
                span: s,
            }
            | Expr::ArrayLiteral {
                elements: _,
                span: s,
            }
            | Expr::ArrayFill {
                elem_type: _,
                len: _,
                span: s,
            }
            | Expr::Range {
                start: _,
                end: _,
                inclusive: _,
                span: s,
            }
            | Expr::For {
                var: _,
                iterable: _,
                body: _,
                span: s,
            }
            | Expr::TypeDef(s)
            | Expr::Match {
                target: _,
                branches: _,
                default: _,
                span: s,
            }
            | Expr::Struct {
                name: _,
                type_params: _,
                fields: _,
                span: s,
            }
            | Expr::StructLiteral {
                name: _,
                type_args: _,
                fields: _,
                span: s,
            }
            | Expr::Union {
                name: _,
                type_params: _,
                fields: _,
                span: s,
            }
            | Expr::UnionLiteral {
                name: _,
                type_args: _,
                fields: _,
                span: s,
            }
            | Expr::Enum {
                name: _,
                members: _,
                span: s,
            }
            | Expr::MemberAccess {
                obj: _,
                field: _,
                span: s,
            }
            | Expr::MemberAssign {
                obj: _,
                field: _,
                value: _,
                span: s,
            }
            | Expr::Lambda {
                params: _,
                body: _,
                return_type: _,
                span: s,
            }
            | Expr::Neg { expr: _, span: s }
            | Expr::FNeg { expr: _, span: s }
            | Expr::BNot { expr: _, span: s }
            | Expr::Inc { name: _, span: s }
            | Expr::Dec { name: _, span: s }
            | Expr::Xor {
                left: _,
                right: _,
                span: s,
            }
            | Expr::BAnd {
                left: _,
                right: _,
                span: s,
            }
            | Expr::BOr {
                left: _,
                right: _,
                span: s,
            }
            | Expr::LAnd {
                left: _,
                right: _,
                span: s,
            }
            | Expr::LOr {
                left: _,
                right: _,
                span: s,
            }
            | Expr::AddAssign {
                name: _,
                value: _,
                span: s,
            }
            | Expr::SubAssign {
                name: _,
                value: _,
                span: s,
            }
            | Expr::MulAssign {
                name: _,
                value: _,
                span: s,
            }
            | Expr::DivAssign {
                name: _,
                value: _,
                span: s,
            }
            | Expr::ModAssign {
                name: _,
                value: _,
                span: s,
            }
            | Expr::AndAssign {
                name: _,
                value: _,
                span: s,
            }
            | Expr::OrAssign {
                name: _,
                value: _,
                span: s,
            }
            | Expr::XorAssign {
                name: _,
                value: _,
                span: s,
            }
            | Expr::ShlAssign {
                name: _,
                value: _,
                span: s,
            }
            | Expr::ShrAssign {
                name: _,
                value: _,
                span: s,
            }
            | Expr::AddressOf { expr: _, span: s }
            | Expr::Deref { expr: _, span: s }
            | Expr::DerefAssign {
                ptr: _,
                value: _,
                span: s,
            }
            | Expr::Cast {
                expr: _,
                ty: _,
                span: s,
            }
            | Expr::FString { segs: _, span: s } => *s,
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
    Int {
        value: isize,
        span: Span,
    },
    Char {
        value: u8,
        span: Span,
    },
    Float {
        value: f64,
        span: Span,
    },
    Bool {
        value: bool,
        span: Span,
    },
    String {
        value: String,
        span: Span,
    },
    Nil(Span),
    Add {
        left: Box<Expr>,
        right: Box<Expr>,
        span: Span,
    },
    Sub {
        left: Box<Expr>,
        right: Box<Expr>,
        span: Span,
    },
    Mul {
        left: Box<Expr>,
        right: Box<Expr>,
        span: Span,
    },
    Div {
        left: Box<Expr>,
        right: Box<Expr>,
        span: Span,
    },
    Mod {
        left: Box<Expr>,
        right: Box<Expr>,
        span: Span,
    },
    FAdd {
        left: Box<Expr>,
        right: Box<Expr>,
        span: Span,
    },
    FSub {
        left: Box<Expr>,
        right: Box<Expr>,
        span: Span,
    },
    FMul {
        left: Box<Expr>,
        right: Box<Expr>,
        span: Span,
    },
    FDiv {
        left: Box<Expr>,
        right: Box<Expr>,
        span: Span,
    },
    Eq {
        left: Box<Expr>,
        right: Box<Expr>,
        span: Span,
    },
    Ne {
        left: Box<Expr>,
        right: Box<Expr>,
        span: Span,
    },
    Lt {
        left: Box<Expr>,
        right: Box<Expr>,
        span: Span,
    },
    Le {
        left: Box<Expr>,
        right: Box<Expr>,
        span: Span,
    },
    Gt {
        left: Box<Expr>,
        right: Box<Expr>,
        span: Span,
    },
    Ge {
        left: Box<Expr>,
        right: Box<Expr>,
        span: Span,
    },
    FEq {
        left: Box<Expr>,
        right: Box<Expr>,
        span: Span,
    },
    FNe {
        left: Box<Expr>,
        right: Box<Expr>,
        span: Span,
    },
    FLt {
        left: Box<Expr>,
        right: Box<Expr>,
        span: Span,
    },
    FLe {
        left: Box<Expr>,
        right: Box<Expr>,
        span: Span,
    },
    FGt {
        left: Box<Expr>,
        right: Box<Expr>,
        span: Span,
    },
    FGe {
        left: Box<Expr>,
        right: Box<Expr>,
        span: Span,
    },
    Neg {
        expr: Box<Expr>,
        span: Span,
    },
    FNeg {
        expr: Box<Expr>,
        span: Span,
    },
    Not {
        expr: Box<Expr>,
        span: Span,
    },
    Inc {
        name: String,
        span: Span,
    },
    Dec {
        name: String,
        span: Span,
    },
    Xor {
        left: Box<Expr>,
        right: Box<Expr>,
        span: Span,
    },
    BAnd {
        left: Box<Expr>,
        right: Box<Expr>,
        span: Span,
    },
    BOr {
        left: Box<Expr>,
        right: Box<Expr>,
        span: Span,
    },
    LAnd {
        left: Box<Expr>,
        right: Box<Expr>,
        span: Span,
    },
    LOr {
        left: Box<Expr>,
        right: Box<Expr>,
        span: Span,
    },
    Shl {
        left: Box<Expr>,
        right: Box<Expr>,
        span: Span,
    },
    Shr {
        left: Box<Expr>,
        right: Box<Expr>,
        span: Span,
    },
    BNot {
        expr: Box<Expr>,
        span: Span,
    },
    AddAssign {
        name: String,
        value: Box<Expr>,
        span: Span,
    },
    SubAssign {
        name: String,
        value: Box<Expr>,
        span: Span,
    },
    MulAssign {
        name: String,
        value: Box<Expr>,
        span: Span,
    },
    DivAssign {
        name: String,
        value: Box<Expr>,
        span: Span,
    },
    ModAssign {
        name: String,
        value: Box<Expr>,
        span: Span,
    },
    AndAssign {
        name: String,
        value: Box<Expr>,
        span: Span,
    },
    OrAssign {
        name: String,
        value: Box<Expr>,
        span: Span,
    },
    XorAssign {
        name: String,
        value: Box<Expr>,
        span: Span,
    },
    ShlAssign {
        name: String,
        value: Box<Expr>,
        span: Span,
    },
    ShrAssign {
        name: String,
        value: Box<Expr>,
        span: Span,
    },
    StrCat {
        left: Box<Expr>,
        right: Box<Expr>,
        span: Span,
    },
    Var {
        name: String,
        span: Span,
    },
    VarDecl {
        name: String,
        ty: Type,
        value: Box<Expr>,
        span: Span,
    },
    ConstDecl {
        name: String,
        ty: Type,
        value: Box<Expr>,
        is_pub: bool,
        span: Span,
    },
    GlobalVar {
        name: String,
        is_pub: bool,
        ty: Type,
        value: Option<Box<Expr>>,
        span: Span,
    },
    VarAssign {
        name: String,
        value: Box<Expr>,
        span: Span,
    },
    FuncDecl {
        name: String,
        attrs: FuncAttrs,
        type_params: Vec<String>,
        params: Vec<(String, Type)>,
        return_type: Type,
        body: Box<Expr>,
        span: Span,
    },
    ExternVar {
        name: String,
        ty: Type,
        span: Span,
    },
    Call {
        callee: Box<Expr>,
        type_args: Vec<Type>,
        args: Vec<Expr>,
        span: Span,
    },
    Return {
        value: Box<Expr>,
        span: Span,
    },
    If {
        cond: Box<Expr>,
        then_branch: Box<Expr>,
        else_branch: Option<Box<Expr>>,
        span: Span,
    },
    While {
        cond: Box<Expr>,
        body: Box<Expr>,
        span: Span,
    },
    Break {
        value: Option<Box<Expr>>,
        span: Span,
    },
    Continue(Span),
    Block {
        stmts: Vec<Expr>,
        span: Span,
    },
    Index {
        array: Box<Expr>,
        index: Box<Expr>,
        span: Span,
    },
    IndexAssign {
        target: Box<Expr>,
        value: Box<Expr>,
        span: Span,
    },
    ArrayLiteral {
        elements: Vec<Expr>,
        span: Span,
    },
    ArrayFill {
        elem_type: Type,
        len: Box<Expr>,
        span: Span,
    },
    Range {
        start: Box<Expr>,
        end: Box<Expr>,
        inclusive: bool,
        span: Span,
    },
    For {
        var: String,
        iterable: Box<Expr>,
        body: Box<Expr>,
        span: Span,
    },
    TypeDef(Span),
    Match {
        target: Box<Expr>,
        branches: Vec<(Expr, Option<Box<Expr>>, Expr)>,
        default: Option<Box<Expr>>,
        span: Span,
    },
    Struct {
        name: String,
        type_params: Vec<String>,
        fields: Vec<(String, Type)>,
        span: Span,
    },
    StructLiteral {
        name: String,
        type_args: Vec<Type>,
        fields: Vec<(String, Expr)>,
        span: Span,
    },
    Union {
        name: String,
        type_params: Vec<String>,
        fields: Vec<(String, Type)>,
        span: Span,
    },
    UnionLiteral {
        name: String,
        type_args: Vec<Type>,
        fields: Vec<(String, Expr)>,
        span: Span,
    },
    Enum {
        name: String,
        members: Vec<(String, isize)>,
        span: Span,
    },
    MemberAccess {
        obj: Box<Expr>,
        field: String,
        span: Span,
    },
    MemberAssign {
        obj: Box<Expr>,
        field: String,
        value: Box<Expr>,
        span: Span,
    },
    Lambda {
        params: Vec<(String, Type)>,
        body: Box<Expr>,
        return_type: Type,
        span: Span,
    },
    AddressOf {
        expr: Box<Expr>,
        span: Span,
    },
    Deref {
        expr: Box<Expr>,
        span: Span,
    },
    DerefAssign {
        ptr: Box<Expr>,
        value: Box<Expr>,
        span: Span,
    },
    Cast {
        expr: Box<Expr>,
        ty: Type,
        span: Span,
    },
    FString {
        segs: Vec<Expr>,
        span: Span,
    },
}
