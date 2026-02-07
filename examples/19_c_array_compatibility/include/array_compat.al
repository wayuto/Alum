$ifndef ALUM_ARRAY_COMPAT
$define ALUM_ARRAY_COMPAT 1

// C-compatible array functions
extern c_array_sum(arr[int], int): int
extern c_array_max(arr[int], int): int
extern c_array_reverse(arr[int], int): void

$endif