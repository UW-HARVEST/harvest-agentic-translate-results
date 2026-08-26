#include "lib.h"

#include <dlfcn.h>
#include <inttypes.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

typedef int (*bitwriter_add_fn)(tflac_bitwriter *, tflac_u32, tflac_uint);

static uint64_t rng_state = UINT64_C(0x8d26f31b4c9a705e);

static uint64_t next_random(void) {
    uint64_t x = rng_state;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    rng_state = x;
    return x;
}

static bitwriter_add_fn load_function(void **handle, const char *path) {
    *handle = dlopen(path, RTLD_NOW | RTLD_LOCAL);
    if (*handle == NULL) {
        fprintf(stderr, "dlopen(%s): %s\n", path, dlerror());
        exit(2);
    }

    dlerror();
    bitwriter_add_fn function = (bitwriter_add_fn)dlsym(*handle, "bitwriter_add");
    const char *error = dlerror();
    if (error != NULL) {
        fprintf(stderr, "dlsym(%s): %s\n", path, error);
        exit(2);
    }
    return function;
}

static void compare_case(bitwriter_add_fn c_function,
                         bitwriter_add_fn rust_function,
                         uint32_t state_bits,
                         uint32_t input_bits,
                         uint64_t input_value,
                         uint64_t case_number) {
    uint8_t buffer[8] = {0};
    tflac_bitwriter c_state = {
        .val = next_random(),
        .bits = state_bits,
        .pos = (uint32_t)next_random(),
        .len = (uint32_t)next_random(),
        .tot = (uint32_t)next_random(),
        .buffer = buffer,
    };
    tflac_bitwriter rust_state = c_state;

    int c_result = c_function(&c_state, input_bits, input_value);
    int rust_result = rust_function(&rust_state, input_bits, input_value);

    if (c_result != rust_result ||
        memcmp(&c_state, &rust_state, sizeof(c_state)) != 0) {
        fprintf(stderr,
                "mismatch case=%" PRIu64 " state_bits=%" PRIu32
                " input_bits=%" PRIu32 " input_value=%" PRIu64 "\n",
                case_number, state_bits, input_bits, input_value);
        fprintf(stderr,
                "C:    rc=%d val=%" PRIu64 " bits=%" PRIu32
                " pos=%" PRIu32 " len=%" PRIu32 " tot=%" PRIu32 "\n",
                c_result, c_state.val, c_state.bits, c_state.pos,
                c_state.len, c_state.tot);
        fprintf(stderr,
                "Rust: rc=%d val=%" PRIu64 " bits=%" PRIu32
                " pos=%" PRIu32 " len=%" PRIu32 " tot=%" PRIu32 "\n",
                rust_result, rust_state.val, rust_state.bits, rust_state.pos,
                rust_state.len, rust_state.tot);
        exit(1);
    }
}

int main(int argc, char **argv) {
    if (argc != 3) {
        fprintf(stderr, "usage: %s C_LIBRARY RUST_LIBRARY\n", argv[0]);
        return 2;
    }

    void *c_handle;
    void *rust_handle;
    bitwriter_add_fn c_function = load_function(&c_handle, argv[1]);
    bitwriter_add_fn rust_function = load_function(&rust_handle, argv[2]);
    uint64_t cases = 0;

    static const uint64_t values[] = {
        0, 1, UINT64_MAX, UINT64_C(0x0123456789abcdef),
        UINT64_C(0x8000000000000000),
    };
    for (uint32_t state_bits = 0; state_bits < 160; ++state_bits) {
        for (uint32_t input_bits = 0; input_bits < 160; ++input_bits) {
            for (size_t value = 0; value < sizeof(values) / sizeof(values[0]);
                 ++value) {
                compare_case(c_function, rust_function, state_bits, input_bits,
                             values[value], cases++);
            }
        }
    }

    for (uint64_t i = 0; i < UINT64_C(1000000); ++i) {
        compare_case(c_function, rust_function, (uint32_t)next_random(),
                     (uint32_t)next_random(), next_random(), cases++);
    }

    dlclose(rust_handle);
    dlclose(c_handle);
    printf("matched %" PRIu64 " cases\n", cases);
    return 0;
}
