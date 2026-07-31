$import "io.ah"
$import "convert.ah"

// Arrays Example
// Demonstrates array operations with new T[N] syntax

fun main(): int {
    // Array literal with explicit values
    // Type is inferred from elements
    let numbers: int[5] = [1, 2, 3, 4, 5]

    // Access array elements
    println("First element: ")
    println(itoa(numbers[0]))

    println("Third element: ")
    println(itoa(numbers[2]))

    // Modify array elements
    numbers[0] = 10
    numbers[2] = 30

    println("After modification:")
    println("First element: ")
    println(itoa(numbers[0]))

    println("Third element: ")
    println(itoa(numbers[2]))

    // Iterate through array using for loop
    // for loop now iterates over arrays directly
    println("All elements (using for loop):")
    for num in numbers {
        println(itoa(num))
    }

    // Range expression creates an array
    println("\nRange 0..5:")
    for i in 0..5 {
        println(itoa(i))
    }

    return 0
}
