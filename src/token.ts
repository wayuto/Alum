export const enum TokenType {
  OP_ADD,
  OP_SUB,
  OP_MUL,
  OP_DIV,
  OP_NEG,
  OP_POS,
  OP_INC,
  OP_DEC,
  OP_EQ,
  COMP_EQ,
  COMP_NE,
  COMP_GT,
  COMP_GE,
  COMP_LT,
  COMP_LE,
  COMP_AND,
  COMP_OR,
  LOG_NOT,
  LOG_AND,
  LOG_OR,
  LOG_XOR,
  LITERAL,
  LPAREN,
  RPAREN,
  LBRACE,
  RBRACE,
  COLON,
  VAR_DECL,
  VAR_DEL,
  VAR,
  IN,
  OUT,
  IF,
  ELSE,
  WHILE,
  LABEL,
  GOTO,
  EXIT,
  FUNC_DECL,
  CALL,
  RETURN,
  IDENT,
  EOF,
}

export type Literal = string | number | boolean | void;

export type Token = {
  type: TokenType;
  name?: string;
  value?: Literal;
};
