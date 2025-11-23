import { assertEquals } from "@std/assert";
import { Lexer, Optimizer, Parser } from "@wayuto/gos";

Deno.test("Optimizer", () => {
  const code = "let x = (1 + 2) * 3";
  const lexer = new Lexer(code);
  const parser = new Parser(lexer);
  const optimizer = new Optimizer();
  const ast = optimizer.optimize(parser.parse());
  assertEquals(ast, {
    type: "Program",
    body: [
      { type: "VarDecl", name: "x", value: { type: "Val", value: 9 } },
    ],
  });
});
