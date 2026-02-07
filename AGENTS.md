# Alum 编程语言项目 - Agent 上下文文档

## 项目概述

Alum 是一个现代化的系统级编程语言，设计简洁且高性能。它具有清晰的语法、强静态类型系统，并使用 Cranelift 代码生成器编译为本地机器代码。项目由三个主要组件组成：

1. **alc (编译器)** - Alum 语言编译器，负责将 `.al` 源代码编译为可执行文件
2. **alum-std (标准库)** - 用 Rust `no_std` 编写的标准库，提供 I/O、数学、字符串、数组、内存管理和类型转换功能
3. **almk (构建工具)** - 项目管理工具，支持依赖管理、混合 C/Alum 项目和自动化构建

### 核心技术栈

- **语言**: Rust (Edition 2024)
- **代码生成**: Cranelift 0.127.2
- **对象文件处理**: object crate
- **CLI 解析**: clap 4.5.54
- **系统调用**: 直接 Linux syscall

### 项目架构

```
Alum/
├── src/                      # 编译器源代码
│   ├── main.rs               # 编译器入口点
│   ├── cli/                  # CLI 参数解析和命令
│   │   ├── args.rs           # 命令行参数定义
│   │   ├── build.rs          # 编译命令实现
│   │   ├── link.rs           # 链接命令实现
│   │   └── mod.rs
│   └── compiler/             # 编译器核心组件
│       ├── lexer.rs          # 词法分析器
│       ├── parser.rs         # 语法分析器
│       ├── ast.rs            # 抽象语法树定义
│       ├── codegen.rs        # 代码生成器 (Cranelift)
│       ├── preprocessor.rs   # 预处理器
│       └── mod.rs
├── alum-std/                 # 标准库 (Rust no_std)
│   ├── alum/                 # 标准库头文件 (.al 文件)
│   │   ├── io.al             # I/O 操作
│   │   ├── math.al           # 数学运算
│   │   ├── string.al         # 字符串操作
│   │   ├── array.al          # 数组工具
│   │   ├── memory.al         # 内存管理
│   │   ├── convert.al        # 类型转换
│   │   └── lib.al            # 主库文件
│   └── src/                  # 标准库实现 (Rust)
│       ├── lib.rs            # 库入口，包含 syscall 包装
│       ├── io.rs             # I/O 实现
│       ├── math.rs           # 数学函数实现
│       ├── string.rs         # 字符串函数实现
│       ├── array.rs          # 数组函数实现
│       ├── memory.rs         # 内存函数实现
│       └── convert.rs        # 类型转换实现
├── alum-make/                # 构建工具 (almk)
│   └── src/
│       ├── main.rs           # 工具入口
│       ├── command.rs        # 命令处理
│       ├── config.rs         # 配置解析 (Alumake.toml)
│       ├── build.rs          # 构建逻辑
│       ├── new.rs            # 项目创建
│       ├── sync.rs           # 依赖同步
│       └── dependencies.rs   # 依赖管理
├── alum-vscode/              # VS Code 扩展
│   ├── syntaxes/alum.tmLanguage.json
│   └── language-configuration.json
├── examples/                 # 示例代码
└── Cargo.toml                # 编译器依赖
```

## 编译管线

1. **预处理**: 处理 `$import`、`$define`、`$ifdef`、`$ifndef`、`$endif` 指令
2. **词法分析**: 将源代码标记化为 tokens
3. **语法分析**: 构建抽象语法树 (AST)
4. **代码生成**: 使用 Cranelift 将 AST 编译为机器代码
5. **链接**: 将目标文件与标准库链接

## Alum 语言特性

### 类型系统

Alum 是静态类型语言，所有变量和函数必须在编译时声明类型。

**支持的类型**:
- `int`: 有符号整数 (isize)
- `float`: 64位浮点数 (f64)
- `bool`: 布尔值
- `string`: 字符串类型
- `void`: 无返回类型
- `arr[T]`: 类型 T 的数组
- `typedef`: 类型别名 (新增特性)

### 语法示例

```al
// 导入模块
$import "io.al"
$import "convert.al"

// 类型别名
typedef MyInt = int
typedef MyArray = arr[int]

// 函数定义
fun add(a: int, b: int): int {
    return a + b;
}

// 外部函数 (FFI)
extern c_add(int, int): int
extern printf(string): int

// 主函数
fun main(): int {
    let x: MyInt = 42;
    let arr: MyArray = [1, 2, 3, 4, 5];
    
    if x > 0 {
        println("Positive");
    }
    
    for i in 0..10 {
        println(itoa(i));
    }
    
    return 0;
}
```

### 控制流

- `if-else`: 条件语句
- `while`: while 循环
- `for`: 范围循环 (`for i in 0..10`)
- `break`: 跳出循环
- `continue`: 继续下一次迭代

## 构建和运行

### 安装整个项目

```bash
./install.sh
```

此脚本会：
1. 构建并安装 `alc` 编译器
2. 构建标准库
3. 将 `libalum_std.a` 安装到 `/usr/local/lib/`
4. 将标准库头文件安装到 `/usr/local/include/alum/`
5. 安装构建工具 `almk`

### 单独构建编译器

```bash
cargo build --release
```

### 单独构建标准库

```bash
cd alum-std
cargo build --release
```

### 单独构建构建工具

```bash
cd alum-make
cargo build --release
```

### 编译和运行 Alum 程序

```bash
# 编译为可执行文件
alc program.al -o program

# 仅编译为目标文件
alc program.al -c -o program.o

# 链接目标文件
alc program.o -o program

# 编译并立即运行
alc -r program.al

# 预处理输出
alc -E program.al

# 输出 AST
alc program.al --ast

# 包含自定义目录
alc program.al -I ./include
```

### 使用 almk 构建项目

```bash
# 创建新项目
almk new hello

# 构建项目
almk build

# 运行项目
almk run

# 清理
almk clean

# 添加依赖
almk add util -u https://www.website.com/util.zip

# 移除依赖
almk rm util
```

## Alumake.toml 配置

标准项目配置：

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

混合 C/Alum 项目配置：

```toml
[package]
name = "mixed_project"
version = "0.1.0"
language = "mixed"

[build]
linker = "alc"
cc = "cc"
alc = "alc"
cflags = "-Wall -O2"
includes = ["./include"]
nostdlib = true
```

## 开发约定

### 编译器代码结构

- **ast.rs**: 定义 AST 节点类型 (Program, Expr, Type)
- **lexer.rs**: 词法分析器，将源代码转换为 Token 流
- **parser.rs**: 语法分析器，将 Token 流转换为 AST
- **codegen.rs**: 代码生成器，使用 Cranelift 生成机器码
- **preprocessor.rs**: 预处理器，处理 import 和宏指令

### 标准库实现约定

- 使用 `#![no_std]` 和 `#![no_main]`
- 直接使用 Linux syscall 实现系统调用
- 所有外部函数通过 `extern "C"` 声明
- 使用 `asm!` 宏进行内联汇编
- Panic 处理器使用 `ud2` 指令

### 代码风格

- 使用 Rust 2024 edition
- 函数和变量使用 snake_case
- 类型使用 PascalCase
- Alum 语法遵循 C 风格但更简洁

### 测试

当前项目没有显式的测试目录。测试应通过：
1. 编译器自举测试
2. examples/ 目录中的示例程序
3. 标准库的集成测试

### 添加新功能

当添加新语言特性时，需要修改以下文件：

1. **AST 定义** (`src/compiler/ast.rs`): 添加新的 Expr 或 Type 变体
2. **词法分析器** (`src/compiler/lexer.rs`): 如果需要新的 Token 类型
3. **语法分析器** (`src/compiler/parser.rs`): 添加解析逻辑
4. **代码生成器** (`src/compiler/codegen.rs`): 实现代码生成逻辑
5. **CLI 参数** (`src/cli/args.rs`): 如果需要新的命令行选项
6. **示例代码** (`examples/`): 添加示例展示新特性

### 标准库扩展

添加新的标准库函数需要：

1. 在 `alum-std/alum/` 中添加 `.al` 头文件声明
2. 在 `alum-std/src/` 中添加对应的 Rust 实现
3. 在 `alum-std/src/lib.rs` 中添加模块声明
4. 重新构建标准库并安装

## 常见任务

### 修复编译错误

1. 识别错误发生的阶段（词法、语法、代码生成）
2. 检查对应的源文件（lexer.rs、parser.rs、codegen.rs）
3. 使用 `-v` 或 `--verbose` 标志获取详细输出
4. 使用 `--ast` 检查 AST 是否正确

### 添加新的运算符

1. 在 `lexer.rs` 中添加 Token 定义
2. 在 `parser.rs` 中添加解析优先级和规则
3. 在 `ast.rs` 中添加对应的 Expr 变体
4. 在 `codegen.rs` 中实现代码生成

### 添加新的控制流语句

1. 在 `ast.rs` 中添加新的 Expr 变体
2. 在 `parser.rs` 中添加解析逻辑
3. 在 `codegen.rs` 中实现基本的块和控制流生成

### 调试代码生成问题

- 使用 `alc --ast` 检查 AST 结构
- 使用 `-v` 标志查看详细编译过程
- 检查 Cranelift IR 输出（如果启用）
- 验证类型推断是否正确

## 依赖管理

almk 支持三种依赖来源：

1. **Git 仓库**:
   ```toml
   [dependencies.dep]
   url = "https://www.website.com/dep.git"
   git = true
   tag = "v1.0"
   ```

2. **ZIP 文件**:
   ```toml
   [dependencies.dep]
   url = "https://www.website.com/dep.zip"
   git = false
   ```

3. **本地路径**:
   ```toml
   [dependencies.dep]
   local = "/path/to/dep"
   git = false
   ```

## 当前开发状态

根据 git 历史，最近的工作包括：
- 添加 `break` 和 `continue` 语句支持
- 更新 Alumake.toml 配置
- 添加 GPL-3.0 许可证
- 修复 alum-make 从 gitlink 转换为常规目录

当前分支：`dev`
最近提交：`39c35f6 feat: break && continue`

## 重要提示

1. **系统依赖**: 标准库直接使用 Linux syscall，仅支持 Linux 平台
2. **标准库安装**: 需要 sudo 权限将库文件安装到 `/usr/local/lib/`
3. **FFI 调用**: 外部函数需要使用 `extern` 关键字声明
4. **数组访问**: 数组索引从 0 开始
5. **类型注解**: 所有变量和函数必须显式声明类型
6. **预处理器**: `$import` 指令用于导入标准库模块
7. **内存管理**: 使用 `malloc` 和指针进行手动内存管理

## 故障排除

### 编译器未找到

```bash
# 确保编译器已安装
which alc

# 如果未找到，重新安装
./install.sh
```

### 标准库链接错误

```bash
# 检查标准库是否安装
ls /usr/local/lib/libalum_std.a

# 如果未找到，重新安装标准库
cd alum-std
cargo build --release
sudo cp target/release/libalum_std.a /usr/local/lib/
```

### 链接器错误

- 检查是否使用了 `--nostdlib` 标志
- 验证标准库路径是否正确
- 对于混合项目，确保 `nostdlib = true` 在 Alumake.toml 中设置

## 参考资源

- 主 README: `/workspaces/Alum/README.md`
- 标准库文档: `/workspaces/Alum/alum-std/README.md`
- 构建工具文档: `/workspaces/Alum/alum-make/README.md`
- 示例代码: `/workspaces/Alum/examples/`