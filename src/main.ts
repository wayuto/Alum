import { Command } from "@cliffy/command";
import {
  compile,
  Compiler,
  Context,
  dis,
  GVM,
  Interpreter,
  Lexer,
  load,
  Optimizer,
  Parser,
  Preprocessor,
} from "@wayuto/gos";

const interpret = async (file: string): Promise<void> => {
  const src = await Deno.readTextFile(file);
  const preprocessor = new Preprocessor(src);
  const code = await preprocessor.preprocess();
  const lexer = new Lexer(code);
  const parser = new Parser(lexer);
  const ast = parser.parse();
  const context = new Context();
  const interpreter = new Interpreter(context);
  interpreter.execute(ast);
};

const run = async (file: string): Promise<void> => {
  if (file.endsWith("gbc")) {
    const { chunk, maxSlot } = await load(file);

    const gvm = new GVM(chunk, maxSlot);
    gvm.run();
    return;
  }
  const src = await Deno.readTextFile(file);
  const preprocessor = new Preprocessor(src);
  const code = await preprocessor.preprocess();
  const lexer = new Lexer(code);
  const parser = new Parser(lexer);
  const optimizer = new Optimizer();
  const ast = optimizer.optimize(parser.parse());
  const compiler = new Compiler();
  const { chunk, maxSlot } = compiler.compile(ast);
  const gvm = new GVM(chunk, maxSlot);
  gvm.run();
};

const printAST = async (file: string): Promise<void> => {
  const src = await Deno.readTextFile(file);
  const preprocessor = new Preprocessor(src);
  const code = await preprocessor.preprocess();
  const lexer = new Lexer(code);
  const parser = new Parser(lexer);
  const optimizer = new Optimizer();
  const ast = optimizer.optimize(parser.parse());
  console.log(ast);
};

const printPreprocessed = async (file: string): Promise<void> => {
  const src = await Deno.readTextFile(file);
  const preprocessor = new Preprocessor(src);
  const code = await preprocessor.preprocess();
  console.log(code);
};

const printBytecode = async (file: string): Promise<void> => {
  const src = await Deno.readTextFile(file);
  const preprocessor = new Preprocessor(src);
  const code = await preprocessor.preprocess();
  const lexer = new Lexer(code);
  const parser = new Parser(lexer);
  const optimizer = new Optimizer();
  const ast = optimizer.optimize(parser.parse());
  const compiler = new Compiler();
  const { chunk } = compiler.compile(ast);
  dis(chunk);
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
      .version("v0.2.7")
      .description("Gos Interpreter")
      .meta("License", "MIT")
      .command("repl", "Gos REPL")
      .action(async () => await repl())
      .command(
        "run <file:string>",
        "Run a Gos source file by bytecode",
      )
      .action(async (_, file: string) => {
        await run(file);
      })
      .command(
        "compile <file:string>",
        "Compile a Gos source file",
      )
      .action(async (_, file: string) => {
        await compile(file);
      })
      .command(
        "interpret <file:string>",
        "Run a Gos source file by ast-walker",
      )
      .action(async (_, file: string) => {
        await interpret(file);
      })
      .command("ast <file:string>", "Show the AST of a Gos source file")
      .action(async (_, file: string) => {
        await printAST(file);
      })
      .command(
        "preprocess <file:string>",
        "Show the proprecessed Gos source file",
      ).action(async (_, file: string) => {
        await printPreprocessed(file);
      })
      .command("dis <file:string>", "Show the bytecode of a Gos source file")
      .action(async (_, file: string) => {
        await printBytecode(file);
      })
      .parse(Deno.args);
  }
};

if (import.meta.main) await main();
