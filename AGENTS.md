# Alum Programming Language - 项目上下文

## 项目概述

Alum 是一个现代系统编程语言，专注于简洁性和性能。它具有清晰的语法、强静态类型，并使用 Cranelift 代码生成器编译为原生机器代码。

**核心特性：**
- 简洁的语法，受现代语言启发
- 静态类型，需要显式类型注解
- 通过 Cranelift 进行原生编译
- 快速编译管道
- FFI 支持，可与 C 语言互操作
- 集成构建系统 (almk)
- 内置优化：常量折叠、死代码消除、未使用代码移除
- Lambda 函数和闭包支持
- 带有参数替换的强大宏系统

**版本：** 0.8.0
**当前分支：** dev
**主要语言：** Rust (编译器) + Alum (用户代码)

## 项目结构

```
Alum/
├── src/                      # 编译器源代码 (Rust)
│   ├── main.rs               # 编译器入口点
│   ├── cli/                  # CLI 参数解析和命令
│   │   ├── args.rs           # CLI 参数定义
│   │   ├── build.rs          # 编译命令
│   │   ├── link.rs           # 链接命令
│   │   └── mod.rs
│   └── compiler/             # 编译器组件
│       ├── ast.rs            # 抽象语法树定义
│       ├── checker.rs        # 类型检查器
│       ├── codegen.rs        # 代码生成器 (Cranelift)
│       ├── lexer.rs          # 词法分析器
│       ├── parser.rs         # 语法分析器
│       ├── preprocessor.rs   # 预处理器 (宏、导入等)
│       └── mod.rs
├── alum-std/                 # 标准库
│   ├── alum/                 # 标准库头文件 (.al 文件)
│   │   ├── lib.al            # 主库入口 (系统调用)
│   │   ├── io.al             # I/O 操作
│   │   ├── math.al           # 数学运算
│   │   ├── string.al         # 字符串操作
│   │   ├── memory.al         # 内存管理
│   │   └── convert.al        # 类型转换
│   └── src/                  # 标准库实现 (Rust no_std)
│       ├── lib.rs
│       ├── io.rs
│       ├── math.rs
│       ├── string.rs
│       ├── memory.rs
│       └── convert.rs
├── alum-make/                # 构建工具 (almk)
│   ├── src/
│   │   ├── main.rs           # 构建工具入口
│   │   ├── command.rs        # 命令处理
│   │   ├── config.rs         # 配置解析
│   │   ├── dependencies.rs   # 依赖管理
│   │   ├── new.rs            # 新项目创建
│   │   ├── sync.rs           # 依赖同步
│   │   └── build.rs          # 构建逻辑
├── alum-vscode/              # VS Code 扩展
│   ├── syntaxes/
│   │   └── alum.tmLanguage.json  # 语法高亮定义
│   └── language-configuration.json
├── examples/                 # 示例程序 (23 个示例)
│   ├── 01_hello.al           # Hello World
│   ├── 02_variables.al       # 变量声明
│   ├── 03_functions.al       # 函数定义
│   ├── 04_control_flow.al    # 控制流
│   ├── 05_arrays.al          # 数组操作
│   ├── 06_math_operations.al # 数学运算
│   ├── 07_string_operations.al # 字符串操作
│   ├── 08_type_conversion.al # 类型转换
│   ├── 09_user_input.al      # 用户输入
│   ├── 10_loops_and_sum.al   # 循环
│   ├── 11_fibonacci.al       # 斐波那契数列
│   ├── 12_array_search.al    # 数组搜索
│   ├── 13_array_sort.al      # 数组排序
│   ├── 14_factorial_comparison.al # 阶乘比较
│   ├── 16_break.al           # break 语句
│   ├── 17_continue.al        # continue 语句
│   ├── 18_typedef.al         # 类型定义
│   ├── 20_cmdline_args.al    # 命令行参数
│   ├── 21_struct.al          # 结构体
│   ├── 22_function_pointer.al # 函数指针
│   ├── 23_lambda.al          # Lambda 函数
│   ├── 15_mixed_c_alum/      # C/Alum 混合项目示例
│   └── 19_c_array_compatibility/ # C 数组兼容性示例
├── Cargo.toml                # 编译器依赖配置
├── install.sh                # 安装脚本
├── LICENSE                   # 许可证文件
└── README.md                 # 项目文档
```

## 构建和运行

### 环境要求
- Rust 工具链 (2024 edition)
- Cargo >= 1.93.0

### 安装

```bash
# 克隆仓库
git clone https://github.com/wayuto/Alum.git
cd Alum

# 运行安装脚本
./install.sh
```

安装脚本会：
1. 构建并安装 `alc` 编译器
2. 构建标准库
3. 安装 `libalum_std.a` 到 `/usr/local/lib/`
4. 安装标准库头文件到 `/usr/local/include/alum/`
5. 安装构建工具 `almk`

### 构建编译器

```bash
# 开发版本
cargo build

# 发布版本
cargo build --release
```

### 构建标准库

```bash
cd alum-std
cargo build --release
```

标准库输出：`target/release/libalum_std.a`

### 构建构建工具 (almk)

```bash
cd alum-make
cargo build --release

# 或直接安装
cargo install --path .
```

### 使用编译器

```bash
# 编译为可执行文件
alc program.al -o program

# 仅编译为目标文件
alc program.al -c -o program.o

# 链接目标文件
alc program.o -o program

# 编译并立即运行
alc -r program.al

# 预处理模式
alc program.al -E

# 输出 AST
alc program.al --ast

# 添加包含目录
alc program.al -I ./include

# 不链接标准库
alc program.al --nostdlib

# 详细输出
alc program.al -v
```

### 使用构建工具 (almk)

```bash
# 创建新项目
almk new hello

# 构建项目
almk build

# 运行项目
almk run

# 清理构建产物
almk clean

# 添加依赖
almk add util -u https://www.website.com/util.zip

# 移除依赖
almk rm util
```

## 语言语法

### 基本类型
- `int` - 有符号整数 (isize)
- `float` - 64位浮点数 (f64)
- `bool` - 布尔值
- `string` - 字符串类型
- `void` - 无返回类型
- `arr[T]` - T 类型的数组

### 变量声明

```al
let name: string = "Alum"
let count: int = 42
let pi: float = 3.14159
let is_valid: bool = true
```

### 函数定义

```al
fun add(a: int, b: int): int {
    return a + b
}

fun main(): int {
    let result: int = add(10, 20)
    return 0
}
```

### 外部函数 (FFI)

```al
extern c_add(int, int): int
extern printf(string): int
```

### 控制流

```al
// If-Else
if x > 0 {
    println("Positive")
} else {
    println("Non-positive")
}

// While Loop
while i < 10 {
    i = i + 1
}

// For Loop
for i in 0..10 {
    println(itoa(i))
}

// Break
while true {
    if condition {
        break
    }
}

// Continue
for i in 0..10 {
    if i % 2 == 0 {
        continue
    }
    println(itoa(i))
}
```

### 数组

```al
// 数组字面量
let numbers: arr[int] = [1, 2, 3, 4, 5]

// 填充数组
let buffer: arr[int] = [int; 100]

// 数组访问
let first: int = numbers[0]
numbers[0] = 10
```

### 结构体

```al
struct Point {
    x: int,
    y: int
}

fun main(): int {
    let p: Point = Point {
        x: 10,
        y: 20
    }
    println(itoa(p.x))
    println(itoa(p.y))
    return 0
}
```

### Lambda 函数

```al
fun apply_function(f: fun(int): int, value: int): int {
    return f(value)
}

fun main(): int {
    // 定义 lambda
    let square: fun(int): int = lamb(x: int): int {
        return x * x
    }

    let result: int = apply_function(square, 5)
    println(itoa(result))  // 输出: 25

    return 0
}
```

### 预处理器指令

```al
// 简单宏（无参数）
$define PI 3.14159
$define HELLO "Hello, World!"

// 带参数的宏
$define ADD(a, b) a + b
$define MAX(a, b) if a > b { a } else { b }

// 条件编译
$ifndef ALUM_LIB
$define ALUM_LIB 1
$endif

// 导入模块
$import "io.al"
$import "math.al"
```

### 类型转换

```al
$import "convert.al"

let num: int = 42
let str: string = itoa(num)  // int -> string
let parsed: int = atoi("123") // string -> int
```

## 编译流程

```
源代码 (.al)
    │
    ▼
┌───────────────┐
│ 预处理器      │ → 处理 $import, $define, $ifdef, $ifndef, $endif
└───────────────┘
    │
    ▼
┌───────────────┐
│ 词法分析器    │ → 将源代码标记化为 token
└───────────────┘
    │
    ▼
┌───────────────┐
│ 语法分析器    │ → 构建抽象语法树 (AST)
└───────────────┘
    │
    ▼
┌───────────────┐
│ 类型检查器    │ → 验证类型安全和语义规则
└───────────────┘
    │
    ▼
┌───────────────┐
│ 优化器        │ → 常量折叠、死代码消除、未使用代码移除
└───────────────┘
    │
    ▼
┌───────────────┐
│ 代码生成器    │ → 使用 Cranelift 将 AST 编译为机器码
└───────────────┘
    │
    ▼
  目标文件 (.o)
    │
    ▼
┌───────────────┐
│ 链接器        │ → 将目标文件与标准库链接
└───────────────┘
    │
    ▼
  可执行文件
```

### 编译流程阶段

1. **预处理**：处理 `$import`、`$define`、`$ifdef`、`$ifndef`、`$endif` 指令和宏展开
2. **词法分析**：将源代码标记化为 token 流
3. **语法分析**：从 token 构建抽象语法树 (AST)
4. **类型检查**：验证类型安全和语义规则
5. **优化**：执行常量折叠、死代码消除和未使用代码移除
6. **代码生成**：使用 Cranelift 将 AST 编译为机器码
7. **链接**：将目标文件与标准库链接以生成可执行文件

## 标准库模块

### I/O 模块 (io.al)
```al
extern write(int, string, int): int    // 写入文件描述符
extern read(int, string, int): int     // 从文件描述符读取
extern print(string): int              // 打印字符串
extern println(string): int            // 打印字符串并换行
extern input(string): string           // 读取用户输入
extern fopen(string, int, int): int    // 打开文件
extern fclose(int): int                // 关闭文件
extern fread(int): string              // 从文件读取
extern fwrite(int, string, int): int   // 写入文件
extern lseek(int, int, int): int       // 文件定位
```

### 数学模块 (math.al)
```al
extern abs(int): int        // 绝对值
extern sqrt(int): int       // 平方 (返回 x * x)
extern max(int, int): int   // 两个数的最大值
extern min(int, int): int   // 两个数的最小值
extern pow(int, int): int   // 幂函数
extern fact(int): int       // 阶乘
```

### 字符串模块 (string.al)
```al
extern strlen(string): int              // 字符串长度
extern strcpy(string, string): string   // 字符串复制
extern strcat(string, string): string   // 字符串连接
extern memcpy(string, string, int): string  // 内存复制
extern memset(string, int, int): string    // 内存设置
extern bcmp(string, string, int): int      // 字节比较
extern memcmp(string, string, int): int    // 内存比较
```

### 内存模块 (memory.al)
```al
extern malloc(int): string  // 分配内存（返回指针）
```

### 转换模块 (convert.al)
```al
extern itoa(int): string    // 整数转字符串
extern atoi(string): int    // 字符串转整数
extern atof(string): float  // 字符串转浮点数
extern ftoa(float): string  // 浮点数转字符串
```

### 主库 (lib.al)
```al
extern syscall(int, int, int, int): int
extern exit(int): void
```

## 构建工具配置 (Alumake.toml)

### 基本配置
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

### C/Alum 混合项目
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

### 依赖配置

#### ZIP 文件依赖
```toml
[dependencies.dep]
url = "https://www.website.com/dep.zip"
git = false
```

#### Git 仓库依赖
```toml
[dependencies.dep]
url = "https://www.website.com/dep.git"
git = true
tag = "v1.0"
```

#### 本地依赖
```toml
[dependencies.dep]
local = "/path/to/dep"
git = false
```

## 开发约定

### 编码风格
- 使用 4 空格缩进
- 类型注解是必需的（静态类型语言）
- 函数参数和返回值必须明确指定类型
- 使用 `let` 声明变量
- 使用 `fun` 定义函数
- 使用 `extern` 声明外部函数（FFI）

### Rust 编译器代码规范
- 使用 Rust 2024 edition
- 使用 `clap` 进行 CLI 参数解析
- 使用 `cranelift` 进行代码生成
- 编译器组件模块化：lexer, parser, checker, codegen
- 错误处理使用 `Result<T, E>` 模式

### 测试
- 示例程序位于 `examples/` 目录
- 使用示例程序验证语言特性
- 标准库测试通过编译示例程序完成

### 提交规范
- 使用清晰的提交消息
- 提交消息应简洁并描述变更原因
- 参考 `git log` 查看历史提交风格

## 重要文件位置

- **编译器入口**: `src/main.rs`
- **AST 定义**: `src/compiler/ast.rs`
- **CLI 参数**: `src/cli/args.rs`
- **类型检查**: `src/compiler/checker.rs`
- **代码生成**: `src/compiler/codegen.rs`
- **标准库头文件**: `alum-std/alum/`
- **标准库实现**: `alum-std/src/`
- **构建工具**: `alum-make/src/`
- **示例程序**: `examples/`
- **安装脚本**: `install.sh`

## 常见任务

### 添加新的语言特性
1. 更新 AST 定义 (`src/compiler/ast.rs`)
2. 更新词法分析器 (`src/compiler/lexer.rs`)
3. 更新语法分析器 (`src/compiler/parser.rs`)
4. 更新类型检查器 (`src/compiler/checker.rs`)
5. 更新代码生成器 (`src/compiler/codegen.rs`)
6. 添加示例程序到 `examples/`
7. 更新文档

### 修复编译器 bug
1. 在 `examples/` 中创建最小复现示例
2. 使用 `alc --ast` 和 `alc -v` 调试
3. 定位问题所在的编译器组件
4. 修复代码
5. 验证修复

### 扩展标准库
1. 在 `alum-std/alum/` 中添加 `.al` 头文件声明
2. 在 `alum-std/src/` 中实现 Rust 代码
3. 重新构建标准库
4. 安装更新后的标准库

### VS Code 支持
- 语法高亮定义：`alum-vscode/syntaxes/alum.tmLanguage.json`
- 语言配置：`alum-vscode/language-configuration.json`

## 注意事项

1. **权限要求**：安装脚本需要 sudo 权限来复制文件到 `/usr/local/lib/` 和 `/usr/local/include/`
2. **标准库路径**：标准库默认安装在 `/usr/local/lib/libalum_std.a`
3. **头文件路径**：标准库头文件默认安装在 `/usr/local/include/alum/`
4. **FFI 使用**：在混合 C/Alum 项目中，记得设置 `nostdlib = true` 以避免 `_start` 符号冲突
5. **类型安全**：Alum 是强类型语言，所有变量和函数必须显式声明类型
6. **宏系统**：支持简单宏和带参数的宏，宏在预处理阶段展开
7. **Lambda 语法**：Lambda 使用 `lamb(params): return_type { body }` 语法

## 相关资源

- **GitHub 仓库**: https://github.com/wayuto/Alum
- **许可证**: 见 LICENSE 文件
- **版本**: 0.8.0
- **当前分支**: dev
- **远程仓库**: origin/main

## 开发环境信息

- **操作系统**: Linux
- **Rust 版本**: 2024 edition
- **Cargo 版本**: >= 1.93.0
- **主要依赖**:
  - clap 4.5.54 (CLI)
  - cranelift 0.127.2 (代码生成)
  - object 0.36 (对象文件处理)
  - serde 1.0.22 (序列化，用于构建工具)
  - toml 0.9.11 (配置解析，用于构建工具)
  - walkdir 2.5.0 (文件遍历，用于构建工具)