# Alum Programming Language

Alum is a modern, systems programming language designed for simplicity and performance. It features a clean syntax, strong static typing, and compiles to native machine code via an optimizing IR pipeline. Currently supports the x86_64 Linux platform.

## 📖 Tutorial

The official tutorial series (中文教程) walks through the language from scratch — installation, syntax, types, and advanced features like FFI, generics, and compile-time evaluation.

**Read the tutorial: [Alum 系列教程](https://cr0.dpdns.org)**

| | |
|---|---|
| [01-介绍](https://cr0.dpdns.org/2026/02/26/01-Introduction/) | [10-数组](https://cr0.dpdns.org/2026/03/01/10-Array/) |
| [02-环境搭建](https://cr0.dpdns.org/2026/02/27/02-Installation/) | [11-指针](https://cr0.dpdns.org/2026/03/01/11-Pointer/) |
| [03-数据类型](https://cr0.dpdns.org/2026/02/27/03-Types/) | [12-结构体](https://cr0.dpdns.org/2026/03/02/12-Struct/) |
| [04-变量](https://cr0.dpdns.org/2026/02/27/04-Variable/) | [13-泛型](https://cr0.dpdns.org/2026/03/02/13-Generic/) |
| [05-代码块](https://cr0.dpdns.org/2026/02/28/05-Block/) | [14-共用体](https://cr0.dpdns.org/2026/08/02/14-Union/) |
| [06-流程控制](https://cr0.dpdns.org/2026/02/28/06-ProcessControl/) | [15-枚举](https://cr0.dpdns.org/2026/08/02/15-Enum/) |
| [07-函数](https://cr0.dpdns.org/2026/02/28/07-Function/) | [16-Result与Maybe类型](https://cr0.dpdns.org/2026/08/02/16-Result-Maybe/) |
| [08-函数式编程](https://cr0.dpdns.org/2026/02/28/08-FP/) | [17-函数注解](https://cr0.dpdns.org/2026/08/06/17-Function-Annotations/) |
| [09-外部函数接口](https://cr0.dpdns.org/2026/02/28/09-FFI/) | [18-编译时求值](https://cr0.dpdns.org/2026/08/08/18-CTE/) |

## Features

- **Native AOT Compilation** — compiles directly to native x86_64 machine code through an optimizing IR pipeline, with a built-in register allocator, assembler, and ELF encoder; links with `rust-lld`
- **Strong Static Typing** — explicit type annotations on `var`/`cst`, full type inference, and compile-time type checking before code generation
- **Rich Type System** — `int`, `float`, `bool`, `string`, arrays `T[]`, pointers `*T`, function pointers, structs, unions, C-style enums (bare references when unambiguous), and generics with monomorphic instantiation
- **Tagged `Result`/`Maybe`** — `Result<T, E>` and `Maybe<T>` built on `struct` + `union` + `enum`, enabling error handling and null-safety
- **Functional Programming** — lambdas, higher-order functions, first-class function pointers, and block expressions that produce values
- **Expression-Oriented Control Flow** — `if-else` and `match` are expressions; `for` iterates arrays and ranges (`n..m`); `while` loops; implicit return of the last expression
- **Function Annotations** — `fun(extern)` for FFI, `fun(pub)` to export symbols, `fun(pure)` to mark side-effect-free functions
- **Compile-Time Evaluation (CTE)** — `pure` functions are evaluated at compile time by the built-in GosVM bytecode interpreter (e.g. `fib(40)` compiles to a single constant in ~10ms)
- **F-String Interpolation** — `println(f"value: {x}")` with embedded expressions
- **C Interop (FFI)** — call C functions directly via `fun(extern)`; memory layout is C-compatible with no conversion overhead
- **Preprocessor** — `$import`, `$define` macros, and conditional compilation via `$ifdef`/`$ifndef`/`$else`/`$endif`
- **Build Toolkit** — integrated `almk` build system for project scaffolding, building, running, dependency management, and mixed C/Alum projects
- **Standard Library** — rich `alum-std` with I/O, string/convert, `Vec`, `Result`/`Maybe`, math, and memory management
- **VS Code Support** — official syntax highlighting extension in `alum-vscode`

## Installation

### Prerequisites

- Rust toolchain (2024 edition)

### Build from Source

```bash
# Clone the repository
git clone https://github.com/wayuto/Alum.git
cd Alum

# Install the compiler
./install.sh
```

This will:
1. Build and install the `alc` compiler
2. Build the standard library
3. Install `libalum_std.a` to `/usr/local/lib/`
4. Install standard library headers to `/usr/local/include/alum/`
5. Install the build tool `almk`

## Quick Start

### Hello World

Create a file `hello.al`:

```al
$import "io.ah"

fun main(): int {
    println("Hello, World!")
    return 0
}
```

Compile and run:

```bash
alc hello.al
./hello
```

Or use the run command:

```bash
alc -r hello.al
```

### A Program Using All Features

```al
$import "io.ah"
$import "string.ah"
$import "convert.ah"
$import "vec.ah"
$import "result.ah"

// enum (C-style)
enum Color {
    RED,          // 0 (auto)
    GREEN = 5,    // 5 (explicit)
    BLUE          // 6
}

// union (all members share memory)
union Value<T> {
    i: T,
    f: float
}

// struct + generic struct
struct Point {
    x: int,
    y: int
}

struct Line {
    a: Point,
    b: Point
}

// pure function (no side effects, can be optimized/constant-folded)
fun(pure) add(a: int, b: int): int {
    return a + b
}

// generic function
fun identity<T>(x: T): T {
    return x
}

// function taking a lambda
fun apply(f: int(int), v: int): int {
    return f(v)
}

fun main(): int {
    // type inference on `var`
    var name = "Alum"
    var n = 42
    println(name)
    println(itoa(n))

    // nested member access
    var line = Line {
        a: Point { x: 1, y: 2 },
        b: Point { x: 3, y: 4 }
    }
    println(itoa(line.b.x))  // 3

    // union member access
    var u = Value<int> { i: 7 }
    println(itoa(u.i))       // 7

    // enum member access (qualified + bare)
    println(itoa(Color.GREEN))  // 5
    println(itoa(BLUE))         // 6

    // arrays + for loop over array
    var arr = [10, 20, 30]
    for x in arr {
        println(itoa(x))
    }

    // while loop + reassignment
    var i = 0
    while i < 3 {
        i = i + 1
    }
    println(itoa(i))  // 3

    // pointers
    var p: *int = &n
    println(itoa(*p))  // 42

    // generic function
    println(itoa(identity(99)))  // 99

    // lambda
    var square: int(int) = \(x: int): int {
        return x * x
    }
    println(itoa(apply(square, 4)))  // 16

    // match with default
    var c: Color = Color.RED
    match c {
        Color.GREEN: {
            println("green")
        }
        _: {
            println("not green")
        }
    }

    // Vec container (nth returns Maybe<T>, check the tag)
    var vec: Vec<int> = vec_new()
    vec.push(&vec, 1)
    vec.push(&vec, 2)
    var m: Maybe<int> = vec.nth(&vec, 1)
    if m.tag == Just {
        println(itoa(m.value))  // 2
    }

    // Result (enum tag + union payload)
    var ok = Result<int, string> {
        result: ResultStatus.Ok,
        value: ResultValue<int, string> {
            ok: 114514
        }
    }
    if ok.result == ResultStatus.Ok {
        println(itoa(ok.value.ok))  // 114514
    }

    return 0
}
```

## CLI Usage

```
alc [OPTIONS] <INPUT>

Arguments:
  <INPUT>...    Input files (.al source files or .o/.obj object files)

Options:
  -o, --output <FILE>       Output file name
  -c, --compile-only        Compile only, do not link
  -r, --run                 Compile and run immediately
  -E                        Preprocess only; do not compile, assemble or link
  --ast                     Output AST representation
  -I <DIR>                  Add include directory (can be used multiple times)
  --nostdlib                Do not link with standard library
  -v, --verbose             Verbose output
  -h, --help                Print help
  -V, --version             Print version
```

## Development

### Building the Compiler

```bash
cargo build --release
```

### Building the Standard Library

```bash
cd alum-std
cargo build --release
```

### Building the Alum Make Tool

```bash
cd alum-make
cargo build --release
```

## Documentation

- **[Standard Library](./alum-std/README.md)** - Comprehensive standard library documentation
- **[Build Tool](./alum-make/README.md)** - almk build tool documentation
- **[Tutorial Series](https://cr0.dpdns.org)** - 18-part Chinese tutorial covering the language from scratch

## License

See LICENSE file for details.
