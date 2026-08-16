import io
using io::println

// TypeDef Example

fun main(): int {
    typedef MyInt = int
    typedef MyArray = int[5]

    var x: MyInt = 42
    println(f"x = {x}")

    var arr: MyArray = [1, 2, 3, 4, 5]
    println(f"arr[0] = {arr[0]}")

    return 0
}




