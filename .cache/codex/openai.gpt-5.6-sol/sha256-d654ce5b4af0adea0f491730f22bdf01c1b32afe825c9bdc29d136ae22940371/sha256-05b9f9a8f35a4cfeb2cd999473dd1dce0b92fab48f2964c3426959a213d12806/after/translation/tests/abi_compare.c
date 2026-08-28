#include <dlfcn.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

typedef struct {
    uint32_t a;
    uint32_t b;
    uint32_t c;
    uint32_t d;
} tflac_md5;

typedef void (*md5_digest_fn)(const tflac_md5 *, uint8_t[16]);

static uint32_t next_random(uint32_t *state) {
    uint32_t value = *state;
    value ^= value << 13;
    value ^= value >> 17;
    value ^= value << 5;
    *state = value;
    return value;
}

static md5_digest_fn load_digest(const char *path, void **handle) {
    *handle = dlopen(path, RTLD_NOW | RTLD_LOCAL);
    if (*handle == NULL) {
        fprintf(stderr, "dlopen(%s): %s\n", path, dlerror());
        exit(1);
    }

    dlerror();
    md5_digest_fn digest = (md5_digest_fn)dlsym(*handle, "md5_digest");
    const char *error = dlerror();
    if (error != NULL) {
        fprintf(stderr, "dlsym(%s): %s\n", path, error);
        exit(1);
    }
    return digest;
}

static void check_known_vector(md5_digest_fn digest, const char *library) {
    const tflac_md5 state = {
        0x01234567U,
        0x89abcdefU,
        0x10203040U,
        0xff00aa55U,
    };
    const uint8_t expected[16] = {
        0x67, 0x45, 0x23, 0x01,
        0xef, 0xcd, 0xab, 0x89,
        0x40, 0x30, 0x20, 0x10,
        0x55, 0xaa, 0x00, 0xff,
    };
    uint8_t actual[16];

    digest(&state, actual);
    if (memcmp(actual, expected, sizeof(expected)) != 0) {
        fprintf(stderr, "known-vector mismatch in %s\n", library);
        exit(1);
    }
}

static void check_regular_inputs(md5_digest_fn reference, md5_digest_fn rust) {
    uint32_t random = 0x6d2b79f5U;

    for (unsigned int iteration = 0; iteration < 100000; ++iteration) {
        tflac_md5 state = {
            next_random(&random),
            next_random(&random),
            next_random(&random),
            next_random(&random),
        };
        uint8_t expected[16];
        uint8_t actual[16];

        reference(&state, expected);
        rust(&state, actual);
        if (memcmp(actual, expected, sizeof(expected)) != 0) {
            fprintf(stderr, "mismatch at generated input %u\n", iteration);
            exit(1);
        }
    }
}

static void check_overlapping_inputs(md5_digest_fn reference,
                                     md5_digest_fn rust) {
    uint32_t random = 0xa341316cU;

    for (int delta = -32; delta <= 32; ++delta) {
        uint8_t *expected = malloc(96);
        uint8_t *actual = malloc(96);
        if (expected == NULL || actual == NULL) {
            fputs("allocation failure\n", stderr);
            exit(1);
        }

        for (size_t index = 0; index < 96; ++index) {
            expected[index] = (uint8_t)next_random(&random);
        }
        memcpy(actual, expected, 96);

        tflac_md5 state = {
            next_random(&random),
            next_random(&random),
            next_random(&random),
            next_random(&random),
        };
        memcpy(expected + 40, &state, sizeof(state));
        memcpy(actual + 40, &state, sizeof(state));

        reference((const tflac_md5 *)(expected + 40), expected + 40 + delta);
        rust((const tflac_md5 *)(actual + 40), actual + 40 + delta);
        if (memcmp(actual, expected, 96) != 0) {
            fprintf(stderr, "overlap mismatch at output delta %d\n", delta);
            exit(1);
        }

        free(actual);
        free(expected);
    }
}

int main(int argc, char **argv) {
    if (argc != 3) {
        fprintf(stderr, "usage: %s REFERENCE_SO RUST_SO\n", argv[0]);
        return 2;
    }

    void *reference_handle;
    void *rust_handle;
    md5_digest_fn reference = load_digest(argv[1], &reference_handle);
    md5_digest_fn rust = load_digest(argv[2], &rust_handle);

    check_known_vector(reference, argv[1]);
    check_known_vector(rust, argv[2]);
    check_regular_inputs(reference, rust);
    check_overlapping_inputs(reference, rust);

    dlclose(rust_handle);
    dlclose(reference_handle);
    puts("ABI comparison passed");
    return 0;
}
