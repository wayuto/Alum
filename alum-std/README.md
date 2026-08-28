# Alum Standard Library

The Alum standard library provides essential functionality for I/O, math, strings, arrays, memory, type conversion, and higher-level containers (`Vec`, `Result`, `Maybe`).

## Installation

The standard library is automatically installed when running the main installation script:

```bash
./install.sh
```

This installs:
- `libalum_std.a` to `/usr/local/lib/`
- Standard library modules to `/usr/local/include/alum/`

## Modules

### Importing Modules

```al
import io
import string
import math
import memory
import convert
import vec
import result
import maybe
import process
import time
import fs
import sys
import lib
```

`import` loads a module (`io.al`) and makes its declarations available.
Qualified references use the `mod::name` syntax; to call a module's
declarations without the prefix, import selected names with `using`:

```al
import io
using io::{write, read, print, println, input, fopen, fclose, fread, fwrite, lseek, pipe, pipe2, dup, dup2, dup3}
```

Modules can be aliased (`import io as i`) and names aliased
(`using io::println as p`).

Visibility: only declarations marked `pub` (`fun(pub)`, `struct(pub)`,
`union(pub)`, `enum(pub)`, `cst(pub)`, `var(pub)`) are importable from
outside the module — a private member referenced via `mod::name` or `using`
is a compile error. Extern declarations (`fun(extern)`, `extern var`) are
always visible: they bind foreign symbols and are never renamed internally;
their link name is the C ABI symbol.

### I/O Module (`io`)

Provides input/output, file operations, and file descriptor manipulation.

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

fun(extern) pipe(*void): int         // Create pipe, pipefd[0]=read [1]=write
fun(extern) pipe2(*void, int): int   // Create pipe with flags (O_CLOEXEC=0x80000)
fun(extern) dup(int): int            // Duplicate fd (lowest available)
fun(extern) dup2(int, int): int      // Duplicate oldfd onto newfd
fun(extern) dup3(int, int, int): int // dup2 with flags
```

### String Module (`string`)

Provides string (byte array) operations. All functions are implemented in Alum;
the comparison functions are `pure` and can participate in compile-time
evaluation.

| Signature | Description |
| --- | --- |
| `fun(pure) strlen(string): int` | String length |
| `fun(pure) strcmp(string, string): int` | Lexicographic comparison |
| `fun(pure) bcmp(string, string, int): int` | Byte comparison |
| `fun(pure) memcmp(string, string, int): int` | Byte comparison (n bytes) |
| `fun strcpy(string, string): string` | Copy string |
| `fun strcat(string, string): string` | Concatenate strings |
| `fun memcpy(string, string, int): string` | Copy n bytes |
| `fun memset(string, int, int): string` | Fill n bytes with a value |

### Math Module (`math`)

Provides mathematical operations.

```al
fun(pure,extern) abs(int): int        // Absolute value
fun(pure,extern) sqrt(int): int       // Integer square root
fun(pure,extern) max(int, int): int   // Maximum of two numbers
fun(pure,extern) min(int, int): int   // Minimum of two numbers
fun(pure,extern) pow(int, int): int   // Power function
fun(pure,extern) fact(int): int       // Factorial
```

### Memory Module (`memory`)

Provides memory management functions using a free-list allocator with block headers, plus the raw memory syscalls.

```al
fun(extern) malloc(int): *void  // Allocate memory, returns byte pointer
fun(extern) free(*void): void   // Free memory (no size needed)

fun(extern) mmap(*void, int, int, int, int, int): *void  // Map memory, -1 = MAP_FAILED
fun(extern) munmap(*void, int): int  // Unmap memory
fun(extern) mprotect(*void, int, int): int  // Change memory protection
fun(extern) brk(int): int            // Adjust program break
```

### Process Module (`process`)

Process identity, scheduling, and control syscalls. All return the raw kernel
value, negative = `-errno`. Signals: `SIGINT=2 SIGKILL=9 SIGTERM=15 SIGCHLD=17`.

```al
fun(extern) getpid(): int            // Process ID
fun(extern) getppid(): int           // Parent process ID
fun(extern) getuid(): int            // User ID
fun(extern) geteuid(): int           // Effective user ID
fun(extern) getgid(): int            // Group ID
fun(extern) getegid(): int           // Effective group ID
fun(extern) sched_yield(): int       // Yield the CPU

fun(extern) fork(): int              // 0 in child, child PID in parent
fun(extern) execve(string, *void, *void): int  // Replace process image (path, argv, envp)
fun(extern) wait4(int, *int, int, *void): int // Wait for child (pid, &status, options, rusage*)
fun(extern) kill(int, int): int      // Send signal to process
fun(extern) exit_group(int): void    // Terminate whole process
```

### Time Module (`time`)

Time syscalls using `Timespec`/`Timeval` structs (matching the kernel ABI:
two 64-bit fields, no padding).

```al
struct Timespec {
    sec: int,
    nsec: int
}

struct Timeval {
    sec: int,
    usec: int
}

fun(extern) nanosleep(*Timespec, *Timespec): int    // Sleep, rem may be nil
fun(extern) clock_gettime(int, *Timespec): int      // Realtime(0)/monotonic(1) clock
fun(extern) gettimeofday(*Timeval, *void): int      // Wall-clock time, tz may be nil
```

### File System Module (`fs`)

File system syscalls. All return the raw kernel value, negative = `-errno`.
Open flags (subset): `O_RDONLY=0 O_WRONLY=1 O_RDWR=2 O_CREAT=0x40 O_TRUNC=0x200`.
(`read`/`write`/`lseek` live in `io`.)

```al
fun(extern) open(string, int, int): int  // Open file, returns fd or -errno
fun(extern) close(int): int              // Close file descriptor
fun(extern) access(string, int): int     // Check file permissions
fun(extern) mkdir(string, int): int      // Create directory
fun(extern) rmdir(string): int           // Remove directory
fun(extern) unlink(string): int          // Remove file
fun(extern) fsync(int): int              // Flush fd to disk
fun(extern) ftruncate(int, int): int     // Truncate file to length
fun(extern) getcwd(string, int): int     // Current working directory
fun(extern) chdir(string): int           // Change directory
```

### Syscall Module (`sys`)

Raw syscall entry points and miscellaneous syscalls. All return the raw kernel value;
on error it is negative (`-errno`).

```al
fun(extern) syscall(int, int, int, int): int                 // Raw 3-arg syscall
fun(extern) syscall6(int, int, int, int, int, int, int): int // Raw 6-arg syscall (mmap)

fun(extern) getrandom(string, int, int): int  // Fill buffer with random bytes
fun(extern) uname(*void): int                 // System info (struct utsname)
```

### Convert Module (`convert`)

Provides type conversion functions.

| Signature | Description |
| --- | --- |
| `fun itoa(int): string` | Integer to string |
| `fun(pure) atoi(string): int` | String to integer |
| `fun(extern) atof(string): float` | String to float (C library) |
| `fun(extern) ftoa(float): string` | Float to string (C library) |

### Vec Container (`vec`)

A generic dynamic array. `Vec<T>` is monomorphically instantiated per element type; methods are stored as function-pointer fields and called with `&vec` as the first argument. Indexing (`vec[i]`) and `next` return a `Maybe<T>` so out-of-range access is safe.

```al
import vec
using vec::{Vec, vec_new}
import maybe
using maybe::{MaybeTag, Maybe, is_some}
import convert
using convert::{itoa, atoi, atof, ftoa}
import io
using io::{write, read, print, println, input, fopen, fclose, fread, fwrite, lseek, pipe, pipe2, dup, dup2, dup3}

fun main(): int {
    var vec: Vec<int> = vec_new()

    vec.push(&vec, 10)
    vec.push(&vec, 20)
    vec.push(&vec, 30)

    var first: Maybe<int> = vec.nth(&vec, 0)
    if first.tag == Just {
        println(itoa(first.value))  // 10
    }
    var last: Maybe<int> = vec.pop(&vec)
    if last.tag == Just {
        println(itoa(last.value))  // 30 (removed from the end)
    }

    vec.clear(&vec)
    return 0
}
```

**Methods:**
- `vec_new<T>()`: Creates a new empty `Vec<T>`
- `vec.nth(&vec, index): Maybe<T>`: Access element at the given index (error-safe)
- `vec.push(&vec, element): void`: Add an element to the end
- `vec.pop(&vec): Maybe<T>`: Remove and return the last element
- `vec.next(&vec): Maybe<T>`: Iterate (resets after the last element)
- `vec.clear(&vec): void`: Remove all elements

### Result Type (`result`)

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
import result
using result::{ResultStatus, ResultValue, Result}
import io
using io::{write, read, print, println, input, fopen, fclose, fread, fwrite, lseek, pipe, pipe2, dup, dup2, dup3}
import string
using string::{strlen, strcpy, strcat, memcpy, memset, bcmp, memcmp}
import convert
using convert::{itoa, atoi, atof, ftoa}

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
    var r = auth("123456")
    if r.result == ResultStatus.Ok {
        println(itoa(r.value.ok))  // 114514
    }
    return 0
}
```

Because enum members are also referable bare when unambiguous, `ResultStatus.Ok` can be written as `Ok` (and `Err`) as long as no other enum in the program defines those names.

### Maybe Type (`maybe`)

A generic optional value using a tag + payload struct:

```al
enum MaybeTag { Nothing, Just }

struct Maybe<T> {
    tag: MaybeTag,
    value: T
}

fun(pure) is_some<T>(m: Maybe<T>): int {
    if m.tag == Just {
        return 1
    }
    return 0
}
```

**Usage:**
```al
import maybe
using maybe::{MaybeTag, Maybe, is_some}
import io
using io::{write, read, print, println, input, fopen, fclose, fread, fwrite, lseek, pipe, pipe2, dup, dup2, dup3}
import convert
using convert::{itoa, atoi, atof, ftoa}

fun main(): int {
    var a = Maybe<int> {
        tag: Just,
        value: 42
    }
    if a.tag == Just {
        println(itoa(a.value))  // 42
    }
    return 0
}
```

Note: as a plain struct, `Maybe<T>` always stores a `value` of type `T` — the `Nothing` case still occupies space and requires a dummy value.

### Main Library (`lib`)

The main library module provides system call access.

```al
fun(extern) syscall(int, int, int, int): int
fun(extern) exit(int): void
```

### Function Annotations

Alum supports function annotations for controlling linkage and optimization:

| Annotation | Meaning |
| --- | --- |
| `pub` | Export the symbol and make it importable from other modules |
| `pure` | Mark the function as side-effect-free (enables optimization) |
| `extern` | Declare an external function (no body, linked at compile time) |

Annotations can be combined, e.g. `fun(pure,pub) name(params): ret`.

`struct(pub)`, `union(pub)`, `enum(pub)`, `cst(pub)` and `var(pub)` follow
the same rule: only `pub` type/constant/global declarations of a module can
be imported by other files.

## Global Variables

Alum supports global mutable variables and exported constants, which are linkable across translation units.

| Syntax | Meaning |
| --- | --- |
| `cst NAME: T = expr` | Compile-time constant (inlined, no runtime symbol) |
| `cst(pub) NAME: T = expr` | Constant that also exports a read-only data symbol |
| `var NAME: T [= expr]` | Global mutable variable (internal linkage, zero-initialized if no init) |
| `var(pub) NAME: T [= expr]` | Global mutable variable (external linkage) |
| `extern NAME: T` | Reference a variable defined in another file or in C |

```al
cst(pub) LIMIT: int = 100
var(pub) counter: int = 0

fun main(): int {
    counter = counter + 1
    return counter
}
```

A `var(pub)`/`cst(pub)` definition can be consumed from another `.al` file with `extern NAME: T`, and from C as a plain global symbol. Initializers must be compile-time constants; only `int`/`float`/`bool` globals are currently supported.

## Building from Source

```bash
cd alum-std
cargo build --release
```

The compiled standard library will be available at `target/release/libalum_std.a`.
