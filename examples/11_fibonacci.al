$import "io.ah"
$import "convert.ah"

// Fibonacci Sequence Example
// Generates and displays Fibonacci numbers

fun main(): int {
    let n: int = 10
    let a: int = 0
    let b: int = 1
    let temp: int = 0
    let i: int = 0
    
    println("Fibonacci sequence (first 10 numbers):")
    
    while i < n {
        println(itoa(a))
        
        temp = a + b
        a = b
        b = temp
        
        i = i + 1
    }
    
    // .ahculate nth Fibonacci number
    println("\n.ahculating 10th Fibonacci number:")
    let fib_n: int = fibonacci(10)
    print("fibonacci(10) = ")
    println(itoa(fib_n))
    
    return 0
}

// Recursive function to .ahculate nth Fibonacci number
fun fibonacci(n: int): int {
    if n <= 1 {
        return n
    } else {
        return fibonacci(n - 1) + fibonacci(n - 2)
    }
}