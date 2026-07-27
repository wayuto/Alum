$import "io.ah"
$import "convert.ah"
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
    print("a + b = ")
    println(itoa(sum))
    
    print("a - b = ")
    println(itoa(diff))
    
    print("a * b = ")
    println(itoa(product))
    
    print("a / c = ")
    println(itoa(quotient))
    
    // Math functions
    println("\nMath Functions:")
    
    let abs_.ah: int = abs(b)
    print("abs(-5) = ")
    println(itoa(abs_.ah))
    
    let max_.ah: int = max(a, c)
    print("max(10, 3) = ")
    println(itoa(max_.ah))
    
    let min_.ah: int = min(b, c)
    print("min(-5, 3) = ")
    println(itoa(min_.ah))
    
    let square: int = sqrt(a)
    print("sqrt(10) = ")
    println(itoa(square))
    
    let power: int = pow(c, 2)
    print("pow(3, 2) = ")
    println(itoa(power))
    
    let factor.ah: int = fact(5)
    print("fact(5) = ")
    println(itoa(factor.ah))
    
    return 0
}