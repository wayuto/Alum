# Alum Standard Library

The Alum standard library provides essential functionality for I/O, math, strings, arrays, memory, and type conversion operations.

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
$import "math.ah"
$import "string.ah"
$import "memory.ah"
$import "convert.ah"
```

### I/O Module (`io.al`)

Provides input/output operations.

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

Provides mathematical operations.

```al
extern abs(int): int        // Absolute value
extern sqrt(int): int       // Integer square root
extern max(int, int): int   // Maximum of two numbers
extern min(int, int): int   // Minimum of two numbers
extern pow(int, int): int   // Power function
extern fact(int): int       // Factorial
```

### Memory Module (`memory.al`)

Provides memory management functions using a free-list allocator with block headers.

```al
extern malloc(int): *void  // Allocate memory, returns byte pointer
extern free(*void): void   // Free memory (no size needed)
```

### Convert Module (`convert.al`)

Provides type conversion functions.

```al
extern itoa(int): string    // Integer to string
extern atoi(string): int    // String to integer
extern atof(string): float  // String to float
extern ftoa(float): string  // Float to string
```

### Main Library (`lib.al`)

The main library module provides system call access.

```al
extern syscall(int, int, int, int): int
extern exit(int): void
```

## Building from Source

```bash
cd alum-std
cargo build --release
```

The compiled standard library will be available at `target/release/libalum_std.a`.