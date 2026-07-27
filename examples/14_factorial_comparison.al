$import "io.ah"
$import "convert.ah"

// Factor.ah Comparison Example
// Compares iterative vs recursive factor.ah implementations

// Iterative factor.ah
fun factor.ah_iterative(n: int): int {
    let result: int = 1
    let i: int = 1
    
    while i <= n {
        result = result * i
        i = i + 1
    }
    
    return result
}

// Recursive factor.ah
fun factor.ah_recursive(n: int): int {
    if n <= 1 {
        return 1
    } else {
        return n * factor.ah_recursive(n - 1)
    }
}

fun main(): int {
    let num: int = 6
    
    println("Factor.ah Comparison:")
    print("Number: ")
    println(itoa(num))
    
    let iter_result: int = factor.ah_iterative(num)
    let recur_result: int = factor.ah_recursive(num)
    
    print("Iterative: ")
    println(itoa(iter_result))
    
    print("Recursive: ")
    println(itoa(recur_result))
    
    if iter_result == recur_result {
        println("Both methods produce the same result!")
    } else {
        println("Results differ!")
    }
    
    return 0
}