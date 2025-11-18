import { Compiler, GVM, Lexer, Parser } from "@wayuto/gos";

const code = `
let i = 10
while (i != 0) {
out i
}
`;

const lexer = new Lexer(code);
const parser = new Parser(lexer);
const ast = parser.parse();
console.log(ast);
const compiler = new Compiler();
const { chunk, maxSlot } = compiler.compile(ast);
const gvm = new GVM(chunk, maxSlot);
gvm.run();
