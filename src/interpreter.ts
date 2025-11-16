import type {
  BinOp,
  Expression,
  NativeFunc,
  Program,
  Value,
  Var,
} from "./ast.ts";
import type { Context, GosFunc } from "./context.ts";
import { type Literal, TokenType } from "./lexer.ts";
import { err } from "./utils.ts";

/**Interpreter */
export class Interpreter {
  private context: Context;
  private pc: number = 0; // program counter

  constructor(ctx: Context) {
    this.context = ctx;
  }

  public execute = async (program: Program): Promise<Literal> => {
    let result: Literal = undefined;
    if (!program || program.body.length === 0) return;

    for (let i = 0; i < program.body.length; i++) {
      const expr = program.body[i];
      if (expr.type === "Label") {
        this.context.setLabel(expr.name, i);
      }
      if (expr.type === "FuncDecl") {
        this.context.setFunc(
          expr.name,
          { params: expr.params, body: expr.body, type: "GosFunc" },
        );
        return;
      }
    }

    const executable = program.body.filter((a) =>
      a.type !== "Label" && a.type !== "FuncDecl"
    );
    while (
      this.pc < executable.length
    ) {
      const expr = executable[this.pc];
      result = await this.eval(expr);
      this.pc++;
    }
    return result;
  };

  private eval = async (expr: Expression): Promise<Literal> => {
    switch (expr.type) {
      case "Value": {
        return (expr as Value).value;
      }
      case "BinOp": {
        const binNode = expr as BinOp;
        const left = await this.eval(binNode.left);
        const right = await this.eval(binNode.right);

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
            return err("Interpreter", `Unknown operator: ${binNode.op}`);
        }
      }
      case "UnaryOp": {
        const val = await this.eval(expr.argument);
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
        this.context.setVar(expr.name, await this.eval(expr.value));
        return;
      }
      case "VarMod": {
        this.context.modifyVar(expr.name, await this.eval(expr.value));
        return;
      }
      case "Var": {
        return this.context.getVar(expr.name) as Literal;
      }
      case "Out": {
        const value = await this.eval(expr.value);
        if (this.context.getVar("N")) {
          await Deno.stdout.write(new TextEncoder().encode(`${value}\n`));
        } else await Deno.stdout.write(new TextEncoder().encode(`${value}`));
        return;
      }
      case "In": {
        const value = prompt("") as string;
        this.context.setVar(expr.name, value);
        break;
      }
      case "If": {
        if (await this.eval(expr.cond)) return await this.eval(expr.body);
        else if (expr.else) return await this.eval(expr.else);
        return;
      }
      case "While": {
        while (await this.eval(expr.cond)) await this.eval(expr.body);
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
        for (const e of expr.body) result = await this.eval(e);
        this.context.exitScope();
        return result;
      }
      case "Exit": {
        return Deno.exit(await this.eval(expr.status) as number);
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
              "Interpreter",
              `Function '${expr.name}' expects ${
                (fn as GosFunc).params.length
              } args`,
            );
          }
          this.context.enterScope();
          for (let i = 0; i < (fn as GosFunc).params.length; i++) {
            const val = await this.eval(expr.args[i]);
            this.context.setVar((fn as GosFunc).params[i], val);
          }
          const result = await this.eval((fn as GosFunc).body);
          this.context.exitScope();
          return result;
        } else {
          const args = await Promise.all(
            expr.args.map(async (arg) => await this.eval(arg)),
          );
          const result = (fn as NativeFunc).fn(...args);
          return await Promise.resolve(result);
        }
      }
      case "Return": {
        return await this.eval(expr.value);
      }
      default:
        return err(
          "Interpreter",
          `Unknown node type: ${(expr as Expression).type}`,
        );
    }
  };
}
