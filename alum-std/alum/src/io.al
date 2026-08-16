fun(pub, extern) write(int, string, int): int
fun(pub, extern) read(int, string, int): int
fun(pub, extern) print(string): int
fun(pub, extern) println(string): int
fun(pub, extern) input(string): string
fun(pub, extern) fopen(string, int, int): int
fun(pub, extern) fclose(int): int
fun(pub, extern) fread(int): string
fun(pub, extern) fwrite(int, string, int): int
fun(pub, extern) lseek(int, int, int): int

fun(pub, extern) pipe(*void): int
fun(pub, extern) pipe2(*void, int): int
fun(pub, extern) dup(int): int
fun(pub, extern) dup2(int, int): int
fun(pub, extern) dup3(int, int, int): int

