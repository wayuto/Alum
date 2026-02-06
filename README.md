# Alum Programming Language

Alum is a modern, systems programming language designed for simplicity and performance. It features a clean syntax, strong static typing, and compiles to native machine code using the Cranelift code generator.

## Features

- **Simple Syntax**: Clean, readable syntax inspired by modern languages
- **Static Typing**: Type safety with explicit type annotations
- **Native Compilation**: Compiles directly to machine code via Cranelift
- **Standard Library**: Comprehensive standard library for I/O, math, strings, arrays, memory, and conversion
- **Preprocessor**: Supports includes, defines, and conditional compilation
- **Fast Compilation**: Efficient compilation pipeline

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

## Quick Start

### Hello World

Create a file `hello.al`:

```al
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

Preprocess only:
```bash
alc -E program.al
```

Include custom directories:
```bash
alc program.al -I ./include
```

Verbose output:
```bash
alc -v program.al
```

## Language Syntax

### Type System

Alum is a statically typed language with explicit type annotations. All variables and functions must have their types declared at compile time. This provides type safety and enables the compiler to generate efficient machine code.

**Key characteristics:**
- Static typing: Types are checked at compile time
- Explicit annotations: Types must be declared using the `:` syntax
- No type inference: You must specify the type for each variable and function parameter
- Type safety: Prevents many common programming errors through compile-time checking

### Types

Alum supports the following primitive types:
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
let nothing: int = nil;
```

### Functions

```al
fun add(a: int, b: int): int {
    return a + b;
}

fun greet(name: string): void {
    println("Hello, ");
}
```

### Extern Functions

Declare external functions (typically from C):

```al
extern syscall(int, int, int, int): int
extern exit(int): void
```

### Control Flow

#### If-Else

```al
if x > 0 {
    println("Positive");
} else {
    println("Non-positive");
}
```

#### While Loop

```al
let i: int = 0;
while i < 10 {
    println(itoa(i));
    i = i + 1;
}
```

#### For Loop

```al
for i in 0..10 {
    println(itoa(i));
}
```

### Operators

**Arithmetic**: `+`, `-`, `*`, `/`

**Comparison**: `==`, `!=`, `<`, `<=`, `>`, `>=`

**Logical**: `&&`, `||`, `!`

**Bitwise**: `&`, `|`, `^`

**Range**: `..`

### Arrays

```al
// Array literal
let numbers: arr[int] = [1, 2, 3, 4, 5];

// Array with fill syntax [type; size]
let buffer: arr[int] = [int; 100];

// Array access
let first: int = numbers[0];
numbers[1] = 10;
```

### Preprocessor Directives

```al
// Define a constant
$define PI 3.14159

// Conditional compilation
$ifndef ALUM_LIB
$define ALUM_LIB 1
$endif

// Import a module
$import "io.al"
```

## Standard Library

The Alum standard library provides essential functionality organized into modules.

### Importing Modules

```al
$import "io.al"
$import "math.al"
$import "string.al"
$import "array.al"
$import "memory.al"
$import "convert.al"
```

### I/O Module (`io.al`)

```al
extern write(int, string, int): int    // Write to file descriptor
extern read(int, string, int): int     // Read from file descriptor
extern print(string): int              // Print string
extern println(string): int            // Print string with newline
extern input(string): string           // Read user input with prompt
extern fopen(string, int, int): int    // Open file
extern fclose(int): int                // Close file
extern fread(int): string              // Read from file
extern fwrite(int, string, int): int   // Write to file
extern lseek(int, int, int): int       // Seek in file
```

### Math Module (`math.al`)

```al
extern abs(int): int        // Absolute value
extern sqrt(int): int       // Square (note: returns x * x)
extern max(int, int): int   // Maximum of two numbers
extern min(int, int): int   // Minimum of two numbers
extern pow(int, int): int   // Power function
extern fact(int): int       // Factorial
```

### String Module (`string.al`)

```al
extern strlen(string): int              // String length
extern strcpy(string, string): string   // String copy
extern strcat(string, string): string   // String concatenation
extern memcpy(string, string, int): string  // Memory copy
extern memset(string, int, int): string    // Memory set
extern bcmp(string, string, int): int      // Byte comparison
extern memcmp(string, string, int): int    // Memory comparison
```

### Array Module (`array.al`)

```al
extern range(int, int): string  // Generate range (returns pointer to array)
```

### Memory Module (`memory.al`)

```al
extern malloc(int): string  // Allocate memory (returns pointer)
```

### Convert Module (`convert.al`)

```al
extern itoa(int): string    // Integer to string
extern atoi(string): int    // String to integer
extern atof(string): float  // String to float
extern ftoa(float): string  // Float to string
```

### Main Library (`lib.al`)

The main library module imports all standard library modules:

```al
$import "io.al"
$import "string.al"
$import "convert.al"
$import "math.al"
$import "array.al"
$import "memory.al"

extern syscall(int, int, int, int): int
extern exit(int): void
```

## Compilation Pipeline

The Alum compiler follows a standard compilation pipeline:

1. **Preprocessing**: Handles `$import`, `$define`, `$ifdef`, `$ifndef`, `$endif` directives
2. **Lexing**: Tokenizes source code into tokens
3. **Parsing**: Builds an Abstract Syntax Tree (AST)
4. **Code Generation**: Compiles AST to machine code using Cranelift
5. **Linking**: Links object files with standard library to create executable

## Project Structure

```
Alum/
├── src/
│   ├── main.rs           # Compiler entry point
│   ├── cli/              # CLI argument parsing and commands
│   │   ├── args.rs       # Command-line argument definitions
│   │   ├── build.rs      # Build command implementation
│   │   ├── link.rs       # Linker implementation
│   │   └── mod.rs        # CLI module exports
│   └── compiler/         # Compiler components
│       ├── lexer.rs      # Lexical analyzer
│       ├── parser.rs     # Parser
│       ├── ast.rs        # AST definitions
│       ├── codegen.rs    # Code generation
│       ├── preprocessor.rs  # Preprocessor
│       └── mod.rs        # Compiler module exports
├── alum-std/             # Standard library
│   ├── alum/             # Standard library headers (.al files)
│   │   ├── lib.al        # Main library module
│   │   ├── io.al         # I/O functions
│   │   ├── math.al       # Math functions
│   │   ├── string.al     # String functions
│   │   ├── array.al      # Array functions
│   │   ├── memory.al     # Memory functions
│   │   └── convert.al    # Type conversion functions
│   └── src/              # Standard library implementation (Rust no_std)
│       ├── lib.rs        # Library entry point with syscalls
│       ├── io.rs         # I/O implementation
│       ├── math.rs       # Math implementation
│       ├── string.rs     # String implementation
│       ├── array.rs      # Array implementation
│       ├── memory.rs     # Memory implementation
│       └── convert.rs    # Conversion implementation
├── alum-vscode/          # VS Code extension
│   ├── syntaxes/
│   │   └── alum.tmLanguage.json  # Syntax highlighting
│   └── language-configuration.json
├── Cargo.toml            # Compiler dependencies
└── install.sh            # Installation script
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
