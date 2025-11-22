import type { Chunk } from "./compiler.ts";

export const enum Op {
  LOAD_CONST,
  LOAD_VAR,
  STORE_VAR,
  ADD,
  SUB,
  MUL,
  DIV,
  NEG,
  POS,
  INC,
  DEC,
  LOG_NOT,
  LOG_AND,
  LOG_OR,
  LOG_XOR,
  EQ,
  NE,
  GT,
  GE,
  LT,
  LE,
  OUT,
  POP,
  JUMP,
  JUMP_IF_FALSE,
  CALL,
  RET,
  EXIT,
  HALT,
}

export const dis = (chunk: Chunk): void => {
  const opNames = [
    "LOAD_CONST",
    "LOAD_VAR",
    "STORE_VAR",
    "ADD",
    "SUB",
    "MUL",
    "DIV",
    "NEG",
    "POS",
    "INC",
    "DEC",
    "LOG_NOT",
    "LOG_AND",
    "LOG_OR",
    "LOG_XOR",
    "EQ",
    "NE",
    "GT",
    "GE",
    "LT",
    "LE",
    "OUT",
    "POP",
    "JUMP",
    "JUMP_IF_FALSE",
    "CALL",
    "RET",
    "EXIT",
    "HALT",
  ];

  console.log("=== Bytecode ===");
  console.log("Constants:", chunk.constants);
  console.log("Code length:", chunk.code.length);
  console.log("");

  let ip = 0;
  const code = chunk.code;

  while (ip < code.length) {
    const address = ip;
    const op = code[ip++];
    const opName = opNames[op] || `UNKNOWN_${op}`;

    const encoder = new TextEncoder();
    const line = `${address.toString().padStart(4, "0")}: ${opName.padEnd(15)}`;
    Deno.stdout.writeSync(encoder.encode(line));

    switch (op) {
      case Op.LOAD_CONST: {
        const constIndex = code[ip++];
        const constant = chunk.constants[constIndex];
        console.log(` ${constIndex} ; ${constant}`);
        break;
      }

      case Op.LOAD_VAR:
      case Op.STORE_VAR: {
        const slot = code[ip++];
        console.log(` ${slot}`);
        break;
      }

      case Op.JUMP:
      case Op.JUMP_IF_FALSE: {
        const high = code[ip++];
        const low = code[ip++];
        const target = (high << 8) | low;
        console.log(
          ` ${high.toString(16).padStart(2, "0")} ${
            low.toString(16).padStart(2, "0")
          } ; -> ${target.toString().padStart(4, "0")}`,
        );
        break;
      }

      case Op.CALL: {
        const high = code[ip++];
        const low = code[ip++];
        const argCount = code[ip++];
        const target = (high << 8) | low;
        console.log(
          ` ${high.toString(16).padStart(2, "0")} ${
            low.toString(16).padStart(2, "0")
          } ${argCount} ; -> ${
            target.toString().padStart(4, "0")
          } (${argCount} args)`,
        );
        break;
      }
      case Op.RET: {
        console.log();
        break;
      }

      default:
        console.log();
        break;
    }
  }

  console.log("============================");
};
