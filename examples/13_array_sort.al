$import "io.ah"

// Array Sort Example
// Demonstrates bubble sort algorithm

fun main(): int {
    var numbers: int[8] = [64, 34, 25, 12, 22, 11, 90, 45]
    var n: int = 8
    var i: int = 0
    var j: int = 0
    var temp: int = 0

    println("Original array:")
    for i in 0..n {
        print(f"{numbers[i]} ")
    }
    println("")

    // Bubble sort algorithm
    i = 0
    while i < n - 1 {
        j = 0
        while j < n - i - 1 {
            if numbers[j] > numbers[j + 1] {
                temp = numbers[j]
                numbers[j] = numbers[j + 1]
                numbers[j + 1] = temp
            }
            j = j + 1
        }
        i = i + 1
    }

    println("Sorted array:")
    for i in 0..n {
        print(f"{numbers[i]} ")
    }
    println("")

    return 0
}
