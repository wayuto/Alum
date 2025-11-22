import { Compiler, Lexer, Parser } from "@wayuto/gos";
import { assertEquals } from "@std/assert/equals";

Deno.test("Compiler", () => {
  const code = `out 1`;

  const lexer = new Lexer(code);
  const parser = new Parser(lexer);
  const ast = parser.parse();
  const compiler = new Compiler();
  const { chunk, maxSlot } = compiler.compile(ast);
  assertEquals(chunk.code, new Uint8Array([0, 0, 18, 24]));
  assertEquals(maxSlot, 0);
});
