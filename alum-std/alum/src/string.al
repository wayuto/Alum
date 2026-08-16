fun(pub, pure) strlen(s: string): int {
    var p: *void = s
    var i: int = 0
    while true {
        var b: int = p[i]
        if b == 0 break
        i = i + 1
    }
    return i
}

fun(pub) strcpy(dst: string, src: string): string {
    var pd: *void = dst
    var ps: *void = src
    var i: int = 0
    while true {
        var b: int = ps[i]
        pd[i] = ps[i]
        if b == 0 break
        i = i + 1
    }
    return dst
}

fun(pub) strcat(dst: string, src: string): string {
    var pd: *void = dst
    var ps: *void = src
    var i: int = 0
    while true {
        var b: int = pd[i]
        if b == 0 break
        i = i + 1
    }
    var j: int = 0
    while true {
        var b: int = ps[j]
        pd[i + j] = ps[j]
        if b == 0 break
        j = j + 1
    }
    return dst
}

fun(pub, pure) strcmp(s1: string, s2: string): int {
    var p1: *void = s1
    var p2: *void = s2
    var i: int = 0
    while true {
        var a: int = p1[i]
        var b: int = p2[i]
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

fun(pub) memcpy(dst: string, src: string, n: int): string {
    var pd: *void = dst
    var ps: *void = src
    var i: int = 0
    while i < n {
        pd[i] = ps[i]
        i = i + 1
    }
    return dst
}

fun(pub) memset(s: string, c: int, n: int): string {
    var p: *void = s
    var arr: int[] = [c]
    var pv: *void = arr
    var i: int = 0
    while i < n {
        p[i] = pv[8]
        i = i + 1
    }
    return s
}

fun(pub, pure) bcmp(s1: string, s2: string, n: int): int {
    var p1: *void = s1
    var p2: *void = s2
    var i: int = 0
    while i < n {
        var a: int = p1[i]
        var b: int = p2[i]
        if a != b {
            return a - b
        }
        i = i + 1
    }
    return 0
}

fun(pub, pure) memcmp(s1: string, s2: string, n: int): int {
    return bcmp(s1, s2, n)
}
