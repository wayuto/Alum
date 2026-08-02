# Alum Programming Language

Alum is a modern, systems programming language designed for simplicity and performance. It features a clean syntax, strong static typing, and compiles to native machine code via an optimizing IR pipeline.

## Features

- **Native Compilation**: Compiles directly to machine code via built-in assembler + LLD
- **Build Toolkit**: Integrated `almk` build system and `almk run` for project management
- **Standard Library**: Rich `alum-std` with I/O, strings, vectors, math, and memory management


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

### Basic Example

```al
$import "convert.ah"

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
- `T[N]`: Fixed-size array of type T with N elements
- `T[]`: Dynamic array (length determined at runtime)
- `struct`/`union`: User-defined composite types
- `T`: Generic type parameter (for parametric polymorphism)

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

// For Loop with Range
for i in 0..10 {
    println(itoa(i))
}

// For Loop iterating over array elements
let numbers: int[5] = [10, 20, 30, 40, 50]
for x in numbers {
    println(itoa(x))  // Prints each element
}
```

### Arrays

Alum arrays have C-compatible memory layout and compile-time known length.

```al
// Fixed-size array with explicit length
let numbers: int[5] = [1, 2, 3, 4, 5]

// Array with fill syntax (creates array of specified size)
let buffer: int[100] = [int; 100]

// Access array elements
let first: int = numbers[0]
```

**C Compatibility:**
Alum arrays are fully compatible with C arrays. You can pass an Alum array directly to C functions:

```al
// C function declaration
extern c_sum(*int, int): int

// Alum array
let arr: int[5] = [10, 20, 30, 40, 50]

// Directly pass to C function (no conversion needed)
let sum: int = c_sum(arr, 5)
```

### Pointers

Alum supports pointers for direct memory access and manipulation. Pointers are declared using the `*` prefix before a type.

**Declaration and Usage:**
```al
$import "memory.ah"

fun main(): int {
    let value: int = 42
    let ptr: *int = &value  // Get address of value
    
    // Dereference to access value
    println(itoa(*ptr))  // Output: 42
    
    // Modify value through pointer
    *ptr = 100
    println(itoa(value))  // Output: 100
    
    return 0
}
```

**Pointer to Struct:**
```al
struct Point {
    x: int,
    y: int
}

fun modify_point(p: *Point): void {
    p.x = 100
    p.y = 200
}

fun main(): int {
    let point: Point = Point {
        x: 10,
        y: 20
    }
    
    modify_point(&point)
    println(itoa(point.x))  // Output: 100
    println(itoa(point.y))  // Output: 200
    
    return 0
}
```

**Dynamic Memory:**
```al
$import "memory.ah"

extern malloc(int): *void  // Allocate memory, returns byte pointer
extern free(*void): void   // Free memory (no size needed — block header stores size)

fun main(): int {
    let ptr: *void = malloc(100)
    free(ptr)
    return 0
}
```

**Pointer Arithmetic:**
```al
fun main(): int {
    let p: *int = malloc(40)  // Space for 5 integers
    p[0] = 10
    p[1] = 20

    let q: *int = p + 1  // Move to next int (8 bytes ahead)
    println(itoa(q[0]))  // Output: 20

    let sub: *int = p - 1  // Move to previous int
    println(itoa(sub[1]))  // Output: 20 (same as p[0]→wait actually sub=p-1 →sub[1]=p[0])

    let eq: bool = p == p
    println(itoa(eq))  // Output: 1

    let lt: bool = p < q
    println(itoa(lt))  // Output: 1

    return 0
}
```

**String Indexing:**
```al
fun main(): int {
    let s: string = "Hello"
    println(itoa(s[0]))  // Output: 72 (ASCII 'H')
    println(itoa(s[4]))  // Output: 111 (ASCII 'o')
    return 0
}
```

### Vec Container

Alum provides a generic dynamic array `Vec<T>` for storing collections of elements that can grow or shrink at runtime. The Vec uses monomorphic instantiation — each element type gets its own compiled version.

**Importing Vec:**
```al
$import "vec.ah"
```

**Creating a Vec:**
```al
fun main(): int {
    let vec: Vec<int> = vec_new()
    return 0
}
```

**Vec Operations:**
```al
$import "vec.ah"
$import "convert.ah"

fun main(): int {
    let vec: Vec<int> = vec_new()
    
    // Push elements
    vec.push(&vec, 10)
    vec.push(&vec, 20)
    vec.push(&vec, 30)
    
    // Access elements by index
    let first: int = vec.at(&vec, 0)
    let second: int = vec.at(&vec, 1)
    
    println(itoa(first))   // Output: 10
    println(itoa(second))  // Output: 20
    
    return 0
}
```

**Vec Methods:**
- `vec_new<T>()`: Creates a new empty `Vec<T>`
- `vec.at(&vec, index)`: Access element at the given index
- `vec.push(&vec, element)`: Add an element to the end
- `vec.pop(&vec)`: Remove and return the last element

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

### Unions

Define custom data structures that share the same memory among all members with the `union` keyword. All union members overlap in memory, and the union size is the size of its largest member:

```al
union Value {
    i: int,
    f: float
}

fun main(): int {
    let v: Value = Value {
        i: 42
    }
    println(itoa(v.i))

    // Assign through any member; they all share storage
    v.f = 3.14
    return 0
}
```

Union members are accessed with the dot operator, and unions support the same features as structs (type parameters, pointer access, etc.).

### Lambda Functions

Alum supports anonymous functions (lambdas) for functional programming patterns:

```al
fun apply_function(f: fun(int): int, value: int): int {
    return f(value)
}

fun main(): int {
    // Define a lambda
    let square: int(int): int = \(x: int): int {
        return x * x
    }

    let result: int = apply_function(square, 5)
    println(itoa(result))  // Output: 25

    // Use lambda directly
    let double: int(int): int = \(x: int): int {
        return x * 2
    }
    let doubled: int = double(10)
    println(itoa(doubled))  // Output: 20

    return 0
}
```

Lambda syntax: `\(param: type, ...): return_type body`

### Generic Types (Parametric Polymorphism)

Alum supports generic functions and types with explicit type parameters:

```al
fun identity<T>(x: T): T {
    return x
}

fun add<T>(a: T, b: T): T {
    return a + b
}

fun main(): int {
    // Type inferred from usage
    let x: int = identity(42)
    let y: float = identity(3.14)

    // Works with arithmetic
    let sum: int = add(10, 20)
    println(itoa(sum))

    return 0
}
```

**Key Features:**
- **Explicit Type Parameters**: `fun foo<T>(x: T): T` declares a generic function
- **Type Inference**: The actual type is inferred from usage context
- **Monomorphic Instantiation**: The compiler generates a specialized version for each type used
- **Generic Containers**: `Vec<T>` holds elements of type `T`
- **Flexibility**: Generics work with all types including structs and functions

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
$import "io.ah"
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
│  Optimizer    │  →  Constant folding, dead code elimination, IR optimizations
└───────────────┘
        │
        ▼
┌───────────────┐
│   IR Gen      │  →  Lowers AST to intermediate representation (IR)
└───────────────┘
        │
        ▼
┌──────────────────┐
│ Code Generator   │  →  Emits x86-64 instructions (Asm IR)
└──────────────────┘
        │
        ▼
┌──────────────────┐
│   Assembler      │  →  Encodes Asm IR → x86-64 machine code → ELF .o
└──────────────────┘
        │
        ▼
  Object File (.o)
        │
        ▼
┌──────────────────┐
│    Linker        │  →  LLD links object file with standard library
└──────────────────┘
        │
        ▼
  Executable File
```

### Pipeline Stages

1. **Preprocessing**: Handles `$import`, `$define`, `$ifdef`, `$ifndef`, `$endif` directives, and macro expansion
2. **Lexing**: Tokenizes source code into a stream of tokens
3. **Parsing**: Builds Abstract Syntax Tree (AST) from tokens
4. **Type Checking**: Validates type safety and semantic rules
5. **Optimization**: Performs constant folding, dead code elimination, and IR optimizations
6. **IR Generation**: Lowers AST to a platform-agnostic intermediate representation
7. **Code Generation**: Emits x86-64 instructions as typed `Asm` IR
8. **Assembly**: Built-in assembler encodes `Asm` IR to x86-64 machine code and produces an ELF64 object file (`.o`)
9. **Linking**: LLD links object files with the standard library to produce an executable

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