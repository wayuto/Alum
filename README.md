# **The Gos Programming Language([中文版](README.zh.md))**

Gos is a lightweight, efficient programming language which is implemented in
**Rust**

## **🚀 Installation**

### **Prerequisites**

Gos is designed for **x86_64 Linux** environments. Ensure you have the following
installed:

- **Rust & Cargo** (2024 edition)
- **NASM** (The Netwide Assembler)
- **ld** (GNU Linker)

### **Setup**

Clone the repository and run the automated installation script to set up the
`gos` binary and the standard library:

```bash
# Clone the repository  
git clone --depth 1 https://github.com/wayuto/Gos ~/Gos
cd ~/Gos

# Run the installation script  
# This installs the 'gos' CLI, builds the standard library,   
# and moves 'libgos.a' to /usr/local/lib  
sh ./install.sh
```

## **🛠 Features & Syntax**

### **Basic Types**

- `int`: 64-bit signed integer.
- `flt`: 64-bit floating-point number (IEEE 754 double precision).
- `str`: String type.
- `bool`: Boolean logic (true / false).
- `arr<N>`: Fixed-size arrays (e.g., arr<5>).
- `void`: Used for functions that do not return a value.

### **Variables & Constants**

```
let x: int = 42 
let pi: flt = 3.14159
let message: str = "Hello, Gos!"
let flags: arr<3> = [1 2 3] 
let dynamic: arr<_> = [1 2 3 4] # Length inferred as 4
```

### **Floating-Point Numbers**

Gos supports 64-bit floating-point numbers using the `flt` keyword.
Floating-point literals can be written with decimal notation:

```gos
let pi: flt = 3.14159
let e: flt = 2.71828
let result: flt = 10.5 + 20.3
```

Floating-point numbers support all standard arithmetic operations (+, -, *, /)
and comparisons (==, !=, >, >=, <, <=).

### **Control Flow**

Gos supports modern control flow structures, including if-else expressions and
loops.

```
# If-Else as an expression
let result: str = if x > 10 "High" else "Low"

# While loop
while x > 0 { x-- }

# Range-based For loop ($import "array" before using `n..m`)
for i in 0..10 { println(itoa(i)) }
```

### **Block Scopes**

In Gos, code blocks are expressions. The last value in a block is returned as
the block's value.

```
let computed: int = { 
  let a: int = 10 
  let b: int = 20 
  a + b # This is the block's value
}
```

## **🔧 Preprocessor Directives**

Gos includes a preprocessor that supports directives for code organization and
reuse.

### **$import**

The `$import` directive includes external Gos source files into your program:

```gos
$import "gosio"
$import "math"
```

The preprocessor searches for imported files in:

1. The same directory as the current file
2. `/usr/local/gos/` (standard library location)

### **$define**

The `$define` directive allows you to define macros for text substitution:

```gos
$define PI 3.14159
$define MAX_SIZE 100
$define GREETING "Hello, Gos!"

pub fun main(): int {
  let arr: arr<MAX_SIZE> = [0]
  let radius: flt = 5.0
  let area: flt = PI * radius * radius
  println(GREETING)
  return 0
}
```

Macros are simple text replacements that occur during preprocessing. They can be
used for constants, simple expressions, or code snippets. Parameterized macros
are not currently supported.

## **📚 Standard Library (gos-std)**

The Gos Standard Library provides essential functionality out of the box. Use
`$import` to include them.

| Module      | Key Functions                          |
| :---------- | :------------------------------------- |
| **gosio**   | print, println, input, read, write     |
| **math**    | abs, sqrt, max, min, pow, fact         |
| **string**  | strlen, strcpy, strcat, memcpy, memset |
| **convert** | itoa, atoi, ftoa, atof                 |
| **array**   | range, find                            |
| **stdlib**  | syscall, exit                          |

## **💻 Language Examples**

### **Hello World**

Save this as `hello.gos`:

```
$import "gosio"

pub fun main(): int {
  println("Hello world!") 
  return 0
}
```

Run with:

```bash
gos -c hello.gos
./hello
```

### **Recursive Fibonacci**

```
$import "gosio"
$import "convert"

fun fib(n: int a: int b: int): int {
  if n == 0 return a
  return fib(n - 1 b a + b)
}

pub fn main(): int {
  let n: int = fib(40 0 1)
  println(
    itoa(n)
  ) 
  return 0 
}
```

### **Floating-Point Calculation Example**

```
$import "gosio"
$import "convert"

pub fun main(): int {
  let radius: flt = 5.0
  let pi: flt = 3.14159
  let area: flt = pi * radius * radius
  println("Circle area: ")
  # Note: You can use the ftoa function from the convert module to convert floats to strings
  return 0
}
```

## **🔗 FFI & Interoperability**

Gos is designed to play well with C. You can declare external functions and call
them directly. **Calling a C function in Gos:**

# Declare the external C function

```
extern println(str): int

pub fun main(): int {
  println("Hello world!")
  return 0
}
```

**Exposing a Gos function to C:**

# Use `pub` to make it visible to the linker

```
pub fun add(x: int y: int): int {
  return x + y
}
```

## **📊 Benchmark**

### Environment

- CPU: Intel i5-8265U (8 cores @ 3.900GHz)
- Memory: 8GB DDR4 (7647MiB)
- Architecture: x86_64
- Operating System: Arch Linux
- Kernel Version: 6.18.1-zen1-2-zen
- Desktop Environment: GNOME 49.2

### Test Content

Performance test comparing three programming languages using tail recursion to
compute the 1000th Fibonacci number:

- Gos Native 0.5.2: Gos language compiled to native
- executable C (GCC -O3): C language compiled with GCC's highest optimization
  level
- Python 3.13.11: Python interpreted execution

```
➜  fibonacci1000 ./run.sh 
Benchmark 1: ./foo
  Time (mean ± σ):     163.6 µs ±  38.8 µs    [User: 94.9 µs, System: 8.2 µs]
  Range (min … max):   121.4 µs … 2036.7 µs    17324 runs
 
  Warning: Statistical outliers were detected. Consider re-running this benchmark on a quiet system without any interferences from other programs. It might help to use the '--warmup' or '--prepare' options.
 
Benchmark 2: ./a.out
  Time (mean ± σ):     479.4 µs ±  42.8 µs    [User: 267.1 µs, System: 117.8 µs]
  Range (min … max):   365.9 µs … 847.5 µs    5582 runs
 
  Warning: Statistical outliers were detected. Consider re-running this benchmark on a quiet system without any interferences from other programs. It might help to use the '--warmup' or '--prepare' options.
 
Benchmark 3: python foo.py
  Time (mean ± σ):      13.6 ms ±   0.6 ms    [User: 10.7 ms, System: 2.7 ms]
  Range (min … max):    12.5 ms …  16.2 ms    223 runs
 
Summary
  ./foo ran
    2.93 ± 0.74 times faster than ./a.out
   83.32 ± 20.15 times faster than python foo.py
```

## **⚙️ CLI Reference**

```bash
The Gos programming language

Usage: gos [OPTIONS] [FILE]

Arguments:
  [FILE]  Run the Gos source file

Options:
  -a, --ast <ast>                  Print AST of the Gos source file
  -c, --compile <compile>          Compile the Gos source file to native
  -s                               Compile the Gos source file to assembly
  -o                               Compile the Gos source file to object
  -n                               Do not link the Gos Standard Library
  -p, --preprocess <preprocess>    Print the preprocessed Gos source file
  -d, --disassemble <disassemble>  Run the Gos source file
  -h, --help                       Print help
  -V, --version                    Print version
```
