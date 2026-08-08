$import "io.ah"

fun(pure) factorial(n: int): int {
    if n < 2 return 1
    return n * factorial(n - 1)
}

fun main(): int {
    var r = factorial(20)
    println(f"{r}")
    return 0
}
