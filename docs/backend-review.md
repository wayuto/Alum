# Alum 编译器后端代码审查报告

> 审查日期:2026-08-12,范围:`src/compiler/{irgen, codegen, bytecode}` + `src/cli` + `src/main.rs`(约 1.5 万行)。全部为只读审查结论。

## 0. 架构总览

```
前端: preprocessor → lexer → parser → visitor/checker(类型检查) → visitor/optimizer
后端: irgen(IR 生成 + CTFE) → codegen(x86-64 汇编) → ELF .o
旁路: bytecode(独立栈式 VM,仅供编译期常量求值 CTFE 使用)
```

**模块定位**:主后端是 `irgen` + `codegen`。`bytecode` 是一套与 irgen 共享同一份 AST 的"第二套后端",唯一生产用途是 CTFE(`irgen/irgen.rs` 的 `eval_const_vm` 用 GVM 跑纯函数)。

## 1. 必须修复的正确性问题(按严重度排序)

| # | 问题 | 位置 | 后果 |
|---|---|---|---|
| 1 | **TAILCALL 与 memo 缓存交互**(已修复,防御性) | `bytecode/gvm.rs:431-450` + `457-459` | 原判断"尾递归会在 CTFE 中静默算错"**实测未复现**(纯函数确定性强,最外层 key 的 memo 值恒等于该次调用结果);已在 TAILCALL 时清除当前帧 `cache_key`,防止未来 TAILCALL 用于非自身调用或 memo 扩展后引入错误缓存。 |
| 2 | **链接成功后删除用户自己的 .o 文件** | `cli/link.rs:66-73` | `main.rs:46-48` 把用户命令行传入的 `.o/.obj/.a` 混入 `obj_files`,链接后这些用户文件被 `remove_file` 删除。已改为仅删除编译器生成的临时目标文件。 |
| 3 | CTFE 吞掉所有 panic | `irgen/irgen.rs:382-389` `catch_unwind` | VM 内部 bug 被静默转成"不是常量";且 GVM 编译期执行**无超时** ,纯函数死循环会挂死编译器。 |

## 2. 结构性问题:职责混杂的函数

| 函数 | 位置 | 混杂的职责 |
|---|---|---|
| `IRGen::compile` | `irgen/irgen.rs:20-141` | 纯函数检查 + lambda 提升 + 编译器内部告警(eprintln)+ native 签名解析 + 全局声明收集 + 全局常量/变量存储 + 函数编译 + 顶层游离表达式编译(把顶层 `Int/Float/.../Var` 包成 `VarDecl("_global", ...)` 强制求值的未注释黑招) |
| `compile_code` | `codegen/compile_code.rs:11-662` | 653 行的巨型 match,任一 op 的编译逻辑都在里面 |
| `Bytecode::print`(死代码) | `bytecode/compiler.rs:22-110` | 整个反汇编器,`#[allow(dead_code)]`,无调用者 |
| `write_elf` | `codegen/asm/elf.rs:4-273` | 符号收集、字符串表、重定位、布局、EHDR/SHDR 全部在一个函数 |
| `compile_fn` | `codegen/compile_fn.rs:18-204` | 栈帧布局 + 序言 + 参数搬运 + 指令分发 + 尾声 |
| `run` | `main.rs:24-164` | 两段几乎相同(72-102 / 132-161)的链接逻辑;`-c -o` 多输入静默覆盖 |

## 3. 大文件与拆分建议

| 文件 | 行数 | 建议 |
|---|---|---|
| `irgen/expr.rs` | 3040 | `compile_expr`(739-2729)约 2000 行大 match。按语法类别拆:`literal.rs`、`var.rs`(Decl/Assign/Read)、`binop.rs`、`control.rs`(If/While/For)、`call.rs`(Call/Index/IndexAssign)、`match.rs`、`type_info.rs`(类型查询族 2731-3039) |
| `irgen/irgen.rs` | 993 | `eval_const`/`eval_const_vm`/`vm_value_to_const`(249-438)→ `const_eval.rs`;`VmSafety`(713-993)→ `vm_safety.rs`;`expr_has_var`+`collect_var_refs`(440-711)→ `ast_util.rs` |
| `bytecode/gvm.rs` | 541 | 450 行巨型 match,算术指令 `pop/pop/match` 模板重复约 30 次;`num_cmp`(535)闭包参数化写法值得推广 |
| `bytecode/compiler.rs` | 732 | 符号表+栈帧分配(112-203)应抽独立模块;`compile_func`(662)与 `Expr::Lambda` 分支(607-646)几乎重复 |
| `codegen/asm/encoder.rs` | 745 | `emit_sized`(150-367)218 行巨型 match 按指令族拆分;20 处 `section: &str` 参数可收敛为 `ByteEmitter` 结构体 |
| `codegen/compile_code.rs` | 664 | 653 行巨型 match 按 op 族拆成方法与约束,`compile_code` 只做分发 |
| `codegen/asm/elf.rs` | 308 | `write_elf` 单函数拆 `build_symbols`/`build_strtab`/`build_rela`/`compute_layout`/`write_ehdr`/`write_shdrs` |

## 4. 代码重复

1. **`key()` 三份逐字节相同**(`compile_fn.rs:9-15`、`operand.rs:9-15`、`regalloc.rs:6-12`):IR 操作数 → 字符串键,应移到 `irgen/ir.rs` 的 `Operand` 方法。
2. **struct/union literal 编译几乎逐行相同**(`expr.rs:12-55` vs `57-94`):差异仅 union 固定 size=8/offset=0/无 resource copy。
3. **AST walker 家族 9 个**(`purity.rs` 两个、`irgen.rs` 的 `expr_has_var`/`collect_var_refs`/`VmSafety::safe`、`func.rs::substitute_expr`、`lambda.rs::hoist_lambdas`、`expr.rs::expr_high_type`、`optimizer.rs`):每个都是对 30+ 变体的机械遍历,`expr_has_var` 与 `collect_var_refs` 结构几乎逐行对应。
4. **"取 const → new_tmp → Move" 字面量发射模式重复 8 次**(`expr.rs:745-805, 1011-1035, 2353-2379, 2482-2491`):应抽 `emit_const_move`。
5. **compile_code.rs 8 个 move 系臂同构**(13-68)、**三条比较臂同构**(160-205)、**ArrayAccess/ByteAccess、LoadAt/StoreAt 互为镜像**(448-609)。
6. **jump 地址 16 位手工编解码**:compiler.rs 编码 6 处(`(x>>8)&0xFF, x&0xFF`)+ gvm.rs 解码 5 处(`(h<<8)|l`)。
7. **Expr→Op 映射三处重复**:bytecode compiler.rs:276-345、irgen.rs:528-560、irgen/expr.rs:1044+。
8. **常量不共享**:bytecode `add_const`(compiler.rs:205)不去重,irgen 有 `constant_pool` 去重(irgen.rs:518)。
9. **主后端与 bytecode 前端的结构性重复**:同一 AST 的两套表达式遍历、两套作用域管理、两套常量语义。

## 5. 错误处理问题

- **`unwrap()/expect()` 约 140 处**,集中在 `codegen/compile_code.rs`(约 81 处)与 `bytecode/(compiler.rs:10, gvm.rs:30 处 panic)`。多数依赖 IRGen 不变量,但任一改动会从"编译错误"降级为"静默 panic"。
- **错误体系三层割裂**:主流水线 `CompilerError`;codegen/irgen 共用 `CodeGenError`(却放在 `codegen/error.rs`);bytecode 层完全绕过,靠 `catch_unwind` + 字符串前缀 panic。
- **假 span 遍布**:`Span::new(0, 0)`(context.rs:98、func.rs:116/144、expr.rs:1039、irgen.rs:529 等),用户拿不到源码位置。
- **静默失败**:`arena`/`elf.rs:136` `sym_map.get(..).unwrap_or(0)` 重定位目标缺失写符号索引 0;`compile_code.rs:413-415` `Op::Jump` 非 Label 时直接不发指令;`Context::slot()` 查找失败静默返回 name。
- bytecode 层错误:**`gvm.rs` 的 `Op::EXIT` 会直接 `std::process::exit`,若出现在 CTFE 字节码中会杀死编译器进程**(当前编译器从不发射该操作码,属死代码,但保留是隐患)。

## 6. 死代码清单

| 死代码 | 位置 | 说明 |
|---|---|---|
| `IRFunction.is_pure`(`#[allow(dead_code)]`) | `ir.rs:117-118` | 只写不读 |
| `Symbol.name`(`#[allow(dead_code)]`) | `context.rs:11-12` | 只写不读 |
| `IRGen.mono_in_progress` | `mod.rs:28` | push/pop 但从未读取,**泛型递归单态化保护疑似失效** |
| `Bytecode::print` + `Op::to_str` + `Op::operand_count` | `bytecode/compiler.rs:22-110`、`bytecode.rs:162-272` | 连锁死代码 |
| 操作码 `POS`/`AND`/`OR`/`EXIT` | `bytecode.rs` + `gvm.rs:261-298,525-528` | 编译器从不发射;`EXIT` 会 exit 进程 |
| `compile_code.rs` 的 `Op::Label`/`Op::Return` 臂 | 407-411 / 655-661 | `compile_fn.rs:177-193` 已提前拦截,不可达 |
| `Mem.size` 字段 | `asm/types.rs:146-157` | 只写不读,编码由指令种类决定 |
| `Asm::Extern("memcpy")` | `codegen.rs:195` | 全库无调用 |
| `regalloc.rs:14` `is_float_op` 的 `_params` 参数、`encoder.rs:448` `_op_reg`、`peephole.rs:241` 空 if | — | 无人使用 |
| `codegen.rs:117` `regs: HashMap<Reg, Option<IROperand>>` 的 Option | — | 从不插入 None |

## 7. 反模式

- **字符串 key 的 HashMap 泛滥**:`IRGen` 15 个字段里 8 个 `HashMap<String, _>`;`find_func` 在 `Vec<IRFunction>` 上线性扫描且**每次克隆整个函数**(func.rs:108-118);codegen 侧 `format!("_tmp_{id}")` 每访问一次重格式化。
- **全局变量两套映射**:`globals` 与 `global_storage` 并存,`Var` 解析要在 4 个数据结构里依次试错(expr.rs:996-1041)。
- **魔法字符串 lambda 标记三套并存**:`"_lambda_{n}"`(lambda.rs:14、irgen.rs:347 等 4 处)、`VM_LAMBDA_MARKER`(irgen.rs:563)、`LAMBDA_MARKER`/`IMPURE_LAMBDA_MARKER`(purity.rs:5-6)。
- **`Op::Return(String)` 携带平台寄存器名**(ir.rs:83、func.rs:89-96、expr.rs:1484):IR 层泄漏 x86 细节。
- **环境变量依赖**:`ALC_DEBUG_IR`(func.rs:78-83)行为不可预测;`Box::leak`(irgen.rs:741)每次 resolve 泄漏内存。
- **0x48|0x01 手拼 REX 无注释**(encoder.rs:155)、栈帧双重对齐 `& !15`(compile_fn.rs:97-102)、各处裸字节操作码。

## 8. 命名问题

- `type2ir_type`(context.rs:102)→ `type_to_ir_type`;`_field_name` 实际被使用(expr.rs:2525/2529);`let _ = ct;`(irgen.rs:199)、`let _span`(irgen.rs:529)死绑定;`compile`/`compile_fn`/`compile_expr` 三级命名无层级暗示;`rel(lbl)` → `data_label()`;`curr_flt_reg` → `flt_arg_count`;`regs` 实为"寄存器缓存";`alloc_regs`(寄存器分配)与 `alloc_str/flt/arr`(数据段常量)共用 `alloc` 前缀但语义完全不同;三个失效函数 `invalidate_caller_saved_regs/_xmm/volatile_registers` 描述同一件事,compile_code.rs:373-393 又手写第四份;bytecode 的 `compile_expr` 与 irgen 的 `compile_expr` 同名不同签名;`Value::Void` 同时充当 nil/unit/空值。

## 9. CLI 层问题

- `exec_run` 丢失被编译程序的退出码(build.rs:133,`.status()?` 只检查能否启动)。
- `CompilerError::new` 的 `src`/`input` 参数被忽略(build.rs 每次白克隆两份大字符串)。
- `input.replace(".al", "")` 替换所有子串(build.rs:105)。
- 链接器路径硬编码 glibc/musl 候选列表(link.rs:35-39)。
- 三份重复的"exe 名派生"逻辑(build.rs:104-108、main.rs:110-114、main.rs:137-141)。

## 10. 建议的重构优先级

1. **修复正确性 bug**(表 1 的 1-3)。
2. **拆分 `expr.rs` 与 `irgen.rs`**(表 3),`compile_expr` 按语法类别拆文件。
3. **bytecode 死代码清理 + `#[repr(u8)]`**:删除 `POS/AND/OR/EXIT` 与 `print`/`to_str`/`operand_count`,用 `TryFrom<u8>` 消除 4 处同步维护。
4. **`key()` 三份拷贝合并为 `Operand` 方法;`find_func`/克隆问题**。
5. **错误类型统一**:bytecode 层接入 `CompilerError` 或至少保留诊断;清除假 span。
6. **CLI 层**:main.rs 链接逻辑合并、exec_run 退出码、死参数。

## 11. 修复记录(2026-08-12,全部经 `cargo build` 0 警告 + 全示例回归验证)

| 项 | 状态 |
|---|---|
| 表 1 正确性 1-2(TAILCALL memo、用户 .o 删除) | 已修复 |
| Table 3 拆分:`expr.rs` → `control.rs`/`call.rs`/`array.rs`/`match_expr.rs`/`type_info.rs`;`irgen.rs` → `const_eval.rs`/`vm_safety.rs`/`globals.rs` | 已拆分 |
| bytecode 死码 + `#[repr(u8)]` + TryFrom | 已做 |
| `key()` 三份拷贝 → `Operand::key()`(ir.rs) | 已做 |
| 死字段/死代码:`is_pure`、`Symbol.name`、`mono_in_progress`、`Mem.size`、`Asm::Extern("memcpy")`、`Op::Label/Return` 不可达臂(留 `unreachable!` 兜底)、`is_float_op._params`、`emit_binop._op_reg`、peephole 空 if、`regs` 的 Option 包装 | 已清 |
| regalloc.rs:391/396 双引用 clone 警告 | 已修 |
| CLI:`CompilerError::new` 死参数、exec_run 退出码、`input.replace(".al")` | 已修 |
| 表 1 第 3 项(CTFE 吞 panic、无超时) | **未动**:改动风险高,需配套错误体系重构 |
| 其余表 4/表 5/表 8 的重复与命名项 | 保留,列为后续重构候选 |