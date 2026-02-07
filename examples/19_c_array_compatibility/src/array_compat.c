// C Array Compatibility Functions
// Demonstrates that Alum arrays have C-compatible memory layout

#include <stddef.h>
#include "array_compat.h"

// Sum elements of an int array
// Alum arrays are laid out exactly like C arrays: contiguous elements
long long c_array_sum(long long* arr, long long length) {
    long long sum = 0;
    for (long long i = 0; i < length; i++) {
        sum += arr[i];
    }
    return sum;
}

// Find maximum element in an int array
long long c_array_max(long long* arr, long long length) {
    if (length <= 0) return 0;
    
    long long max = arr[0];
    for (long long i = 1; i < length; i++) {
        if (arr[i] > max) {
            max = arr[i];
        }
    }
    return max;
}

// Reverse an array in-place
void c_array_reverse(long long* arr, long long length) {
    for (long long i = 0; i < length / 2; i++) {
        long long temp = arr[i];
        arr[i] = arr[length - 1 - i];
        arr[length - 1 - i] = temp;
    }
}