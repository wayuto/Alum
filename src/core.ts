import type { NativeFunc } from "./ast.ts";
import type { Literal } from "./lexer.ts";

export const nativeFuncs: Record<string, NativeFunc> = {
  abs: {
    type: "NativeFunc",
    fn: (x) => Math.abs(x as number),
  },
  max: {
    type: "NativeFunc",
    fn: (a, b) => Math.max(a as number, b as number),
  },
  min: {
    type: "NativeFunc",
    fn: (a, b) => Math.min(a as number, b as number),
  },
  sqrt: {
    type: "NativeFunc",
    fn: (a) => Math.sqrt(a as number),
  },
  now: {
    type: "NativeFunc",
    fn: () => Date.now(),
  },
  toNum: {
    type: "NativeFunc",
    fn: (a) => Number(a),
  },
  toStr: {
    type: "NativeFunc",
    fn: (a) => String(a),
  },
  readFile: {
    type: "NativeFunc",
    fn: async (a) => await Deno.readTextFile(a as string),
  },
  writeFile: {
    type: "NativeFunc",
    fn: async (a, b) => await Deno.writeTextFile(a as string, b as string),
  },
  shell: {
    type: "NativeFunc",
    fn: async (a, ...b) => await shell(a, ...b),
  },
};

const shell = async (a: Literal, ...b: Literal[]) => {
  const { stdout } = await (new Deno.Command(a as string, {
    args: [...b as string[]],
    stdout: "piped",
    stderr: "inherit",
  })).output();
  return new TextDecoder().decode(stdout)
    .trimEnd();
};
