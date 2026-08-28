/*
 * Independent ABI cross-check for Phase D.
 *
 * Every other test reaches `tfm` through libloading/dlsym. This probe is a
 * plain C program that LINKS DIRECTLY against a shared object (real dynamic
 * linking, real PLT, C calling convention chosen by the C compiler rather than
 * by Rust's `extern "C"` shim). It is built twice -- once against the C .so and
 * once against the Rust .so -- and the two outputs are diffed.
 *
 * That closes the last gap in the argument: it proves an ordinary external C
 * consumer cannot tell the two libraries apart, which is the actual contract.
 *
 * Usage: abi_probe            (prints every case)
 * Build: gcc -O0 -o probe abi_probe.c <lib.so> -Wl,-rpath,<dir>
 */
#include <stdio.h>
#include <stdint.h>
#include <string.h>

void tfm(float *dest, const float *src, int count);

static uint32_t bits(float f) { uint32_t u; memcpy(&u, &f, 4); return u; }
static float    val(uint32_t u) { float x; memcpy(&x, &u, 4); return x; }

/* The same 24-value special alphabet the Rust harness uses. */
static const uint32_t ALPHA[24] = {
    0x00000000u, 0x80000000u, 0x00000001u, 0x80000001u, 0x007FFFFFu,
    0x00800000u, 0x80800000u, 0x3F000000u, 0xBF000000u, 0x3F800000u,
    0xBF800000u, 0x40000000u, 0xC0000000u, 0x7F7FFFFFu, 0xFF7FFFFFu,
    0x7149F2CAu, 0xF149F2CAu, 0x7F800000u, 0xFF800000u, 0x7FC00000u,
    0xFFC00000u, 0x7FA00000u, 0xFFA00000u, 0x7F800001u
};

/* xorshift64* so both builds walk identical pseudo-random inputs. */
static uint64_t st = 0x123456789ABCDEFull;
static uint32_t rnd(void) {
    st ^= st >> 12; st ^= st << 25; st ^= st >> 27;
    return (uint32_t)((st * 0x2545F4914F6CDD1Dull) >> 32);
}

int main(void) {
    /* 1. Exhaustive alphabet^3, one element per call. */
    for (int i = 0; i < 24; i++)
      for (int j = 0; j < 24; j++)
        for (int k = 0; k < 24; k++) {
            float src[3] = { val(ALPHA[i]), val(ALPHA[j]), val(ALPHA[k]) };
            float dst[2] = { val(0xDEAD0000u), val(0xDEAD0001u) };
            tfm(dst, src, 1);
            printf("A %08x %08x %08x -> %08x %08x\n",
                   ALPHA[i], ALPHA[j], ALPHA[k], bits(dst[0]), bits(dst[1]));
        }

    /* 2. Random bit patterns, batched so the loop and pointer stepping run. */
    enum { N = 4096 };
    static float src[3 * N], dst[2 * N];
    for (int i = 0; i < 3 * N; i++) src[i] = val(rnd());
    for (int i = 0; i < 2 * N; i++) dst[i] = val(0xDEAD0000u + (uint32_t)i);
    tfm(dst, src, N);
    for (int i = 0; i < 2 * N; i++) printf("B %d %08x\n", i, bits(dst[i]));

    /* 3. count <= 0 must be a no-op (canaries must survive). */
    int counts[] = { 0, -1, -2, -1000, (-2147483647 - 1) };
    for (unsigned c = 0; c < sizeof counts / sizeof *counts; c++) {
        float d[4];
        for (int i = 0; i < 4; i++) d[i] = val(0xCAFE0000u + (uint32_t)i);
        tfm(d, src, counts[c]);
        printf("C %d -> %08x %08x %08x %08x\n", counts[c],
               bits(d[0]), bits(d[1]), bits(d[2]), bits(d[3]));
    }

    /* 4. In-place / overlapping buffers at several offsets. */
    for (int off = 0; off <= 3; off++) {
        static float buf[3 * 64 + 8];
        for (int i = 0; i < 3 * 64 + 8; i++) buf[i] = val(rnd());
        tfm(buf + off, buf, 64);
        for (int i = 0; i < 3 * 64 + 8; i++)
            printf("D %d %d %08x\n", off, i, bits(buf[i]));
    }
    return 0;
}
