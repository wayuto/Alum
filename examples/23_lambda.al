$import "io.ah"

// Lambda function example

fun main(): int {
    var f: int(int) = \(n: int): int {
        return n + 1
    }

    println(f"{f(1)}")
    return 0
}