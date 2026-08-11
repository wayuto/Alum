$import "memory.ah"

fun(pub) itoa(n: int): string {
    var buf: *void = malloc(64)
    var num: int = n
    var idx: int = 0
    if num == 0 {
        var z0: int[] = [48]
        var pz0: *void = z0
        buf[0] = pz0[8]
        idx = 1
    } else {
        if num < 0 {
            var n0: int[] = [45]
            var pn0: *void = n0
            buf[0] = pn0[8]
            idx = 1
            num = -num
        }
        var start: int = idx
        while num > 0 {
            var d: int[] = [48 + num % 10]
            var pd: *void = d
            buf[idx] = pd[8]
            num = num / 10
            idx = idx + 1
        }
        var end: int = idx - 1
        while start < end {
            var t: int = buf[start]
            buf[start] = buf[end]
            var tb: int[] = [t]
            var ptb: *void = tb
            buf[end] = ptb[8]
            start = start + 1
            end = end - 1
        }
    }
    var nz: int[] = [0]
    var pnz: *void = nz
    buf[idx] = pnz[8]
    return buf
}