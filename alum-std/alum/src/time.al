struct(pub) Timespec {
    sec: int,
    nsec: int
}

struct(pub) Timeval {
    sec: int,
    usec: int
}

fun(pub, extern) nanosleep(*Timespec, *Timespec): int
fun(pub, extern) clock_gettime(int, *Timespec): int
fun(pub, extern) gettimeofday(*Timeval, *void): int

import memory
using memory::{malloc, free}

fun(pub) sleep_ms(ms: int): int {
    var req: *Timespec = malloc(16)@*Timespec
    req.sec = ms / 1000
    req.nsec = (ms % 1000) * 1000000
    var r: int = nanosleep(req, nil)
    free(req)
    return r
}

