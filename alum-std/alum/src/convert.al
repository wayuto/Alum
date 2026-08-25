import memory
using memory::{malloc, free}

fun(pub) itoa(n: int): string {
    var buf: *void = malloc(32)
    var num: int = n
    var idx: int = 0
    if num == 0 {
        buf[0] = 48
        idx = 1
    } else {
        if num < 0 {
            buf[0] = 45
            idx = 1
            num = -num
        }
        var start: int = idx
        if num > 0 {
            while num > 0 {
                buf[idx] = 48 + num % 10
                num = num / 10
                idx = idx + 1
            }
        } else {
            while num < 0 {
                buf[idx] = 48 - num % 10
                num = num / 10
                idx = idx + 1
            }
        }
        var end: int = idx - 1
        while start < end {
            var t: int = buf[start]
            buf[start] = buf[end]
            buf[end] = t
            start = start + 1
            end = end - 1
        }
    }
    buf[idx] = 0
    return buf
}

fun(pub, pure) atoi(s: string): int {
    var p: *void = s
    var i: int = 0
    while true {
        var b: int = p[i]
        if b != 32 && b != 9 && b != 10 && b != 13 break
        i = i + 1
    }
    var sign: int = 1
    var b: int = p[i]
    if b == 43 {
        i = i + 1
    } else {
        if b == 45 {
            sign = -1
            i = i + 1
        }
    }
    var result: int = 0
    while true {
        var d: int = p[i]
        if d < 48 || d > 57 break
        var dgt: int = d - 48
        
        
        var limit: int = (9223372036854775807 - dgt) / 10
        if result - limit > 0 {
            if sign < 0 {
                return 0 - 9223372036854775807 - 1
            }
            return 9223372036854775807
        }
        result = result * 10 + dgt
        i = i + 1
    }
    return result * sign
}

fun(pub, extern) atof(string): float
fun(pub, extern) ftoa(float): string
