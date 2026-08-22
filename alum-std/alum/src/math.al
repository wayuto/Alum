fun(pub, pure) abs(n: int): int {
    if n < 0 {
        var m: int = -n
        if m < 0 {
            return n
        }
        return m
    }
    return n
}

fun(pub, pure) sqrt(n: int): int {
    if n <= 0 {
        return 0
    }
    var i: int = 1
   
   
    while i <= n / i {
        i = i + 1
    }
    return i - 1
}

fun(pub, pure) max(a: int, b: int): int {
    if a > b {
        return a
    }
    return b
}

fun(pub, pure) min(a: int, b: int): int {
    if a < b {
        return a
    }
    return b
}

fun(pub, pure) pow(base: int, exp: int): int {
   
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

fun(pub, pure) fact(n: int): int {
    if n <= 1 {
        return 1
    }
    return n * fact(n - 1)
}

