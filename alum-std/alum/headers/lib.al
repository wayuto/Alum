$ifndef ALUM_LIB
$define ALUM_LIB 1

extern syscall(int, int, int, int): int
extern exit(int): void
extern malloc(int): string
extern free(string, int): void
extern sys_write(int, string, int): int
extern sys_read(int, string, int): int
extern print(string): int
extern println(string): int

$endif