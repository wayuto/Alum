import { BinOp, Expression, NativeFunc, Program, Value, Var } from "./ast.ts";
import { Context, GosFunc } from "./context.ts";
import { Literal, TokenType } from "./lexer.ts";
import { err } from "./utils.ts";

export class Interpreter {
  private context: Context;
  private pc: number = 0;

  constructor(ctx: Context) {
    this.context = ctx;
  }

  public execute = (program: Program): Literal => {
    let result: Literal = undefined;
    if (!program) return;
    while (
      this.pc < program.body.length
    ) {
      const expr = program.body[this.pc];
      result = this.eval(expr);
      this.pc++;
    }
    return result;
  };

  private eval = (expr: Expression): Literal => {
    switch (expr.type) {
      case "Value": {
        return (expr as Value).value as number;
      }
      case "BinOp": {
        const binNode = expr as BinOp;
        const left = this.eval(binNode.left);
        const right = this.eval(binNode.right);

        switch (binNode.op) {
          case TokenType.OP_ADD:
            return (left as number) + (right as number);
          case TokenType.OP_SUB:
            return (left as number) - (right as number);
          case TokenType.OP_MUL:
            return (left as number) * (right as number);
          case TokenType.OP_DIV:
            return (left as number) / (right as number);
          case TokenType.COMP_EQ:
            return left === right;
          case TokenType.COMP_NE:
            return left !== right;
          case TokenType.COMP_GT:
            return left > right;
          case TokenType.COMP_GE:
            return left >= right;
          case TokenType.COMP_LT:
            return left < right;
          case TokenType.COMP_LE:
            return left <= right;
          case TokenType.COMP_AND:
            return left && right;
          case TokenType.COMP_OR:
            return left || right;
          case TokenType.LOG_AND:
            return (left as number) & (right as number);
          case TokenType.LOG_OR:
            return (left as number) | (right as number);
          case TokenType.LOG_XOR:
            return (left as number) ^ (right as number);
          default:
            return err("Evaluate", `Unknown operator: ${binNode.op}`);
        }
      }
      case "UnaryOp": {
        const val = this.eval(expr.argument);
        switch (expr.op) {
          case TokenType.LOG_NOT: {
            return !val;
          }

          case TokenType.OP_INC: {
            this.context.modifyVar(
              (expr.argument as Var).name,
              val as number + 1,
            );
            return val;
          }
          case TokenType.OP_DEC: {
            this.context.modifyVar(
              (expr.argument as Var).name,
              val as number - 1,
            );
            return val;
          }
          default: {
            return expr.op === TokenType.OP_NEG ? -val : val;
          }
        }
      }
      case "VarDecl": {
        this.context.setVar(expr.name, this.eval(expr.value));
        return;
      }
      case "VarMod": {
        this.context.modifyVar(expr.name, this.eval(expr.value));
        return;
      }
      case "Var": {
        return this.context.getVar(expr.name) as Literal;
      }
      case "Out": {
        const value = this.eval(expr.value);
        if (this.context.getVar("NN")) {
          Deno.stdout.writeSync(new TextEncoder().encode(`${value}\n`));
        } else Deno.stdout.writeSync(new TextEncoder().encode(`${value}`));
        return;
      }
      case "In": {
        const value = prompt("") as string;
        this.context.setVar(expr.name, value);
        break;
      }
      case "If": {
        if (this.eval(expr.cond)) return this.eval(expr.body);
        else if (expr.else) return this.eval(expr.else);
        return;
      }
      case "While": {
        while (this.eval(expr.cond)) this.eval(expr.body);
        return;
      }
      case "Goto": {
        const targetPc = this.context.getLabel(expr.name);
        this.pc = targetPc;
        return;
      }
      case "Label": {
        this.context.setLabel(expr.name, this.pc);
        return;
      }
      case "Del": {
        this.context.delVar(expr.name);
        return;
      }
      case "Stmt": {
        let result: Literal = undefined;
        this.context.enterScope();
        for (const e of expr.body) result = this.eval(e);
        this.context.exitScope();
        return result;
      }
      case "Exit": {
        return Deno.exit(this.eval(expr.status) as number);
      }
      case "FuncDecl": {
        this.context.setFunc(
          expr.name,
          { params: expr.params, body: expr.body, type: "GosFunc" },
        );
        return;
      }
      case "FuncCall": {
        const fn = this.context.getFunc(expr.name);
        if (fn.type !== "NativeFunc") {
          if ((fn as GosFunc).params.length !== expr.args.length) {
            return err(
              "Evaluate",
              `Function '${expr.name}' expects ${
                (fn as GosFunc).params.length
              } args`,
            );
          }
          this.context.enterScope();
          for (let i = 0; i < (fn as GosFunc).params.length; i++) {
            const val = this.eval(expr.args[i]);
            this.context.setVar((fn as GosFunc).params[i], val);
          }
          const result = this.eval((fn as GosFunc).body);
          this.context.exitScope();
          return result;
        } else {
          const args = expr.args.map((arg) => this.eval(arg));
          return (fn as NativeFunc).fn(...args);
        }
      }
      case "Return": {
        return this.eval(expr.value);
      }
      default:
        return err(
          "Evaluate",
          `Unknown node type: ${(expr as Expression).type}`,
        );
    }
  };
}
