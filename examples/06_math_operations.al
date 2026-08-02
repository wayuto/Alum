$import "io.ah"
$import "math.ah"

// Math Operations Example
// Demonstrates math module functions

fun main(): int {
    let a: int = 10
    let b: int = -5
    let c: int = 3
    
    // Basic arithmetic
    let sum: int = a + b
    let diff: int = a - b
    let product: int = a * b
    let quotient: int = a / c
    
    println("Arithmetic:")
    println(f"a + b = {sum}")
    println(f"a - b = {diff}")
    println(f"a * b = {product}")
    println(f"a / c = {quotient}")
    
    // Math functions
    println("\nMath Functions:")
    
    let abs_val: int = abs(b)
    println(f"abs(-5) = {abs_val}")

    let max_val: int = max(a, c)
    println(f"max(10, 3) = {max_val}")

    let min_val: int = min(b, c)
    println(f"min(-5, 3) = {min_val}")
    
    let square: int = sqrt(a)
    println(f"sqrt(10) = {square}")
    
    let power: int = pow(c, 2)
    println(f"pow(3, 2) = {power}")
    
    let factorial: int = fact(5)
    println(f"fact(5) = {factorial}")
    
    return 0
}