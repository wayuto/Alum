$import "io.ah"

// Factorial Comparison Example
// Compares iterative vs recursive factorial implementations

// Iterative factorial
fun factorial_iterative(n: int): int {
    let result: int = 1
    let i: int = 1
    
    while i <= n {
        result = result * i
        i = i + 1
    }
    
    return result
}

// Recursive factorial
fun factorial_recursive(n: int): int {
    if n <= 1 {
        return 1
    } else {
        return n * factorial_recursive(n - 1)
    }
}

fun main(): int {
    let num: int = 6

    println("Factorial Comparison:")
    println(f"Number: {num}")

    let iter_result: int = factorial_iterative(num)
    let recur_result: int = factorial_recursive(num)
    
    println(f"Iterative: {iter_result}")
    println(f"Recursive: {recur_result}")
    
    if iter_result == recur_result {
        println("Both methods produce the same result!")
    } else {
        println("Results differ!")
    }
    
    return 0
}