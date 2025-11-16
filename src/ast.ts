import type { Literal, TokenType } from "./lexer.ts";

export type NodeType =
  | "Program"
  | "Value"
  | "BinOp"
  | "UnaryOp"
  | "VarDecl"
  | "VarMod"
  | "Var"
  | "Out"
  | "In"
  | "Label"
  | "Goto"
  | "If"
  | "While"
  | "Del"
  | "Stmt"
  | "ExprStmt"
  | "FuncDecl"
  | "FuncCall"
  | "NativeFunc"
  | "Return"
  | "Exit";

export interface Node {
  type: NodeType;
}
export interface Program extends Node {
  type: "Program";
  body: Expression[];
}

export type Expression =
  | Value
  | BinOp
  | UnaryOp
  | VarDecl
  | VarMod
  | Var
  | Out
  | In
  | If
  | While
  | Label
  | Goto
  | Del
  | Stmt
  | FuncDecl
  | FuncCall
  | Return
  | Exit;

export interface Value extends Node {
  type: "Value";
  value: Literal;
}

export interface BinOp extends Node {
  type: "BinOp";
  op: TokenType;
  left: Expression;
  right: Expression;
}

export interface UnaryOp extends Node {
  type: "UnaryOp";
  op: TokenType;
  argument: Expression;
}

export interface VarDecl extends Node {
  type: "VarDecl";
  name: string;
  value: Expression;
}

export interface VarMod extends Node {
  type: "VarMod";
  name: string;
  value: Expression;
}

export interface Var extends Node {
  type: "Var";
  name: string;
}

export interface Out extends Node {
  type: "Out";
  value: Expression;
}

export interface In extends Node {
  type: "In";
  name: string;
}

export interface If extends Node {
  type: "If";
  cond: Expression;
  body: Expression;
  else?: Expression;
}

export interface While extends Node {
  type: "While";
  cond: Expression;
  body: Expression;
}

export interface Label extends Node {
  type: "Label";
  name: string;
}

export interface Goto extends Node {
  type: "Goto";
  name: string;
}

export interface Del extends Node {
  type: "Del";
  name: string;
}

export interface Exit extends Node {
  type: "Exit";
  status: Expression;
}

export interface Stmt extends Node {
  type: "Stmt";
  body: Expression[];
}

export interface FuncDecl extends Node {
  type: "FuncDecl";
  name: string;
  params: string[];
  body: Expression;
}

export interface FuncCall extends Node {
  type: "FuncCall";
  name: string;
  args: Expression[];
}

export interface NativeFunc extends Node {
  type: "NativeFunc";
  fn: (...args: Literal[]) => Literal | Promise<Literal>;
} // typescript functions

export interface Return extends Node {
  type: "Return";
  value: Expression;
}
