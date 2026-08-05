# Alum Standard Library

The Alum standard library provides essential functionality for I/O, math, strings, arrays, memory, type conversion, and higher-level containers (`Vec`, `Result`, `Maybe`).

## Installation

The standard library is automatically installed when running the main installation script:

```bash
./install.sh
```

This installs:
- `libalum_std.a` to `/usr/local/lib/`
- Standard library headers to `/usr/local/include/alum/`

## Modules

### Importing Modules

```al
$import "io.ah"
$import "string.ah"
$import "math.ah"
$import "memory.ah"
$import "convert.ah"
$import "vec.ah"
$import "result.ah"
$import "maybe.ah"
```

### I/O Module (`io.ah`)

Provides input/output and file operations.

```al
fun(extern) write(int, string, int): int    // Write to file descriptor
fun(extern) read(int, string, int): int     // Read from file descriptor
fun(extern) print(string): int              // Print string
fun(extern) println(string): int            // Print string with newline
fun(extern) input(string): string           // Read user input with prompt
fun(extern) fopen(string, int, int): int    // Open file
fun(extern) fclose(int): int                // Close file
fun(extern) fread(int): string              // Read from file
fun(extern) fwrite(int, string, int): int   // Write to file
fun(extern) lseek(int, int, int): int       // Seek in file
```

### String Module (`string.ah`)

Provides string (byte array) operations.

```al
fun(extern) strlen(string): int             // String length
fun(extern) strcpy(string, string): string  // Copy string
fun(extern) strcat(string, string): string  // Concatenate strings
fun(extern) memcpy(string, string, int): string  // Copy n bytes
fun(extern) memset(string, int, int): string     // Fill n bytes with a value
fun(extern) bcmp(string, string, int): int       // Byte comparison
fun(extern) memcmp(string, string, int): int     // Byte comparison (n bytes)
```

### Math Module (`math.ah`)

Provides mathematical operations.

```al
fun(extern) abs(int): int        // Absolute value
fun(extern) sqrt(int): int       // Integer square root
fun(extern) max(int, int): int   // Maximum of two numbers
fun(extern) min(int, int): int   // Minimum of two numbers
fun(extern) pow(int, int): int   // Power function
fun(extern) fact(int): int       // Factorial
```

### Memory Module (`memory.ah`)

Provides memory management functions using a free-list allocator with block headers.

```al
fun(extern) malloc(int): *void  // Allocate memory, returns byte pointer
fun(extern) free(*void): void   // Free memory (no size needed)
```

### Convert Module (`convert.ah`)

Provides type conversion functions.

```al
fun(extern) itoa(int): string    // Integer to string
fun(extern) atoi(string): int    // String to integer
fun(extern) atof(string): float  // String to float
fun(extern) ftoa(float): string  // Float to string
```

### Vec Container (`vec.ah`)

A generic dynamic array. `Vec<T>` is monomorphically instantiated per element type; methods are stored as function-pointer fields and called with `&vec` as the first argument.

```al
$import "vec.ah"
$import "convert.ah"

fun main(): int {
    let vec: Vec<int> = vec_new()

    vec.push(&vec, 10)
    vec.push(&vec, 20)
    vec.push(&vec, 30)

    println(itoa(vec.at(&vec, 0)))  // 10
    println(itoa(vec.at(&vec, 2)))  // 30
    println(itoa(vec.pop(&vec)))    // 30 (removed from the end)

    vec.clear(&vec)
    return 0
}
```

**Methods:**
- `vec_new<T>()`: Creates a new empty `Vec<T>`
- `vec.at(&vec, index)`: Access element at the given index
- `vec.push(&vec, element)`: Add an element to the end
- `vec.pop(&vec)`: Remove and return the last element
- `vec.clear(&vec)`: Remove all elements

### Result Type (`result.ah`)

A generic tagged-union style result for error handling, built from an `enum` tag, a `union` payload, and a `struct`:

```al
enum ResultStatus { Ok, Err }

union ResultValue<T, E> {
    ok: T,
    err: E
}

struct Result<T, E> {
    result: ResultStatus,
    value: ResultValue<T, E>
}
```

**Usage:**
```al
$import "io.ah"
$import "convert.ah"
$import "string.ah"
$import "result.ah"

fun auth(password: string): Result<int, string> {
    if memcmp(password, "123456", 6) == 0 {
        return Result<int, string> {
            result: ResultStatus.Ok,
            value: ResultValue<int, string> {
                ok: 114514
            }
        }
    }
    return Result<int, string> {
        result: ResultStatus.Err,
        value: ResultValue<int, string> {
            err: "Wrong password!"
        }
    }
}

fun main(): int {
    let r = auth("123456")
    if r.result == ResultStatus.Ok {
        println(itoa(r.value.ok))  // 114514
    }
    return 0
}
```

Because enum members are also referable bare when unambiguous, `ResultStatus.Ok` can be written as `Ok` (and `Err`) as long as no other enum in the program defines those names.

### Maybe Type (`maybe.ah`)

A generic optional value using a tag + payload struct:

```al
enum MaybeTag { Nothing, Just }

struct Maybe<T> {
    tag: MaybeTag,
    value: T
}

fun is_some<T>(m: Maybe<T>): int  // 1 if Just, 0 if Nothing
```

**Usage:**
```al
$import "io.ah"
$import "convert.ah"
$import "maybe.ah"

fun main(): int {
    let a = Maybe<int> {
        tag: Just,
        value: 42
    }
    if is_some(a) {
        println(itoa(a.value))  // 42
    }
    return 0
}
```

Note: as a plain struct, `Maybe<T>` always stores a `value` of type `T` — the `Nothing` case still occupies space and requires a dummy value.

### Main Library (`lib.ah`)

The main library module provides system call access.

```al
fun(extern) syscall(int, int, int, int): int
fun(extern) exit(int): void
```

## Global Variables

Alum supports global mutable variables and exported constants, which are linkable across translation units.

| Syntax | Meaning |
| --- | --- |
| `cst NAME: T = expr` | Compile-time constant (inlined, no runtime symbol) |
| `cst(pub) NAME: T = expr` | Constant that also exports a read-only data symbol |
| `mut NAME: T [= expr]` | Global mutable variable (internal linkage, zero-initialized if no init) |
| `mut(pub) NAME: T [= expr]` | Global mutable variable (external linkage) |
| `extern NAME: T` | Reference a variable defined in another file or in C |

```al
cst(pub) LIMIT: int = 100
mut(pub) counter: int = 0

fun main(): int {
    counter = counter + 1
    return counter
}
```

A `mut(pub)`/`cst(pub)` definition can be consumed from another `.al` file with `extern NAME: T`, and from C as a plain global symbol. Initializers must be compile-time constants; only `int`/`float`/`bool` globals are currently supported.

## Building from Source

```bash
cd alum-std
cargo build --release
```

The compiled standard library will be available at `target/release/libalum_std.a`.
