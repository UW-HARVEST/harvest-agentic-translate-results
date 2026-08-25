#include <dlfcn.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>

typedef uint16_t (*float2half_fn)(float);

static float2half_fn load_function(const char *library_path) {
    void *library = dlopen(library_path, RTLD_NOW);
    if (library == NULL) {
        fprintf(stderr, "dlopen(%s): %s\n", library_path, dlerror());
        return NULL;
    }

    dlerror();
    float2half_fn function = (float2half_fn)dlsym(library, "float2half");
    const char *error = dlerror();
    if (error != NULL) {
        fprintf(stderr, "dlsym(%s): %s\n", library_path, error);
        return NULL;
    }
    return function;
}

static unsigned shift_for_index(uint32_t index) {
    uint32_t exponent = index & 0xff;
    if (exponent <= 102 || (exponent >= 143 && exponent <= 254)) {
        return 24;
    }
    if (exponent <= 112) {
        return 126 - exponent;
    }
    return 13;
}

static int compare(
    float2half_fn c_function,
    float2half_fn rust_function,
    uint32_t bits
) {
    float input;
    memcpy(&input, &bits, sizeof(input));
    uint16_t c_result = c_function(input);
    uint16_t rust_result = rust_function(input);
    if (c_result != rust_result) {
        fprintf(
            stderr,
            "mismatch for 0x%08x: C=0x%04x Rust=0x%04x\n",
            bits,
            c_result,
            rust_result
        );
        return 1;
    }
    return 0;
}

int main(int argc, char **argv) {
    if (argc != 3) {
        fprintf(stderr, "usage: %s C_LIBRARY RUST_LIBRARY\n", argv[0]);
        return 2;
    }

    float2half_fn c_function = load_function(argv[1]);
    float2half_fn rust_function = load_function(argv[2]);
    if (c_function == NULL || rust_function == NULL) {
        return 2;
    }

    uint64_t comparisons = 0;
    for (uint32_t index = 0; index < 512; ++index) {
        unsigned shift = shift_for_index(index);
        uint32_t quotient_max = 0x007fffffU >> shift;
        uint32_t remainder_mask = shift == 24
            ? 0x007fffffU
            : (1U << shift) - 1;

        for (uint32_t quotient = 0; quotient <= quotient_max; ++quotient) {
            uint32_t low = (index << 23) | (quotient << shift);
            uint32_t high = low | remainder_mask;
            if (compare(c_function, rust_function, low) != 0 ||
                compare(c_function, rust_function, high) != 0) {
                return 1;
            }
            comparisons += 2;
        }
    }

    printf("%llu ABI results matched\n", (unsigned long long)comparisons);
    return 0;
}
