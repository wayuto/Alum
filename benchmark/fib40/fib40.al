$import "io.ah"

fun(pure) fib(n: int): int {
    if n < 2 return n
    return fib(n - 1) + fib(n - 2)
}

fun main(): int {
    var n = fib(40)
    println(f"{n}")
    return 0
}