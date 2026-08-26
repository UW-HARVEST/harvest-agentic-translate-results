#include <dlfcn.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

typedef float (*ldexp_q2_fn)(float, int);

static uint32_t float_bits(float value) {
    uint32_t bits;
    memcpy(&bits, &value, sizeof(bits));
    return bits;
}

static float bits_float(uint32_t bits) {
    float value;
    memcpy(&value, &bits, sizeof(value));
    return value;
}

static void compare(ldexp_q2_fn c_fn, ldexp_q2_fn rust_fn,
                    uint32_t input_bits, int exponent) {
    uint32_t c_bits = float_bits(c_fn(bits_float(input_bits), exponent));
    uint32_t rust_bits = float_bits(rust_fn(bits_float(input_bits), exponent));

    if (c_bits != rust_bits) {
        fprintf(stderr,
                "mismatch: y=0x%08x exp=%d C=0x%08x Rust=0x%08x\n",
                input_bits, exponent, c_bits, rust_bits);
        exit(1);
    }
}

int main(void) {
    static const uint32_t edge_values[] = {
        0x00000000, 0x80000000, 0x00000001, 0x80000001,
        0x007fffff, 0x807fffff, 0x00800000, 0x80800000,
        0x3f800000, 0xbf800000, 0x7f7fffff, 0xff7fffff,
        0x7f800000, 0xff800000, 0x7fc00000, 0xffc00000,
        0x7f800001, 0xff800001,
    };
    void *c_lib = dlopen("c_build/libtranslated_rust.so", RTLD_NOW);
    void *rust_lib = dlopen("target/release/libtranslated_rust.so", RTLD_NOW);
    if (c_lib == NULL || rust_lib == NULL) {
        fprintf(stderr, "dlopen failed: %s\n", dlerror());
        return 2;
    }

    ldexp_q2_fn c_fn = (ldexp_q2_fn)dlsym(c_lib, "ldexp_q2");
    ldexp_q2_fn rust_fn = (ldexp_q2_fn)dlsym(rust_lib, "ldexp_q2");
    if (c_fn == NULL || rust_fn == NULL) {
        fprintf(stderr, "dlsym failed: %s\n", dlerror());
        return 2;
    }

    uint64_t comparisons = 0;
    for (int exponent = -65536; exponent <= 4096; ++exponent) {
        for (size_t i = 0; i < sizeof(edge_values) / sizeof(edge_values[0]); ++i) {
            compare(c_fn, rust_fn, edge_values[i], exponent);
            ++comparisons;
        }
    }

    uint32_t random = 0x6d2b79f5;
    for (int i = 0; i < 1000000; ++i) {
        random ^= random << 13;
        random ^= random >> 17;
        random ^= random << 5;
        int exponent = (int)((random >> 16) & 0xffff) - 32768;
        compare(c_fn, rust_fn, random, exponent);
        ++comparisons;
    }

    printf("%llu byte-identical results\n", (unsigned long long)comparisons);
    dlclose(rust_lib);
    dlclose(c_lib);
    return 0;
}
