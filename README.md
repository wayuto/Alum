# _The Gos Programming Language_

### _A simple interpreter/compiler implemented in `Rust`_

## _**Getting Started**_

- ### _**In CLI**_

```bash
➜ git clone --depth 1 https://github.com/wayuto/Gos ~/Gos
➜ cd ~/Gos
➜ cargo install --path .
➜ gos -h
The Gos programming language

Usage: gos [OPTIONS] [FILE]

Arguments:
  [FILE]  Run the Gos source/bytecode file

Options:
  -a, --ast <ast>                  Print AST of the Gos source file
  -b, --bytecode <bytecode>        Compile the Gos source file to bytecode
  -c, --compile <compile>          Compile the Gos source file to native
  -s                               Compile the Gos source file to assembly
  -o                               Compile the Gos source file to object
  -l                               Link the libc when compiling to native
  -p, --preprocess <preprocess>    Print the preprocessed Gos source file
  -d, --disassemble <disassemble>  Run the Gos source/bytecode file
  -h, --help                       Print help
  -V, --version                    Print version
```

- ### _**In Rust**_
- _**Import necessary modules**_

```rust
use crate::{
    bytecode::GVM,
    lexer::Lexer,
    parser::Parser,
    preprocessor::Preprocessor,
    serialize::{compile, load},
};
use std::{fs, path::Path};
```

- _**Usage**_

```rust
let file = "path/to/file.gos"

// Gos/GVM
let src = fs::read_to_string(file).unwrap();
let path = Path::new(&file.clone())
	.parent()
	.unwrap()
	.to_str()
	.unwrap()
	.to_string();
let mut preprocessor = Preprocessor::new(src.as_str(), path);
let code = preprocessor.preprocess();
let lexer = Lexer::new(code.as_str());
let mut parser = Parser::new(lexer);
let ast = parser.parse();
let mut compiler = bytecode::Compiler::new();
let bytecode = compiler.compile(ast);
let mut gvm = GVM::new(bytecode);
gvm.run();

// Gos/Native
let src = fs::read_to_string(file).unwrap();
let path = Path::new(&file)
	.parent()
	.unwrap()
	.to_str()
	.unwrap()
	.to_string();
let mut preprocessor = Preprocessor::new(&src, path);
let code = preprocessor.preprocess();
let lexer = Lexer::new(&code);
let mut parser = Parser::new(lexer);
let ast = parser.parse();
let mut compiler = native::Compiler::new();
let assembly = compiler.compile(ast);

let asm_file = format!("foo.s");
let obj_file = format!("foo.o");
let bin_file = stem.to_string();
fs::write(&asm_file, &assembly).unwrap();
let nasm_status = std::process::Command::new("nasm")
	.args(&["-f", "elf64", "-o", &obj_file, &asm_file])
	.status()
	.unwrap()
let lib_path = format!("/tmp/lib.o");
let mut ld_args = vec!["-o", &bin_file, &lib_path, &obj_file];
let ld_status = std::process::Command::new("ld")
.args(&ld_args)
.status()
.unwrap()
```

## _My First Gos/Native Program:_

```gos
extern puts

fun main() {
    puts("Hello world!")
}
```

- ### _**Running**_

```bash
➜  RsGos git:(rust) ✗ gos -c foo.gos -l
➜  RsGos git:(rust) ✗ ./foo   
Hello world!
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
