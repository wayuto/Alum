$ifndef ALUM_LIB
$define ALUM_LIB 1

extern syscall(int, int, int, int): int
extern exit(int): void
extern malloc(int): string

$endif