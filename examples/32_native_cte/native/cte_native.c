#include <stdint.h>
#include <string.h>
#include <stdio.h>

/* Native helper library for the `native_cte` Alum project.
 * These C functions are loaded by `alumake` into target/libnative_cte_native.so
 * and used in two ways by the Alum sources:
 *   - at compile time: `fun(extern, pure) F(...): T` constants are folded to
 *     literals (`alc -c --cte-lib ...`);
 *   - at runtime: any call that could not be folded (e.g. with dynamic
 *     arguments) is linked directly against the shared library. */

int32_t cte_add(int32_t a, int32_t b) { return a + b; }

int32_t cte_max3(int32_t a, int32_t b, int32_t c) {
    int32_t m = a > b ? a : b;
    return m > c ? m : c;
}

/* Squared norm (a^2 + b^2), not the hypotenuse: using sqrt() here would
 * leave an undefined `sqrt` reference in the .so, which the compiler
 * process (linked against libc only) cannot resolve when dlopen-ing it
 * for compile-time evaluation. For a 3-4-5 triangle this yields 25. */
double cte_sqnorm(double a, double b) { return a * a + b * b; }

double cte_price(double base, int32_t tax_pct) { return base * (1.0 + tax_pct / 100.0); }

int32_t cte_join_len(const char *a, const char *b) {
    return (int32_t)(strlen(a) + strlen(b));
}

const char *cte_upper(const char *s) {
    static char buf[128];
    size_t i = 0;
    for (; s[i] && i < sizeof(buf) - 1; i++) {
        buf[i] = (s[i] >= 'a' && s[i] <= 'z') ? (char)(s[i] - 'a' + 'A') : s[i];
    }
    buf[i] = '\0';
    return buf;
}