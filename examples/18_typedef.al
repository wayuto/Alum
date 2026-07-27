$import "io.ah"
$import "convert.ah"

// TypeDef Example

fun main(): int {
    typedef MyInt = int
    typedef MyArray = int[5]

    let x: MyInt = 42
    print("x = ")
    println(itoa(x))

    let arr: MyArray = [1, 2, 3, 4, 5]
    print("arr[0] = ")
    let elem: MyInt = arr[0]
    println(itoa(elem))

    return 0
}
