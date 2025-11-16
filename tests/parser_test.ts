import { assertEquals } from "@std/assert";
import { Lexer } from "../src/lexer.ts";
import { Parser } from "../src/parser.ts";

Deno.test("Parser", () => {
  const code = "let x = 1";
  const lexer = new Lexer(code);
  const parser = new Parser(lexer);
  const ast = parser.parse();
  assertEquals(ast, {
    type: "Program",
    body: [
      { type: "VarDecl", name: "x", value: { type: "Value", value: 1 } },
    ],
  });
});
