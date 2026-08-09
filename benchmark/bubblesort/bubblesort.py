def sort(a: list) -> list:
    n = len(a)
    for j in range(n - 1):
        for k in range(n - j - 1):
            if a[k] > a[k + 1]:
                a[k], a[k + 1] = a[k + 1], a[k]
    return a

for x in sort([9, 2, 7, 1, 8, 3, 6, 4, 10, 5]):
    print(x)