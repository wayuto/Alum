import io
using io::println

// Union declaration and usage Example

union Value {
    i: int,
    f: float
}

fun main(): int {
    var v: Value = Value {
        i: 42
    }

    // Read the int member
    println(f"v.i = {v.i}")

    // Assign through the int member
    v.i = 100
    println(f"v.i after assign = {v.i}")

    return 0
}




