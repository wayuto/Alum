import io
using io::println

// Fibonacci Sequence Example
// Generates and displays Fibonacci numbers

fun main(): int {
    var n: int = 10
    var a: int = 0
    var b: int = 1
    var temp: int = 0
    var i: int = 0
    
    println("Fibonacci sequence (first 10 numbers):")
    
    while i < n {
        println(f"{a}")
        
        temp = a + b
        a = b
        b = temp
        
        i = i + 1
    }
    
    // Calculate nth Fibonacci number
    println("\nCalculating 10th Fibonacci number:")
    var fib_n: int = fibonacci(10)
    println(f"fibonacci(10) = {fib_n}")
    
    return 0
}

// Recursive function to calculate nth Fibonacci number
fun fibonacci(n: int): int {
    if n <= 1 {
        return n
    } else {
        return fibonacci(n - 1) + fibonacci(n - 2)
    }
}



