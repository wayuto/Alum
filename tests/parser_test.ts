import { assertEquals } from "@std/assert";
import { Lexer, Parser } from "@wayuto/gos";

Deno.test("Parser", () => {
  const code = "let x = 1";
  const lexer = new Lexer(code);
  const parser = new Parser(lexer);
  const ast = parser.parse();
  assertEquals(ast, {
    type: "Program",
    body: [
      { type: "VarDecl", name: "x", value: { type: "Val", value: 1 } },
    ],
  });
});
