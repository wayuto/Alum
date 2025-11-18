import type { Expression, Program } from "../ast.ts";
import { type Literal, TokenType } from "../token.ts";
import { err } from "../utils.ts";
import { Op } from "./bytecode.ts";

export interface Chunk {
  code: Uint8Array;
  constants: Literal[];
}

/**Compiler */
export class Compiler {
  private constants: Literal[] = [];
  private codes: number[] = [];
  private locals = new Map<string, number>();
  private nextSlot = 0;

  private emit = (op: Op, ...arg: number[]): void => {
    this.codes.push(op);
    if (arg !== undefined) this.codes.push(...arg);
  };

  public compile = (program: Program): { chunk: Chunk; maxSlot: number } => {
    for (const expr of program.body) {
      this.compileExpr(expr);
    }

    this.emit(Op.HALT);

    return {
      chunk: { code: new Uint8Array(this.codes), constants: this.constants },
      maxSlot: this.nextSlot,
    };
  };

  private compileExpr = (
    expr: Expression,
  ): void => {
    switch (expr.type) {
      case "Val": {
        const val = expr.value;
        this.constants.push(val);
        this.emit(Op.LOAD_CONST, this.constants.length - 1);
        break;
      }
      case "Var": {
        const slot = this.locals.get(expr.name);
        if (slot === undefined) {
          return err(
            "Compiler",
            `Variable '${expr.name}' has not been defined`,
          );
        }
        this.emit(Op.LOAD_VAR, slot);
        break;
      }
      case "VarDecl": {
        this.compileExpr(expr.value);
        const slot = this.nextSlot++;
        this.locals.set(expr.name, slot);
        this.emit(Op.STORE_VAR, slot);
        break;
      }
      case "BinOp": {
        this.compileExpr(expr.left);
        this.compileExpr(expr.right);
        switch (expr.op) {
          case TokenType.OP_ADD: {
            this.emit(Op.ADD);
            break;
          }
          case TokenType.OP_SUB: {
            this.emit(Op.SUB);
            break;
          }
          case TokenType.OP_MUL:
            this.emit(Op.MUL);
            break;
          case TokenType.OP_DIV: {
            this.emit(Op.DIV);
            break;
          }
          case TokenType.COMP_EQ: {
            this.emit(Op.EQ);
            break;
          }
          case TokenType.COMP_NE: {
            this.emit(Op.NE);
            break;
          }
          case TokenType.COMP_GT: {
            this.emit(Op.GT);
            break;
          }
          case TokenType.COMP_GE: {
            this.emit(Op.GE);
            break;
          }
          case TokenType.COMP_LT: {
            this.emit(Op.LT);
            break;
          }
          case TokenType.COMP_LE: {
            this.emit(Op.LE);
            break;
          }
        }
        break;
      }
      case "UnaryOp": {
        this.compileExpr(expr.argument);
        if (expr.op === TokenType.OP_INC || expr.op === TokenType.OP_DEC) {
          if (expr.argument.type === "Var") {
            const varName = expr.argument.name;
            const slot = this.locals.get(varName);
            if (slot === undefined) {
              return err(
                "Compiler",
                `Variable '${varName}' has not been defined`,
              );
            }

            this.emit(Op.LOAD_VAR, slot);
            if (expr.op === TokenType.OP_INC) {
              this.emit(Op.INC);
            } else {
              this.emit(Op.DEC);
            }
            this.emit(Op.STORE_VAR, slot);
            break;
          }
        }
        switch (expr.op) {
          case TokenType.LOG_NOT: {
            this.emit(Op.LOG_NOT);
            break;
          }
          default: {
            expr.op === TokenType.OP_NEG
              ? this.emit(Op.NEG)
              : this.emit(Op.POS);
          }
        }
        break;
      }
      case "Out": {
        this.compileExpr(expr.value);
        this.emit(Op.OUT);
        break;
      }
      case "Stmt": {
        for (const e of expr.body) {
          this.compileExpr(e);
        }
        break;
      }
      case "If": {
        this.compileExpr(expr.cond);

        const thenPos = this.codes.length;
        this.emit(Op.JUMP_IF_FALSE, 0, 0);
        this.compileExpr(expr.body);

        let elsePos = -1;
        if (expr.else) {
          elsePos = this.codes.length;
          this.emit(Op.JUMP, 0, 0);
        }

        const thenEndPos = this.codes.length;
        this.patchJumpAddr(thenPos + 1, thenEndPos);

        if (expr.else) {
          this.compileExpr(expr.else);
          const elseEndPos = this.codes.length;
          this.patchJumpAddr(elsePos + 1, elseEndPos);
        }
        break;
      }

      case "While": {
        const loopPos = this.codes.length;
        this.compileExpr(expr.cond);

        const jumpIfFalse = this.codes.length;
        this.emit(Op.JUMP_IF_FALSE, 0, 0);

        this.compileExpr(expr.body);
        this.emit(Op.JUMP, (loopPos >> 8) & 0xff, loopPos & 0xff);

        const breakPos = this.codes.length;
        this.patchJumpAddr(jumpIfFalse + 1, breakPos);
        break;
      }
      case "Label":
      case "Goto": {
        console.warn("'goto' isn't available in bytecode mode");
        break;
      }
      default: {
        return err("Compiler", `Unknown node type: ${expr.type}`);
      }
    }
  };

  private patchJumpAddr = (pos: number, addr: number): void => {
    this.codes[pos] = (addr >> 8) & 0xff;
    this.codes[pos + 1] = addr & 0xff;
  };
}
