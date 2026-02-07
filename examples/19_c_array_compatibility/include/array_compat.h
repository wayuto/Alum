#ifndef ARRAY_COMPAT_H
#define ARRAY_COMPAT_H

// C Array Compatibility Functions
// These functions work with Alum arrays because they have C-compatible layout

#ifdef __cplusplus
extern "C" {
#endif

// Sum elements of an int array
long long c_array_sum(long long* arr, long long length);

// Find maximum element in an int array
long long c_array_max(long long* arr, long long length);

// Reverse an array in-place
void c_array_reverse(long long* arr, long long length);

#ifdef __cplusplus
}
#endif

#endif