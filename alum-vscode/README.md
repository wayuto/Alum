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

## Installation

Install from the VS Code Marketplace or build locally:

```bash
cd alum-vscode
npm install
npx vsce package
```

The resulting `.vsix` file can be installed via:

```bash
code --install-alum-vscode-0.9.6.vsix
```

## Building

```bash
npm install
npx vsce package
```

This generates `alum-vscode-0.9.6.vsix` in the project root.
