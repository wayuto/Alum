#include <stdint.h>
#include <string.h>
#include <stdio.h>

// Helper library for examples/32_native_cte.al
//
// Build the shared library, then run with:
//   gcc -shared -fPIC -o libcte32.so 32_native_cte.c
//   alc -r --cte-lib ./libcte32.so 32_native_cte.al

int32_t cte_add(int32_t a, int32_t b) { return a + b; }

int32_t cte_max3(int32_t a, int32_t b, int32_t c) {
    int32_t m = a > b ? a : b;
    return m > c ? m : c;
}

double cte_hypot(double a, double b) { return a * a + b * b; }

double cte_price(double base, int32_t tax_pct) { return base * (1.0 + tax_pct / 100.0); }

int32_t cte_join_len(const char *a, const char *b) {
    return (int32_t)(strlen(a) + strlen(b));
}

const char *cte_upper(const char *s) {
    static char buf[128];
    size_t i = 0;
    for (; s[i] && i < sizeof(buf) - 1; i++) {
        buf[i] = (s[i] >= 'a' && s[i] <= 'z') ? (char)(s[i] - 32) : s[i];
    }
    buf[i] = '\0';
    return buf;
}