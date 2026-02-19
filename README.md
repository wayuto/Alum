# Alum Programming Language

Alum is a modern, systems programming language designed for simplicity and performance. It features a clean syntax, strong static typing, and compiles to native machine code using the Cranelift code generator.

## Features

- **Simple Syntax**: Clean, readable syntax inspired by modern languages
- **Static Typing**: Type safety with explicit type annotations
- **Native Compilation**: Compiles directly to machine code via Cranelift
- **Fast Compilation**: Efficient compilation pipeline
- **FFI Support**: Interoperability with C for low-level operations
- **Build Tool**: Integrated build system (almk) for project management
- **Lambda Functions**: Support for anonymous functions and closures
- **Parametric Macros**: Powerful macro system with parameter substitution

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
$import "io.al"

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

### Basic Example

```al
$import "convert.al"

fun main(): int {
    let x: int = 10
    let y: int = 20
    let sum: int = x + y
    
    println(itoa(sum))
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

### Examples

Compile to executable:
```bash
alc program.al -o program
```

Compile only (object file):
```bash
alc program.al -c -o program.o
```

Link object files:
```bash
alc program.o -o program
```

Run immediately:
```bash
alc -r program.al
```

Include custom directories:
```bash
alc program.al -I ./include
```

Library mode (keep all functions for library building):
```bash
alc lib.al -o lib.o --lib
```

## Language Syntax

### Type System

Alum is a statically typed language with explicit type annotations. All variables and functions must have their types declared at compile time.

**Supported types:**
- `int`: Signed integer (isize)
- `float`: 64-bit floating point number (f64)
- `bool`: Boolean value
- `string`: String type
- `void`: No return type
- `arr[T]`: Array of type T
- `any`: Generic type with automatic type inference

The `any` type supports generic-like programming with automatic type inference. Variables and parameters of type `any` can accept any value, and the compiler infers the actual type based on usage context.

### Variables

```al
let name: string = "Alum"
let count: int = 42
let pi: float = 3.14159
let is_valid: bool = true
```

### Functions

```al
fun add(a: int, b: int): int {
    return a + b
}
```

### Extern Functions (FFI)

Declare external functions for C interoperability:

```al
extern c_add(int, int): int
extern printf(string): int
```

### Control Flow

```al
// If-Else
if x > 0 {
    println("Positive")
}

// While Loop
while i < 10 {
    i = i + 1
}

// For Loop
for i in 0..10 {
    println(itoa(i))
}
```

### Arrays

```al
let numbers: arr[int] = [1, 2, 3, 4, 5]
let buffer: arr[int] = [int; 100]
let first: int = numbers[0]
```

### Structs

Define custom data structures with the `struct` keyword:

```al
struct Point {
    x: int,
    y: int
}

fun main(): int {
    let p: Point = Point {
        x: 10,
        y: 20
    }
    return 0
}
```

Access struct fields using the dot operator:
```al
println(itoa(p.x))
println(itoa(p.y))
```

### Lambda Functions

Alum supports anonymous functions (lambdas) for functional programming patterns:

```al
fun apply_function(f: fun(int): int, value: int): int {
    return f(value)
}

fun main(): int {
    // Define a lambda
    let square: int(int): int = lamb(x: int): int {
        return x * x
    }

    let result: int = apply_function(square, 5)
    println(itoa(result))  // Output: 25

    // Use lambda directly
    let double: int(int): int = lamb(x: int): int {
        return x * 2
    }
    let doubled: int = double(10)
    println(itoa(doubled))  // Output: 20

    return 0
}
```

Lambda syntax: `lamb(param: type, ...): return_type body`

### Any Type (Generic Programming)

The `any` type provides generic-like functionality with automatic type inference:

```al
fun identity(x: any): any {
    return x
}

fun add(a: any, b: any): any {
    return a + b
}

fun main(): int {
    // any type accepts any value
    let x: any = 42
    let y: any = "hello"
    let z: any = 3.14

    // Type is inferred from usage
    let result: any = identity(x)  // inferred as int
    println(itoa(result))

    // Works with arithmetic operations
    let sum: any = add(10, 20)  // inferred as int
    println(itoa(sum))

    // Functions can return any type
    fun get_func(): any {
        fun helper(): any {
            return 42
        }
        return helper
    }

    let func_ptr: any = get_func()
    let value: any = func_ptr()  // inferred as int
    println(itoa(value))

    return 0
}
```

**Key Features:**
- **Type Inference**: The actual type is inferred from how the value is used
- **Flexibility**: Can represent any type (int, float, string, functions, etc.)
- **Safety**: Type checking is still performed at compile time
- **Compatibility**: `any` is compatible with all other types

### Preprocessor Directives

```al
// Simple macro (no parameters)
$define PI 3.14159
$define HELLO "Hello, World!"

// Parametric macro with parameters
$define ADD(a, b) a + b
$define MAX(a, b) if a > b { a } else { b }

// Conditional compilation
$ifndef ALUM_LIB
$define ALUM_LIB 1
$endif

// Import modules
$import "io.al"
```

**Macro Usage:**
- Simple macros are used directly without a prefix: `println(HELLO)`
- Parametric macros are called like functions: `let sum: int = ADD(10, 20)`
- Macros support nested calls: `MAX(ADD(x, y), 100)`

## Compilation Pipeline

```
Source Code (.al)
        │
        ▼
┌───────────────┐
│ Preprocessor  │  →  Handles $import, $define, $ifdef, $ifndef, $endif
└───────────────┘
        │
        ▼
┌───────────────┐
│    Lexer      │  →  Tokenizes source code into tokens
└───────────────┘
        │
        ▼
┌───────────────┐
│    Parser     │  →  Builds Abstract Syntax Tree (AST)
└───────────────┘
        │
        ▼
┌───────────────┐
│  Type Checker │  →  Validates type safety and semantic rules
└───────────────┘
        │
        ▼
┌───────────────┐
│   Optimizer   │  →  Constant folding, dead code elimination
└───────────────┘
        │
        ▼
┌───────────────┐
│ Code Generator│  →  Compiles AST to machine code using Cranelift
└───────────────┘
        │
        ▼
  Object File (.o)
        │
        ▼
┌───────────────┐
│    Linker     │  →  Links object files with standard library
└───────────────┘
        │
        ▼
 Executable File
```

### Pipeline Stages

1. **Preprocessing**: Handles `$import`, `$define`, `$ifdef`, `$ifndef`, `$endif` directives, and macro expansion
2. **Lexing**: Tokenizes source code into a stream of tokens
3. **Parsing**: Builds Abstract Syntax Tree (AST) from tokens
4. **Type Checking**: Validates type safety and semantic rules
5. **Optimization**: Performs constant folding, dead code elimination, and other optimizations
6. **Code Generation**: Compiles AST to machine code using Cranelift
7. **Linking**: Links object files with standard library to produce executable

**Optimizations performed:**
- Constant folding (e.g., `2 + 3` → `5`)
- Algebraic simplifications (e.g., `x + 0` → `x`)
- Dead code elimination
- Branch elimination (e.g., removing unreachable code)

## Project Structure

```
Alum/
├── src/                      # Compiler source code
│   ├── main.rs               # Compiler entry point
│   ├── cli/                  # CLI argument parsing and commands
│   └── compiler/             # Compiler components
├── alum-std/                 # Standard library
│   ├── alum/                 # Standard library headers (.al files)
│   └── src/                  # Standard library implementation (Rust no_std)
├── alum-make/                # Build tool (almk)
│   └── src/                  # Build tool source code
├── alum-vscode/              # VS Code extension
│   └── syntaxes/             # Syntax highlighting
├── Cargo.toml                # Compiler dependencies
└── install.sh                # Installation script
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

## License

See LICENSE file for details.