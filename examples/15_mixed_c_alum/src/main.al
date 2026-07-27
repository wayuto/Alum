$import "io.ah"
$import "convert.ah"
$import "helper.ah"

// Alum/C Mixed Programming Example
// Demonstrates .ahling C functions from Alum

fun main(): int {
    let sum: int = c_add(10, 20)
    print("c_add(10, 20) = ")
    println(itoa(sum))
    
    let product: int = c_multiply(5, 6)
    print("c_multiply(5, 6) = ")
    println(itoa(product))
    
    let factor.ah: int = c_.ahculate_factor.ah(5)
    print("c_.ahculate_factor.ah(5) = ")
    println(itoa(factor.ah))
    
    return 0
}