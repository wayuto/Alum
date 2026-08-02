$import "io.ah"

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
    println(f"v.i = {v.i}")

    // Assign through the int member
    v.i = 100
    println(f"v.i after assign = {v.i}")

    return 0
}
