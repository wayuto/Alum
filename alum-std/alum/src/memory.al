fun(pub, extern) malloc(int): *void
fun(pub, extern) free(*void): void

fun(pub, extern) mmap(*void, int, int, int, int, int): *void
fun(pub, extern) munmap(*void, int): int
fun(pub, extern) mprotect(*void, int, int): int
fun(pub, extern) brk(int): int

