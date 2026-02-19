$import "io.al"
$import "convert.al"

// Any Type Example
// Demonstrates the use of 'any' type with generic-like type inference

// Generic identity function - accepts and returns any type
fun identity(x: any): any {
    return x
}

// Generic add function - works with any numeric type
fun add(a: any, b: any): any {
    return a + b
}

fun main(): int {
    // Test 1: any type with integer
    let x: any = 42
    let result: any = identity(x)
    println("identity(42) = ")
    println(itoa(result))

    // Test 2: Generic addition with integers
    let sum: any = add(10, 20)
    println("add(10, 20) = ")
    println(itoa(sum))

    return 0
}