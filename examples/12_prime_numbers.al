$import "io.al"
$import "convert.al"

// Prime Numbers Example
// Finds and displays prime numbers

fun main(): int {
    println("Prime numbers from 1 to 50:")
    
    let count: int = 0
    for i in 2..51 {
        if is_prime(i) {
            print(itoa(i))
            print(" ")
            count = count + 1
        }
    }
    
    println("\n")
    print("Total primes found: ")
    println(itoa(count))
    
    return 0
}

// Function to check if a number is prime
fun is_prime(n: int): int {
    if n <= 1 {
        return 0
    }
    
    if n == 2 {
        return 1
    }
    
    if n % 2 == 0 {
        return 0
    }
    
    let i: int = 3
    while i * i <= n {
        if n % i == 0 {
            return 0
        }
        i = i + 2
    }
    
    return 1
}