# _The Gos Programming Language_

### _A simple interpreter implemented in `TypeScript`_

_Now the interpretersupports both `Tree-walking` and `GVM`(Experimental, limited
feature support)_

## _**Getting Started**_

- ### _**In CLI**_

```bash
➜  deno install -A -n --global gos jsr:@wayuto/gos/gos
➜  gos -h

Usage:   gos   
Version: v0.2.7

License: MIT

Description:

  Gos Interpreter

Options:

  -h, --help     - Show this help.                            
  -V, --version  - Show the version number for this program.  

Commands:

  repl                - Gos REPL                              
  run         <file>  - Run a Gos source file by bytecode     
  compile     <file>  - Compile a Gos source file             
  interpret   <file>  - Run a Gos source file by ast-walker   
  ast         <file>  - Show the AST of a Gos source file     
  preprocess  <file>  - Show the proprecessed Gos source file 
  dis         <file>  - Show the bytecode of a Gos source file
```

- ### _**In TypeScript**_
- _**Import necessary modules**_

```typescript
// tree-walking
import {
  Context,
  Interpreter,
  Lexer,
  Parser,
  Preprocessor,
} from "jsr:@wayuto/gos";

// bytecode
import { Compiler, GVM, Lexer, Parser, Preprocessor } from "@wayuto/gos";
```

- _**Initialization**_

```typescript
const src = "out 'Hello world!'";

const preprocessor = new Preprocessor(src);
const code = await preprocessor.preprocess();
const lexer = new Lexer(code);
const parser = new Parser(lexer);
const ast = parser.parse();

// tree-walking
const context = new Context();
const interpreter = new Interpreter(context);

// bytecode
const compiler = new Compiler();
const { chunk, maxSlot } = compiler.compile(ast);
const gvm = new GVM(chunk, maxSlot);
```

- _**Execution**_

```typescript
// tree-walking
interpreter.execute(ast);
// bytecode
gvm.run();
```

## _Supported Features:_

- ### _**Basic Calculate**_

```gos
(1 + 2) * 3
```

- ### _**Variables**_

```gos
let x = 1
let y = 2
x + y
```

- ### _**Basic I/O**_

```gos
# Console I/O
$import "examples/std.gos"

let name = input("Enter your name: ")
print("Hello " + name + "!")
```

```gos
# File I/O
$import "examples/std.gos"

writeFile('testFile', '1')
let raw = readFile('testFile')
```

- ### _**Flow Control**_

```gos
let x = 1
let y = 2
if (x < y) out y else out x

while (true) {
    out 1
}
```

- ### _**Functions**_

```gos
fun sum(x y) { return x + y }
sum(1 2)
```

- ### _**Goto**_

```gos
let n = 10

label:
out n
out ' '
n--
if n != 0 goto label
```

- ### _**Block Scope**_

```gos
fun f() out "outer"
{
  fun f() out "inner"
  f()
}
f()

let x = {
    let a = 1
    let b = 2
    a + b
}

let y = if (true) 1 else 0
```

- ### _**Importing module**_

```gos
$import "fibonacci.gos"

let n = 10
let result = f(n)
out result
```

- ### _**Running Typescript in Gos**_

```
let x = eval "1 + 1"
```
