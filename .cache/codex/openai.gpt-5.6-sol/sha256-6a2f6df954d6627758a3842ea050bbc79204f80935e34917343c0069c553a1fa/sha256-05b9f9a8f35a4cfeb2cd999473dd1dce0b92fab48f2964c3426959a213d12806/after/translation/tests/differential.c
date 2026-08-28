#include "../../c_src/include/lib.h"

#include <dlfcn.h>
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

typedef int (*read_side_info_fn)(bs_t *, L3_gr_info_t *, const uint8_t *);

static uint32_t rng_state = 0x8d31f5a7u;

static uint32_t next_random(void) {
    uint32_t x = rng_state;
    x ^= x << 13;
    x ^= x >> 17;
    x ^= x << 5;
    rng_state = x;
    return x;
}

static read_side_info_fn load_function(void *handle) {
    void *symbol = dlsym(handle, "read_side_info");
    read_side_info_fn function;
    memcpy(&function, &symbol, sizeof(function));
    return function;
}

static int selected_table_length(const L3_gr_info_t *gr) {
    if (gr->n_short_sfb != 0) {
        return 40;
    }
    if (gr->n_long_sfb != 0) {
        return 23;
    }
    return 0;
}

int main(int argc, char **argv) {
    if (argc != 3) {
        fprintf(stderr, "usage: %s C_LIBRARY RUST_LIBRARY\n", argv[0]);
        return 2;
    }

    void *c_handle = dlopen(argv[1], RTLD_NOW | RTLD_LOCAL);
    void *rust_handle = dlopen(argv[2], RTLD_NOW | RTLD_LOCAL);
    if (c_handle == NULL || rust_handle == NULL) {
        fprintf(stderr, "dlopen failed: %s\n", dlerror());
        return 2;
    }

    read_side_info_fn c_function = load_function(c_handle);
    read_side_info_fn rust_function = load_function(rust_handle);
    if (c_function == NULL || rust_function == NULL) {
        fprintf(stderr, "dlsym failed: %s\n", dlerror());
        return 2;
    }

    if (sizeof(bs_t) != 16 || sizeof(L3_gr_info_t) != 32 ||
        offsetof(L3_gr_info_t, part_23_length) != 8 ||
        offsetof(L3_gr_info_t, scfsi) != 31) {
        fprintf(stderr, "unexpected C ABI layout\n");
        return 2;
    }

    for (unsigned iteration = 0; iteration < 500000; ++iteration) {
        uint8_t input[2048];
        uint8_t hdr[4];
        for (size_t i = 0; i < sizeof(input); ++i) {
            input[i] = (uint8_t)next_random();
        }
        for (size_t i = 0; i < sizeof(hdr); ++i) {
            hdr[i] = (uint8_t)next_random();
        }

        int raw_sr_idx = ((hdr[2] >> 2) & 3) +
                         (((hdr[1] >> 3) & 1) + ((hdr[1] >> 4) & 1)) * 3;
        raw_sr_idx -= raw_sr_idx != 0;
        if (raw_sr_idx >= 8) {
            --iteration;
            continue;
        }

        bs_t c_bs = {
            .buf = input,
            .pos = (int)(next_random() & 7),
            .limit = 128 + (int)(next_random() % 16000),
        };
        bs_t rust_bs = c_bs;
        L3_gr_info_t c_gr[4];
        L3_gr_info_t rust_gr[4];
        memset(c_gr, 0xa5, sizeof(c_gr));
        memset(rust_gr, 0xa5, sizeof(rust_gr));

        int c_result = c_function(&c_bs, c_gr, hdr);
        int rust_result = rust_function(&rust_bs, rust_gr, hdr);
        if (c_result != rust_result || c_bs.pos != rust_bs.pos ||
            c_bs.limit != rust_bs.limit || c_bs.buf != rust_bs.buf) {
            fprintf(stderr, "scalar mismatch at iteration %u\n", iteration);
            return 1;
        }

        for (size_t i = 0; i < 4; ++i) {
            const uint8_t *c_table = c_gr[i].sfbtab;
            const uint8_t *rust_table = rust_gr[i].sfbtab;
            int c_changed = c_table != (const uint8_t *)(uintptr_t)0xa5a5a5a5a5a5a5a5ull;
            int rust_changed =
                rust_table != (const uint8_t *)(uintptr_t)0xa5a5a5a5a5a5a5a5ull;

            c_gr[i].sfbtab = NULL;
            rust_gr[i].sfbtab = NULL;
            if (c_changed != rust_changed ||
                memcmp(&c_gr[i], &rust_gr[i], sizeof(c_gr[i])) != 0) {
                fprintf(stderr, "struct mismatch at iteration %u, granule %zu\n",
                        iteration, i);
                return 1;
            }

            if (c_changed) {
                int length = selected_table_length(&c_gr[i]);
                if (length != selected_table_length(&rust_gr[i]) ||
                    memcmp(c_table, rust_table, (size_t)length) != 0) {
                    fprintf(stderr,
                            "table mismatch at iteration %u, granule %zu\n",
                            iteration, i);
                    return 1;
                }
            }
        }
    }

    puts("500000 differential cases passed");
    dlclose(rust_handle);
    dlclose(c_handle);
    return 0;
}
