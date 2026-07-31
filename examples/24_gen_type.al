$import "io.ah"
$import "convert.ah"

// Generic Type Example
// Demonstrates generic functions with type parameters

// Generic identity function - works for any type T
fun identity<T>(x: T): T {
    return x
}

fun main(): int {
    // Test 1: identity with integer
    let x: int = 42
    let result: int = identity(x)
    println("identity(42) = ")
    println(itoa(result))

    // Test 2: identity with float
    let y: float = 3.14
    let f_result: float = identity(y)
    println("identity(3.14) = ")
    println(ftoa(f_result))

    return 0
}
