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
- `T[N]`: Fixed-size array of type T with N elements
- `T[]`: Dynamic array (length determined at runtime)
- `gen`: Generic type with automatic type inference

The `gen` type supports generic-like programming with automatic type inference. Variables and parameters of type `gen` can accept gen value, and the compiler infers the actual type based on usage context.

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
$import "memory.al"

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
extern malloc(int): *int  // Allocate memory
extern free(*int): void   // Free memory

fun main(): int {
    let ptr: *int = malloc(10)  // Allocate memory for 10 integers
    *ptr = 42
    println(itoa(*ptr))
    free(ptr)
    return 0
}
```

### Vec Container

Alum provides a dynamic array (Vec) for storing collections of elements that can grow or shrink at runtime. The Vec is part of the standard library and supports various operations.

**Importing Vec:**
```al
$import "vec.al"
```

**Creating a Vec:**
```al
fun main(): int {
    let vec: Vec = vec_new()
    return 0
}
```

**Vec Operations:**
```al
$import "vec.al"
$import "convert.al"

fun main(): int {
    let vec: Vec = vec_new()
    
    // Push elements
    vec.push(&vec, 10)
    vec.push(&vec, 20)
    vec.push(&vec, 30)
    
    // Access elements by index
    let first: gen = vec.at(&vec, 0)
    let second: gen = vec.at(&vec, 1)
    
    println(itoa(first))   // Output: 10
    println(itoa(second))  // Output: 20
    
    // Pop elements
    let popped: gen = vec.pop(&vec)
    println(itoa(popped))  // Output: 30
    
    return 0
}
```

**Vec with Structs:**
```al
struct Point {
    x: int,
    y: int
}

$import "vec.al"

fun main(): int {
    let vec: Vec = vec_new()
    
    let p1: Point = Point { x: 10, y: 20 }
    let p2: Point = Point { x: 30, y: 40 }
    
    vec.push(&vec, p1)
    vec.push(&vec, p2)
    
    let point: gen = vec.at(&vec, 0)
    // Access struct fields (requires casting or direct access pattern)
    
    return 0
}
```

**Vec Methods:**
- `vec_new()`: Creates a new empty Vec
- `vec.at(&vec, index)`: Access element at the given index
- `vec.push(&vec, element)`: Add an element to the end
- `vec.pop(&vec)`: Remove and return the last element

**Note:** The Vec uses `gen` type to store elements, allowing it to hold values of gen type. The compiler automatically infers the actual type based on usage context.

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

### Any Type (Generic Programming)

The `gen` type provides generic-like functionality with automatic type inference:

```al
fun identity(x: gen): gen {
    return x
}

fun add(a: gen, b: gen): gen {
    return a + b
}

fun main(): int {
    // gen type accepts gen value
    let x: gen = 42
    let y: gen = "hello"
    let z: gen = 3.14

    // Type is inferred from usage
    let result: gen = identity(x)  // inferred as int
    println(itoa(result))

    // Works with arithmetic operations
    let sum: gen = add(10, 20)  // inferred as int
    println(itoa(sum))

    // Functions can return gen type
    fun get_func(): gen {
        fun helper(): gen {
            return 42
        }
        return helper
    }

    let func_ptr: gen = get_func()
    let value: gen = func_ptr()  // inferred as int
    println(itoa(value))

    return 0
}
```

**Key Features:**
- **Type Inference**: The actual type is inferred from how the value is used
- **Flexibility**: Can represent gen type (int, float, string, functions, etc.)
- **Safety**: Type checking is still performed at compile time
- **Compatibility**: `gen` is compatible with all other types

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