import { assertEquals } from "@std/assert";
import { Lexer } from "../src/lexer.ts";
import { Parser } from "../src/parser.ts";
import { Interpreter } from "../src/interpreter.ts";
import { Context } from "../src/context.ts";

Deno.test("Context", () => {
  const context = new Context();
  context.setVar("test", 1);
  const var_ = context.getVar("test");
  assertEquals(var_, 1);
});

Deno.test("Interpreter", () => {
  const code = `
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
  `;

  const lexer = new Lexer(code);
  const parser = new Parser(lexer);
  const ast = parser.parse();
  const context = new Context();
  const interpreter = new Interpreter(context);
  const result = interpreter.execute(ast);

  assertEquals(result, 55);
});
