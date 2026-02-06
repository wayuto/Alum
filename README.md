# Alum Programming Language

Alum is a modern, systems programming language designed for simplicity and performance. It features a clean syntax, strong static typing, and compiles to native machine code using the Cranelift code generator.

## Features

- **Simple Syntax**: Clean, readable syntax inspired by modern languages
- **Static Typing**: Type safety with explicit type annotations
- **Native Compilation**: Compiles directly to machine code via Cranelift
- **Fast Compilation**: Efficient compilation pipeline
- **FFI Support**: Interoperability with C for low-level operations
- **Build Tool**: Integrated build system (almk) for project management

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
    println("Hello, World!");
    return 0;
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
    let x: int = 10;
    let y: int = 20;
    let sum: int = x + y;
    
    println(itoa(sum));
    return 0;
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

### Variables

```al
let name: string = "Alum";
let count: int = 42;
let pi: float = 3.14159;
let is_valid: bool = true;
```

### Functions

```al
fun add(a: int, b: int): int {
    return a + b;
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
    println("Positive");
}

// While Loop
while i < 10 {
    i = i + 1;
}

// For Loop
for i in 0..10 {
    println(itoa(i));
}
```

### Arrays

```al
let numbers: arr[int] = [1, 2, 3, 4, 5];
let buffer: arr[int] = [int; 100];
let first: int = numbers[0];
```

### Preprocessor Directives

```al
$define PI 3.14159
$ifndef ALUM_LIB
$define ALUM_LIB 1
$endif
$import "io.al"
```

## Compilation Pipeline

1. **Preprocessing**: Handles `$import`, `$define`, `$ifdef`, `$ifndef`, `$endif`
2. **Lexing**: Tokenizes source code
3. **Parsing**: Builds Abstract Syntax Tree (AST)
4. **Code Generation**: Compiles AST to machine code using Cranelift
5. **Linking**: Links object files with standard library

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