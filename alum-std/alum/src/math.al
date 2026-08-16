fun(pure) abs(n: int): int {
    if n < 0 {
        return -n
    }
    return n
}

fun(pure) sqrt(n: int): int {
    if n <= 0 {
        return 0
    }
    var i: int = 1
    while i * i <= n {
        i = i + 1
    }
    return i - 1
}

fun(pure) max(a: int, b: int): int {
    if a > b {
        return a
    }
    return b
}

fun(pure) min(a: int, b: int): int {
    if a < b {
        return a
    }
    return b
}

fun(pure) pow(base: int, exp: int): int {
    if exp == 0 {
        return 1
    }
    if exp < 0 {
        return 0
    }

    var result: int = 1
    var i: int = 0
    while i < exp {
        result = result * base
        i = i + 1
    }
    return result
}

fun(pure) fact(n: int): int {
    if n <= 1 {
        return 1
    }
    return n * fact(n - 1)
}

