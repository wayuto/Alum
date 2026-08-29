# Alum Programming Language

Alum is a modern, systems programming language designed for simplicity and performance. It features a clean syntax, strong static typing, and compiles to native machine code via an optimizing IR pipeline. Currently supports the x86_64 Linux platform.

## Tutorial

The official tutorial series (中文教程) walks through the language from scratch — installation, syntax, types, and advanced features like FFI, generics, and compile-time evaluation.

**Read the tutorial: [Alum 系列教程](https://cr0.dpdns.org)**

|                                                                    |                                                                           |
| ------------------------------------------------------------------ | ------------------------------------------------------------------------- |
| [01-介绍](https://cr0.dpdns.org/2026/02/26/01-Introduction/)       | [10-数组](https://cr0.dpdns.org/2026/03/01/10-Array/)                     |
| [02-环境搭建](https://cr0.dpdns.org/2026/02/27/02-Installation/)   | [11-指针](https://cr0.dpdns.org/2026/03/01/11-Pointer/)                   |
| [03-数据类型](https://cr0.dpdns.org/2026/02/27/03-Types/)          | [12-结构体](https://cr0.dpdns.org/2026/03/02/12-Struct/)                  |
| [04-变量](https://cr0.dpdns.org/2026/02/27/04-Variable/)           | [13-泛型](https://cr0.dpdns.org/2026/03/02/13-Generic/)                   |
| [05-代码块](https://cr0.dpdns.org/2026/02/28/05-Block/)            | [14-共用体](https://cr0.dpdns.org/2026/08/02/14-Union/)                   |
| [06-流程控制](https://cr0.dpdns.org/2026/02/28/06-ProcessControl/) | [15-枚举](https://cr0.dpdns.org/2026/08/02/15-Enum/)                      |
| [07-函数](https://cr0.dpdns.org/2026/02/28/07-Function/)           | [16-Result与Maybe类型](https://cr0.dpdns.org/2026/08/02/16-Result-Maybe/) |
| [08-函数式编程](https://cr0.dpdns.org/2026/02/28/08-FP/)           | [17-函数注解](https://cr0.dpdns.org/2026/08/06/17-Function-Annotations/)  |
| [09-外部函数接口](https://cr0.dpdns.org/2026/02/28/09-FFI/)        | [18-编译时求值](https://cr0.dpdns.org/2026/08/08/18-CTE/)                 |
| [19-模块](https://cr0.dpdns.org/2026/08/17/19-Module/)             | [20-移动与深拷贝](https://cr0.dpdns.org/2026/08/25/20-Move-DeepCopy/)     |

## Benchmarks

[Click Here](https://github.com/fgaoxing/LangBench)

## Features

### Core Language

- **Everything is an Expression (EiaE)** — there are no statements: blocks, `if`/`else`, `match`, loops and function bodies are all expressions, and the last expression is the result
- **Strict Static Typing** — compile-time checking with full local inference; `int` and `float` never mix implicitly — conversions go through explicit `@` casts
- **Explicit Casts (`@T`)** — `int`↔`float` (truncating), `int`↔`bool` (normalized to 0/1), anything → `void` (discard), `void` → any primitive (zero value), pointer ↔ pointer (bitwise reinterpretation: `malloc(n)@*Point`)
- **Generics** — monomorphic instantiation with type inference at call sites
- **Tagged `Result` / `Maybe`** — error handling and null-safety built from `struct` + `union` + `enum`

### Memory Management

- **Move Semantics + Explicit Copies** — assignment and argument passing transfer ownership (O(1)); use `$expr` for an explicit deep copy and `*T` pointers for shared access; every owned value is released automatically when its scope exits
- **Manual Control When Needed** — raw `malloc` / `free` from the standard library for custom allocators and fine-grained lifetimes

### Functional & Control Flow

- **Lambdas & Higher-Order Functions** — first-class function types; methods are function-pointer fields on structs, invoked through dot sugar with `&self` as the first argument (`vec.push(&vec, x)`)
- **Expression-Oriented Control Flow** — `if`/`else`, `match` (arms need no separators, patterns evaluate lazily; `n..m` patterns match `n <= t < m`; arms may take a bool guard: `pat if cond: body`), `while`, `for-in` over arrays and ranges (`n..m`); `break expr` makes the innermost loop evaluate to a value
- **Short-Circuit Logic** — `&&` and `||` evaluate lazily
- **F-String Interpolation** — `println(f"value: {x}")`; every primitive interpolates, including `void` rendered as `nil`
- **Function Annotations** — `(pub)` export for modules, `(extern)` external symbols, `(pure)` pure functions; return-type annotations optional (default `void`)

### Compiler & Toolchain

- **Native AOT Compilation** — x86_64 machine code via an optimizing IR pipeline with a built-in register allocator, assembler, and ELF encoder; links with `lld`
- **Compile-Time Evaluation (CTE)** — `pure` functions run at compile time on the built-in bytecode VM: loops, ranges, `match`, literals, casts, recursion with memoization (`fib(40)` folds into one constant); guarded by step, time, stack, and recursion limits
- **Native Compile-Time Evaluation** — fold `fun(extern, pure)` calls against a shared library via `--cte-lib ./libfoo.so` (`libffi`); see `examples/32_native_cte/`
- **Preprocessor** — `#include` textual includes, `#define` object/function macros with recursion detection, conditional compilation via `#ifdef` / `#ifndef` / `#else` / `#endif`
- **C Interop (FFI)** — call C functions directly; C-compatible memory layout with zero conversion overhead
- **Build Toolkit** — integrated `almk`: scaffolding, building, running, git/zip dependencies, mixed C/Alum projects
- **Standard Library** — I/O, string utilities, conversions, `Vec`, `Result`/`Maybe`, math
- **Editor Support** — official VS Code extension plus `alum-lsp`, whose diagnostics run the same pipeline as the compiler
- **Formal Grammar** — complete EBNF in [GRAMMAR.md](./GRAMMAR.md)

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
4. Install standard library modules to `/usr/local/include/alum/`
5. Install the build tool `almk`

## Quick Start

### Hello World

Create a file `hello.al`:

```al
import io
using io::println

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
import io
using io::{write, read, print, println, input, fopen, fclose, fread, fwrite, lseek, pipe, pipe2, dup, dup2, dup3}
import string
using string::{strlen, strcpy, strcat, memcpy, memset, bcmp, memcmp}
import convert
using convert::{itoa, atoi, atof, ftoa}
import vec
using vec::{Vec, vec_new}
import maybe
using maybe::{MaybeTag, Maybe, is_some}
import result
using result::{ResultStatus, ResultValue, Result}

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

    // type cast: int -> float -> int
    var f = 42@float        // 42.0
    var truncated = 3.99@int  // 3
    println(itoa(truncated))   // 3

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

## Compiling and Running

### Single-file program

```bash
alc -r hello.al          # compile to ./hello and run
alc -c hello.al -o hello.o
alc hello.o /usr/local/lib/libalum.a -o hello   # manual link
```

### Native libraries (compile-time + runtime)

Attach a `.so` to fold `fun(extern, pure)` calls at compile time and to resolve
any remaining calls at runtime:

```bash
gcc -shared -fPIC -o libfoo.so foo.c
alc -r --cte-lib ./libfoo.so main.al
# --cte-lib may be repeated; the listed libraries are also linked into the
# produced executable (with an rpath).
```

Each `fun(extern, pure)` declaration emits
`warning: purity of external function '<name>' cannot be verified` — this is
expected, since an external symbol's side effects cannot be statically proven.

> **Note on native signatures:** the compiler assumes alum `int` corresponds to
> C `int64_t` (signed 64-bit) and alum `float` to C `double`. If your C function
> returns `int32_t`, sign-extend or widen it to `int64_t` before using it with
> compile-time evaluation, otherwise folded constants may contain garbage in the
> upper bits. Boolean returns should return 0 or 1 as a full-width value.

## Project Builds with almk

For larger projects, `almk` scaffolds, builds, and runs Alum/C projects.
A project is described by `Alumake.toml`:

```toml
[package]
name = "native_cte"
version = "0.1.0"
language = "alum"

[build]
linker = "alc"
cc = "cc"
alc = "alc"

[native]
shared = true               # produce lib<name>_native.so
sources = ["native/*.c"]    # C sources (supports a single `*` glob)
```

`almk run` compiles the `[native]` C sources into `target/lib<name>_native.so`,
passes it to `alc --cte-lib` (so `fun(extern, pure)` constants are folded), and
links the `.so` into the final executable with an rpath (when using
`linker = "alc"`, alc handles `-dynamic-linker` and rpath automatically; with
`cc`/`gcc` almk passes them explicitly). See
[`examples/32_native_cte`](./examples/32_native_cte).

## CLI Usage

```
Alum compiler

Usage: alc [OPTIONS] [INPUT]...

Arguments:
  [INPUT]...  Input files (.al source files or .o/.obj object files)

Options:
  -o, --output <FILE>   Output file name
  -c, --compile-only    Compile only, do not link
      --emit-ast        Output AST representation
      --emit-ir         Dump optimized IR to stderr, then continue compiling
      --emit-asm        Dump generated assembly to stderr, then continue compiling
  -r, --run             Compile and run immediately
  -I <DIR>              Add include directory
  -E                    Preprocess only; do not compile, assemble or link
      --nostdlib        Do not link with standard library
  -v, --verbose         Verbose output
      --library <TYPE>  Build library (static or shared)
      --cte-lib <PATH>  Shared library to dlopen for compile-time evaluation of fun(extern, pure) functions
  -h, --help            Print help
  -V, --version         Print version
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

- **[Grammar (EBNF)](./GRAMMAR.md)** - Formal grammar derived from the parser
- **[Standard Library](./alum-std/README.md)** - Comprehensive standard library documentation
- **[Build Tool](./alum-make/README.md)** - almk build tool documentation
- **[Tutorial Series](https://cr0.dpdns.org)** - 20-part Chinese tutorial covering the language from scratch

## License

See LICENSE file for details.