$ifndef ALUM_ARRAY_COMPAT
$define ALUM_ARRAY_COMPAT 1

// C-compatible array functions
// Alum arrays are fully compatible with C arrays

extern c_array_sum(*int, int): int
extern c_array_max(*int, int): int
extern c_array_reverse(*int, int): void

$endif
