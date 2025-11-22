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
  private scopes: { vars: Map<string, number>; slotCount: number }[] = [{
    vars: new Map(),
    slotCount: 0,
  }];
  private nextSlot = 0;
  private funcs: Map<string, { addr: number; paramCount: number }>[] = [
    new Map(),
  ];

  emit = (op: Op, ...arg: number[]): void => {
    this.codes.push(op);
    if (arg !== undefined) this.codes.push(...arg);
  };

  private enterScope = () => {
    this.scopes.push({ vars: new Map(), slotCount: 0 });
    this.funcs.push(new Map());
  };

  private exitScope = () => {
    const scope = this.scopes.pop();
    if (scope) {
      this.nextSlot -= scope.slotCount;
    }
    this.funcs.pop();
  };

  private loadVar = (name: string): number | null => {
    for (let i = this.scopes.length - 1; i >= 0; i--) {
      const slot = this.scopes[i].vars.get(name);
      if (slot !== undefined) return slot;
    }
    return null;
  };

  private loadFunc = (
    name: string,
  ): { addr: number; paramCount: number } | null => {
    for (let i = this.funcs.length - 1; i >= 0; i--) {
      const func = this.funcs[i].get(name);
      if (func !== undefined) return func;
    }
    return null;
  };

  private declVar = (name: string): number => {
    const currentScope = this.scopes[this.scopes.length - 1];
    const slot = this.nextSlot++;
    currentScope.vars.set(name, slot);
    currentScope.slotCount++;
    return slot;
  };

  private modVar = (name: string): number => {
    const slot = this.loadVar(name);
    if (slot === null) {
      return err(
        "Compiler",
        `Variable '${name}' has not been defined`,
      );
    }
    return slot;
  };

  public compile = (program: Program): { chunk: Chunk; maxSlot: number } => {
    for (const expr of program.body) {
      this.compileExpr(expr);
    }

    this.emit(Op.HALT);

    return {
      chunk: {
        code: new Uint8Array(this.codes),
        constants: this.constants,
      },
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
        const slot = this.loadVar(expr.name);
        if (slot === null) {
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
        const slot = this.declVar(expr.name);
        this.emit(Op.STORE_VAR, slot);
        this.emit(Op.POP);
        break;
      }
      case "VarMod": {
        this.compileExpr(expr.value);
        const slot = this.modVar(expr.name);
        this.emit(Op.STORE_VAR, slot);
        this.emit(Op.POP);
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
            const name = expr.argument.name;
            const slot = this.loadVar(name)!;
            if (slot === undefined) {
              return err(
                "Compiler",
                `Variable '${name}' has not been defined`,
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
        this.enterScope();
        const body = expr.body;

        for (let i = 0; i < body.length - 1; i++) {
          this.compileExpr(body[i]);
          this.emit(Op.POP);
        }

        if (body.length > 0) {
          this.compileExpr(body[body.length - 1]);
        } else {
          this.constants.push(undefined);
          this.emit(Op.LOAD_CONST, this.constants.length - 1);
        }

        this.exitScope();
        break;
      }
      case "If": {
        this.compileExpr(expr.cond);

        const thenPos = this.codes.length;
        this.emit(Op.JUMP_IF_FALSE, 0, 0);

        this.enterScope();
        this.compileExpr(expr.body);
        this.exitScope();

        let elsePos = -1;
        if (expr.else) {
          elsePos = this.codes.length;
          this.emit(Op.JUMP, 0, 0);
        }

        const thenEndPos = this.codes.length;
        this.patchJumpAddr(thenPos + 1, thenEndPos);

        if (expr.else) {
          this.enterScope();
          this.compileExpr(expr.else);
          this.exitScope();
          const elseEndPos = this.codes.length;
          this.patchJumpAddr(elsePos + 1, elseEndPos);
        }
        break;
      }

      case "While": {
        this.enterScope();

        const loopPos = this.codes.length;
        this.compileExpr(expr.cond);

        const jumpIfFalse = this.codes.length;
        this.emit(Op.JUMP_IF_FALSE, 0, 0);

        this.compileExpr(expr.body);
        this.emit(Op.JUMP, (loopPos >> 8) & 0xff, loopPos & 0xff);

        const breakPos = this.codes.length;
        this.patchJumpAddr(jumpIfFalse + 1, breakPos);

        this.exitScope();
        break;
      }
      case "FuncDecl": {
        const jumpAddr = this.codes.length;
        this.emit(Op.JUMP, 0, 0);
        const funcAddr = this.codes.length;
        const currFunc = this.funcs[this.funcs.length - 1];

        if (currFunc.has(expr.name)) {
          return err(
            "Compiler",
            `Function '${expr.name}' has already been declared in this scope`,
          );
        }

        currFunc.set(expr.name, {
          addr: funcAddr,
          paramCount: expr.params.length,
        });

        this.enterScope();

        for (const param of expr.params) {
          this.declVar(param);
        }

        this.compileExpr(expr.body);

        this.emit(Op.RET);

        this.exitScope();
        this.patchJumpAddr(jumpAddr + 1, this.codes.length);
        this.emit(Op.POP);
        break;
      }
      case "FuncCall": {
        for (const arg of expr.args) {
          this.compileExpr(arg);
        }

        const func = this.loadFunc(expr.name);

        if (func === null) {
          return err(
            "Compiler",
            `Function '${expr.name}' hasn't been declared in this scope`,
          );
        }
        if (func.paramCount !== expr.args.length) {
          return err(
            "Compiler",
            `Function ${expr.name} expected ${func.paramCount} arguments, got ${expr.args.length}`,
          );
        }

        const target = func.addr;

        this.emit(
          Op.CALL,
          (target >> 8) & 0xff,
          target & 0xff,
          func.paramCount,
        );

        break;
      }
      case "Exit": {
        this.compileExpr(expr.status);
        this.emit(Op.EXIT);
        break;
      }
      case "Return": {
        if (expr.value) {
          this.compileExpr(expr.value);
        } else {
          this.constants.push(undefined);
          this.emit(Op.LOAD_CONST, this.constants.length - 1);
        }
        this.emit(Op.RET);
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
