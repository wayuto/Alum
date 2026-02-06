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

| Field    | Type        | Optionality | Description                                |
| -------- | ----------- | ----------- | ------------------------------------------ |
| linker   | String      | required    | linking `target/objects/*.o` to executable |
| cc       | String      | optional    | C compiler (for C/Alum mixed projects)     |
| alc      | String      | optional    | Alum compiler                              |
| cflags   | String      | optional    | compilation parameters for C compiler      |
| alflags  | String      | optional    | compilation parameters for Alum compiler   |
| lnflags  | String      | optional    | compilation parameters for linker          |
| includes | Vec<String> | optional    | add `-I./path/to/include` when compiling   |
| nostdlib | bool        | optional    | don't link with Alum standard library      |

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
extern c_add(int, int): int
extern c_print_int(int): void

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