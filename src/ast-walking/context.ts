import type { Expression, NativeFunc } from "../ast.ts";
import { Literal } from "../token.ts";
import { err } from "../utils.ts";
import { nativeFuncs } from "./core.ts";

export type GosFunc = { params: string[]; body: Expression; type: string }; // functions defined in `gos` code

type FuncVal =
  | GosFunc
  | NativeFunc;

/** Runtime container for variables, functions and labels*/
export class Context {
  private vars: Map<string, Literal>[] = [new Map()];
  private funcs: Map<string, FuncVal>[] = [new Map()];
  private labels = new Map<string, number>();

  constructor() {
    this.vars[0].set("gos", Date.now()); // nothing special
    this.vars[0].set("N", true); // N = newline, if true, `out` appends '\n', else doesn't

    for (const [name, nf] of Object.entries(nativeFuncs)) {
      this.setFunc(name, nf);
    }
  }

  public enterScope = (): void => {
    this.vars.push(new Map());
    this.funcs.push(new Map());
  };

  public exitScope = (): void => {
    this.vars.pop();
    this.funcs.pop();
  };

  // for modify the value of the variable after declared
  public modifyVar = (name: string, value: Literal): void => {
    for (let i = this.vars.length - 1; i >= 0; i--) {
      if (this.vars[i].has(name)) {
        this.vars[i].set(name, value);
        return;
      }
    }
    return err("Context", `Variable '${name}' has not been defined`);
  };

  // for the first declaration with `let` keyword
  public setVar = (name: string, value: Literal): void => {
    if (this.vars[this.vars.length - 1].get(name) === undefined) {
      this.vars[this.vars.length - 1].set(name, value);
    } else {
      return err("Context", `Variable '${name}' has already been defined`);
    }
  };

  public getVar = (name: string): Literal => {
    for (let i = this.vars.length - 1; i >= 0; i--) {
      const v = this.vars[i].get(name);
      if (v !== undefined) return v;
    }
    return err(
      "Context",
      `No variable '${name}' found`,
    );
  };

  public delVar = (name: string): void => {
    const top = this.vars[this.vars.length - 1];
    if (!top.delete(name)) {
      return err("Context", `No variable '${name}' in current scope`);
    }
  };

  public setLabel = (name: string, idx: number): void => {
    this.labels.set(name, idx);
  };

  public getLabel = (name: string): number => {
    const label = this.labels.get(name);
    if (label !== undefined) return label;
    return err("Context", `No label '${name}' found`);
  };

  public setFunc = (name: string, val: FuncVal): void => {
    if (this.funcs[this.funcs.length - 1].get(name) === undefined) {
      this.funcs[this.funcs.length - 1].set(name, val);
    } else {
      return err(
        "Context",
        `Function '${name}' has already been defined in current scope`,
      );
    }
  };

  public getFunc = (name: string): FuncVal => {
    for (let i = this.funcs.length - 1; i >= 0; i--) {
      const fn = this.funcs[i].get(name);
      if (fn) return fn;
    }
    return err("Context", `No function '${name}' found`);
  };
}
