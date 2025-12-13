use crate::ast::Expr;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum TokenType {
    ADD,
    SUB,
    MUL,
    DIV,
    NEG,
    INC,
    DEC,
    EQ,
    COMPEQ,
    COMPNE,
    COMPGT,
    COMPGE,
    COMPLT,
    COMPLE,
    COMPAND,
    COMPOR,
    LOGNOT,
    LOGAND,
    LOGOR,
    LOGXOR,
    LITERAL(VarType),
    LPAREN,
    RPAREN,
    LBRACE,
    RBRACE,
    LBRACKET,
    RBRACKET,
    COLON,
    VARDECL,
    VAR,
    IN,
    OUT,
    IF,
    ELSE,
    WHILE,
    LABEL,
    GOTO,
    FUNCDECL,
    CALL,
    RETURN,
    IDENT,
    EXTERN,
    PUB,
    Type(VarType),
    SIZEOF,
    EOF,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Literal {
    Number(i64),
    Bool(bool),
    Str(String),
    Array(usize, Vec<Expr>),
    Void,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum VarType {
    Number,
    Bool,
    Str,
    Array(usize),
    Void,
}

#[derive(Debug, Clone)]
pub struct Token {
    pub token: TokenType,
    pub value: Option<Literal>,
    pub row: usize,
    pub col: usize,
}
