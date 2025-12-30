$import "lib"

fun fib(n: int a: int b: int): int {
    if n == 0 return a
    return fib(n - 1 b a + b)
}


pub fun main(): int {
    let n: int = fib(1000 0 1)
    println(
        itoa(n)
    )
    return 0
}