import sys

sys.setrecursionlimit(2000)


def fib(n, a, b):
    if n == 0:
        return a
    return fib(n - 1, b, a + b)


n = fib(1000, 0, 1)
print(n)
