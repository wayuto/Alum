# Alum 编程语言 - AI 代理上下文指南

## 项目概述

Alum 是一个现代的、高性能的系统编程语言，使用 Rust 实现。它具有简洁的语法、强静态类型系统，并通过 Cranelift 代码生成器直接编译为本地机器代码。该项目包含三个主要组件：

1. **alc (编译器)**: 将 `.al` 源文件编译为机器代码
2. **almk (构建工具)**: 项目管理和构建系统
3. **alum-std (标准库)**: 提供核心功能（I/O、数学、字符串、内存等）

### 核心特性

- **简洁语法**: 受现代语言启发的清晰可读语法
- **静态类型**: 显式类型注释，类型安全
- **本地编译**: 通过 Cranelift 直接编译为机器代码
- **快速编译**: 高效的编译流水线
- **FFI 支持**: 与 C 语言的互操作性
- **Lambda 函数**: 支持匿名函数和闭包
- **参数化宏**: 强大的宏系统
- **泛型类型**: 通过 `gen` 类型支持类似泛型的编程
- **指针支持**: 支持直接内存操作
- **动态数组**: Vec 容器用于动态集合管理

## 项目结构

```
Alum/
├── src/                      # 编译器源代码
│   ├── main.rs               # 编译器入口点
│   ├── cli/                  # CLI 参数解析和命令
│   │   ├── args.rs           # CLI 参数定义
│   │   ├── build.rs          # 构建命令实现
│   │   ├── link.rs           # 链接命令实现
│   │   └── mod.rs            # CLI 模块
│   └── compiler/             # 编译器组件
│       ├── ast.rs            # 抽象语法树定义
│       ├── checker.rs        # 类型检查器
│       ├── codegen.rs        # 代码生成器（Cranelift）
│       ├── lexer.rs          # 词法分析器
│       ├── mod.rs            # 编译器模块
│       ├── optimizer.rs      # 优化器
│       ├── parser.rs         # 语法分析器
│       └── preprocessor.rs   # 预处理器
├── alum-std/                 # 标准库
│   ├── alum/                 # 标准库头文件（.al 文件）
│   │   ├── headers/          # 标准库头文件
│   │   │   ├── convert.al    # 类型转换
│   │   │   ├── io.al         # 输入输出
│   │   │   ├── lib.al        # 系统调用
│   │   │   ├── math.al       # 数学运算
│   │   │   ├── memory.al     # 内存管理
│   │   │   ├── string.al     # 字符串操作
│   │   │   └── vec.al        # 动态数组
│   │   ├── src/              # 标准库实现（Alum 源码）
│   │   │   ├── math.al       # 数学库实现
│   │   │   └── vector.al     # 向量库实现
│   │   ├── Alumake.toml      # 标准库构建配置
│   │   └── target/objects/   # 编译的对象文件
│   ├── src/                  # Rust 运行时实现（no_std）
│   │   ├── convert.rs
│   │   ├── io.rs
│   │   ├── lib.rs
│   │   ├── memory.rs
│   │   └── string.rs
│   └── Cargo.toml            # 标准库依赖
├── alum-make/                # 构建工具（almk）
│   ├── src/                  # 构建工具源代码
│   │   ├── main.rs           # 构建工具入口
│   │   ├── build.rs          # 构建命令
│   │   ├── command.rs        # 命令定义
│   │   ├── config.rs         # 配置管理
│   │   ├── dependencies.rs   # 依赖管理
│   │   ├── new.rs            # 项目创建
│   │   └── sync.rs           # 同步命令
│   └── Cargo.toml            # 构建工具依赖
├── alum-vscode/              # VS Code 扩展
│   ├── package.json          # 扩展配置
│   ├── syntaxes/             # 语法高亮
│   │   └── alum.tmLanguage.json
│   └── alum-vscode-0.9.1.vsix # 打包的扩展
├── examples/                 # 示例程序
│   ├── 01_hello.al
│   ├── 02_variables.al
│   ├── ...                   # 更多示例
│   ├── 15_mixed_c_alum/      # C/Alum 混合项目示例
│   └── 19_c_array_compatibility/ # C 数组兼容性示例
├── Cargo.toml                # 编译器依赖
├── Cargo.lock                # 依赖锁定文件
├── install.sh                # 安装脚本
├── README.md                 # 项目文档
└── LICENSE                   # 许可证
```

## 编译流程

Alum 编译器使用多阶段流水线将源代码转换为可执行文件：

```
源代码 (.al)
    │
    ▼
┌───────────────┐
│   预处理器    │  → 处理 $import、$define、$ifdef、$ifndef、$endif
└───────────────┘
    │
    ▼
┌───────────────┐
│   词法分析器  │  → 将源代码标记化为 token 流
└───────────────┘
    │
    ▼
┌───────────────┐
│   语法分析器  │  → 构建抽象语法树（AST）
└───────────────┘
    │
    ▼
┌───────────────┐
│   类型检查器  │  → 验证类型安全和语义规则
└───────────────┘
    │
    ▼
┌───────────────┐
│    优化器     │  → 常量折叠、死代码消除
└───────────────┘
    │
    ▼
┌───────────────┐
│  代码生成器   │  → 使用 Cranelift 编译为机器代码
└───────────────┘
    │
    ▼
  对象文件 (.o)
    │
    ▼
┌───────────────┐
│    链接器     │  → 链接对象文件和标准库
└───────────────┘
    │
    ▼
  可执行文件
```

### 优化阶段

编译器执行以下优化：
- 常量折叠（例如：`2 + 3` → `5`）
- 代数简化（例如：`x + 0` → `x`）
- 死代码消除
- 分支消除（例如：删除不可达代码）

## 构建和运行

### 安装编译器和工具链

```bash
# 运行安装脚本
./install.sh

# 这将：
# 1. 构建并安装 alc 编译器
# 2. 构建标准库
# 3. 安装 libalum_std.a 到 /usr/local/lib/
# 4. 安装标准库头文件到 /usr/local/include/alum/
# 5. 安装构建工具 almk
```

### 从源代码构建

```bash
# 构建编译器
cargo build --release

# 构建标准库
cd alum-std
cargo build --release

# 构建构建工具
cd ../alum-make
cargo build --release

# 使用 cargo install 安装
cargo install --path .
cd ../alum-make
cargo install --path .
```

### 使用编译器 (alc)

```bash
# 编译单个文件
alc program.al -o program

# 编译并立即运行
alc -r program.al

# 仅编译（生成目标文件）
alc program.al -c -o program.o

# 链接对象文件
alc program.o -o program

# 添加包含目录
alc program.al -I ./include

# 不链接标准库
alc program.al --nostdlib

# 输出 AST
alc program.al --ast

# 仅预处理
alc program.al -E

# 详细输出
alc program.al -v

# 构建静态库
alc lib.al --lib static -o libmylib.a

# 构建共享库
alc lib.al --lib shared -o libmylib.so
```

### 使用构建工具 (almk)

```bash
# 创建新项目
almk new project_name

# 构建项目
almk build

# 运行项目
almk run

# 清理构建产物
almk clean

# 添加依赖
almk add dependency_name -u https://url/to/dependency.zip

# 移除依赖
almk rm dependency_name
```

## 语言特性详解

### 类型系统

Alum 是强静态类型语言，支持以下类型：

- **基本类型**: `int` (isize), `float` (f64), `bool`, `string`, `void`
- **复合类型**: `arr[T]` (数组), `*T` (指针)
- **泛型类型**: `gen` (自动类型推断)
- **结构体**: 自定义数据结构

### 变量声明

```al
let x: int = 10
let name: string = "Alum"
let pi: float = 3.14159
let is_valid: bool = true
```

### 函数定义

```al
fun add(a: int, b: int): int {
    return a + b
}
```

### Lambda 函数

```al
let square: int(int): int = lamb(x: int): int {
    return x * x
}
```

### 指针操作

```al
$import "memory.al"

let value: int = 42
let ptr: *int = &value
*ptr = 100  // 通过指针修改值
```

### Vec 动态数组

```al
$import "vec.al"

let vec: Vec = vec_new()
vec.push(&vec, 10)
vec.push(&vec, 20)
let first: gen = vec.at(&vec, 0)
```

### 预处理器指令

```al
// 简单宏定义
$define PI 3.14159
$define MAX(a, b) if a > b { a } else { b }

// 条件编译
$ifdef DEBUG
println("Debug mode")
$endif

// 导入模块
$import "io.al"
```

### FFI（外部函数接口）

```al
// 声明 C 函数
extern printf(string): int
extern malloc(int): *int

// 使用 C 函数
fun main(): int {
    let ptr: *int = malloc(10)
    printf("Hello from C\n")
    return 0
}
```

### 控制流

```al
// if-else
if x > 0 {
    println("Positive")
} else {
    println("Non-positive")
}

// while 循环
while i < 10 {
    i = i + 1
}

// for 循环
for i in 0..10 {
    println(itoa(i))
}

// break 和 continue
for i in 0..100 {
    if i == 50 {
        break
    }
    if i % 2 == 0 {
        continue
    }
}
```

## 标准库模块

### IO 模块 (`io.al`)

```al
extern print(string): int
extern println(string): int
extern input(string): string
extern fopen(string, int, int): int
extern fclose(int): int
extern fread(int): string
extern fwrite(int, string, int): int
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
```

### 类型转换模块 (`convert.al`)

```al
extern itoa(int): string
extern atoi(string): int
extern atof(string): float
extern ftoa(float): string
```

### 内存模块 (`memory.al`)

```al
extern malloc(int): string  // 返回指针
```

## 项目配置 (Alumake.toml)

### 基本配置

```toml
[package]
name = "project_name"
version = "0.1.0"
author = "Your Name"
license = "MIT"
language = "alum"

[build]
linker = "alc"
alc = "alc"
includes = ["./include"]
```

### 库类型配置

```toml
[build]
library_type = "static"  # 或 "shared" / "a" / "so"
```

### 混合 C/Alum 项目

```toml
[package]
language = "mixed"

[build]
linker = "alc"
cc = "cc"
cflags = "-Wall -O2"
alflags = ""
includes = ["./include"]
nostdlib = true
```

### 依赖管理

```toml
# ZIP 依赖
[dependencies.dep]
url = "https://example.com/dep.zip"
git = false

# Git 依赖
[dependencies.dep]
url = "https://github.com/user/repo.git"
git = true
tag = "v1.0"

# 本地依赖
[dependencies.dep]
local = "/path/to/dep"
git = false
```

## 开发约定

### 代码风格

- **Rust 代码**: 遵循 Rust 标准格式化（`cargo fmt`）
- **Alum 代码**: 使用 4 空格缩进，函数和变量使用 snake_case
- **类型注释**: 所有变量和函数都需要显式类型注释

### 编译器架构

编译器采用模块化设计，各组件职责清晰：

- **lexer.rs**: 词法分析，将源代码转换为 token 流
- **parser.rs**: 语法分析，构建 AST
- **preprocessor.rs**: 处理预处理器指令
- **checker.rs**: 类型检查和语义分析
- **optimizer.rs**: 代码优化
- **codegen.rs**: 使用 Cranelift 生成机器代码
- **ast.rs**: AST 节点定义

### 标准库组织

标准库分为两部分：
1. **Alum 头文件** (`alum/headers/`): .al 格式的头文件，供用户导入
2. **Rust 实现** (`src/`): 使用 Rust no_std 实现的实际功能

### 错误处理

- 编译器使用 `Result<T, E>` 进行错误处理
- 错误信息应该清晰明确，帮助用户定位问题
- 使用适当的错误传播（`?` 操作符）

## 技术栈

### 核心依赖

- **Rust**: 2024 edition
- **clap**: 4.5.54 - CLI 参数解析
- **cranelift**: 0.127.2 - 代码生成后端
- **object**: 0.36 - 对象文件处理
- **serde**: 1.0.22 - 序列化/反序列化（构建工具）
- **toml**: 0.9.11 - TOML 配置解析（构建工具）
- **walkdir**: 2.5.0 - 文件遍历（构建工具）

### 编译器组件

- **词法分析**: 手写的词法分析器
- **语法分析**: 递归下降解析器
- **类型系统**: 静态类型检查
- **代码生成**: Cranelift IR 到机器代码

## 常见任务

### 添加新的语言特性

1. 在 `src/compiler/ast.rs` 中定义新的 AST 节点
2. 在 `src/compiler/lexer.rs` 中添加 token 识别
3. 在 `src/compiler/parser.rs` 中实现解析逻辑
4. 在 `src/compiler/checker.rs` 中添加类型检查规则
5. 在 `src/compiler/codegen.rs` 中实现代码生成

### 添加标准库函数

1. 在 `alum-std/alum/headers/` 中创建或更新头文件
2. 在 `alum-std/src/` 中实现函数逻辑（Rust）
3. 重新构建标准库: `cd alum-std && cargo build --release`
4. 更新安装脚本以正确安装新函数

### 扩展构建工具

1. 在 `alum-make/src/` 中添加新命令或修改现有命令
2. 更新 `command.rs` 定义命令结构
3. 实现命令逻辑
4. 重新构建: `cd alum-make && cargo build --release`

### 调试编译器

```bash
# 启用详细输出
alc program.al -v

# 输出 AST 检查解析
alc program.al --ast

# 仅预处理检查宏展开
alc program.al -E

# 使用 Rust 调试器
rust-gdb ./target/release/alc program.al
```

## 测试

### 运行示例程序

```bash
# 编译并运行示例
alc -r examples/01_hello.al

# 使用构建工具
cd examples/01_hello
almk run
```

### 编译器测试

- 查看 `examples/` 目录中的示例程序
- 确保所有示例都能正确编译和运行
- 测试新特性时创建相应的示例程序

## VS Code 支持

项目包含 VS Code 扩展，提供：
- 语法高亮
- 代码片段
- 语言配置

扩展位于 `alum-vscode/` 目录，可以打包为 `.vsix` 文件进行安装。

## 版本信息

- **当前版本**: 0.9.1
- **Rust 版本要求**: >= 1.93.0
- **Git 分支**: dev

## 相关资源

- **项目主页**: https://github.com/wayuto/Alum
- **标准库文档**: `alum-std/README.md`
- **构建工具文档**: `alum-make/README.md`
- **示例程序**: `examples/` 目录

## 重要注意事项

1. **标准库安装**: 标准库需要安装到 `/usr/local/lib/` 和 `/usr/local/include/alum/`，需要 sudo 权限
2. **混合项目**: C/Alum 混合项目需要设置 `nostdlib = true` 以避免 `_start` 符号冲突
3. **指针安全**: Alum 支持指针操作，但不提供自动内存管理，需要手动管理内存
4. **类型推断**: `gen` 类型提供自动类型推断，但类型推断基于使用上下文
5. **宏展开**: 预处理器宏在编译前展开，没有运行时开销

## 故障排除

### 编译错误

- 检查类型注释是否正确
- 确保所有导入的模块都存在
- 验证函数签名匹配

### 链接错误

- 确保标准库已正确安装
- 检查 `-nostdlib` 标志是否正确使用
- 验证对象文件路径

### 运行时错误

- 检查指针是否正确初始化
- 验证数组访问是否在边界内
- 确保内存分配和释放配对

## 贡献指南

修改代码时：
1. 遵循现有的代码风格和架构模式
2. 添加适当的错误处理
3. 更新相关文档
4. 测试所有修改
5. 确保构建成功: `cargo build --release`

## 总结

Alum 是一个简洁而强大的系统编程语言，结合了现代语言特性和高性能编译。项目结构清晰，模块化设计良好，便于扩展和维护。无论是添加新特性、扩展标准库还是改进工具链，都有明确的开发路径和约定。