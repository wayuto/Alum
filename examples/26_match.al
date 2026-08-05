$import "io.ah"

// Match Example

fun func(n: int): void {
    match n {
        0: {
            println("`s` is equal to 0")
        }
        1: {
            println("`s` is equal to 1")
        }
        _: {
            println("`s` is not equal to 0 or 1")
        }
    }
}

fun main(): int {
    for n in 0..3 {
        func(n)
    }

    return 0
}