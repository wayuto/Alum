$import "io.ah"
$import "convert.ah"

// Union declaration and usage Example
// All union members share the same memory, so the size is the size of the largest member.

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

    // All members share the same storage
    println("union members share memory")

    return 0
}
