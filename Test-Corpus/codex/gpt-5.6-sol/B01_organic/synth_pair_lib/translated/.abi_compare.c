#include <dlfcn.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

typedef void (*synth_pair_fn)(int16_t *, int, const float *);

static uint32_t random_state = 0x243f6a88U;

static uint32_t next_random(void) {
    random_state ^= random_state << 13;
    random_state ^= random_state >> 17;
    random_state ^= random_state << 5;
    return random_state;
}

static synth_pair_fn load_function(void *library) {
    synth_pair_fn function;
    *(void **)(&function) = dlsym(library, "synth_pair");
    return function;
}

static void compare_case(
    synth_pair_fn c_function,
    synth_pair_fn rust_function,
    const float *z,
    int nch,
    unsigned int case_number
) {
    int16_t c_pcm[64];
    int16_t rust_pcm[64];

    memset(c_pcm, 0xa5, sizeof(c_pcm));
    memset(rust_pcm, 0xa5, sizeof(rust_pcm));
    c_function(c_pcm, nch, z);
    rust_function(rust_pcm, nch, z);

    if (memcmp(c_pcm, rust_pcm, sizeof(c_pcm)) != 0) {
        fprintf(
            stderr,
            "mismatch in case %u (nch=%d): C=(%d,%d), Rust=(%d,%d)\n",
            case_number,
            nch,
            c_pcm[0],
            c_pcm[16 * nch],
            rust_pcm[0],
            rust_pcm[16 * nch]
        );
        exit(1);
    }
}

int main(int argc, char **argv) {
    float z[899] = {0};
    void *c_library;
    void *rust_library;
    synth_pair_fn c_function;
    synth_pair_fn rust_function;
    unsigned int case_number = 0;

    if (argc != 3) {
        fprintf(stderr, "usage: %s C_LIBRARY RUST_LIBRARY\n", argv[0]);
        return 2;
    }

    c_library = dlopen(argv[1], RTLD_NOW | RTLD_LOCAL);
    rust_library = dlopen(argv[2], RTLD_NOW | RTLD_LOCAL);
    if (c_library == NULL || rust_library == NULL) {
        fprintf(stderr, "dlopen failed: %s\n", dlerror());
        return 2;
    }

    c_function = load_function(c_library);
    rust_function = load_function(rust_library);
    if (c_function == NULL || rust_function == NULL) {
        fprintf(stderr, "dlsym failed: %s\n", dlerror());
        return 2;
    }

    compare_case(c_function, rust_function, z, 1, case_number++);

    for (size_t i = 0; i < 899; ++i) {
        z[i] = (i & 1U) == 0 ? 32766.5f : -32767.5f;
    }
    compare_case(c_function, rust_function, z, 2, case_number++);

    for (unsigned int iteration = 0; iteration < 20000; ++iteration) {
        for (size_t i = 0; i < 899; ++i) {
            int32_t centered = (int32_t)(next_random() >> 8) - 8388608;
            z[i] = (float)centered / (8388608.0f * 1024.0f);
        }
        compare_case(c_function, rust_function, z, 1 + iteration % 3, case_number++);
    }

    z[0] = __builtin_nanf("");
    compare_case(c_function, rust_function, z, 1, case_number++);
    z[0] = __builtin_inff();
    compare_case(c_function, rust_function, z, 2, case_number++);
    z[0] = -__builtin_inff();
    compare_case(c_function, rust_function, z, 3, case_number++);

    printf("matched %u cases\n", case_number);
    dlclose(rust_library);
    dlclose(c_library);
    return 0;
}
