$import "io.ah"

// Global variables and exported constants
// - `cst(pub)`  exports a read-only data symbol (linkable via `extern`)
// - `var(pub)`  exports a mutable global variable (linkable via `extern`)
// - `var`       is a file-local global variable (internal linkage)
// - `cst`       is a compile-time constant (inlined, no symbol)

cst(pub) LIMIT: int = 100
cst GREETING: string = "hello"
var(pub) counter: int = 0
var ratio: float = 1.5
var zero: int

fun main(): int {
    counter = counter + 3
    zero = zero + 7
    ratio = ratio * 2.0
    println(f"LIMIT = {LIMIT}")
    println(f"GREETING = {GREETING}")
    println(f"counter = {counter}")
    println(f"zero = {zero}")
    println(f"ratio = {ratio}")
    return 0
}
