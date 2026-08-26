# **The Alum Programming Language**

Alum is a lightweight, efficient programming language implemented in **Rust**. This VS Code extension provides syntax highlighting and language support for Alum.

## Features

- Syntax highlighting for `.al` and `.ah` files
- Language configuration (brackets, indentation)
- Support for all Alum language features including:
  - Generics (parametric polymorphism)
  - Pointer arithmetic and indexing
  - String indexing
  - Generic containers (`Vec<T>`)
  - Lambda functions
  - Preprocessor directives
  - Structs, unions and enums (including generic declarations and literals)
  - F-strings with interpolation
  - Increment/decrement operators (`++`, `--`)
  - Bitwise operators (`^`, `|`, `&`, `<<`, `>>`, `~`) and compound assignments (`+=`, `-=`, `*=`, `/=`, `%=`, `&=`, `|=`, `^=`, `<<=`, `>>=`)
  - Type casts (`42@float`)

## Installation

Install from the VS Code Marketplace or build locally:

```bash
cd alum-vscode
npm install
npx vsce package
```

The resulting `.vsix` file can be installed via:

```bash
code --install-extension alum-vscode-0.9.8.vsix
```

## Building

```bash
npm install
npx vsce package
```
