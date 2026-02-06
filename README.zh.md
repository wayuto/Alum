# Alum 编程语言

Alum 是一个现代的系统编程语言，专为简洁性和高性能而设计。它具有简洁的语法、强静态类型，并使用 Cranelift 代码生成器编译为原生机器码。

## 特性

- **简洁语法**: 受现代语言启发的清晰、可读的语法
- **静态类型**: 具有显式类型注解的类型安全
- **原生编译**: 通过 Cranelift 直接编译为机器码
- **标准库**: 为 I/O、数学、字符串、数组、内存和类型转换提供的综合标准库
- **预处理器**: 支持包含、定义和条件编译
- **快速编译**: 高效的编译流水线

## 安装

### 前置要求

- Rust 工具链（2024 版本）
- Linux x86_64 系统

### 从源码构建

```bash
# 克隆仓库
git clone https://github.com/wayuto/Alum.git
cd Alum

# 安装编译器
./install.sh
```

这将：
1. 构建并安装 `alc` 编译器
2. 构建标准库
3. 将 `libalum_std.a` 安装到 `/usr/local/lib/`
4. 将标准库头文件安装到 `/usr/local/include/alum/`

## 快速开始

### Hello World

创建文件 `hello.al`：

```al
fun main(): int {
    println("Hello, World!");
    return 0;
}
```

编译并运行：

```bash
alc hello.al
./hello
```

或使用运行命令：

```bash
alc -r hello.al
```

### 基本示例

```al
$import "convert.al"

fun main(): int {
    let x: int = 10;
    let y: int = 20;
    let sum: int = x + y;
    
    println(itoa(sum));
    return 0;
}
```

## CLI 用法

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
  --nostdlib                不链接标准库
  -v, --verbose             详细输出
  -h, --help                显示帮助
  -V, --version             显示版本
```

### 示例

编译为可执行文件：
```bash
alc program.al -o program
```

仅编译（目标文件）：
```bash
alc program.al -c -o program.o
```

链接目标文件：
```bash
alc program.o -o program
```

立即运行：
```bash
alc -r program.al
```

仅预处理：
```bash
alc -E program.al
```

包含自定义目录：
```bash
alc program.al -I ./include
```

详细输出：
```bash
alc -v program.al
```

## 语言语法

### 类型系统

Alum 是一个具有显式类型注解的静态类型语言。所有变量和函数必须在编译时声明其类型。这提供了类型安全性，并使编译器能够生成高效的机器码。

**主要特点：**
- 静态类型：在编译时检查类型
- 显式注解：必须使用 `:` 语法声明类型
- 无类型推断：必须为每个变量和函数参数指定类型
- 类型安全：通过编译时检查防止许多常见的编程错误

### 类型

Alum 支持以下原始类型：
- `int`: 有符号整数（isize）
- `float`: 64 位浮点数（f64）
- `bool`: 布尔值
- `string`: 字符串类型
- `void`: 无返回类型
- `arr[T]`: T 类型的数组

### 变量

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

声明外部函数（通常来自 C）：

```al
extern syscall(int, int, int, int): int
extern exit(int): void
```

### 控制流

#### If-Else

```al
if x > 0 {
    println("Positive");
} else {
    println("Non-positive");
}
```

#### While 循环

```al
let i: int = 0;
while i < 10 {
    println(itoa(i));
    i = i + 1;
}
```

#### For 循环

```al
for i in 0..10 {
    println(itoa(i));
}
```

### 运算符

**算术**: `+`, `-`, `*`, `/`

**比较**: `==`, `!=`, `<`, `<=`, `>`, `>=`

**逻辑**: `&&`, `||`, `!`

**位运算**: `&`, `|`, `^`

**范围**: `..`

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

## 标准库

Alum 标准库提供了按模块组织的核心功能。

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
extern println(string): int            // 打印字符串并换行
extern input(string): string           // 读取用户输入（带提示）
extern fopen(string, int, int): int    // 打开文件
extern fclose(int): int                // 关闭文件
extern fread(int): string              // 从文件读取
extern fwrite(int, string, int): int   // 写入文件
extern lseek(int, int, int): int       // 在文件中定位
```

### 数学模块 (`math.al`)

```al
extern abs(int): int        // 绝对值
extern sqrt(int): int       // 平方（注意：返回 x * x）
extern max(int, int): int   // 两个数中的最大值
extern min(int, int): int   // 两个数中的最小值
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
extern range(int, int): string  // 生成范围（返回数组指针）
```

### 内存模块 (`memory.al`)

```al
extern malloc(int): string  // 分配内存（返回指针）
```

### 类型转换模块 (`convert.al`)

```al
extern itoa(int): string    // 整数转字符串
extern atoi(string): int    // 字符串转整数
extern atof(string): float  // 字符串转浮点数
extern ftoa(float): string  // 浮点数转字符串
```

### 主库 (`lib.al`)

主库模块导入所有标准库模块：

```al
$import "io.al"
$import "string.al"
$import "convert.al"
$import "math.al"
$import "array.al"
$import "memory.al"

extern syscall(int, int, int, int): int
extern exit(int): void
```

## 编译流水线

Alum 编译器遵循标准的编译流水线：

1. **预处理**: 处理 `$import`、`$define`、`$ifdef`、`$ifndef`、`$endif` 指令
2. **词法分析**: 将源代码标记化为标记
3. **语法分析**: 构建抽象语法树（AST）
4. **代码生成**: 使用 Cranelift 将 AST 编译为机器码
5. **链接**: 将目标文件与标准库链接以创建可执行文件

## 项目结构

```
Alum/
├── src/
│   ├── main.rs           # 编译器入口
│   ├── cli/              # CLI 参数解析和命令
│   │   ├── args.rs       # 命令行参数定义
│   │   ├── build.rs      # 构建命令实现
│   │   ├── link.rs       # 链接器实现
│   │   └── mod.rs        # CLI 模块导出
│   └── compiler/         # 编译器组件
│       ├── lexer.rs      # 词法分析器
│       ├── parser.rs     # 解析器
│       ├── ast.rs        # AST 定义
│       ├── codegen.rs    # 代码生成
│       ├── preprocessor.rs  # 预处理器
│       └── mod.rs        # 编译器模块导出
├── alum-std/             # 标准库
│   ├── alum/             # 标准库头文件（.al 文件）
│   │   ├── lib.al        # 主库模块
│   │   ├── io.al         # I/O 函数
│   │   ├── math.al       # 数学函数
│   │   ├── string.al     # 字符串函数
│   │   ├── array.al      # 数组函数
│   │   ├── memory.al     # 内存函数
│   │   └── convert.al    # 类型转换函数
│   └── src/              # 标准库实现（Rust no_std）
│       ├── lib.rs        # 库入口及系统调用
│       ├── io.rs         # I/O 实现
│       ├── math.rs       # 数学实现
│       ├── string.rs     # 字符串实现
│       ├── array.rs      # 数组实现
│       ├── memory.rs     # 内存实现
│       └── convert.rs    # 转换实现
├── alum-vscode/          # VS Code 扩展
│   ├── syntaxes/
│   │   └── alum.tmLanguage.json  # 语法高亮
│   └── language-configuration.json
├── Cargo.toml            # 编译器依赖
└── install.sh            # 安装脚本
```

## 开发

### 构建编译器

```bash
cargo build --release
```

### 构建标准库

```bash
cd alum-std
cargo build --release
```
