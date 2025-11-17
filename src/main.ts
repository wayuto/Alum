import { Context } from "./context.ts";
import { Interpreter } from "./interpreter.ts";
import { Lexer } from "./lexer.ts";
import { Parser } from "./parser.ts";
import { Command } from "@cliffy/command";

const run = async (file: string): Promise<void> => {
  const src = await Deno.readTextFile(file);
  const lexer = new Lexer(src);
  const parser = new Parser(lexer);
  const ast = parser.parse();
  const context = new Context();
  const interpreter = new Interpreter(context);
  interpreter.execute(ast);
};

const printAST = async (file: string): Promise<void> => {
  const src = await Deno.readTextFile(file);
  const lexer = new Lexer(src);
  const parser = new Parser(lexer);
  const ast = parser.parse();
  console.log(ast);
};

const repl = async (): Promise<void> => {
  console.log("Gos REPL");

  const context = new Context();

  while (true) {
    try {
      const line = prompt("> ");
      const lexer = new Lexer(line as string);
      const parser = new Parser(lexer);
      const ast = parser.parse();
      const interpreter = new Interpreter(context);
      const result = await interpreter.execute(ast);
      if (result !== undefined) console.log(result);
    } catch (e) {
      console.log(e);
    }
  }
};

const main = async (): Promise<void> => {
  if (Deno.args.length === 0) await repl();
  else {
    await new Command()
      .name("gos")
      .version("v0.1.9")
      .description("Gos Interpreter")
      .meta("License", "MIT")
      .command("run <file:string>", "Run a Gos source file")
      .action(async (_, file: string) => {
        await run(file);
      })
      .command("repl", "Gos REPL")
      .action(async () => await repl())
      .command("ast <file:string>", "Show the AST of a Gos source file")
      .action(async (_, file: string) => {
        await printAST(file);
      })
      .parse(Deno.args);
  }
};

if (import.meta.main) await main();
