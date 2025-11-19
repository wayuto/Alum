import { Compiler, GVM, Lexer, Parser } from "@wayuto/gos";
import { dis } from "../src/bytecode/bytecode.ts";

const code = `
let n = 10
while (n > 0) {
  out n
  n = n - 1
}
`;

const lexer = new Lexer(code);
const parser = new Parser(lexer);
const ast = parser.parse();
const compiler = new Compiler();
const { chunk, maxSlot } = compiler.compile(ast);
dis(chunk);
const gvm = new GVM(chunk, maxSlot);
gvm.run();
