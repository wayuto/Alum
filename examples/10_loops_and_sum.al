$import "io.al"
$import "convert.al"

// Loops and Sum Example
// Demonstrates calculating sum of numbers using loops

fun main(): int {
    // Sum of first 10 natural numbers using while loop
    println("Sum of first 10 natural numbers (while loop):")
    
    let i: int = 1
    let sum: int = 0
    
    while i <= 10 {
        sum = sum + i
        i = i + 1
    }
    
    print("Sum = ")
    println(itoa(sum))
    
    // Sum of first 10 natural numbers using for loop
    println("\nSum of first 10 natural numbers (for loop):")
    
    sum = 0
    for i in 1..11 {
        sum = sum + i
    }
    
    print("Sum = ")
    println(itoa(sum))
    
    // Sum of even numbers from 1 to 20
    println("\nSum of even numbers from 1 to 20:")
    
    sum = 0
    for i in 1..21 {
        if i % 2 == 0 {
            sum = sum + i
        }
    }
    
    print("Sum = ")
    println(itoa(sum))
    
    // Factorial of 5 using while loop
    println("\nFactorial of 5:")
    
    i = 5
    let factorial: int = 1
    
    while i > 0 {
        factorial = factorial * i
        i = i - 1
    }
    
    print("5! = ")
    println(itoa(factorial))
    
    return 0
}