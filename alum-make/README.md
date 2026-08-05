# Almk
An Alum build tool

## Installation

### Dependencies

| Dependency    | Version   |
| ------------- | --------- |
| Rust && Cargo | >= 1.93.0 |
| clap          | >= 4.5.54 |
| serde         | >= 1.0.22 |
| toml          | >= 0.9.11 |
| walkdir       | >= 2.5.0  |

### Install from source
```bash
$ cargo install --path .
```

## Quick Start

### Create a new project
```bash
$ almk new hello
'hello' has been created successfully.
```

### Build the project
```bash
$ almk build
alc ./src/main.al -c -o target/objects/main.o
alc target/objects/main.o -o target/hello
```

### Run the project
```bash
$ almk run
Hello, World!
```

### Clean up
```bash
$ almk clean
Removed object files in 'target' directory.
```

### Add a dependency
```bash
$ almk add util -u https://www.website.com/util.zip
Added dependency 'util'
```

### Remove a dependency
```bash
$ almk rm util
```

## Features

- Automatically find and compile all Alum files in `src/`
- Manage projects with `Alumake.toml`
- Support for dependencies (local, git, or zip)

## Configuration

Projects are managed by `Alumake.toml`. A default `Alumake.toml` looks like:

```toml
[package]
name = "your project name"
version = "0.1.0"
author = ""
license = "No License"
language = "alum"

[build]
linker = "alc"
alc = "alc"
```

### Package

| Field    | Type   | Optionality |
| -------- | ------ | ----------- |
| name     | String | required    |
| version  | String | optional    |
| author   | String | optional    |
| license  | String | optional    |
| language | String | optional    |

### Build

| Field       | Type        | Optionality | Description                                |
| ----------- | ----------- | ----------- | ------------------------------------------ |
| linker      | String      | required    | linking `target/objects/*.o` to executable |
| cc          | String      | optional    | C compiler (for C/Alum mixed projects)     |
| alc         | String      | optional    | Alum compiler                              |
| cflags      | String      | optional    | compilation parameters for C compiler      |
| alflags     | String      | optional    | compilation parameters for Alum compiler   |
| lnflags     | String      | optional    | compilation parameters for linker          |
| includes    | Vec<String> | optional    | add `-I./path/to/include` when compiling   |
| nostdlib    | bool        | optional    | don't link with Alum standard library      |
| library_type| String      | optional    | library type: "static", "a", "shared", "so"|

### Library Types

The `library_type` field in the `[build]` section specifies the type of library to build. If omitted, almk will build an executable.

#### Static Library (`static` or `a`)

Builds a static library (`.a` file) that can be linked statically with other projects.

**Alumake.toml:**
```toml
[package]
name = "mylib"
version = "1.0.0"
language = "alum"

[build]
linker = "alc"
alc = "alc"
library_type = "static"
```

After running `almk build`, this will generate `target/libmylib.a`.

**Using the static library in another project:**
```al
// main.al
fun(extern) add(int, int): int

fun main(): int {
    let result: int = add(10, 20)
    return 0
}
```

Compile and link:
```bash
alc main.al -c -o main.o
cc main.o -L./target -lmylib -L/usr/local/lib -lalum -o main
```

#### Shared Library (`shared` or `so`)

Builds a shared library (`.so` file) that can be loaded at runtime.

**Alumake.toml:**
```toml
[package]
name = "mylib"
version = "1.0.0"
language = "alum"

[build]
linker = "alc"
alc = "alc"
library_type = "shared"
```

After running `almk build`, this will generate `target/libmylib.so`.

**Using the shared library:**
```bash
# Set library path
export LD_LIBRARY_PATH=./target:$LD_LIBRARY_PATH

# Compile and link your program
alc main.al -L./target -lmylib -o main
```

#### Executable (default)

If `library_type` is omitted or set to any value other than `"static"`, `"a"`, `"shared"`, or `"so"`, almk will build an executable file.

**Alumake.toml:**
```toml
[package]
name = "myapp"
version = "1.0.0"
language = "alum"

[build]
linker = "alc"
alc = "alc"
# library_type is omitted - builds executable
```

After running `almk build`, this will generate `target/myapp`.

### Mixed C/Alum Projects

Almk supports mixed C and Alum projects where Alum can call C functions via FFI.

**Alumake.toml configuration:**

```toml
[package]
name = "mixed_project"
version = "0.1.0"
author = "Your Name"
license = "MIT"
language = "mixed"

[build]
linker = "alc"
cc = "cc"
alc = "alc"
cflags = "-Wall -O2"
alflags = ""
includes = ["./include"]
nostdlib = true
```

**Project structure:**

```
mixed_project/
├── Alumake.toml
├── src/
│   ├── main.al      # Alum source files
│   └── helper.c     # C source files (optional)
└── include/         # Optional include directories
    └── helper.h
```

**Alum code calling C functions:**

```al
// src/main.al
fun(extern) c_add(int, int): int
fun(extern) c_print_int(int): void

fun main(): int {
    let result: int = c_add(10, 20);
    c_print_int(result);
    return 0;
}
```

**C helper code:**

```c
// src/helper.c
int c_add(int a, int b) {
    return a + b;
}

void c_print_int(int value) {
    printf("C: %d\n", value);
}
```

**Notes:**
- Set `nostdlib = true` in mixed projects to avoid `_start` symbol conflicts
- C files in `./include` directory will also be compiled
- Use `includes` to specify additional include paths for both C and Alum

### Dependencies

#### For `.zip` file:
```toml
[dependencies.dep]
url = "https://www.website.com/dep.zip"
git = false
```

#### For `git` repo:
```toml
[dependencies.dep]
url = "https://www.website.com/dep.git"
git = true
tag = "v1.0"
```

#### For `local` file:
```toml
[dependencies.dep]
local = "/path/to/dep"
git = false
```