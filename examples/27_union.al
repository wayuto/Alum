$import "io.ah"
$import "convert.ah"

// Union declaration and usage Example

union Value {
    i: int,
    f: float
}

fun main(): int {
    let v: Value = Value {
        i: 42
    }

    // Read the int member
    print("v.i = ")
    println(itoa(v.i))

    // Assign through the int member
    v.i = 100
    print("v.i after assign = ")
    println(itoa(v.i))

    return 0
}
