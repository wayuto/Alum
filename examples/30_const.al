$import "io.ah"

// Compile-time constants: `cst NAME: T = expr`
// - Global `cst` declarations must have compile-time constant initializers.
// - Local `cst` declarations are immutable; reassignment is rejected by the checker.

cst MAX: int = 100
cst PI: float = 3.14
cst GREETING: string = "hello"
cst FLAG: bool = true
cst BASE: int = 10
cst SUM: int = MAX + 5        // constant expressions may reference other constants
cst HALF: float = PI / 2.0
cst NEG: int = -7
cst NEGF: float = -2.5

fun area(r: float): float {
    cst TWO: float = 2.0
    var scaled: float = r * TWO
    return scaled
}

fun main(): int {
    cst LOCAL: int = 42
    var total: int = LOCAL + MAX + SUM + NEG
    println(f"MAX = {MAX}")
    println(f"PI = {PI}")
    println(f"GREETING = {GREETING}")
    println(f"FLAG = {FLAG}")
    println(f"SUM = {SUM}")
    println(f"HALF = {HALF}")
    println(f"NEG = {NEG}")
    println(f"NEGF = {NEGF}")
    println(f"LOCAL = {LOCAL}")
    println(f"total = {total}")
    println(f"area(3.0) = {area(3.0)}")
    return 0
}
