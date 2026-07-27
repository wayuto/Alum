$import "io.ah"
$import "convert.ah"

// Lambda function example

fun main(): int {
    let f: int(int) = \(n: int): int {
        return n + 1
    }

    println(itoa(f(1)))
    return 0
}