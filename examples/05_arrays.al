$import "io.ah"

// Arrays Example
// Demonstrates array operations with new T[N] syntax

fun main(): int {
    // Array literal with explicit values
    // Type is inferred from elements
    let numbers: int[5] = [1, 2, 3, 4, 5]

    // Access array elements
    println(f"First element: {numbers[0]}")

    println(f"Third element: {numbers[2]}")

    // Modify array elements
    numbers[0] = 10
    numbers[2] = 30

    println("After modification:")
    println(f"First element: {numbers[0]}")

    println(f"Third element: {numbers[2]}")

    // Iterate through array using for loop
    // for loop now iterates over arrays directly
    println("All elements (using for loop):")
    for num in numbers {
        println(f"{num}")
    }

    // Range expression creates an array
    println("\nRange 0..5:")
    for i in 0..5 {
        println(f"{i}")
    }

    return 0
}
