#include <dlfcn.h>
#include <inttypes.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

typedef void (*hsl_to_rgb_fn)(float *, const float *);

static uint32_t state = UINT32_C(0x243f6a88);

static uint32_t next_u32(void) {
    uint32_t x = state;
    x ^= x << 13;
    x ^= x >> 17;
    x ^= x << 5;
    state = x;
    return x;
}

static float from_bits(uint32_t bits) {
    float value;
    memcpy(&value, &bits, sizeof(value));
    return value;
}

static uint32_t to_bits(float value) {
    uint32_t bits;
    memcpy(&bits, &value, sizeof(bits));
    return bits;
}

static int compare_one(
    hsl_to_rgb_fn c_fn,
    hsl_to_rgb_fn rust_fn,
    const uint32_t input_bits[3],
    uint64_t case_number
) {
    float input[3] = {
        from_bits(input_bits[0]),
        from_bits(input_bits[1]),
        from_bits(input_bits[2]),
    };
    float c_output[3] = {from_bits(UINT32_C(0xdeadbeef)),
                         from_bits(UINT32_C(0xdeadbeef)),
                         from_bits(UINT32_C(0xdeadbeef))};
    float rust_output[3] = {from_bits(UINT32_C(0xdeadbeef)),
                            from_bits(UINT32_C(0xdeadbeef)),
                            from_bits(UINT32_C(0xdeadbeef))};

    c_fn(c_output, input);
    rust_fn(rust_output, input);
    if (memcmp(c_output, rust_output, sizeof(c_output)) == 0) {
        return 0;
    }

    fprintf(stderr,
            "mismatch at case %" PRIu64 "\n"
            "input: %08" PRIx32 " %08" PRIx32 " %08" PRIx32 "\n"
            "C:     %08" PRIx32 " %08" PRIx32 " %08" PRIx32 "\n"
            "Rust:  %08" PRIx32 " %08" PRIx32 " %08" PRIx32 "\n",
            case_number,
            input_bits[0], input_bits[1], input_bits[2],
            to_bits(c_output[0]), to_bits(c_output[1]), to_bits(c_output[2]),
            to_bits(rust_output[0]), to_bits(rust_output[1]),
            to_bits(rust_output[2]));
    return 1;
}

static hsl_to_rgb_fn load_function(void *handle, const char *library_path) {
    void *symbol;
    hsl_to_rgb_fn function;

    dlerror();
    symbol = dlsym(handle, "hsl_to_rgb");
    const char *error = dlerror();
    if (error != NULL) {
        fprintf(stderr, "%s: %s\n", library_path, error);
        exit(EXIT_FAILURE);
    }
    memcpy(&function, &symbol, sizeof(function));
    return function;
}

int main(int argc, char **argv) {
    static const uint32_t edge_bits[] = {
        UINT32_C(0x00000000), UINT32_C(0x80000000),
        UINT32_C(0x00000001), UINT32_C(0x80000001),
        UINT32_C(0x007fffff), UINT32_C(0x807fffff),
        UINT32_C(0x00800000), UINT32_C(0x80800000),
        UINT32_C(0x3f000000), UINT32_C(0xbf000000),
        UINT32_C(0x3f800000), UINT32_C(0xbf800000),
        UINT32_C(0x40000000), UINT32_C(0xc0000000),
        UINT32_C(0x42700000), UINT32_C(0xc2700000),
        UINT32_C(0x42700001), UINT32_C(0xc2700001),
        UINT32_C(0x42f00000), UINT32_C(0xc2f00000),
        UINT32_C(0x43340000), UINT32_C(0xc3340000),
        UINT32_C(0x43700000), UINT32_C(0xc3700000),
        UINT32_C(0x43960000), UINT32_C(0xc3960000),
        UINT32_C(0x43b40000), UINT32_C(0xc3b40000),
        UINT32_C(0x7f7fffff), UINT32_C(0xff7fffff),
        UINT32_C(0x7f800000), UINT32_C(0xff800000),
        UINT32_C(0x7fc00000), UINT32_C(0xffc00000),
        UINT32_C(0x7fa00001), UINT32_C(0xffa00001),
    };
    const uint64_t random_cases = UINT64_C(5000000);
    uint64_t case_number = 0;

    if (argc != 3) {
        fprintf(stderr, "usage: %s C_LIBRARY RUST_LIBRARY\n", argv[0]);
        return EXIT_FAILURE;
    }

    void *c_handle = dlopen(argv[1], RTLD_NOW | RTLD_LOCAL);
    if (c_handle == NULL) {
        fprintf(stderr, "%s: %s\n", argv[1], dlerror());
        return EXIT_FAILURE;
    }
    void *rust_handle = dlopen(argv[2], RTLD_NOW | RTLD_LOCAL);
    if (rust_handle == NULL) {
        fprintf(stderr, "%s: %s\n", argv[2], dlerror());
        return EXIT_FAILURE;
    }

    hsl_to_rgb_fn c_fn = load_function(c_handle, argv[1]);
    hsl_to_rgb_fn rust_fn = load_function(rust_handle, argv[2]);

    for (size_t h = 0; h < sizeof(edge_bits) / sizeof(edge_bits[0]); ++h) {
        for (size_t s = 0; s < sizeof(edge_bits) / sizeof(edge_bits[0]); ++s) {
            for (size_t l = 0; l < sizeof(edge_bits) / sizeof(edge_bits[0]); ++l) {
                uint32_t input[3] = {edge_bits[h], edge_bits[s], edge_bits[l]};
                if (compare_one(c_fn, rust_fn, input, case_number++)) {
                    return EXIT_FAILURE;
                }
            }
        }
    }

    for (uint64_t i = 0; i < random_cases; ++i) {
        uint32_t input[3] = {next_u32(), next_u32(), next_u32()};
        if (compare_one(c_fn, rust_fn, input, case_number++)) {
            return EXIT_FAILURE;
        }
    }

    printf("matched %" PRIu64 " cases\n", case_number);
    dlclose(rust_handle);
    dlclose(c_handle);
    return EXIT_SUCCESS;
}
