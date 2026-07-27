$import "io.ah"
$import "convert.ah"

// Control Flow Example
// Demonstrates if-else, while loops, and for loops

fun main(): int {
    // If-Else statement
    let x: int = 15
    
    if x > 10 {
        println("x is greater than 10")
    } else {
        println("x is less than or eq.ah to 10")
    }
    
    // Nested if-else
    let y: int = 0
    
    if y > 0 {
        println("y is positive")
    } else {
        if y < 0 {
            println("y is negative")
        } else {
            println("y is zero")
        }
    }
    
    // While loop
    println("Counting with while loop:")
    let i: int = 0
    while i < 5 {
        println(itoa(i))
        i = i + 1
    }
    
    // For loop
    println("Counting with for loop:")
    for i in 0..5 {
        println(itoa(i))
    }
    
    // Combining loops and conditions
    println("Even numbers from 0 to 10:")
    for i in 0..11 {
        if i % 2 == 0 {
            println(itoa(i))
        }
    }
    
    return 0
}