// Byte/string operations. Manipulation functions take raw pointers (*void):
// strings pass implicitly, no move semantics, callers keep ownership.

fun(pub, pure) strlen(s: *void): int {
    var i: int = 0
    while true {
        var b: int = s[i]
        if b == 0 break;
        i = i + 1
    }
    return i
}

fun(pub) strcpy(dst: *void, src: *void): *void {
    var i: int = 0
    while true {
        var b: int = src[i]
        dst[i] = b
        if b == 0 break;
        i = i + 1
    }
    return dst
}

fun(pub) strcat(dst: *void, src: *void): *void {
    var i: int = 0
    while true {
        var b: int = dst[i]
        if b == 0 break;
        i = i + 1
    }
    var j: int = 0
    while true {
        var b: int = src[j]
        dst[i + j] = b
        if b == 0 break;
        j = j + 1
    }
    return dst
}

fun(pub, pure) strcmp(s1: *void, s2: *void): int {
    var i: int = 0
    while true {
        var a: int = s1[i]
        var b: int = s2[i]
        if a != b {
            return a - b
        }
        if a == 0 {
            return 0
        }
        i = i + 1
    }
    return 0
}

fun(pub) memcpy(dst: *void, src: *void, n: int): *void {
    var i: int = 0
    while i < n {
        dst[i] = src[i]
        i = i + 1
    }
    return dst
}

fun(pub) memset(p: *void, c: int, n: int): *void {
    var i: int = 0
    while i < n {
        p[i] = c
        i = i + 1
    }
    return p
}

fun(pub, pure) bcmp(s1: *void, s2: *void, n: int): int {
    var i: int = 0
    while i < n {
        var a: int = s1[i]
        var b: int = s2[i]
        if a != b {
            return a - b
        }
        i = i + 1
    }
    return 0
}

fun(pub, pure) memcmp(s1: *void, s2: *void, n: int): int {
    return bcmp(s1, s2, n)
}
