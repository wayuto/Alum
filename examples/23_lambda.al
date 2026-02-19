$import "io.al"
$import "convert.al"

// Lambda function example

fun main(): int {
    let f: int(int) = lamb(n: int): int {
        return n + 1
    }

    println(itoa(f(1)))
    return 0
}