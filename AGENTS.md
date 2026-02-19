# Alum 编程语言项目 - AI Agent 上下文

## 项目概述

Alum 是一个现代系统级编程语言，专为简洁性和高性能而设计。它具有清晰的语法、强静态类型系统，并使用 Cranelift 代码生成器编译为原生机器代码。

### 核心特性

- **简洁语法**：受现代语言启发的清晰、可读语法
- **静态类型**：带有显式类型注解的类型安全
- **原生编译**：通过 Cranelift 直接编译为机器代码
- **快速编译**：高效的编译流水线
- **FFI 支持**：与 C 的互操作性，用于底层操作
- **构建工具**：集成构建系统 (almk) 用于项目管理
- **优化器**：内置优化，包括常量折叠、死代码消除和未使用代码移除

### 项目结构

```
Alum/
├── src/                      # 编译器源代码
│   ├── main.rs               # 编译器入口点
│   ├── cli/                  # CLI 参数解析和命令
│   │   ├── args.rs           # 命令行参数定义
│   │   ├── build.rs          # 构建逻辑
│   │   ├── link.rs           # 链接逻辑
│   │   └── mod.rs            # CLI 模块
│   └── compiler/             # 编译器组件
│       ├── ast.rs            # 抽象语法树定义
│       ├── checker.rs        # 类型检查器
│       ├── codegen.rs        # 代码生成器 (Cranelift)
│       ├── lexer.rs          # 词法分析器
│       ├── optimizer.rs      # 优化器
│       ├── parser.rs         # 语法分析器
│       ├── preprocessor.rs   # 预处理器
│       └── mod.rs            # 编译器模块
├── alum-std/                 # 标准库
│   ├── alum/                 # 标准库头文件 (.al 文件)
│   │   ├── convert.al        # 类型转换
│   │   ├── io.al             # 输入/输出
│   │   ├── lib.al            # 主库 (系统调用)
│   │   ├── math.al           # 数学运算
│   │   ├── memory.al         # 内存管理
│   │   └── string.al         # 字符串操作
│   ├── src/                  # 标准库实现 (Rust no_std)
│   │   ├── convert.rs
│   │   ├── io.rs
│   │   ├── lib.rs
│   │   ├── math.rs
│   │   ├── memory.rs
│   │   └── string.rs
│   └── Cargo.toml            # 标准库依赖
├── alum-make/                # 构建工具 (almk)
│   ├── src/                  # 构建工具源代码
│   │   ├── main.rs           # 构建工具入口
│   │   ├── command.rs        # 命令处理
│   │   ├── config.rs         # 配置管理
│   │   ├── dependencies.rs   # 依赖管理
│   │   ├── new.rs            # 新项目创建
│   │   ├── sync.rs           # 依赖同步
│   │   └── build.rs          # 构建逻辑
│   └── Cargo.toml            # 构建工具依赖
├── alum-vscode/              # VS Code 扩展
│   ├── package.json          # 扩展配置
│   ├── language-configuration.json
│   └── syntaxes/             # 语法高亮
│       └── alum.tmLanguage.json
├── examples/                 # 示例程序
│   ├── 01_hello.al           # Hello World
│   ├── 02_variables.al       # 变量示例
│   ├── 03_functions.al       # 函数示例
│   ├── 04_control_flow.al    # 控制流
│   ├── 05_arrays.al          # 数组操作
│   ├── 06_math_operations.al # 数学运算
│   ├── 07_string_operations.al # 字符串操作
│   ├── 08_type_conversion.al # 类型转换
│   ├── 09_user_input.al      # 用户输入
│   ├── 10_loops_and_sum.al   # 循环和求和
│   ├── 11_fibonacci.al       # 斐波那契数列
│   ├── 12_array_search.al    # 数组搜索
│   ├── 13_array_sort.al      # 数组排序
│   ├── 14_factorial_comparison.al # 阶乘比较
│   ├── 15_mixed_c_alum/      # C/Alum 混合编程示例
│   │   ├── Alumake.toml      # 项目配置
│   │   ├── src/
│   │   │   ├── main.al       # Alum 主程序
│   │   │   └── helper.c      # C 辅助函数
│   │   └── include/
│   │       ├── helper.al     # Alum 外部声明
│   │       └── helper.h      # C 头文件
│   ├── 19_c_array_compatibility/ # C 数组兼容性示例
│   ├── 20_cmdline_args.al    # 命令行参数
│   └── 21_struct.al          # 结构体
├── Cargo.toml                # 编译器依赖
├── Cargo.lock                # 依赖锁定文件
├── install.sh                # 安装脚本
└── README.md                 # 项目文档
```

## 技术栈

### 编译器 (alc)
- **语言**: Rust (edition 2024)
- **关键依赖**:
  - `clap` 4.5.54 - CLI 参数解析
  - `cranelift` 0.127.2 - 代码生成后端
  - `cranelift-module` 0.127.2 - Cranelift 模块接口
  - `cranelift-object` 0.127.2 - 对象文件生成
  - `object` 0.36 - 对象文件读写

### 标准库 (alum-std)
- **语言**: Rust (edition 2024, no_std)
- **构建类型**: 静态库 (staticlib)
- **编译选项**:
  - release: `panic = "abort"`, `opt-level = 3`
  - dev: `panic = "abort"`

### 构建工具 (almk)
- **语言**: Rust (edition 2024)
- **关键依赖**:
  - `clap` 4.5.54 - CLI 参数解析
  - `toml` 0.9.11 - 配置文件解析
  - `serde` 1.0.228 - 序列化/反序列化
  - `git2` 0.20.3 - Git 仓库操作
  - `walkdir` 2.5.0 - 目录遍历
  - `zip` 7.2.0 - ZIP 文件处理
  - `ureq` 2.9.6 - HTTP 客户端

## 编译流水线

```
源代码 (.al)
        │
        ▼
┌───────────────┐
│  预处理器     │  →  处理 $import, $define, $ifdef, $ifndef, $endif
└───────────────┘
        │
        ▼
┌───────────────┐
│  词法分析器   │  →  将源代码标记化为 token 流
└───────────────┘
        │
        ▼
┌───────────────┐
│  语法分析器   │  →  从 token 构建 AST
└───────────────┘
        │
        ▼
┌───────────────┐
│  类型检查器   │  →  验证类型安全和语义规则
└───────────────┘
        │
        ▼
┌───────────────┐
│   优化器      │  →  常量折叠、死代码消除、未使用代码移除
└───────────────┘
        │
        ▼
┌───────────────┐
│  代码生成器   │  →  使用 Cranelift 编译 AST 为机器代码
└───────────────┘
        │
        ▼
  目标文件 (.o)
        │
        ▼
┌───────────────┐
│   链接器      │  →  将目标文件与标准库链接
└───────────────┘
        │
        ▼
  可执行文件
```

## 构建和运行

### 构建编译器

```bash
# 从源代码构建
cargo build --release

# 编译后的二进制文件位于: target/release/alc
```

### 构建标准库

```bash
cd alum-std
cargo build --release

# 编译后的库文件位于: target/release/libalum_std.a
```

### 构建构建工具

```bash
cd alum-make
cargo build --release

# 编译后的二进制文件位于: target/release/almk
```

### 安装

```bash
# 使用安装脚本
./install.sh

# 这将:
# 1. 构建并安装 alc 编译器
# 2. 构建标准库
# 3. 安装 libalum_std.a 到 /usr/local/lib/
# 4. 安装标准库头文件到 /usr/local/include/alum/
# 5. 安装构建工具 almk
```

### 编译 Alum 程序

```bash
# 基本编译
alc program.al

# 编译到可执行文件
alc program.al -o program

# 仅编译 (不链接)
alc program.al -c -o program.o

# 编译并立即运行
alc -r program.al

# 添加包含目录
alc program.al -I ./include

# 库模式 (保留所有函数)
alc lib.al -o lib.o --lib

# 链接目标文件
alc program.o -o program

# 不使用标准库
alc program.al --nostdlib

# 输出 AST
alc program.al --ast

# 仅预处理
alc program.al -E

# 详细输出
alc program.al -v
```

### 使用构建工具

```bash
# 创建新项目
almk new hello

# 构建项目
almk build

# 运行项目
almk run

# 清理构建文件
almk clean

# 添加依赖
almk add util -u https://www.website.com/util.zip

# 移除依赖
almk rm util
```

## 语言特性

### 支持的类型

- `int`: 有符号整数 (isize)
- `float`: 64 位浮点数 (f64)
- `bool`: 布尔值
- `string`: 字符串类型
- `void`: 无返回类型
- `arr[T]`: T 类型的数组

### 基本语法

```al
// 导入模块
$import "io.al"
$import "convert.al"

// 变量声明
let name: string = "Alum"
let count: int = 42
let pi: float = 3.14159
let is_valid: bool = true

// 函数定义
fun add(a: int, b: int): int {
    return a + b
}

// 外部函数声明 (FFI)
extern c_add(int, int): int
extern printf(string): int

// 控制流
if x > 0 {
    println("Positive")
}

while i < 10 {
    i = i + 1
}

for i in 0..10 {
    println(itoa(i))
}

// 数组
let numbers: arr[int] = [1, 2, 3, 4, 5]
let buffer: arr[int] = [int; 100]

// 结构体
struct Point {
    x: int,
    y: int
}

fun main(): int {
    let p: Point = Point {
        x: 10,
        y: 20
    }
    return 0
}

// 预处理器指令
$define PI 3.14159
$ifndef ALUM_LIB
$define ALUM_LIB 1
$endif
```

### FFI (Foreign Function Interface)

Alum 支持与 C 语言的互操作性：

```al
// 在 Alum 中声明外部 C 函数
extern c_add(int, int): int
extern printf(string): int

fun main(): int {
    let result: int = c_add(10, 20);
    printf(itoa(result));
    return 0;
}
```

对于混合 C/Alum 项目，使用 almk 构建工具可以自动处理 C 和 Alum 文件的编译和链接。

## 标准库模块

### I/O 模块 (`io.al`)

```al
extern write(int, string, int): int
extern read(int, string, int): int
extern print(string): int
extern println(string): int
extern input(string): string
extern fopen(string, int, int): int
extern fclose(int): int
extern fread(int): string
extern fwrite(int, string, int): int
extern lseek(int, int, int): int
```

### 数学模块 (`math.al`)

```al
extern abs(int): int
extern sqrt(int): int
extern max(int, int): int
extern min(int, int): int
extern pow(int, int): int
extern fact(int): int
```

### 字符串模块 (`string.al`)

```al
extern strlen(string): int
extern strcpy(string, string): string
extern strcat(string, string): string
extern memcpy(string, string, int): string
extern memset(string, int, int): string
extern bcmp(string, string, int): int
extern memcmp(string, string, int): int
```

### 内存模块 (`memory.al`)

```al
extern malloc(int): string
```

### 转换模块 (`convert.al`)

```al
extern itoa(int): string
extern atoi(string): int
extern atof(string): float
extern ftoa(float): string
```

### 主库 (`lib.al`)

```al
extern syscall(int, int, int, int): int
extern exit(int): void
```

## Alumake.toml 配置

### 基本项目配置

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

### 混合 C/Alum 项目配置

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

```toml
# ZIP 依赖
[dependencies.dep]
url = "https://www.website.com/dep.zip"
git = false

# Git 依赖
[dependencies.dep]
url = "https://www.website.com/dep.git"
git = true
tag = "v1.0"

# 本地依赖
[dependencies.dep]
local = "/path/to/dep"
git = false
```

## 开发约定

### 代码风格

- **Rust 代码**: 遵循 Rust 官方代码风格 (使用 `rustfmt`)
- **Alum 代码**: 
  - 使用 4 空格缩进
  - 函数和变量名使用蛇形命名法 (snake_case)
  - 类型名使用帕斯卡命名法 (PascalCase)
  - 使用显式类型注解

### 提交信息

从 git log 中观察到的提交信息风格：
- 使用简短、描述性的提交信息
- 常见前缀: `feat:`, `refact:`, `dev:`, `fix:`
- 示例:
  - `refact: dce`
  - `feat: optimizer`
  - `dev: optimizer: dce, unused code removal`

### 测试

项目当前没有明确的测试框架配置。在添加测试时，应考虑：
- 对于 Rust 代码: 使用 Rust 内置的测试框架
- 对于 Alum 代码: 创建测试示例并验证编译和执行

### 编译器组件开发

当修改编译器组件时：
1. 确保理解编译流水线的各个阶段
2. 保持组件之间的接口稳定
3. 在 `src/compiler/mod.rs` 中正确导出模块
4. 验证更改不会破坏现有示例

### 优化器开发

优化器位于 `src/compiler/optimizer.rs`，当前支持：
- 常量折叠 (Constant Folding)
- 死代码消除 (Dead Code Elimination)
- 未使用代码移除 (Unused Code Removal)

添加新优化时：
- 保持与现有优化流程的兼容性
- 确保优化不改变程序语义
- 测试优化对编译时间和生成代码质量的影响

## CLI 参数参考

```
alc [OPTIONS] <INPUT>

参数:
  <INPUT>...    输入文件 (.al 源文件或 .o/.obj 目标文件)

选项:
  -o, --output <FILE>       输出文件名
  -c, --compile-only        仅编译，不链接
  -r, --run                 编译并立即运行
  -E                        仅预处理；不编译、汇编或链接
  --ast                     输出 AST 表示
  -I <DIR>                  添加包含目录 (可多次使用)
  --nostdlib                不链接标准库
  --lib                     库模式 - 保留所有函数 (用于构建库)
  -v, --verbose             详细输出
  -h, --help                显示帮助
  -V, --version             显示版本
```

## 当前开发状态

根据 git log，最近的开发工作集中在：
- 优化器的实现和完善
- 死代码消除 (DCE) 功能
- 未使用代码移除
- 代码重构

## 常见任务

### 添加新的语言特性

1. 在 `src/compiler/lexer.rs` 中添加词法规则
2. 在 `src/compiler/parser.rs` 中添加语法规则
3. 在 `src/compiler/ast.rs` 中更新 AST 定义
4. 在 `src/compiler/checker.rs` 中添加类型检查
5. 在 `src/compiler/codegen.rs` 中添加代码生成逻辑
6. 更新文档和示例

### 添加标准库函数

1. 在 `alum-std/alum/` 中添加 `.al` 头文件声明
2. 在 `alum-std/src/` 中添加 Rust 实现
3. 重新构建标准库: `cd alum-std && cargo build --release`
4. 安装更新后的标准库

### 修复编译器 bug

1. 识别 bug 发生的编译阶段
2. 使用 `--ast` 或 `-E` 标志进行调试
3. 使用 `-v` 标志获取详细输出
4. 修复相应组件代码
5. 验证所有示例程序仍能正常编译

## 依赖管理

### 编译器依赖

- Rust 2024 edition
- Cranelift 0.127.2
- object 0.36

### 构建工具依赖

- Rust 2024 edition
- clap, toml, serde 等常用 Rust 库

### 外部依赖

- 无运行时依赖（标准库在编译时静态链接）

## 许可证

见 LICENSE 文件获取详细信息。