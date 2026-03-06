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

static void check_remw(int32_t a, int32_t b, int64_t expected) {
    int64_t result;
    asm volatile("remw %0, %1, %2" : "=r"(result) : "r"((int64_t)a), "r"((int64_t)b));
    if (result != expected) {
        failed = 1;
    }
}

static void check_remuw(uint32_t a, uint32_t b, int64_t expected) {
    int64_t result;
    asm volatile("remuw %0, %1, %2" : "=r"(result) : "r"((int64_t)a), "r"((int64_t)b));
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

    /* remw: normal cases */
    check_remw(10, 3, 1);
    check_remw(-10, 3, -1);
    check_remw(10, -3, 1);
    check_remw(-10, -3, -1);

    /* remw: division by zero yields the dividend sign-extended */
    check_remw(42, 0, 42);
    check_remw(INT32_MIN, 0, (int64_t)INT32_MIN);

    /* remw: INT32_MIN % -1 overflow yields 0 */
    check_remw(INT32_MIN, -1, 0);

    /* remw: result is sign-extended to 64 bits */
    check_remw(INT32_MIN + 1, 1, 0);

    /* remuw: normal cases */
    check_remuw(10, 3, 1);
    check_remuw(0, 5, 0);

    /* remuw: division by zero yields the dividend sign-extended */
    check_remuw(42, 0, 42);
    check_remuw(UINT32_MAX, 0, -1LL);

    /* remuw: dividend with bit 31 set is treated as unsigned */
    check_remuw(0x80000003u, 4, (int64_t)(int32_t)3u);

    /* remuw: result is sign-extended to 64 bits */
    check_remuw(UINT32_MAX, 2, -1LL);

    return failed;
}
