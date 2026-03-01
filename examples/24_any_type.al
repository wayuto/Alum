$import "io.al"
$import "convert.al"

// Gen Type Example
// Demonstrates the use of 'gen' type with generic-like type inference

// Generic identity function - accepts and returns gen type
fun identity(x: gen): gen {
    return x
}

// Generic add function - works with gen numeric type
fun add(a: gen, b: gen): gen {
    return a + b
}

fun main(): int {
    // Test 1: gen type with integer
    let x: gen = 42
    let result: gen = identity(x)
    println("identity(42) = ")
    println(itoa(result))

    // Test 2: Generic addition with integers
    let sum: gen = add(10, 20)
    println("add(10, 20) = ")
    println(itoa(sum))

    return 0
}