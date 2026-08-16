struct(pub) Timespec {
    sec: int,
    nsec: int
}

struct(pub) Timeval {
    sec: int,
    usec: int
}

fun(extern) nanosleep(*Timespec, *Timespec): int
fun(extern) clock_gettime(int, *Timespec): int
fun(extern) gettimeofday(*Timeval, *void): int

