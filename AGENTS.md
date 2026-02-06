# Alum Programming Language - Agent Context

## 项目概述

Alum 是一个现代的系统编程语言，专为简洁性和高性能而设计。它具有清晰的语法、强静态类型系统，并使用 Cranelift 代码生成器编译为本地机器代码。

**核心特性：**
- 简洁语法：受现代语言启发的清晰、可读的语法
- 静态类型：具有显式类型注解的类型安全
- 原生编译：通过 Cranelift 直接编译为机器码
- 标准库：提供 I/O、数学、字符串、数组、内存和转换的综合标准库
- 预处理器：支持包含、定义和条件编译
- 快速编译：高效的编译管道

**技术栈：**
- 编译器语言：Rust (2024 edition)
- 代码生成器：Cranelift
- 目标平台：Linux x86_64
- 版本：0.7.0

## 项目结构

```
Alum/
├── src/                      # 编译器源代码
│   ├── main.rs               # 编译器入口点
│   ├── cli/                  # CLI 参数解析和命令
│   │   ├── args.rs           # 命令行参数定义
│   │   ├── build.rs          # 构建命令实现
│   │   ├── link.rs           # 链接器实现
│   │   └── mod.rs            # CLI 模块导出
│   └── compiler/             # 编译器组件
│       ├── lexer.rs          # 词法分析器
│       ├── parser.rs         # 解析器
│       ├── ast.rs            # AST 定义
│       ├── codegen.rs        # 代码生成
│       ├── preprocessor.rs   # 预处理器
│       └── mod.rs            # 编译器模块导出
├── alum-std/                 # 标准库
│   ├── alum/                 # 标准库头文件 (.al 文件)
│   │   ├── lib.al            # 主库模块
│   │   ├── io.al             # I/O 函数
│   │   ├── math.al           # 数学函数
│   │   ├── string.al         # 字符串函数
│   │   ├── array.al          # 数组函数
│   │   ├── memory.al         # 内存函数
│   │   └── convert.al        # 类型转换函数
│   └── src/                  # 标准库实现 (Rust no_std)
│       ├── lib.rs            # 库入口点和系统调用
│       ├── io.rs             # I/O 实现
│       ├── math.rs           # 数学实现
│       ├── string.rs         # 字符串实现
│       ├── array.rs          # 数组实现
│       ├── memory.rs         # 内存实现
│       └── convert.rs        # 转换实现
├── alum-vscode/              # VS Code 扩展
│   ├── syntaxes/
│   │   └── alum.tmLanguage.json  # 语法高亮
│   └── language-configuration.json
├── examples/                 # 示例程序
│   ├── 01_hello.al           # Hello World
│   ├── 02_variables.al       # 变量
│   ├── 03_functions.al       # 函数
│   ├── 04_control_flow.al    # 控制流
│   ├── 05_arrays.al          # 数组
│   ├── 06_math_operations.al # 数学运算
│   ├── 07_string_operations.al # 字符串运算
│   ├── 08_type_conversion.al # 类型转换
│   ├── 09_user_input.al      # 用户输入
│   ├── 10_loops_and_sum.al   # 循环和求和
│   ├── 11_fibonacci.al       # 斐波那契数列
│   ├── 12_prime_numbers.al   # 质数
│   ├── 13_array_search.al    # 数组搜索
│   ├── 14_array_sort.al      # 数组排序
│   └── 15_factorial_comparison.al # 阶乘比较
├── Cargo.toml                # 编译器依赖
├── install.sh                # 安装脚本
└── README.md                 # 项目文档
```

## 构建和运行

### 构建编译器

```bash
cargo build --release
```

### 构建标准库

```bash
cd alum-std
cargo build --release
```

### 完整安装

```bash
./install.sh
```

此脚本将：
1. 构建并安装 `alc` 编译器
2. 构建标准库
3. 将 `libalum_std.a` 安装到 `/usr/local/lib/`
4. 将标准库头文件安装到 `/usr/local/include/alum/`

### 编译 Alum 程序

```bash
# 编译为可执行文件
alc program.al -o program

# 仅编译（目标文件）
alc program.al -c -o program.o

# 链接目标文件
alc program.o -o program

# 编译并立即运行
alc -r program.al

# 仅预处理
alc -E program.al

# 包含自定义目录
alc program.al -I ./include

# 详细输出
alc -v program.al
```

## 语言特性

### 类型系统

Alum 是一个静态类型语言，具有显式类型注解。所有变量和函数必须在编译时声明其类型。

**支持的类型：**
- `int`: 有符号整数 (isize)
- `float`: 64 位浮点数 (f64)
- `bool`: 布尔值
- `string`: 字符串类型
- `void`: 无返回类型
- `arr[T]`: 类型 T 的数组

### 基本语法示例

```al
$import "io.al"
$import "convert.al"

fun main(): int {
    let x: int = 10;
    let y: int = 20;
    let sum: int = x + y;
    
    println(itoa(sum));
    return 0;
}
```

### 变量声明

```al
let name: string = "Alum";
let count: int = 42;
let pi: float = 3.14159;
let is_valid: bool = true;
let nothing: int = nil;
```

### 函数

```al
fun add(a: int, b: int): int {
    return a + b;
}

fun greet(name: string): void {
    println("Hello, ");
}
```

### 外部函数

```al
extern syscall(int, int, int, int): int
extern exit(int): void
```

### 控制流

```al
// If-Else
if x > 0 {
    println("Positive");
} else {
    println("Non-positive");
}

// While 循环
let i: int = 0;
while i < 10 {
    println(itoa(i));
    i = i + 1;
}

// For 循环
for i in 0..10 {
    println(itoa(i));
}
```

### 数组

```al
// 数组字面量
let numbers: arr[int] = [1, 2, 3, 4, 5];

// 使用填充语法的数组 [类型; 大小]
let buffer: arr[int] = [int; 100];

// 数组访问
let first: int = numbers[0];
numbers[1] = 10;
```

### 预处理器指令

```al
// 定义常量
$define PI 3.14159

// 条件编译
$ifndef ALUM_LIB
$define ALUM_LIB 1
$endif

// 导入模块
$import "io.al"
```

### 运算符

- **算术**: `+`, `-`, `*`, `/`
- **比较**: `==`, `!=`, `<`, `<=`, `>`, `>=`
- **逻辑**: `&&`, `||`, `!`
- **位运算**: `&`, `|`, `^`
- **范围**: `..`

## 标准库

### 导入模块

```al
$import "io.al"
$import "math.al"
$import "string.al"
$import "array.al"
$import "memory.al"
$import "convert.al"
```

### I/O 模块 (`io.al`)

```al
extern write(int, string, int): int    // 写入文件描述符
extern read(int, string, int): int     // 从文件描述符读取
extern print(string): int              // 打印字符串
extern println(string): int            // 打印字符串带换行
extern input(string): string           // 读取用户输入
extern fopen(string, int, int): int    // 打开文件
extern fclose(int): int                // 关闭文件
extern fread(int): string              // 从文件读取
extern fwrite(int, string, int): int   // 写入文件
extern lseek(int, int, int): int       // 文件定位
```

### 数学模块 (`math.al`)

```al
extern abs(int): int        // 绝对值
extern sqrt(int): int       // 平方（返回 x * x）
extern max(int, int): int   // 两个数的最大值
extern min(int, int): int   // 两个数的最小值
extern pow(int, int): int   // 幂函数
extern fact(int): int       // 阶乘
```

### 字符串模块 (`string.al`)

```al
extern strlen(string): int              // 字符串长度
extern strcpy(string, string): string   // 字符串复制
extern strcat(string, string): string   // 字符串连接
extern memcpy(string, string, int): string  // 内存复制
extern memset(string, int, int): string    // 内存设置
extern bcmp(string, string, int): int      // 字节比较
extern memcmp(string, string, int): int    // 内存比较
```

### 数组模块 (`array.al`)

```al
extern range(int, int): string  // 生成范围（返回指向数组的指针）
```

### 内存模块 (`memory.al`)

```al
extern malloc(int): string  // 分配内存（返回指针）
```

### 转换模块 (`convert.al`)

```al
extern itoa(int): string    // 整数转字符串
extern atoi(string): int    // 字符串转整数
extern atof(string): float  // 字符串转浮点数
extern ftoa(float): string  // 浮点数转字符串
```

## 编译管道

Alum 编译器遵循标准的编译管道：

1. **预处理**：处理 `$import`、`$define`、`$ifdef`、`$ifndef`、`$endif` 指令
2. **词法分析**：将源代码标记化为标记
3. **解析**：构建抽象语法树（AST）
4. **代码生成**：使用 Cranelift 将 AST 编译为机器代码
5. **链接**：将目标文件与标准库链接以创建可执行文件

## AST 结构

核心 AST 节点定义在 `src/compiler/ast.rs`：

```rust
pub struct Program {
    pub body: Vec<Expr>,
}

pub enum Type {
    Int,
    Float,
    Bool,
    String,
    Array(Box<Type>),
    Void,
}

pub enum Expr {
    Int(isize),
    Float(f64),
    Bool(bool),
    String(String),
    Nil,
    Add(Box<Expr>, Box<Expr>),
    Sub(Box<Expr>, Box<Expr>),
    Mul(Box<Expr>, Box<Expr>),
    Div(Box<Expr>, Box<Expr>),
    Var(String),
    VarDecl(String, Type, Box<Expr>),
    VarAssign(String, Box<Expr>),
    FuncDecl(String, Vec<(String, Type)>, Type, Box<Expr>),
    Extern(String, Vec<(String, Type)>, Type),
    Call(Box<Expr>, Vec<Expr>),
    Return(Box<Expr>),
    If(Box<Expr>, Box<Expr>, Option<Box<Expr>>),
    While(Box<Expr>, Box<Expr>),
    Stmt(Vec<Expr>),
    Index(Box<Expr>, Box<Expr>),
    ArrayLiteral(Vec<Expr>),
    ArrayFill(Type, Box<Expr>),
    For(String, Box<Expr>, Box<Expr>, Box<Expr>),
}
```

## 开发约定

### 代码风格

- 使用 Rust 2024 edition
- 使用 `clap` 库进行命令行参数解析
- 使用 `cranelift` 进行代码生成
- 标准库使用 `no_std` 模式
- 编译器和标准库都使用静态链接

### 编译器架构

- **词法分析器** (`lexer.rs`)：将源代码转换为标记流
- **解析器** (`parser.rs`)：将标记流转换为 AST
- **预处理器** (`preprocessor.rs`)：处理预处理指令
- **代码生成器** (`codegen.rs`)：将 AST 转换为机器代码
- **链接器** (`link.rs`)：链接目标文件和标准库

### 标准库架构

- 使用 `no_std` 模式，没有标准库支持
- 通过内联汇编实现系统调用
- 提供自定义的 panic 处理器
- 所有外部函数通过系统调用实现

### 测试

示例程序位于 `examples/` 目录，可以用作测试用例：
```bash
alc -r examples/01_hello.al
alc -r examples/11_fibonacci.al
```

## 依赖项

### 编译器依赖 (`Cargo.toml`)

- `clap` 4.5.54: 命令行参数解析
- `cranelift` 0.127.2: 代码生成后端
- `cranelift-module` 0.127.2: Cranelift 模块
- `cranelift-object` 0.127.2: Cranelift 对象文件生成
- `object` 0.36: 对象文件读写

### 标准库依赖 (`alum-std/Cargo.toml`)

标准库使用 `no_std` 模式，没有外部依赖（除了核心库）。

## CLI 参数

```
alc [OPTIONS] <INPUT>

参数:
  <INPUT>...    输入文件（.al 源文件或 .o/.obj 目标文件）

选项:
  -o, --output <FILE>       输出文件名
  -c, --compile-only        仅编译，不链接
  -r, --run                 编译并立即运行
  -E                        仅预处理；不编译、汇编或链接
  --ast                     输出 AST 表示
  -I <DIR>                  添加包含目录（可多次使用）
  --nostdlib                不与标准库链接
  -v, --verbose             详细输出
  -h, --help                打印帮助
  -V, --version             打印版本
```

## 关键注意事项

1. **类型系统**：Alum 是静态类型的，所有变量和函数必须显式声明类型
2. **内存管理**：标准库使用 `no_std` 模式，通过系统调用实现内存分配
3. **系统调用**：所有 I/O 操作通过内联汇编的系统调用实现
4. **链接**：默认链接标准库，可以使用 `--nostdlib` 禁用
5. **预处理**：支持类似 C 的预处理指令
6. **错误处理**：标准库使用 `panic = abort` 配置

## 常见任务

### 添加新的语言特性

1. 在 `src/compiler/ast.rs` 中添加新的 AST 节点
2. 在 `src/compiler/lexer.rs` 中添加词法规则
3. 在 `src/compiler/parser.rs` 中添加解析逻辑
4. 在 `src/compiler/codegen.rs` 中添加代码生成逻辑

### 添加标准库函数

1. 在 `alum-std/alum/` 中添加 `.al` 头文件声明
2. 在 `alum-std/src/` 中添加 Rust 实现
3. 在 `alum-std/alum/lib.al` 中导出新函数
4. 重新构建标准库

### 调试编译器

使用 `-v` 标志启用详细输出：
```bash
alc -v program.al
```

使用 `--ast` 标志查看 AST：
```bash
alc --ast program.al
```

### 创建新示例

在 `examples/` 目录中创建新的 `.al` 文件，遵循现有示例的命名约定（`NN_name.al`）。