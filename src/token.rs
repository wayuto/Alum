#[derive(PartialEq, PartialOrd, Debug, Clone)]
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
    LITERAL,
    LPAREN,
    RPAREN,
    LBRACE,
    RBRACE,
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
    EXIT,
    FUNCDECL,
    CALL,
    RETURN,
    IDENT,
    EVAL,
    EOF,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    Number(f64),
    Bool(bool),
    Str(String),
    Void,
}

#[derive(Debug, Clone)]
pub struct Token {
    pub token: TokenType,
    pub value: Option<Literal>,
}
