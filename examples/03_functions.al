$import "io.ah"

// Functions Example
// Demonstrates function definitions and calls

// Function that adds two integers
fun add(a: int, b: int): int {
    return a + b
}

// Function that multiplies two integers
fun multiply(a: int, b: int): int {
    return a * b
}

// Function that returns the larger of two numbers
fun get_max(a: int, b: int): int {
    if a > b {
        return a
    } else {
        return b
    }
}

// Function with no return value
fun greet(name: string): void {
    print("Hello, ")
    println(name)
}

fun main(): int {
    let x: int = 10
    let y: int = 20
    
    let sum: int = add(x, y)
    let product: int = multiply(x, y)
    let maximum: int = get_max(x, y)
    
    greet("Alum")
    
    println(f"Sum: {sum}")
    println(f"Product: {product}")
    println(f"Maximum: {maximum}")
    
    return 0
}