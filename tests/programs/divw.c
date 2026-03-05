#include <stdint.h>

static int failed = 0;

static void check_divw(int32_t a, int32_t b, int64_t expected) {
    int64_t result;
    asm volatile("divw %0, %1, %2" : "=r"(result) : "r"((int64_t)a), "r"((int64_t)b));
    if (result != expected) {
        failed = 1;
    }
}

static void check_divuw(uint32_t a, uint32_t b, int64_t expected) {
    int64_t result;
    asm volatile("divuw %0, %1, %2" : "=r"(result) : "r"((int64_t)a), "r"((int64_t)b));
    if (result != expected) {
        failed = 1;
    }
}

int main() {
    /* divw: normal cases */
    check_divw(10, 3, 3);
    check_divw(-10, 3, -3);
    check_divw(10, -3, -3);
    check_divw(-10, -3, 3);

    /* divw: division by zero yields -1 (all bits set) */
    check_divw(42, 0, -1);
    check_divw(-1, 0, -1);

    /* divw: INT32_MIN / -1 overflow yields INT32_MIN sign-extended */
    check_divw(INT32_MIN, -1, (int64_t)INT32_MIN);

    /* divw: result is sign-extended to 64 bits */
    check_divw(INT32_MIN + 1, 1, (int64_t)(INT32_MIN + 1));

    /* divuw: normal cases */
    check_divuw(10, 3, 3);
    check_divuw(0, 5, 0);

    /* divuw: division by zero yields -1 (all bits set) */
    check_divuw(42, 0, -1);

    /* divuw: dividend with bit 31 set is treated as unsigned */
    check_divuw(0x80000000u, 2, (int64_t)(int32_t)0x40000000u);

    /* divuw: result is sign-extended to 64 bits */
    check_divuw(UINT32_MAX, 1, -1LL);
    check_divuw(6, 2, 3);

    return failed;
}
