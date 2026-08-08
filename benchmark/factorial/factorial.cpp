#include <stdio.h>

constexpr long long factorial(int n) {
    if (n < 2) return 1;
    return n * factorial(n - 1);
}

int main() {
    constexpr long long n = factorial(20);
    printf("%lld\n", n);
    return 0;
}
