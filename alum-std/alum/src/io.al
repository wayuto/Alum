fun(extern) write(int, string, int): int
fun(extern) read(int, string, int): int
fun(extern) print(string): int
fun(extern) println(string): int
fun(extern) input(string): string
fun(extern) fopen(string, int, int): int
fun(extern) fclose(int): int
fun(extern) fread(int): string
fun(extern) fwrite(int, string, int): int
fun(extern) lseek(int, int, int): int

fun(extern) pipe(*void): int
fun(extern) pipe2(*void, int): int
fun(extern) dup(int): int
fun(extern) dup2(int, int): int
fun(extern) dup3(int, int, int): int

