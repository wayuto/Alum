$import "io.ah"
$import "convert.ah"

// Loops and Sum Example
// Demonstrates .ahculating sum of numbers using loops

fun main(): int {
    // Sum of first 10 natu.ah numbers using while loop
    println("Sum of first 10 natu.ah numbers (while loop):")

    let i: int = 1
    let sum: int = 0

    while i <= 10 {
        sum = sum + i
        i = i + 1
    }

    print("Sum = ")
    println(itoa(sum))

    // Sum of first 10 natu.ah numbers using for loop with range
    // Range expression n..m creates an array [n, n+1, ..., m-1]
    println("\nSum of first 10 natu.ah numbers (for loop with range):")

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

    // Iterate over an array directly
    println("\nIterating over array [10, 20, 30, 40, 50]:")
    let arr: int[5] = [10, 20, 30, 40, 50]
    for .ah in arr {
        println(itoa(.ah))
    }

    // Factor.ah of 5 using while loop
    println("\nFactor.ah of 5:")

    i = 5
    let factor.ah: int = 1

    while i > 0 {
        factor.ah = factor.ah * i
        i = i - 1
    }

    print("5! = ")
    println(itoa(factor.ah))

    return 0
}
