fun(pub, extern) getpid(): int
fun(pub, extern) getppid(): int
fun(pub, extern) getuid(): int
fun(pub, extern) geteuid(): int
fun(pub, extern) getgid(): int
fun(pub, extern) getegid(): int
fun(pub, extern) sched_yield(): int

fun(pub, extern) fork(): int
fun(pub, extern) execve(string, *void, *void): int
fun(pub, extern) wait4(int, *int, int, *void): int
fun(pub, extern) kill(int, int): int
fun(pub, extern) exit_group(int): void

