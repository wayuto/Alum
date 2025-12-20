# _The Gos Programming Language_

### _A simple interpreter/compiler implemented in `Rust`_

## _**Getting Started**_

- ### _**In CLI**_

```bash
➜ git clone --depth 1 https://github.com/wayuto/Gos ~/Gos
➜ cd ~/Gos
➜ sh ./install.sh
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
  -n                               Do not link the Gos Standard Library
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

- ### _**Usage**_

- #### _**Dependencies**_
- NASM
- ld
- cargo
- x64 Linux kernel

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
let mut ld_args = vec!["-o", &bin_file, "/usr/local/lib/libgos.a", &obj_file];
let ld_status = std::process::Command::new("ld")
.args(&ld_args)
.status()
.unwrap()
```

## _My First Gos/Native Program:_

```gos
$import "gosio"

pub fun main(): num {
    println("Hello world!")
    return 0
}
```

- ### _**Running**_

```bash
➜ gos -c foo.gos
➜ ./foo   
Hello world!
```

## _Supported Features:_

- ### _**Gos Standard Library**_
- stdlib (_start, syscall, exit, ...)
- gosio (write, read, print, println, input)
- string (strlen)
- convert (itoa, atoi)
- math (abs, sqrt, max, min, pow, fact)
- array (range, find)

- ### _**Basic Types**_
- num
- str
- bool
- arr<N>
- void

- ### _**Variables**_

```gos
let x: num = 1
let a: arr<5> = [1 2 3 4 5]
let b: arr<_> = [1 2 3]
let c: arr<5> = [0] # [0 0 0 0 0]
```

- ### _**Flow Control**_

```gos
let x: num = 1
let y: num = 2
if x < y print(itoa(y)) else print(itoa(x))

while true {
    print("I love you!")
}
```

- ### _**Functions**_

```gos
fun sum(x: num y: num): num { return x + y }
sum(1 2)
```

- ### _**Goto**_

```gos
label:
goto label
```

- ### _**Block Scope**_

```gos
let x: num = {
    let a: num = 1
    let b: num = 2
    a + b
}

let y: num = if true 1 else 0
```

- ### _**Importing module**_

```gos
$import "fibonacci.gos"

let n: num = 10
let result: str = itoa(f(n))
print(result)

# Only for Gos/Native
extern print
print("Hello world!")
```

- ### _**Interaction with C**_

```gos
pub fun add(x: num y:num): num {
	return x + y
}
```

```c
extern add(int x, int y);

int main(void) {
	int sum = add(1, 2);
  	return 0;
}
```

or

```c
int add(int x, int y) {
	return x + y
}
```

```gos
extern add

pub fun main(): num {
	let sum: num = add(1 2)
	return 0
}
```
