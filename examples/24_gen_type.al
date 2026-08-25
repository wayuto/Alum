import io
using io::println

// Generic Type Example
// Demonstrates generic functions with type parameters

// Generic identity function - works for any type T
fun identity<T>(x: T): T {
    return x
}

fun main(): int {
    // Test 1: identity with integer
    var x: int = 42
    var result: int = identity(x)
    println(f"identity(42) = {result}")

    // Test 2: identity with float
    var y: float = 3.14
    var f_result: float = identity(y)
    println(f"identity(3.14) = {f_result}")

    return 0
}
