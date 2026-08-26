#include <dlfcn.h>
#include <fenv.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

typedef void (*rgb_to_hsv_fn)(float *, const float *);

static uint32_t rng_state = UINT32_C(0x9e3779b9);

static uint32_t next_u32(void) {
    uint32_t x = rng_state;
    x ^= x << 13;
    x ^= x >> 17;
    x ^= x << 5;
    rng_state = x;
    return x;
}

static float from_bits(uint32_t bits) {
    float value;
    memcpy(&value, &bits, sizeof(value));
    return value;
}

static void print_values(const char *label, const float values[3]) {
    uint32_t bits[3];
    memcpy(bits, values, sizeof(bits));
    fprintf(stderr, "%s: %08x %08x %08x\n",
            label, bits[0], bits[1], bits[2]);
}

static int compare_call(
        rgb_to_hsv_fn c_fn,
        rgb_to_hsv_fn rust_fn,
        const float input[3]) {
    float c_output[3];
    float rust_output[3];

    c_fn(c_output, input);
    rust_fn(rust_output, input);
    if (memcmp(c_output, rust_output, sizeof(c_output)) != 0) {
        print_values("input", input);
        print_values("C output", c_output);
        print_values("Rust output", rust_output);
        return 0;
    }
    return 1;
}

static int compare_aliases(
        rgb_to_hsv_fn c_fn,
        rgb_to_hsv_fn rust_fn,
        const float input[3]) {
    float c_same[4] = {input[0], input[1], input[2], from_bits(next_u32())};
    float rust_same[4];
    memcpy(rust_same, c_same, sizeof(c_same));
    c_fn(c_same, c_same);
    rust_fn(rust_same, rust_same);
    if (memcmp(c_same, rust_same, sizeof(c_same)) != 0) {
        return 0;
    }

    float c_forward[4] = {
        input[0], input[1], input[2], from_bits(next_u32())
    };
    float rust_forward[4];
    memcpy(rust_forward, c_forward, sizeof(c_forward));
    c_fn(c_forward + 1, c_forward);
    rust_fn(rust_forward + 1, rust_forward);
    if (memcmp(c_forward, rust_forward, sizeof(c_forward)) != 0) {
        return 0;
    }

    float c_backward[4] = {
        from_bits(next_u32()), input[0], input[1], input[2]
    };
    float rust_backward[4];
    memcpy(rust_backward, c_backward, sizeof(c_backward));
    c_fn(c_backward, c_backward + 1);
    rust_fn(rust_backward, rust_backward + 1);
    return memcmp(c_backward, rust_backward, sizeof(c_backward)) == 0;
}

static rgb_to_hsv_fn load_function(void *library) {
    rgb_to_hsv_fn function = NULL;
    *(void **)(&function) = dlsym(library, "rgb_to_hsv");
    return function;
}

int main(int argc, char **argv) {
    static const uint32_t edge_bits[] = {
        UINT32_C(0x00000000), UINT32_C(0x80000000),
        UINT32_C(0x00000001), UINT32_C(0x80000001),
        UINT32_C(0x007fffff), UINT32_C(0x807fffff),
        UINT32_C(0x00800000), UINT32_C(0x80800000),
        UINT32_C(0x3f800000), UINT32_C(0xbf800000),
        UINT32_C(0x437f0000), UINT32_C(0xc37f0000),
        UINT32_C(0x7f7fffff), UINT32_C(0xff7fffff),
        UINT32_C(0x7f800000), UINT32_C(0xff800000),
        UINT32_C(0x7fc00000), UINT32_C(0xffc00000),
        UINT32_C(0x7fc12345), UINT32_C(0xffc12345),
        UINT32_C(0x7fa00001), UINT32_C(0xffa00001)
    };
    static const int rounding_modes[] = {
        FE_TONEAREST, FE_DOWNWARD, FE_UPWARD, FE_TOWARDZERO
    };
    const size_t edge_count = sizeof(edge_bits) / sizeof(edge_bits[0]);
    size_t comparisons = 0;

    if (argc != 3) {
        fprintf(stderr, "usage: %s C_LIBRARY RUST_LIBRARY\n", argv[0]);
        return 2;
    }

    void *c_library = dlopen(argv[1], RTLD_NOW | RTLD_LOCAL);
    void *rust_library = dlopen(argv[2], RTLD_NOW | RTLD_LOCAL);
    if (c_library == NULL || rust_library == NULL) {
        fprintf(stderr, "dlopen failed: %s\n", dlerror());
        return 2;
    }

    rgb_to_hsv_fn c_fn = load_function(c_library);
    rgb_to_hsv_fn rust_fn = load_function(rust_library);
    if (c_fn == NULL || rust_fn == NULL) {
        fprintf(stderr, "dlsym failed: %s\n", dlerror());
        return 2;
    }

    for (size_t i = 0; i < edge_count; ++i) {
        for (size_t j = 0; j < edge_count; ++j) {
            for (size_t k = 0; k < edge_count; ++k) {
                float input[3] = {
                    from_bits(edge_bits[i]),
                    from_bits(edge_bits[j]),
                    from_bits(edge_bits[k])
                };
                if (!compare_call(c_fn, rust_fn, input)
                        || !compare_aliases(c_fn, rust_fn, input)) {
                    fprintf(stderr, "edge comparison failed\n");
                    return 1;
                }
                ++comparisons;
            }
        }
    }

    for (size_t mode = 0;
            mode < sizeof(rounding_modes) / sizeof(rounding_modes[0]);
            ++mode) {
        if (fesetround(rounding_modes[mode]) != 0) {
            fprintf(stderr, "fesetround failed\n");
            return 2;
        }
        for (size_t i = 0; i < 500000; ++i) {
            float input[3] = {
                from_bits(next_u32()),
                from_bits(next_u32()),
                from_bits(next_u32())
            };
            if (!compare_call(c_fn, rust_fn, input)) {
                fprintf(stderr, "random comparison failed in rounding mode %zu\n",
                        mode);
                return 1;
            }
            ++comparisons;
        }
    }

    printf("byte-identical comparisons: %zu\n", comparisons);
    dlclose(rust_library);
    dlclose(c_library);
    return 0;
}
