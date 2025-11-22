import type { Chunk } from "./compiler.ts";
import { Op } from "./bytecode.ts";
import type { Literal } from "../token.ts";

/**GosVM */
export class GVM {
  private ip = 0;
  private stack: Literal[] = [];
  private slots: Literal[] = [];
  private callStack: { returnIp: number; baseSlot: number }[] = [];
  private currBaseSlot = 0;

  constructor(private chunk: Chunk, private maxSlot: number) {
    this.slots = new Array<Literal>(this.maxSlot);
  }

  public run = (): void => {
    const jmpTable: (() => void)[] = [];
    jmpTable[Op.LOAD_CONST] = () => {
      const idx = this.chunk.code[this.ip++];
      this.stack.push(this.chunk.constants[idx]);
    };
    jmpTable[Op.LOAD_VAR] = () => {
      const slot = this.chunk.code[this.ip++];
      this.stack.push(this.slots[slot]);
    };
    jmpTable[Op.STORE_VAR] = () => {
      const slot = this.chunk.code[this.ip++];
      this.slots[slot] = this.stack.pop();
    };
    jmpTable[Op.ADD] = () => {
      const right = this.stack.pop();
      const left = this.stack.pop();
      const val = (left as number) + (right as number);
      this.stack.push(val);
    };
    jmpTable[Op.SUB] = () => {
      const right = this.stack.pop();
      const left = this.stack.pop();
      const val = (left as number) - (right as number);
      this.stack.push(val);
    };
    jmpTable[Op.MUL] = () => {
      const right = this.stack.pop();
      const left = this.stack.pop();
      const val = (left as number) * (right as number);
      this.stack.push(val);
    };
    jmpTable[Op.DIV] = () => {
      const right = this.stack.pop();
      const left = this.stack.pop();
      const val = (left as number) / (right as number);
      this.stack.push(val);
    };
    jmpTable[Op.EQ] = () => {
      const right = this.stack.pop();
      const left = this.stack.pop();
      const val = left === right;
      this.stack.push(val);
    };
    jmpTable[Op.NE] = () => {
      const right = this.stack.pop();
      const left = this.stack.pop();
      const val = left !== right;
      this.stack.push(val);
    };
    jmpTable[Op.GT] = () => {
      const right = this.stack.pop();
      const left = this.stack.pop();
      const val = left! > right!;
      this.stack.push(val);
    };
    jmpTable[Op.GE] = () => {
      const right = this.stack.pop();
      const left = this.stack.pop();
      const val = left! >= right!;
      this.stack.push(val);
    };
    jmpTable[Op.LT] = () => {
      const right = this.stack.pop();
      const left = this.stack.pop();
      const val = left! < right!;
      this.stack.push(val);
    };
    jmpTable[Op.LE] = () => {
      const right = this.stack.pop();
      const left = this.stack.pop();
      const val = left! <= right!;
      this.stack.push(val);
    };
    jmpTable[Op.OUT] = () => {
      const value = this.stack.pop();
      console.log(value);
    };
    jmpTable[Op.POP] = () => {
      this.stack.pop();
    };
    jmpTable[Op.NEG] = () => {
      const val = this.stack.pop();
      this.stack.push(-(val as number));
    };
    jmpTable[Op.INC] = () => {
      const val = this.stack.pop();
      this.stack.push((val as number) + 1);
    };
    jmpTable[Op.DEC] = () => {
      const val = this.stack.pop();
      this.stack.push((val as number) - 1);
    };
    jmpTable[Op.LOG_NOT] = () => {
      const val = this.stack.pop();
      this.stack.push(!val);
    };
    jmpTable[Op.LOG_AND] = () => {
      const right = this.stack.pop();
      const left = this.stack.pop();
      const val = (left as number) & (right as number);
      this.stack.push(val);
    };
    jmpTable[Op.LOG_OR] = () => {
      const right = this.stack.pop();
      const left = this.stack.pop();
      const val = (left as number) | (right as number);
      this.stack.push(val);
    };
    jmpTable[Op.LOG_XOR] = () => {
      const right = this.stack.pop();
      const left = this.stack.pop();
      const val = (left as number) ^ (right as number);
      this.stack.push(val);
    };
    jmpTable[Op.JUMP] = () => {
      const high = this.chunk.code[this.ip++];
      const low = this.chunk.code[this.ip++];
      const target = (high << 8) | low;
      this.ip = target;
    };
    jmpTable[Op.JUMP_IF_FALSE] = () => {
      const high = this.chunk.code[this.ip++];
      const low = this.chunk.code[this.ip++];
      const target = (high << 8) | low;

      const cond = this.stack.pop();
      if (!cond) this.ip = target;
    };
    jmpTable[Op.CALL] = () => {
      const high = this.chunk.code[this.ip++];
      const low = this.chunk.code[this.ip++];
      const argsCount = this.chunk.code[this.ip++];
      const target = (high << 8) | low;

      this.callStack.push({
        returnIp: this.ip,
        baseSlot: this.currBaseSlot | 0,
      });

      const newBaseSlot = this.slots.length;

      const args = [];
      for (let i = 0; i < argsCount; i++) {
        args.push(this.stack.pop());
      }
      args.reverse();
      this.slots.push(...args);

      this.currBaseSlot = newBaseSlot;
      this.ip = target;
    };
    jmpTable[Op.RET] = () => {
      const value = this.stack.pop();
      if (this.callStack.length === 0) return;
      const frame = this.callStack.pop();

      const currFrameSize = this.slots.length - this.currBaseSlot;
      this.slots.splice(this.currBaseSlot, currFrameSize);

      this.ip = frame!.returnIp;
      this.currBaseSlot = frame!.baseSlot;

      if (value !== undefined) this.stack.push(value);
    };
    jmpTable[Op.EXIT] = () => {
      const status = this.stack.pop() as number;
      return Deno.exit(status);
    };
    while (true) {
      const op = this.chunk.code[this.ip++];
      if (op === Op.HALT) return;
      jmpTable[op]();
    }
  };
}
