import { err, isalpha, isdigit } from "./utils.ts";

export const enum TokenType {
  OP_ADD = "OP_ADD",
  OP_SUB = "OP_SUB",
  OP_MUL = "OP_MUL",
  OP_DIV = "OP_DIV",
  OP_NEG = "OP_NEG",
  OP_POS = "OP_POS",
  OP_INC = "OP_INC",
  OP_DEC = "OP_DEC",
  OP_EQ = "OP_EQ",
  COMP_EQ = "COMP_EQ",
  COMP_NE = "COMP_NE",
  COMP_GT = "COMP_GT",
  COMP_GE = "COMP_GE",
  COMP_LT = "COMP_LT",
  COMP_LE = "COMP_LE",
  COMP_AND = "COMP_AND",
  COMP_OR = "COMP_OR",
  LOG_NOT = "LOG_NOT",
  LOG_AND = "LOG_AND",
  LOG_OR = "LOG_OR",
  LOG_XOR = "LOG_XOR",
  LITERAL = "LITERAL",
  LPAREN = "LPAREN",
  RPAREN = "RPAREN",
  LBRACE = "LBRACE",
  RBRACE = "RBRACE",
  COLON = "COLON",
  VAR_DECL = "VAR_DECL",
  VAR_DEL = "DEL",
  VAR = "VAR",
  IN = "IN",
  OUT = "OUT",
  IF = "IF",
  ELSE = "ELSE",
  WHILE = "WHILE",
  LABEL = "LABEL",
  GOTO = "GOTO",
  EXIT = "EXIT",
  FUNC_DECL = "FUNC_DECL",
  CALL = "CALL",
  RETURN = "RETURN",
  IDENT = "IDENT",
  EOF = "EOF",
}

export type Literal = string | number | boolean | void;

type Token = {
  type: TokenType;
  name?: string;
  value?: Literal;
};

/**Lexer */
export class Lexer {
  private src: string;
  private pos: number;
  private tok: Token;

  constructor(code: string) {
    this.src = code;
    this.pos = 0;
    this.tok = { type: TokenType.EOF };
  }

  private current = (): string =>
    this.pos < this.src.length ? this.src[this.pos] : "\0";

  private bump = (): void => {
    this.pos++;
  };

  private skipSpaces = (): void => {
    while (
      this.current() == " " || this.current() == "\t" || this.current() == "\n"
    ) this.bump();
  };

  private parseNumber = (): number => {
    let intPart = 0;
    let fracPart = 0;
    let fracDiv = 1;

    while (isdigit(this.current())) {
      intPart = intPart * 10 + parseInt(this.current());
      this.bump();
    }

    if (this.current() === ".") {
      this.bump();
      if (!isdigit(this.current())) {
        err("Lexer", "Invalid number: expected digit after '.'");
      }
      while (isdigit(this.current())) {
        fracDiv *= 10;
        fracPart = fracPart * 10 + parseInt(this.current());
        this.bump();
      }
    }

    const value = intPart + fracPart / fracDiv;
    return value;
  };

  private parseAlpha = (): string => {
    let alpha: string = "";
    while (isalpha(this.current())) {
      alpha += this.current();
      this.bump();
    }
    return alpha;
  };

  private isPrefix = (): boolean =>
    this.tok.type === TokenType.EOF ||
    this.tok.type === TokenType.LPAREN;

  public nextToken = (): void => {
    this.skipSpaces();
    if (this.current() === "\0") {
      this.tok = { type: TokenType.EOF };
      return;
    } else if (isdigit(this.current())) {
      const value = this.parseNumber();
      this.tok = { type: TokenType.LITERAL, value };
      return;
    } else if (isalpha(this.current())) {
      const ident = this.parseAlpha();
      switch (ident) {
        case "let": {
          this.tok = { type: TokenType.VAR_DECL };
          break;
        }
        case "out": {
          this.skipSpaces();
          this.tok = { type: TokenType.OUT };
          break;
        }
        case "in": {
          this.skipSpaces();
          this.tok = { type: TokenType.IN };
          break;
        }
        case "true": {
          this.tok = { type: TokenType.LITERAL, value: true };
          break;
        }
        case "false": {
          this.tok = { type: TokenType.LITERAL, value: false };
          break;
        }
        case "null": {
          this.tok = { type: TokenType.LITERAL, value: undefined };
          break;
        }
        case "if": {
          this.tok = { type: TokenType.IF };
          break;
        }
        case "else": {
          this.tok = { type: TokenType.ELSE };
          break;
        }
        case "while": {
          this.tok = { type: TokenType.WHILE };
          break;
        }
        case "goto": {
          this.tok = { type: TokenType.GOTO };
          break;
        }
        case "del": {
          this.tok = { type: TokenType.VAR_DEL };
          break;
        }
        case "exit": {
          this.tok = { type: TokenType.EXIT };
          break;
        }
        case "fun": {
          this.tok = { type: TokenType.FUNC_DECL };
          break;
        }
        case "return": {
          this.tok = { type: TokenType.RETURN };
          break;
        }
        default: {
          this.tok = { type: TokenType.IDENT, name: ident };
          return;
        }
      }
    } else if (this.current() === '"') {
      this.bump();
      let s: string = "";
      while (this.current() !== '"') {
        if (this.current() === "\0") {
          return err("Lexer", "Expected: '\"'");
        }
        s += this.current();
        this.bump();
      }
      this.bump();
      this.tok = { type: TokenType.LITERAL, value: s };
    } else if (this.current() === "'") {
      this.bump();
      let s: string = "";
      while (this.current() !== "'") {
        if (this.current() === "\0") {
          return err("Lexer", 'Expected: "\'"');
        }
        s += this.current();
        this.bump();
      }
      this.bump();
      this.tok = { type: TokenType.LITERAL, value: s };
    } else if (this.current() === "+") {
      if (this.isPrefix()) {
        this.tok = { type: TokenType.OP_POS };
        this.bump();
        return;
      }
      this.bump();
      if (this.current() === "+") {
        this.tok = { type: TokenType.OP_INC };
        this.bump();
        return;
      } else {
        this.tok = { type: TokenType.OP_ADD };
        return;
      }
    } else if (this.current() === "-") {
      if (this.isPrefix()) {
        this.tok = { type: TokenType.OP_NEG };
        return;
      }
      this.bump();
      if (this.current() === "-") {
        this.tok = { type: TokenType.OP_DEC };
        this.bump();
        return;
      } else {
        this.tok = { type: TokenType.OP_SUB };
        this.bump();
        return;
      }
    } else if (this.current() === "*") {
      this.tok = { type: TokenType.OP_MUL };
      this.bump();
      return;
    } else if (this.current() === "/") {
      this.tok = { type: TokenType.OP_DIV };
      this.bump();
      return;
    } else if (this.current() === "(") {
      this.tok = { type: TokenType.LPAREN };
      this.bump();
      return;
    } else if (this.current() === ")") {
      this.tok = { type: TokenType.RPAREN };
      this.bump();
      return;
    } else if (this.current() === "{") {
      this.tok = { type: TokenType.LBRACE };
      this.bump();
      return;
    } else if (this.current() === "}") {
      this.tok = { type: TokenType.RBRACE };
      this.bump();
      return;
    } else if (this.current() === "=") {
      this.bump();
      if (this.current() === "=") {
        this.tok = { type: TokenType.COMP_EQ };
        this.bump();
      } else {
        this.tok = { type: TokenType.OP_EQ };
      }
      return;
    } else if (this.current() === "!") {
      this.bump();
      if (this.current() === "=") {
        this.tok = { type: TokenType.COMP_NE };
        this.bump();
      } else {
        this.tok = { type: TokenType.LOG_NOT };
      }
      return;
    } else if (this.current() === ">") {
      this.bump();
      if (this.current() === "=") {
        this.tok = { type: TokenType.COMP_GE };
        this.bump();
      } else {
        this.tok = { type: TokenType.COMP_GT };
      }
      return;
    } else if (this.current() === "<") {
      this.bump();
      if (this.current() === "=") {
        this.tok = { type: TokenType.COMP_LE };
        this.bump();
      } else {
        this.tok = { type: TokenType.COMP_LT };
      }
      return;
    } else if (this.current() === "&") {
      this.bump();
      if (this.current() === "&") {
        this.tok = { type: TokenType.COMP_AND };
        this.bump();
      } else {
        this.tok = { type: TokenType.LOG_AND };
      }
      return;
    } else if (this.current() === "|") {
      this.bump();
      if (this.current() === "|") {
        this.tok = { type: TokenType.COMP_OR };
        this.bump();
      } else {
        this.tok = { type: TokenType.LOG_OR };
      }
      return;
    } else if (this.current() === "^") {
      this.tok = { type: TokenType.LOG_XOR };
      this.bump();
      return;
    } else if (this.current() === ":") {
      this.tok = { type: TokenType.COLON };
      this.bump();
      return;
    } else if (this.current() === "#") {
      while (this.current() !== "\n" && this.current() !== "\0") this.bump();
      this.nextToken();
      return;
    } else {
      err("Lexer", `Error: Unknown token: '${this.current()}`);
    }
  };

  public currentToken = (): Token => {
    return this.tok;
  };
}
