import io
using io::println

// `fun(extern, pure)` declares call into the native shared library built from
// native/cte_native.c. The compiler emits a warning that an extern function's
// purity cannot be verified:
//   warning: purity of external function '<name>' cannot be verified

fun(extern, pure) cte_add(int, int): int
fun(extern, pure) cte_max3(int, int, int): int
fun(extern, pure) cte_hypot(float, float): float
fun(extern, pure) cte_price(float, int): float
fun(extern, pure) cte_join_len(string, string): int
fun(extern, pure) cte_upper(string): string

// Constant initializers are folded at compile time (the .so is attached via
// `--cte-lib` by `alumake`).
cst SUM: int = cte_add(30, 12)
cst M3: int = cte_max3(4, 9, 2)
cst HYP: float = cte_hypot(3.0, 4.0)
cst PRICE: float = cte_price(42.0, 4)
cst JOINED: int = cte_join_len("alum", "-lang")
cst UPPER: string = cte_upper("hello")

fun next(n: int): int {
    return cte_add(n, 1)   // folds here too: 10 -> 11
}

fun main(): int {
    var v: int = next(10) + SUM + M3   // 11 + 42 + 9 = 62
    println(f"cte_add(30,12)      = {SUM}")
    println(f"cte_max3(4,9,2)     = {M3}")
    println(f"cte_hypot(3,4)      = {HYP}")
    println(f"cte_price(42,4)     = {PRICE}")
    println(f"cte_join_len(...)   = {JOINED}")
    println(f"cte_upper(\"hello\")  = {UPPER}")
    println(f"fold in pure fn    = {v}")
    return 0
}



