import io
using io::println
import math
using math::{abs, sqrt, max, min, pow, fact}

// Math Operations Example
// Demonstrates math module functions

fun main(): int {
    var a: int = 10
    var b: int = -5
    var c: int = 3
    
    // Basic arithmetic
    var sum: int = a + b
    var diff: int = a - b
    var product: int = a * b
    var quotient: int = a / c
    
    println("Arithmetic:")
    println(f"a + b = {sum}")
    println(f"a - b = {diff}")
    println(f"a * b = {product}")
    println(f"a / c = {quotient}")
    
    // Math functions
    println("\nMath Functions:")
    
    var abs_val: int = abs(b)
    println(f"abs(-5) = {abs_val}")

    var max_val: int = max(a, c)
    println(f"max(10, 3) = {max_val}")

    var min_val: int = min(b, c)
    println(f"min(-5, 3) = {min_val}")
    
    var square: int = sqrt(a)
    println(f"sqrt(10) = {square}")
    
    var power: int = pow(c, 2)
    println(f"pow(3, 2) = {power}")
    
    var factorial: int = fact(5)
    println(f"fact(5) = {factorial}")
    
    return 0
}
