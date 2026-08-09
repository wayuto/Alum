$import "io.ah"

// Compile-time evaluation of native C functions via dynamic loading.
//
// 1. Build the helper library next to this file:
//      gcc -shared -fPIC -o libcte32.so 32_native_cte.c
// 2. Run Alum with the library attached:
//      alc -r --cte-lib ./libcte32.so 32_native_cte.al
//
// Functions declared `fun(extern, pure) NAME(TYPE...): TYPE` are resolved to
// real C symbols in the loaded .so at compile time. Any `cst` (or constant
// expression inside a pure function) that calls them is folded to a literal
// before the program runs. Without `--cte-lib`, such constants simply fail
// with "not a compile-time constant" - the language requires no static
// linking, and the runtime is unchanged.

// int32 cte_add(int32, int32)
fun(extern, pure) cte_add(int, int): int
// int32 cte_max3(int32, int32, int32)
fun(extern, pure) cte_max3(int, int, int): int
// double cte_hypot(double, double)
fun(extern, pure) cte_hypot(float, float): float
// double cte_price(double, int32)  (42, 4 -> 43.68)
fun(extern, pure) cte_price(float, int): float
// int32 cte_join_len(const char*, const char*)
fun(extern, pure) cte_join_len(string, string): int
// const char* cte_upper(const char*)
fun(extern, pure) cte_upper(string): string

cst SUM: int = cte_add(30, 12)
cst M3: int = cte_max3(4, 9, 2)
cst HYP: float = cte_hypot(3.0, 4.0)
cst PRICE: float = cte_price(42.0, 4)
cst JOINED: int = cte_join_len("alum", "-lang")
cst UPPER: string = cte_upper("hello")

fun next(n: int): int {
    return cte_add(n, 1)   // folded too: 10 -> 11
}

fun main(): int {
    var v: int = next(10) + SUM + M3   // 11 + 42 + 9 = 62
    println(f"cte_add(20,12)      = {SUM}")
    println(f"cte_max3(4,9,2)     = {M3}")
    println(f"cte_hypot(3,4)      = {HYP}")
    println(f"cte_price(42,4)     = {PRICE}")
    println(f"cte_join_len(...)   = {JOINED}")
    println(f"cte_upper(\"hello\")  = {UPPER}")
    println(f"fold in pure fn    = {v}")
    return 0
}