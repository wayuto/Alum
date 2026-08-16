import io
using io::{print, println}

// Array Search Example
// Demonstrates searching for elements in an array

fun main(): int {
    var numbers: int[10] = [5, 12, 8, 25, 3, 10, 15, 7, 20, 1]
    var n: int = 10
    var target: int = 15
    var found: int = 0
    var index: int = 0
    var i: int = 0

    println("Array elements:")
    for i in 0..n {
        print(f"{numbers[i]} ")
    }
    println("")

    // Linear search
    println(f"Searching for {target} using linear search:")

    i = 0
    found = 0
    while i < n {
        if numbers[i] == target {
            found = 1
            index = i
            break
        }
        i = i + 1
    }

    if found == 1 {
        println(f"Found at index: {index}")
    } else {
        println("Not found")
    }

    // Find minimum and maximum
    println("\nFinding min and max:")

    var min_val: int = numbers[0]
    var max_val: int = numbers[0]

    for i in 1..n {
        if numbers[i] < min_val {
            min_val = numbers[i]
        }
        if numbers[i] > max_val {
            max_val = numbers[i]
        }
    }

    println(f"Minimum: {min_val}")
    println(f"Maximum: {max_val}")

    return 0
}




