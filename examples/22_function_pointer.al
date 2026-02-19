// Test: Function Pointer Assignment and Call
$import "io.al"
$import "convert.al"

fun add_numbers(a: int, b: int): int {
    return a + b
}

fun main(): int {
    // Test 1: Assign function to function pointer variable
    let f: string(int) = itoa

    // Test 2: Call through function pointer
    let result: string = f(10)

    println("Function pointer test:")
    println("f(10) = ")
    println(result)

    // Test 3: Another function pointer
    let g: int(int, int) = add_numbers
    let sum: int = g(5, 7)

    println("g(5, 7) = ")
    println(itoa(sum))

    println("Function pointer test completed successfully!")

    return 0
}