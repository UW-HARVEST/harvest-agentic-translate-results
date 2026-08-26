#include <dlfcn.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

typedef void (*tfm_fn)(float *, const float *, int);

static uint32_t rng_state = 0x91e10da5u;

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

static uint32_t to_bits(float value) {
    uint32_t bits;
    memcpy(&bits, &value, sizeof(bits));
    return bits;
}

static int compare_one(tfm_fn c_tfm, tfm_fn rust_tfm, const float src[3],
                       uint64_t case_number) {
    float c_dest[2] = {from_bits(0xa5a5a5a5u), from_bits(0x5a5a5a5au)};
    float rust_dest[2] = {c_dest[0], c_dest[1]};

    c_tfm(c_dest, src, 1);
    rust_tfm(rust_dest, src, 1);

    if (memcmp(c_dest, rust_dest, sizeof(c_dest)) == 0) {
        return 0;
    }

    fprintf(stderr,
            "mismatch at case %llu: src=%08x,%08x,%08x "
            "c=%08x,%08x rust=%08x,%08x\n",
            (unsigned long long)case_number, to_bits(src[0]), to_bits(src[1]),
            to_bits(src[2]), to_bits(c_dest[0]), to_bits(c_dest[1]),
            to_bits(rust_dest[0]), to_bits(rust_dest[1]));
    return 1;
}

static int compare_overlap(tfm_fn c_tfm, tfm_fn rust_tfm, int dest_offset,
                           int src_offset, int count) {
    enum { BUFFER_FLOATS = 128 };
    float c_buffer[BUFFER_FLOATS];
    float rust_buffer[BUFFER_FLOATS];

    for (int i = 0; i < BUFFER_FLOATS; ++i) {
        c_buffer[i] = from_bits(next_u32());
    }
    memcpy(rust_buffer, c_buffer, sizeof(c_buffer));

    c_tfm(c_buffer + dest_offset, c_buffer + src_offset, count);
    rust_tfm(rust_buffer + dest_offset, rust_buffer + src_offset, count);

    if (memcmp(c_buffer, rust_buffer, sizeof(c_buffer)) == 0) {
        return 0;
    }

    fprintf(stderr, "overlap mismatch: dest=%d src=%d count=%d\n", dest_offset,
            src_offset, count);
    return 1;
}

static tfm_fn load_tfm(const char *path, void **handle) {
    *handle = dlopen(path, RTLD_NOW | RTLD_LOCAL);
    if (*handle == NULL) {
        fprintf(stderr, "dlopen(%s): %s\n", path, dlerror());
        exit(2);
    }

    dlerror();
    void *symbol = dlsym(*handle, "tfm");
    const char *error = dlerror();
    if (error != NULL) {
        fprintf(stderr, "dlsym(%s, tfm): %s\n", path, error);
        exit(2);
    }

    tfm_fn function;
    memcpy(&function, &symbol, sizeof(function));
    return function;
}

int main(int argc, char **argv) {
    static const uint32_t edge_bits[] = {
        0x00000000u, 0x80000000u, 0x00000001u, 0x80000001u, 0x007fffffu,
        0x807fffffu, 0x00800000u, 0x80800000u, 0x3f000000u, 0xbf000000u,
        0x3f800000u, 0xbf800000u, 0x40000000u, 0xc0000000u, 0x7f7fffffu,
        0xff7fffffu, 0x7f800000u, 0xff800000u, 0x7fc00000u, 0xffc00000u,
        0x7fa00001u, 0xffa00001u, 0x7fffffffu, 0xffffffffu,
    };
    void *c_handle;
    void *rust_handle;
    uint64_t cases = 0;

    if (argc != 3) {
        fprintf(stderr, "usage: %s C_LIBRARY RUST_LIBRARY\n", argv[0]);
        return 2;
    }

    tfm_fn c_tfm = load_tfm(argv[1], &c_handle);
    tfm_fn rust_tfm = load_tfm(argv[2], &rust_handle);

    c_tfm(NULL, NULL, 0);
    rust_tfm(NULL, NULL, 0);
    c_tfm(NULL, NULL, -1);
    rust_tfm(NULL, NULL, -1);

    for (size_t i = 0; i < sizeof(edge_bits) / sizeof(edge_bits[0]); ++i) {
        for (size_t j = 0; j < sizeof(edge_bits) / sizeof(edge_bits[0]); ++j) {
            for (size_t k = 0; k < sizeof(edge_bits) / sizeof(edge_bits[0]); ++k) {
                float src[3] = {from_bits(edge_bits[i]), from_bits(edge_bits[j]),
                                from_bits(edge_bits[k])};
                if (compare_one(c_tfm, rust_tfm, src, cases++) != 0) {
                    return 1;
                }
            }
        }
    }

    for (int i = 0; i < 1000000; ++i) {
        float src[3] = {from_bits(next_u32()), from_bits(next_u32()),
                        from_bits(next_u32())};
        if (compare_one(c_tfm, rust_tfm, src, cases++) != 0) {
            return 1;
        }
    }

    for (int i = 0; i < 1000; ++i) {
        float c_src[51];
        float rust_src[51];
        float c_dest[34];
        float rust_dest[34];

        for (size_t j = 0; j < sizeof(c_src) / sizeof(c_src[0]); ++j) {
            c_src[j] = from_bits(next_u32());
        }
        memcpy(rust_src, c_src, sizeof(c_src));
        memset(c_dest, 0xa5, sizeof(c_dest));
        memcpy(rust_dest, c_dest, sizeof(c_dest));

        c_tfm(c_dest, c_src, 17);
        rust_tfm(rust_dest, rust_src, 17);
        if (memcmp(c_dest, rust_dest, sizeof(c_dest)) != 0) {
            fprintf(stderr, "batch mismatch at batch %d\n", i);
            return 1;
        }
        cases += 17;
    }

    if (compare_overlap(c_tfm, rust_tfm, 0, 0, 17) != 0 ||
        compare_overlap(c_tfm, rust_tfm, 1, 0, 17) != 0 ||
        compare_overlap(c_tfm, rust_tfm, 2, 0, 17) != 0 ||
        compare_overlap(c_tfm, rust_tfm, 0, 2, 17) != 0) {
        return 1;
    }

    dlclose(rust_handle);
    dlclose(c_handle);
    printf("bit-identical outputs for %llu scalar values plus overlap cases\n",
           (unsigned long long)cases);
    return 0;
}
