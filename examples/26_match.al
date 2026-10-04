import io
using io::println

// Match Example

fun func(n: int): void {
    match n {
        0: {
            println("`n` is equal to 0")@void
        }
        1: {
            println("`n` is equal to 1")@void
        }
        _: {
            println("`n` is not equal to 0 or 1")@void
        }
    }
}

fun main(): int {
    for n in 0..3 {
        func(n)
    }

    return 0
}
