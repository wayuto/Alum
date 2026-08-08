#include <cstdio>
#include <array>

constexpr std::array<int, 10> sort(std::array<int, 10> a) {
    constexpr int n = 10;
    for (int j = 0; j < n - 1; ++j) {
        for (int k = 0; k < n - j - 1; ++k) {
            if (a[k] > a[k + 1]) {
                int t = a[k];
                a[k] = a[k + 1];
                a[k + 1] = t;
            }
        }
    }
    return a;
}

int main() {
    constexpr auto sorted = sort(std::array<int, 10>{9, 2, 7, 1, 8, 3, 6, 4, 10, 5});
    for (int x : sorted) {
        printf("%d\n", x);
    }
    return 0;
}
