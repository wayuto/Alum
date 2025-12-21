#include <stdio.h>

long long fib(int n, long long a, long long b) {
  if (n == 0)
    return a;
  return fib(n - 1, b, a + b);
}

int main() {
  long long result = fib(1000, 0, 1);
  printf("%lld\n", result);
  return 0;
}