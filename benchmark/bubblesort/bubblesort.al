$import "io.ah"

fun(pure) sort(a: int[]): int[] {
    var n = 10
    var arr: int[] = [int; n]
    var i = 0
    while i < n {
        arr[i] = a[i]
        i = i + 1
    }
    var j = 0
    while j < n - 1 {
        var k = 0
        while k < n - j - 1 {
            if arr[k] > arr[k + 1] {
                var t = arr[k]
                arr[k] = arr[k + 1]
                arr[k + 1] = t
            }
            k = k + 1
        }
        j = j + 1
    }
    return arr
}

fun main(): int {
    var sorted: int[] = sort([9, 2, 7, 1, 8, 3, 6, 4, 10, 5])
    for x in sorted {
        println(f"{x}")
    }
    return 0
}
