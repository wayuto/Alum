# _The Gos Programming Language_

_A `Tree-walking` interpreter implementing in `TypeScript`_

## _**Getting Started**_

- ### _**In CLI**_

```bash
➜  deno install -A -n --global gos jsr:@wayuto/gos/gos
➜  gos -h    

  Usage:   gos 
  Version: [VERSION]

  License: MIT

  Description:

    Gos Interpreter

  Options:

    -h, --help     - Show this help.                            
    -V, --version  - Show the version number for this program.  

  Commands:

    run   <file>  - Run a Gos source file            
    repl          - Gos REPL                         
    ast   <file>  - Show the AST of a Gos source file
```

- ### _**In TypeScript**_
- _**Import necessary modules**_

```typescript
import { Context, Interpreter, Lexer, Parser } from "jsr:@wayuto/gos";
```

- _**Initialization**_

```typescript
const code = "out 'Hello world!'";

const lexer = new Lexer(code);
const parser = new Parser(lexer);
const ast = parser.parse();
const context = new Context();
const interpreter = new Interpreter(context);
```

- _**Execution**_

```typescript
interpreter.execute(ast);
```

## _Supported Features:_

- ### _**Basic Calculate**_

```typescript
(1 + 2) * 3 # Output: 9
```

- ### _**Variables**_

```typescript
let x = 1
let y = 2
x + y # Output: 3
```

- ### _**Basic I/O**_

```typescript
# Console I/O
out 1 # Output: 1
in n # n = something you entered
```

```typescript
# File I/O
writeFile('testFile', '1')
let content = readFile('testFile')
out content # Output: 1
```

- ### _**Flow Control**_

```typescript
let x = 1
let y = 2
if (x < y) out y else out x # Output: 2

while (true) {
    out 1
} # Output: 1 1 1 1 ...
```

- ### _**Functions**_

```typescript
fun sum(x y) { return x + y }
sum(1 2) # Output: 3
```

- ### _**Goto**_

```typescript
let n = 10

N = false # no newline when print values
goto label
del n # this expression will be skipped

label:
out n
out ' '
n--
if n != 0 goto label # Output: 10 9 8 7 6 5 4 3 2 1
```

- ### _**Block Scope**_

```typescript
fun f() out "outer"
{
  fun f() out "inner"
  f()
}
f()
# Output:
# inner
# outer

# Block scope as a expression
let x = {
    let a = 1
    let b = 2
    a + b
} # x = 3

let y = if (true) 1 else 0 # y = 1
```
