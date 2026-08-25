// Test: Function Pointer Assignment and call
import io
using io::println
import convert
using convert::itoa

fun add(a: int, b: int): int {
    return a + b
}

fun main(): int {
    // Test 1: Assign function to function pointer variable
    var f: string(int) = itoa

    // Test 2: Call through function pointer
    var result: string = f(10)

    println("Function pointer test:")
    println(f"f(10) = {result}")

    // Test 3: Another function pointer
    var g: int(int, int) = add
    var sum: int = g(5, 7)

    println(f"g(5, 7) = {sum}")

    println("Function pointer test completed successfully!")

    return 0
}
