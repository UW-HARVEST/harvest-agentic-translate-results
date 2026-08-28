#include <dlfcn.h>
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

typedef uint8_t tflac_u8;
typedef int32_t tflac_s32;
typedef uint32_t tflac_u32;
typedef uint64_t tflac_u64;

typedef struct {
    tflac_u32 pos;
    tflac_u64 total;
    tflac_u8 buffer[64 + 8];
} tflac_md5;

typedef struct {
    tflac_md5 md5_ctx;
    tflac_u32 cur_blocksize;
    tflac_u32 channels;
} tflac;

typedef void (*pack_fn)(tflac_u8 *, tflac_u64);
typedef void (*addsample_fn)(tflac_md5 *, tflac_u32, tflac_u64);
typedef tflac_u32 (*update_fn)(tflac *, const tflac_s32 *);

typedef struct {
    void *handle;
    pack_fn pack;
    addsample_fn addsample;
    update_fn update;
} library;

static uint64_t random_state = UINT64_C(0x8d12e5f7a09bc463);

static uint64_t next_random(void) {
    uint64_t x = random_state;
    x ^= x >> 12;
    x ^= x << 25;
    x ^= x >> 27;
    random_state = x;
    return x * UINT64_C(0x2545f4914f6cdd1d);
}

static void fill_random(void *destination, size_t size) {
    uint8_t *bytes = destination;
    for (size_t i = 0; i < size; ++i) {
        bytes[i] = (uint8_t)next_random();
    }
}

static void load_function(void *handle, const char *name, void *destination,
                          size_t destination_size) {
    dlerror();
    void *symbol = dlsym(handle, name);
    const char *error = dlerror();
    if (error != NULL || symbol == NULL) {
        fprintf(stderr, "could not load %s: %s\n", name,
                error == NULL ? "missing symbol" : error);
        exit(2);
    }
    if (destination_size != sizeof(symbol)) {
        fprintf(stderr, "unexpected function pointer size for %s\n", name);
        exit(2);
    }
    memcpy(destination, &symbol, sizeof(symbol));
}

static library load_library(const char *path) {
    library result = {0};
    result.handle = dlopen(path, RTLD_NOW | RTLD_LOCAL);
    if (result.handle == NULL) {
        fprintf(stderr, "could not load %s: %s\n", path, dlerror());
        exit(2);
    }
    load_function(result.handle, "tflac_pack_u64le", &result.pack,
                  sizeof(result.pack));
    load_function(result.handle, "tflac_md5_addsample", &result.addsample,
                  sizeof(result.addsample));
    load_function(result.handle, "update_md5", &result.update,
                  sizeof(result.update));
    return result;
}

static void compare_pack(const library *c, const library *rust) {
    static const uint64_t boundary_values[] = {
        0,
        1,
        UINT64_MAX,
        UINT64_C(0x0123456789abcdef),
    };
    for (size_t iteration = 0; iteration < 10000; ++iteration) {
        uint8_t c_bytes[96];
        uint8_t rust_bytes[96];
        fill_random(c_bytes, sizeof(c_bytes));
        memcpy(rust_bytes, c_bytes, sizeof(c_bytes));
        size_t offset = iteration < 2
                            ? iteration * (sizeof(c_bytes) - 8)
                            : (size_t)(next_random() % (sizeof(c_bytes) - 7));
        uint64_t value =
            iteration < sizeof(boundary_values) / sizeof(boundary_values[0])
                ? boundary_values[iteration]
                : next_random();

        c->pack(c_bytes + offset, value);
        rust->pack(rust_bytes + offset, value);
        if (memcmp(c_bytes, rust_bytes, sizeof(c_bytes)) != 0) {
            fprintf(stderr, "tflac_pack_u64le differs at iteration %zu\n",
                    iteration);
            exit(1);
        }
    }
}

static void compare_addsample(const library *c, const library *rust) {
    static const uint32_t boundary_positions[] = {0, 55, 56, 57, 63, 64};
    static const uint32_t boundary_bits[] = {0, 1, 7, 8, 63, 64, 71};
    const size_t boundary_count =
        sizeof(boundary_positions) / sizeof(boundary_positions[0]) *
        sizeof(boundary_bits) / sizeof(boundary_bits[0]);

    for (size_t iteration = 0; iteration < 100000; ++iteration) {
        tflac_md5 c_md5;
        tflac_md5 rust_md5;
        fill_random(&c_md5, sizeof(c_md5));
        memcpy(&rust_md5, &c_md5, sizeof(c_md5));

        uint32_t pos =
            iteration < boundary_count
                ? boundary_positions[iteration %
                                     (sizeof(boundary_positions) /
                                      sizeof(boundary_positions[0]))]
                : (uint32_t)(next_random() % 65);
        uint32_t bits =
            iteration < boundary_count
                ? boundary_bits[(iteration /
                                 (sizeof(boundary_positions) /
                                  sizeof(boundary_positions[0]))) %
                                (sizeof(boundary_bits) /
                                 sizeof(boundary_bits[0]))]
                : (uint32_t)(next_random() % 72);
        uint64_t total =
            iteration < boundary_count ? UINT64_MAX - 32 : next_random();
        uint64_t value = next_random();
        c_md5.pos = rust_md5.pos = pos;
        c_md5.total = rust_md5.total = total;

        c->addsample(&c_md5, bits, value);
        rust->addsample(&rust_md5, bits, value);
        if (memcmp(&c_md5, &rust_md5, sizeof(c_md5)) != 0) {
            fprintf(stderr,
                    "tflac_md5_addsample differs at iteration %zu "
                    "(pos=%u, bits=%u)\n",
                    iteration, pos, bits);
            exit(1);
        }
    }
}

static void compare_update(const library *c, const library *rust) {
    static const uint32_t boundary_dimensions[][2] = {
        {0, 0},
        {1, 1},
        {UINT32_MAX, UINT32_MAX},
        {0, UINT32_MAX},
        {5, 7},
        {UINT32_MAX, 2},
    };
    for (size_t iteration = 0; iteration < 100000; ++iteration) {
        tflac c_state;
        tflac rust_state;
        tflac_s32 samples[136];
        fill_random(&c_state, sizeof(c_state));
        memcpy(&rust_state, &c_state, sizeof(c_state));
        fill_random(samples, sizeof(samples));

        c_state.md5_ctx.pos = rust_state.md5_ctx.pos =
            iteration < 8 ? (uint32_t)(56 + iteration) :
                            (uint32_t)(next_random() % 64);
        c_state.md5_ctx.total = rust_state.md5_ctx.total =
            iteration < 8 ? UINT64_MAX - 128 : next_random();
        if (iteration <
            sizeof(boundary_dimensions) / sizeof(boundary_dimensions[0])) {
            c_state.cur_blocksize = rust_state.cur_blocksize =
                boundary_dimensions[iteration][0];
            c_state.channels = rust_state.channels =
                boundary_dimensions[iteration][1];
        } else {
            c_state.cur_blocksize = rust_state.cur_blocksize =
                (uint32_t)next_random();
            c_state.channels = rust_state.channels = (uint32_t)next_random();
        }

        uint32_t c_result = c->update(&c_state, samples);
        uint32_t rust_result = rust->update(&rust_state, samples);
        if (c_result != rust_result ||
            memcmp(&c_state, &rust_state, sizeof(c_state)) != 0) {
            fprintf(stderr, "update_md5 differs at iteration %zu\n", iteration);
            exit(1);
        }
    }
}

int main(int argc, char **argv) {
    if (argc != 3) {
        fprintf(stderr, "usage: %s C_LIBRARY RUST_LIBRARY\n", argv[0]);
        return 2;
    }

    if (sizeof(tflac_md5) != 88 || offsetof(tflac_md5, total) != 8 ||
        offsetof(tflac_md5, buffer) != 16 || sizeof(tflac) != 96 ||
        offsetof(tflac, cur_blocksize) != 88 ||
        offsetof(tflac, channels) != 92) {
        fprintf(stderr, "unexpected C ABI layout\n");
        return 2;
    }

    library c = load_library(argv[1]);
    library rust = load_library(argv[2]);
    compare_pack(&c, &rust);
    compare_addsample(&c, &rust);
    compare_update(&c, &rust);
    dlclose(rust.handle);
    dlclose(c.handle);

    puts("all ABI differential tests passed");
    return 0;
}
