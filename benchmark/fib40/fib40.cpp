#include <stdio.h>

constexpr long long fib(int n) {
    if (n < 2) return n;
    return fib(n - 1) + fib(n - 2);
}

int main() {
    constexpr long long n = fib(40);
    printf("%lld\n", n);
    return 0;
}