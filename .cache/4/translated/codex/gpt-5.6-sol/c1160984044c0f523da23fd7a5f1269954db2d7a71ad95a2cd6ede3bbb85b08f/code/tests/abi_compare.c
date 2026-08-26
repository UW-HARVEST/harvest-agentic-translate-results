#include <dlfcn.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>

typedef struct cb_rgb_255 {
    unsigned char R;
    unsigned char G;
    unsigned char B;
} cb_rgb_255;

typedef float (*contrast_ratio_fn)(cb_rgb_255, cb_rgb_255);

static contrast_ratio_fn load_function(const char *path, void **handle) {
    *handle = dlopen(path, RTLD_NOW | RTLD_LOCAL);
    if (*handle == NULL) {
        fprintf(stderr, "dlopen(%s): %s\n", path, dlerror());
        return NULL;
    }

    void *symbol = dlsym(*handle, "contrast_ratio");
    contrast_ratio_fn function;
    memcpy(&function, &symbol, sizeof(function));
    return function;
}

static uint32_t float_bits(float value) {
    uint32_t bits;
    memcpy(&bits, &value, sizeof(bits));
    return bits;
}

int main(int argc, char **argv) {
    if (argc != 3) {
        fprintf(stderr, "usage: %s C_LIBRARY RUST_LIBRARY\n", argv[0]);
        return 2;
    }

    void *c_handle;
    void *rust_handle;
    contrast_ratio_fn c_ratio = load_function(argv[1], &c_handle);
    contrast_ratio_fn rust_ratio = load_function(argv[2], &rust_handle);
    if (c_ratio == NULL || rust_ratio == NULL) {
        return 2;
    }

    for (uint32_t value = 0; value < (1U << 24); ++value) {
        cb_rgb_255 a = {
            .R = (unsigned char)value,
            .G = (unsigned char)(value >> 8),
            .B = (unsigned char)(value >> 16),
        };
        cb_rgb_255 b = {
            .R = (unsigned char)~a.G,
            .G = (unsigned char)(a.B * 197U + 101U),
            .B = (unsigned char)(a.R * 193U + 17U),
        };
        uint32_t c_bits = float_bits(c_ratio(a, b));
        uint32_t rust_bits = float_bits(rust_ratio(a, b));
        if (c_bits != rust_bits) {
            fprintf(stderr,
                    "mismatch at %u: (%u,%u,%u) (%u,%u,%u) %08x != %08x\n",
                    value, a.R, a.G, a.B, b.R, b.G, b.B, c_bits, rust_bits);
            return 1;
        }
    }

    cb_rgb_255 black = {0, 0, 0};
    if (float_bits(c_ratio(black, black)) !=
        float_bits(rust_ratio(black, black))) {
        fprintf(stderr, "black/black NaN mismatch\n");
        return 1;
    }

    dlclose(rust_handle);
    dlclose(c_handle);
    puts("bit-identical cases: 16777217");
    return 0;
}
