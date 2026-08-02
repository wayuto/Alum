$import "io.ah"

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

    println(f"Sum = {sum}")

    // Sum of first 10 natural numbers using for loop with range
    // Range expression n..m creates an array [n, n+1, ..., m-1]
    println("\nSum of first 10 natural numbers (for loop with range):")

    sum = 0
    for i in 1..11 {
        sum = sum + i
    }

    println(f"Sum = {sum}")

    // Sum of even numbers from 1 to 20
    println("\nSum of even numbers from 1 to 20:")

    sum = 0
    for i in 1..21 {
        if i % 2 == 0 {
            sum = sum + i
        }
    }

    println(f"Sum = {sum}")

    // Iterate over an array directly
    println("\nIterating over array [10, 20, 30, 40, 50]:")
    let arr: int[5] = [10, 20, 30, 40, 50]
    for val in arr {
        println(f"{val}")
    }

    // Factorial of 5 using while loop
    println("\nFactorial of 5:")

    i = 5
    let factorial: int = 1

    while i > 0 {
        factorial = factorial * i
        i = i - 1
    }

    println(f"5! = {factorial}")

    return 0
}
