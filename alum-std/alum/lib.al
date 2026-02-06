$ifndef ALUM_LIB
$define ALUM_LIB 1

$import "io.al"
$import "string.al"
$import "convert.al"
$import "math.al"
$import "array.al"
$import "memory.al"

extern syscall(int, int, int, int): int
extern exit(int): void

$endif