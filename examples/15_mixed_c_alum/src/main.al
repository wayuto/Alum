$import "io.ah"
$import "convert.ah"
$import "helper.ah"

// Alum/C Mixed Programming Example
// Demonstrates calling C functions from Alum

fun main(): int {
    let sum: int = c_add(10, 20)
    println(f"c_add(10, 20) = {sum}")
    
    let product: int = c_multiply(5, 6)
    println(f"c_multiply(5, 6) = {product}")
    
    let factorial: int = c_calculate_factorial(5)
    println(f"c_calculate_factorial(5) = {factorial}")
    
    return 0
}