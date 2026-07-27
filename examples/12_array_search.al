$import "io.ah"
$import "convert.ah"

// Array Search Example
// Demonstrates searching for elements in an array

fun main(): int {
    let numbers: int[10] = [5, 12, 8, 25, 3, 10, 15, 7, 20, 1]
    let n: int = 10
    let target: int = 15
    let found: int = 0
    let index: int = 0
    let i: int = 0

    println("Array elements:")
    for i in 0..n {
        print(itoa(numbers[i]))
        print(" ")
    }
    println("")

    // Linear search
    print("Searching for ")
    print(itoa(target))
    println(" using linear search:")

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
        print("Found at index: ")
        println(itoa(index))
    } else {
        println("Not found")
    }

    // Find minimum and maximum
    println("\nFinding min and max:")

    let min_.ah: int = numbers[0]
    let max_.ah: int = numbers[0]

    for i in 1..n {
        if numbers[i] < min_.ah {
            min_.ah = numbers[i]
        }
        if numbers[i] > max_.ah {
            max_.ah = numbers[i]
        }
    }

    print("Minimum: ")
    println(itoa(min_.ah))

    print("Maximum: ")
    println(itoa(max_.ah))

    return 0
}
