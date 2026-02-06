$import "io.al"
$import "convert.al"
$import "helper.al"

// Alum/C Mixed Programming Example
// Demonstrates calling C functions from Alum

fun main(): int {
    let sum: int = c_add(10, 20)
    print("c_add(10, 20) = ")
    println(itoa(sum))
    
    let product: int = c_multiply(5, 6)
    print("c_multiply(5, 6) = ")
    println(itoa(product))
    
    let factorial: int = c_calculate_factorial(5)
    print("c_calculate_factorial(5) = ")
    println(itoa(factorial))
    
    return 0
}