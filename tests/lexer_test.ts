import { assertEquals } from "@std/assert";
import { Lexer } from "@wayuto/gos";

Deno.test("Lexer", () => {
  const code = "1 + 1";
  const lexer = new Lexer(code);
  lexer.nextToken();
  assertEquals(lexer.currentToken().value, 1);
});
