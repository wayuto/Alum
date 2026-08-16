fun(extern) malloc(int): *void
fun(extern) free(*void): void

fun(extern) mmap(*void, int, int, int, int, int): *void
fun(extern) munmap(*void, int): int
fun(extern) mprotect(*void, int, int): int
fun(extern) brk(int): int

