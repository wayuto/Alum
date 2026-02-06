// C Helper Functions
// Provide computation utilities for Alum

#include "helper.h"

int c_add(int a, int b) {
    return a + b;
}

int c_multiply(int a, int b) {
    return a * b;
}

int c_calculate_factorial(int n) {
    if (n < 0) {
        return 0;
    }
    
    int result = 1;
    for (int i = 1; i <= n; i++) {
        result *= i;
    }
    
    return result;
}