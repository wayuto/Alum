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

