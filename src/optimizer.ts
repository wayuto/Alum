// deno-lint-ignore-file
import type { Expression, Program, Val } from "./ast.ts";
import { TokenType } from "./token.ts";

export class Optimizer {
  public optimize = (program: Program): Program => {
    const optimized: Program = {
      type: "Program",
      body: program.body.filter((expr) => expr !== null)
        .map((expr) => this.fold(expr)!)
        .filter((expr) => expr !== undefined),
    };
    return optimized;
  };

  private fold = (expr: Expression): Expression | void => {
    switch (expr.type) {
      case "BinOp": {
        const left = this.fold(expr.left)!;
        const right = this.fold(expr.right)!;

        if (
          left.type === "Val" && right.type === "Val"
        ) {
          switch (expr.op) {
            case TokenType.OP_ADD:
              return {
                type: "Val",
                value: (left.value as any) + (right.value as any),
              };
            case TokenType.OP_SUB:
              return {
                type: "Val",
                value: (left.value as any) - (right.value as any),
              };
            case TokenType.OP_MUL:
              return {
                type: "Val",
                value: (left.value as any) * (right.value as any),
              };
            case TokenType.OP_DIV:
              return {
                type: "Val",
                value: (left.value as any) / (right.value as any),
              };
            case TokenType.COMP_EQ: {
              return {
                type: "Val",
                value: (left.value as any) === (right.value as any),
              };
            }
            case TokenType.COMP_NE: {
              return {
                type: "Val",
                value: (left.value as any) !== (right.value as any),
              };
            }
            case TokenType.COMP_GT: {
              return {
                type: "Val",
                value: (left.value as any) > (right.value as any),
              };
            }
            case TokenType.COMP_GE: {
              return {
                type: "Val",
                value: (left.value as any) >= (right.value as any),
              };
            }
            case TokenType.COMP_LT: {
              return {
                type: "Val",
                value: (left.value as any) < (right.value as any),
              };
            }
            case TokenType.COMP_LE: {
              return {
                type: "Val",
                value: (left.value as any) <= (right.value as any),
              };
            }
            case TokenType.COMP_AND: {
              return {
                type: "Val",
                value: (left.value as any) && (right.value as any),
              };
            }
            case TokenType.COMP_OR: {
              return {
                type: "Val",
                value: (left.value as any) && (right.value as any),
              };
            }
            case TokenType.LOG_AND: {
              return {
                type: "Val",
                value: (left.value as any) & (right.value as any),
              };
            }
            case TokenType.LOG_OR: {
              return {
                type: "Val",
                value: (left.value as any) | (right.value as any),
              };
            }
            case TokenType.LOG_XOR: {
              return {
                type: "Val",
                value: (left.value as any) ^ (right.value as any),
              };
            }
            default:
              return { type: "BinOp", op: expr.op, left, right };
          }
        }
        return { type: "BinOp", op: expr.op, left, right };
      }
      case "UnaryOp": {
        const val = this.fold(expr.argument)!;
        if (val.type === "Val") {
          switch (expr.op) {
            case TokenType.OP_NEG:
              return {
                type: "Val",
                value: -(val.value as any),
              };
            case TokenType.LOG_NOT:
              return {
                type: "Val",
                value: !(val.value as any),
              };
            default:
              return { type: "UnaryOp", op: expr.op, argument: val };
          }
        }
        return { type: "UnaryOp", op: expr.op, argument: val };
      }
      case "VarDecl": {
        return {
          type: "VarDecl",
          name: expr.name,
          value: this.fold(expr.value)!,
        };
      }
      case "If": {
        if (
          this.fold(expr.cond)!.type === "Val"
        ) {
          if ((this.fold(expr.cond) as Val).value) {
            return this.fold(expr.body);
          } else if (expr.else) {
            return this.fold(expr.else);
          } else {
            return;
          }
        }
        return {
          type: "If",
          cond: expr.cond,
          body: this.fold(expr.body)!,
          else: expr.else ? this.fold(expr.else)! : undefined,
        };
      }
      case "While": {
        if (
          this.fold(expr.cond)!.type === "Val" &&
          !(this.fold(expr.cond) as Val).value
        ) {
          return;
        }
        return {
          type: "While",
          cond: this.fold(expr.cond)!,
          body: this.fold(expr.body)!,
        };
      }
      case "Stmt": {
        if (expr.body.length === 0) return;
        return {
          type: "Stmt",
          body: expr.body.map((e) => this.fold(e)!).filter((e) =>
            e !== undefined
          ),
        };
      }
      default:
        return expr;
    }
  };
}
