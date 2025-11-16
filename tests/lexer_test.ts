import { assertEquals } from "@std/assert";
import { Lexer } from "../src/lexer.ts";

Deno.test("Lexer", () => {
  const code = "1 + 1";
  const lexer = new Lexer(code);
  lexer.nextToken();
  assertEquals(lexer.currentToken().value, 1);
});
