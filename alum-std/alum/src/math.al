fun abs(n: int): int {
    if n < 0 {
        return -n
    }
    return n
}

fun sqr(n: int): int {
    return n * n
}

fun max(a: int, b: int): int {
    if a > b {
        return a
    }
    return b
}

fun min(a: int, b: int): int {
    if a < b {
        return a
    }
    return b
}

fun pow(base: int, exp: int): int {
    if exp == 0 {
        return 1
    }
    if exp < 0 {
        return 0
    }
    
    let result: int = 1
    let i: int = 0
    while i < exp {
        result = result * base
        i = i + 1
    }
    return result
}

fun fact(n: int): int {
    if n <= 1 {
        return 1
    }
    return n * fact(n - 1)
}