$import "io.al"
$import "convert.al"

// Arrays Example
// Demonstrates array operations

fun main(): int {
    // Array literal with explicit values
    let numbers: arr[int] = [1, 2, 3, 4, 5]
    
    // Array with fill syntax (creates array of specified size)
    let buffer: arr[int] = [int; 10]
    
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
    
    // Iterate through array
    println("All elements:")
    for i in 0..5 {
        println(itoa(numbers[i]))
    }
    
    return 0
}