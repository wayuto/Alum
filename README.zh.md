# **Gos 编程语言([English](README.md))**

Gos 是一门用**Rust**实现的，轻量且高效的编程语言

## **🚀 安装**

### **先决条件**

Gos 针对 **x86_64 Linux** 环境设计。请确保已安装：

- **Rust & Cargo**（2024 版）
- **NASM**（Netwide Assembler）
- **ld**（GNU 链接器）

### **设置**

克隆仓库并运行自动安装脚本来设置 `gos` 可执行文件和标准库：

```bash
# 克隆仓库  
git clone --depth 1 https://github.com/wayuto/Gos ~/Gos  
cd ~/Gos

# 运行安装脚本  
# 该脚本会安装 'gos' CLI、构建标准库，
# 并将 'libgos.a' 移动到 /usr/local/lib  
sh ./install.sh
```

## **🛠 特性与语法**

### **基本类型**

- `int`：64 位有符号整数/数字。
- `flt`：64 位浮点数（IEEE 754 双精度）。
- `str`：字符串类型。
- `bool`：布尔类型（true / false）。
- `arr<N>`：定长数组（例如 `arr<5>`）。
- `void`：用于不返回值的函数。

### **变量与常量**

```gos
let x: int = 42 
let pi: flt = 3.14159
let message: str = "Hello, Gos!"
let flags: arr<3> = [1 2 3] 
let dynamic: arr<_> = [1 2 3 4] # 长度被推断为 4
```

### **浮点数**

Gos 使用 `flt` 关键字支持 64 位浮点数。浮点数字面量可以使用十进制表示法：

```gos
let pi: flt = 3.14159
let e: flt = 2.71828
let result: flt = 10.5 + 20.3
```

浮点数支持所有标准算术运算（+, -, *, /）和比较运算（==, !=, >, >=, <, <=）。

### **控制流**

Gos 支持现代控制流结构，包括 if-else 表达式和循环。

```gos
# If-Else 作为表达式
let result: str = if x > 10 "High" else "Low"

# While 循环
while x > 0 { x-- }

# 基于区间的 For 循环（使用 `n..m` 之前需 $import "array"）
for i in 0..10 { println(itoa(i)) }
```

### **代码块作用域**

在 Gos 中，代码块是表达式。代码块中的最后一个值作为该块的返回值。

```gos
let computed: int = { 
  let a: int = 10 
  let b: int = 20 
  a + b # 这是代码块的值
}
```

## **🔧 预处理器指令**

Gos 包含一个预处理器，支持用于代码组织和重用的指令。

### **$import**

`$import` 指令将外部 Gos 源文件包含到你的程序中：

```gos
$import "gosio"
$import "math"
```

预处理器会在以下位置搜索导入的文件：

1. 当前文件所在目录
2. `/usr/local/gos/`（标准库位置）

### **$define**

`$define` 指令允许你定义宏以进行文本替换：

```gos
$define PI 3.14159
$define MAX_SIZE 100

let radius: flt = 5.0
let area: flt = PI * radius * radius
```

宏是简单的文本替换，在预处理阶段进行。它们可用于常量、简单表达式或代码片段。

## **📚 标准库（gos-std）**

Gos 标准库提供了常用的基础功能。使用 `$import` 引入模块。

| 模块        | 主要函数                               |
| :---------- | :------------------------------------- |
| **gosio**   | print, println, input, read, write     |
| **math**    | abs, sqrt, max, min, pow, fact         |
| **string**  | strlen, strcpy, strcat, memcpy, memset |
| **convert** | itoa, atoi, ftoa, atof                 |
| **array**   | range, find                            |
| **stdlib**  | syscall, exit                          |

## **💻 语言示例**

### **Hello World**

将以下内容保存为 `hello.gos`：

```gos
$import "gosio"

pub fun main(): int {
  println("Hello world!") 
  return 0
}
```

### **浮点数示例**

```gos
$import "gosio"

pub fun main(): int {
  let pi: flt = 3.14159
  let radius: flt = 5.0
  let area: flt = pi * radius * radius
  println(itoa(area as int))  # 转换为整数以便打印
  return 0
}
```

运行方式：

```bash
gos -c hello.gos
./hello
```

### **递归斐波那契（示例）**

```gos
$import "gosio"
$import "convert"

fun fib(n: int a: int b: int): int {
  if n == 0 return a
  return fib(n - 1 b a + b)
}

pub fun main(): int {
  let n: int = fib(40 0 1)
  println(
    itoa(n)
  ) 
  return 0 
}
```

### **浮点数计算示例**

```gos
$import "gosio"
$import "convert"

pub fun main(): int {
  let radius: flt = 5.0
  let pi: flt = 3.14159
  let area: flt = pi * radius * radius
  println("圆的面积: ")
  # 注意：可以使用 convert 模块中的 ftoa 函数将浮点数转换为字符串
  return 0
}
```

## **🔗 FFI 与互操作性**

Gos 设计为可与 C 很好互操作。你可以声明外部函数并直接调用它们。

**在 Gos 中调用 C 函数：**

# 声明外部 C 函数

```gos
extern println(str): int

pub fun main(): int {
  println("Hello world!")
  return 0
}
```

**将 Gos 函数导出给 C：**

# 使用 `pub` 使其对链接器可见

```gos
pub fun add(x: int y: int): int {
  return x + y
}
```

## **📊 基准测试**

### 环境

- CPU: Intel i5-8265U（8 核 @ 3.900GHz）
- 内存: 8GB DDR4（7647MiB）
- 架构: x86_64
- 操作系统: Arch Linux
- 内核版本: 6.18.1-zen1-2-zen
- 桌面环境: GNOME 49.2

### 测试内容

性能测试对比了三种使用尾递归计算第 1000 个斐波那契数的实现：

- Gos Native 0.5.2：Gos 语言编译为本地可执行文件
- C 可执行文件（GCC -O3）：使用 GCC 高优化级别编译的 C
- Python 3.13.11：Python 解释执行

```
➜  fibonacci1000 ./run.sh 
Benchmark 1: ./foo
  Time (mean ± σ):     163.6 µs ±  38.8 µs    [User: 94.9 µs, System: 8.2 µs]
  Range (min … max):   121.4 µs … 2036.7 µs    17324 runs
 
  Warning: Statistical outliers were detecteConsider re-running this benchmark on a quiet system without any interferences from other programIt might help to use the '--warmup' or '--prepare' options.
 
Benchmark 2: ./a.out
  Time (mean ± σ):     479.4 µs ±  42.8 µs    [User: 267.1 µs, System: 117.8 µs]
  Range (min … max):   365.9 µs … 847.5 µs    5582 runs
 
  Warning: Statistical outliers were detecteConsider re-running this benchmark on a quiet system without any interferences from other programIt might help to use the '--warmup' or '--prepare' options.
 
Benchmark 3: python foo.py
  Time (mean ± σ):      13.6 ms ±   0.6 ms    [User: 10.7 ms, System: 2.7 ms]
  Range (min … max):    12.5 ms …  16.2 ms    223 runs
 
Summary
  ./foo ran
    2.93 ± 0.74 times faster than ./a.out
   83.32 ± 20.15 times faster than python foo.py
```

## **⚙️ CLI 参考**

```bash
The Gos programming language

Usage: gos [OPTIONS] [FILE]

Arguments:
  [FILE]  Run the Gos source file

Options:
  -a, --ast <ast>                  Print AST of the Gos source file
  -c, --compile <compile>          Compile the Gos source file to native
  -s                               Compile the Gos source file to assembly
  -o                               Compile the Gos source file to object
  -n                               Do not link the Gos Standard Library
  -p, --preprocess <preprocess>    Print the preprocessed Gos source file
  -d, --disassemble <disassemble>  Run the Gos source file
  -h, --help                       Print help
  -V, --version                    Print version
```
