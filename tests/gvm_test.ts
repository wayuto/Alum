import { Compiler, GVM, Lexer, Parser } from "@wayuto/gos";
import { dis } from "../src/bytecode/bytecode.ts";

const code = `
let n = 10
out {
    fun f(x) {
        if (x <= 1) return x
        else {
            let a = 0
            let b = 1
            while (x > 1) {
                let tmp = a + b
                a = b
                b = tmp
                x--
            }
            return b
        }
    }
    f(10)
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
