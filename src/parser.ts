import type { Expression, Program } from "./ast.ts";
import type { Lexer } from "@wayuto/gos";
import { err } from "./utils.ts";
import { type Literal, TokenType } from "./token.ts";

/**Parser */
export class Parser {
  private lexer: Lexer;
  constructor(lexer: Lexer) {
    this.lexer = lexer;
  }

  public parse = (): Program => {
    this.lexer.nextToken();
    const expressions: Expression[] = [];
    while (this.lexer.currentToken().type != TokenType.EOF) {
      expressions.push(this.ctrl());
    }
    return { type: "Program", body: expressions };
  };

  private ctrl = (): Expression => {
    if (this.lexer.currentToken().type === TokenType.IF) {
      this.lexer.nextToken();
      const cond = this.expr();
      const body = this.stmt();
      if (this.lexer.currentToken().type === TokenType.ELSE) {
        this.lexer.nextToken();
        const elseBody = this.stmt();
        return { type: "If", cond, body, else: elseBody };
      }
      return { type: "If", cond, body };
    }
    if (this.lexer.currentToken().type === TokenType.WHILE) {
      this.lexer.nextToken();
      const cond = this.expr();
      const body = this.stmt();
      return { type: "While", cond, body };
    }
    if (this.lexer.currentToken().type === TokenType.FUNC_DECL) {
      this.lexer.nextToken();
      const name = this.lexer.currentToken().name as string;
      const params: string[] = [];
      this.lexer.nextToken();
      if (this.lexer.currentToken().type !== TokenType.LPAREN) {
        return err("Parser", "Expected '('");
      }
      while (this.lexer.currentToken().type !== TokenType.RPAREN) {
        if (this.lexer.currentToken().type === TokenType.IDENT) {
          params.push(this.lexer.currentToken().name as string);
        }
        this.lexer.nextToken();
      }
      this.lexer.nextToken();
      const body = this.stmt();
      return {
        type: "FuncDecl",
        name,
        params,
        body,
      };
    }
    return this.stmt();
  };

  private stmt = (): Expression => {
    if (this.lexer.currentToken().type === TokenType.LBRACE) {
      const expressions: Expression[] = [];
      this.lexer.nextToken();

      while (this.lexer.currentToken().type !== TokenType.RBRACE) {
        if (this.lexer.currentToken().type === TokenType.EOF) {
          return err("Parser", "Expected '}'");
        }
        expressions.push(this.ctrl());
      }

      this.lexer.nextToken();
      return { type: "Stmt", body: expressions };
    }
    if (
      this.lexer.currentToken().type === TokenType.IF ||
      this.lexer.currentToken().type === TokenType.WHILE ||
      this.lexer.currentToken().type === TokenType.FUNC_DECL
    ) {
      return this.ctrl();
    }
    return this.expr();
  };

  private expr = (): Expression => {
    switch (this.lexer.currentToken().type) {
      case TokenType.EXIT: {
        this.lexer.nextToken();
        const status = this.expr();
        this.lexer.nextToken();
        return { type: "Exit", status };
      }
      case TokenType.GOTO: {
        this.lexer.nextToken();
        const name = this.lexer.currentToken().name as string;
        this.lexer.nextToken();
        this.lexer.nextToken();
        return { type: "Goto", name };
      }
      case TokenType.VAR_DEL: {
        this.lexer.nextToken();
        const name = this.lexer.currentToken().name as string;
        this.lexer.nextToken();
        return { type: "Del", name };
      }
      case TokenType.VAR_DECL: {
        this.lexer.nextToken();
        const name = this.lexer.currentToken().name as string;
        this.lexer.nextToken();
        if (this.lexer.currentToken().type !== TokenType.OP_EQ) {
          return err("Parser", "Expected '='");
        }
        this.lexer.nextToken();
        const value = this.expr();
        return {
          type: "VarDecl",
          name,
          value,
        };
      }
      case TokenType.OUT: {
        this.lexer.nextToken();
        const value = this.expr();
        return {
          type: "Out",
          value,
        };
      }
      case TokenType.IN: {
        this.lexer.nextToken();
        const name = this.lexer.currentToken().name as string;
        return {
          type: "In",
          name,
        };
      }
      case TokenType.RETURN: {
        this.lexer.nextToken();
        const value = this.expr();
        return {
          type: "Return",
          value,
        };
      }
      case TokenType.IF:
      case TokenType.WHILE:
      case TokenType.LBRACE: {
        return this.ctrl();
      } // to make statement be a expression
      default: {
        return this.logical();
      }
    }
  };

  private logical = (): Expression => {
    let left = this.comparison();
    while (
      this.lexer.currentToken().type === TokenType.LOG_AND ||
      this.lexer.currentToken().type === TokenType.LOG_OR ||
      this.lexer.currentToken().type === TokenType.LOG_NOT ||
      this.lexer.currentToken().type === TokenType.LOG_XOR
    ) {
      const op = this.lexer.currentToken().type;
      this.lexer.nextToken();
      const right = this.comparison();
      left = {
        type: "BinOp",
        op: op,
        left,
        right,
      };
    }
    return left;
  };

  private comparison = (): Expression => {
    let left = this.additive();
    while (
      this.lexer.currentToken().type === TokenType.COMP_EQ ||
      this.lexer.currentToken().type === TokenType.COMP_NE ||
      this.lexer.currentToken().type === TokenType.COMP_LT ||
      this.lexer.currentToken().type === TokenType.COMP_LE ||
      this.lexer.currentToken().type === TokenType.COMP_GT ||
      this.lexer.currentToken().type === TokenType.COMP_GE ||
      this.lexer.currentToken().type === TokenType.COMP_AND ||
      this.lexer.currentToken().type === TokenType.COMP_OR
    ) {
      const op = this.lexer.currentToken().type;
      this.lexer.nextToken();
      const right = this.additive();
      left = {
        type: "BinOp",
        op: op,
        left,
        right,
      };
    }
    return left;
  };

  private additive = (): Expression => {
    let left = this.term();
    while (
      this.lexer.currentToken().type === TokenType.OP_ADD ||
      this.lexer.currentToken().type === TokenType.OP_SUB
    ) {
      const op = this.lexer.currentToken().type;
      this.lexer.nextToken();
      const right = this.term();
      left = {
        type: "BinOp",
        op: op,
        left,
        right,
      };
    }
    return left;
  };

  private term = (): Expression => {
    let left = this.factor();
    while (
      this.lexer.currentToken().type === TokenType.OP_MUL ||
      this.lexer.currentToken().type === TokenType.OP_DIV
    ) {
      const op = this.lexer.currentToken().type;
      this.lexer.nextToken();
      const right = this.factor();
      left = {
        type: "BinOp",
        op: op,
        left: left,
        right: right,
      };
    }
    return left;
  };

  private factor = (): Expression => {
    const typ = this.lexer.currentToken().type;

    if (typ === TokenType.LITERAL) {
      const value = this.lexer
        .currentToken().value as Literal;
      this.lexer.nextToken();
      return { type: "Val", value };
    }

    if (typ === TokenType.IDENT) {
      const name = this.lexer.currentToken().name as string;
      this.lexer.nextToken();
      switch (this.lexer.currentToken().type) {
        case TokenType.COLON: {
          this.lexer.nextToken();
          return { type: "Label", name };
        }
        case TokenType.OP_INC: {
          this.lexer.nextToken();
          return {
            type: "UnaryOp",
            op: TokenType.OP_INC,
            argument: { type: "Var", name },
          };
        }
        case TokenType.OP_DEC: {
          this.lexer.nextToken();
          return {
            type: "UnaryOp",
            op: TokenType.OP_DEC,
            argument: { type: "Var", name },
          };
        }
        case TokenType.LPAREN: {
          const args: Expression[] = [];
          this.lexer.nextToken();
          while (this.lexer.currentToken().type !== TokenType.RPAREN) {
            args.push(this.expr());
          }
          this.lexer.nextToken();
          return { type: "FuncCall", name, args };
        }
        case TokenType.OP_EQ: {
          this.lexer.nextToken();
          const value = this.expr();
          return { type: "VarMod", name, value };
        }
        default: {
          return { type: "Var", name };
        }
      }
    }

    if (
      typ === TokenType.OP_POS || typ === TokenType.OP_NEG ||
      typ === TokenType.LOG_NOT
    ) {
      this.lexer.nextToken();
      const value = this.factor();
      return { type: "UnaryOp", op: typ, argument: value };
    }

    if (typ === TokenType.LPAREN) {
      this.lexer.nextToken();
      const expr = this.expr();
      if (this.lexer.currentToken().type !== TokenType.RPAREN) {
        return err("Parser", "Expected: ')'");
      }
      this.lexer.nextToken();
      return expr;
    }

    return err("Parser", `Unexpected token: '${typ}'`);
  };
}
