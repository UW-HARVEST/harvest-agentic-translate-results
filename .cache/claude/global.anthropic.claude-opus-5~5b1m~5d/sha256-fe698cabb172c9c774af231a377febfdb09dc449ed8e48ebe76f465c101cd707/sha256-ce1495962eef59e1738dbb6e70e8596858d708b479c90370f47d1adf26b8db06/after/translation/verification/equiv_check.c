#include <stdint.h>
#include <stdio.h>
/* Original, copied verbatim from c_src/src/lib.c */
static inline uint32_t orig(uint32_t a) {
    a = ((a & 0xAAAA) >> 1) | ((a & 0x5555) << 1);
    a = ((a & 0xCCCC) >> 2) | ((a & 0x3333) << 2);
    a = ((a & 0xF0F0) >> 4) | ((a & 0x0F0F) << 4);
    a = ((a & 0xFF00) >> 8) | ((a & 0x00FF) << 8);
    return a;
}
/* M2: statement-1 masks widened to 32 bits */
static inline uint32_t m2(uint32_t a) {
    a = ((a & 0xAAAAAAAA) >> 1) | ((a & 0x55555555) << 1);
    a = ((a & 0xCCCC) >> 2) | ((a & 0x3333) << 2);
    a = ((a & 0xF0F0) >> 4) | ((a & 0x0F0F) << 4);
    a = ((a & 0xFF00) >> 8) | ((a & 0x00FF) << 8);
    return a;
}
/* M5: bit 16 force-set on entry */
static inline uint32_t m5(uint32_t a0) {
    uint32_t a = a0 | 0x00010000u;
    a = ((a & 0xAAAA) >> 1) | ((a & 0x5555) << 1);
    a = ((a & 0xCCCC) >> 2) | ((a & 0x3333) << 2);
    a = ((a & 0xF0F0) >> 4) | ((a & 0x0F0F) << 4);
    a = ((a & 0xFF00) >> 8) | ((a & 0x00FF) << 8);
    return a;
}
int main(void) {
    uint64_t bad2 = 0, bad5 = 0; uint32_t ex2 = 0, ex5 = 0;
    uint64_t i = 0;
    do {
        uint32_t a = (uint32_t)i, o = orig(a);
        if (m2(a) != o) { if (!bad2) ex2 = a; bad2++; }
        if (m5(a) != o) { if (!bad5) ex5 = a; bad5++; }
    } while (++i <= 0xFFFFFFFFull);
    printf("checked 2^32 = 4294967296 inputs\n");
    printf("M2 mismatches: %llu", (unsigned long long)bad2);
    if (bad2) printf("  first at 0x%08X (orig=0x%08X m2=0x%08X)", ex2, orig(ex2), m2(ex2));
    printf("\nM5 mismatches: %llu", (unsigned long long)bad5);
    if (bad5) printf("  first at 0x%08X (orig=0x%08X m5=0x%08X)", ex5, orig(ex5), m5(ex5));
    printf("\n");
    return 0;
}
