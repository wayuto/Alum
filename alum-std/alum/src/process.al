fun(extern) getpid(): int
fun(extern) getppid(): int
fun(extern) getuid(): int
fun(extern) geteuid(): int
fun(extern) getgid(): int
fun(extern) getegid(): int
fun(extern) sched_yield(): int

fun(extern) fork(): int
fun(extern) execve(string, *void, *void): int
fun(extern) wait4(int, *int, int, *void): int
fun(extern) kill(int, int): int
fun(extern) exit_group(int): void

