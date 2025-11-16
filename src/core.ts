import { NativeFunc } from "./ast.ts";

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
    fn: (a) => Deno.readTextFileSync(a as string),
  },
  writeFile: {
    type: "NativeFunc",
    fn: (a, b) => Deno.writeTextFileSync(a as string, b as string),
  },
};
