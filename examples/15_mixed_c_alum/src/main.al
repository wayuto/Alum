import helper
import io
using io::println

// Alum/C Mixed Programming Example
// Demonstrates calling C functions from Alum

fun main(): int {
    var sum: int = c_add(10, 20)
    println(f"c_add(10, 20) = {sum}")
    
    var product: int = c_multiply(5, 6)
    println(f"c_multiply(5, 6) = {product}")
    
    var factorial: int = c_calculate_factorial(5)
    println(f"c_calculate_factorial(5) = {factorial}")
    
    return 0
}



