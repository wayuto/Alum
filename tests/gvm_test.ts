import { Compiler, GVM, Lexer, Parser, Preprocessor } from "@wayuto/gos";
import { dis } from "../src/bytecode/bytecode.ts";

const src = `
$import "examples/fibonacci.gos"
out f(10)
`;

const preprocessor = new Preprocessor(src);
const code = await preprocessor.preprocess();
const lexer = new Lexer(code);
const parser = new Parser(lexer);
const ast = parser.parse();
const compiler = new Compiler();
const { chunk, maxSlot } = compiler.compile(ast);
dis(chunk);
const gvm = new GVM(chunk, maxSlot);
gvm.run();
