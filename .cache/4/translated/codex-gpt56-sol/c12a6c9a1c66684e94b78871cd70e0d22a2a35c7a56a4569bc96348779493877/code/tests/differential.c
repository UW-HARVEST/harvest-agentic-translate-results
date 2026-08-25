#include <dlfcn.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "../c_src/include/lib.h"

typedef int (*dequantize_fn)(float *, bs_t *, L12_scale_info *, int);

enum {
    BITSTREAM_BYTES = 32768,
    OUTPUT_FLOATS = 2048,
    RANDOM_CASES = 5000,
};

static uint32_t random_state = 0x7c43a9d1;

static uint32_t next_random(void) {
    random_state ^= random_state << 13;
    random_state ^= random_state >> 17;
    random_state ^= random_state << 5;
    return random_state;
}

static int run_case(dequantize_fn c_fn, dequantize_fn rust_fn, int case_number,
                    int total_bands, int group_size, int short_limit) {
    uint8_t c_bits[BITSTREAM_BYTES], rust_bits[BITSTREAM_BYTES];
    float c_output[OUTPUT_FLOATS], rust_output[OUTPUT_FLOATS];
    L12_scale_info c_sci, rust_sci;

    for (size_t i = 0; i < sizeof(c_bits); ++i)
        c_bits[i] = (uint8_t)next_random();
    memcpy(rust_bits, c_bits, sizeof(c_bits));

    for (size_t i = 0; i < sizeof(c_output); ++i)
        ((uint8_t *)c_output)[i] = (uint8_t)next_random();
    memcpy(rust_output, c_output, sizeof(c_output));

    for (size_t i = 0; i < sizeof(c_sci); ++i)
        ((uint8_t *)&c_sci)[i] = (uint8_t)next_random();
    c_sci.total_bands = (uint8_t)total_bands;
    for (int i = 0; i < 2 * total_bands; ++i)
        c_sci.bitalloc[i] = (uint8_t)(next_random() % 21);
    memcpy(&rust_sci, &c_sci, sizeof(c_sci));

    int initial_pos = (int)(next_random() & 7);
    int limit = short_limit ? (int)(next_random() % 2048)
                            : BITSTREAM_BYTES * 8;
    bs_t c_bs = {c_bits, initial_pos, limit};
    bs_t rust_bs = {rust_bits, initial_pos, limit};

    int c_result = c_fn(c_output, &c_bs, &c_sci, group_size);
    int rust_result = rust_fn(rust_output, &rust_bs, &rust_sci, group_size);

    if (c_result != rust_result || c_bs.pos != rust_bs.pos ||
        c_bs.limit != rust_bs.limit ||
        memcmp(c_output, rust_output, sizeof(c_output)) != 0 ||
        memcmp(&c_sci, &rust_sci, sizeof(c_sci)) != 0) {
        fprintf(stderr,
                "case %d differs: bands=%d group=%d short_limit=%d "
                "result=%d/%d pos=%d/%d\n",
                case_number, total_bands, group_size, short_limit, c_result,
                rust_result, c_bs.pos, rust_bs.pos);
        return 1;
    }
    return 0;
}

int main(int argc, char **argv) {
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

    dequantize_fn c_fn = (dequantize_fn)dlsym(c_library, "dequantize_granule");
    dequantize_fn rust_fn =
        (dequantize_fn)dlsym(rust_library, "dequantize_granule");
    if (c_fn == NULL || rust_fn == NULL) {
        fprintf(stderr, "dlsym failed: %s\n", dlerror());
        return 2;
    }

    int case_number = 0;
    for (int bands = 0; bands <= 32; ++bands) {
        for (int group = 0; group <= 18; ++group) {
            if (run_case(c_fn, rust_fn, case_number++, bands, group, 0) ||
                run_case(c_fn, rust_fn, case_number++, bands, group, 1))
                return 1;
        }
    }
    while (case_number < RANDOM_CASES) {
        int bands = (int)(next_random() % 33);
        int group = (int)(next_random() % 19);
        int short_limit = (int)(next_random() & 1);
        if (run_case(c_fn, rust_fn, case_number++, bands, group, short_limit))
            return 1;
    }

    printf("%d differential cases passed\n", case_number);
    dlclose(rust_library);
    dlclose(c_library);
    return 0;
}
