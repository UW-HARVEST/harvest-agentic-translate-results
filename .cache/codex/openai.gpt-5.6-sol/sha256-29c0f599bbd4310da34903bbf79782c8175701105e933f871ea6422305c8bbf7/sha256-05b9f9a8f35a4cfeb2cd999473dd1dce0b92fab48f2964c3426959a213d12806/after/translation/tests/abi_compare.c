#include <dlfcn.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

typedef int (*match_fn)(double *, double *, int, double);
typedef double (*contrast_fn)(double *, double *, int);

static uint64_t state = UINT64_C(0x6a09e667f3bcc909);

static uint64_t random_u64(void) {
    state ^= state << 13;
    state ^= state >> 7;
    state ^= state << 17;
    return state;
}

static double random_double(void) {
    return (double)(random_u64() % 1000001) / 10000.0 - 50.0;
}

static float random_float(void) {
    return (float)((int64_t)(random_u64() % 200001) - 100000) / 1000.0f;
}

static uint64_t double_bits(double value) {
    uint64_t bits;
    memcpy(&bits, &value, sizeof(bits));
    return bits;
}

static void *required_symbol(void *handle, const char *name) {
    void *symbol = dlsym(handle, name);
    const char *error = dlerror();
    if (error != NULL) {
        fprintf(stderr, "dlsym(%s): %s\n", name, error);
        exit(2);
    }
    return symbol;
}

int main(int argc, char **argv) {
    if (argc != 3) {
        fprintf(stderr, "usage: %s C_LIBRARY RUST_LIBRARY\n", argv[0]);
        return 2;
    }

    void *c_library = dlopen(argv[1], RTLD_NOW | RTLD_LOCAL);
    void *rust_library = dlopen(argv[2], RTLD_NOW | RTLD_LOCAL);
    if (c_library == NULL || rust_library == NULL) {
        fprintf(stderr, "dlopen: %s\n", dlerror());
        return 2;
    }

    match_fn c_match = (match_fn)required_symbol(c_library, "match");
    match_fn rust_match = (match_fn)required_symbol(rust_library, "match");
    contrast_fn c_contrast =
        (contrast_fn)required_symbol(c_library, "spectral_contrast");
    contrast_fn rust_contrast =
        (contrast_fn)required_symbol(rust_library, "spectral_contrast");

    static const double thresholds[] = {
        -1.0, 0.0, 0.25, 0.5, 0.75, 1.0, 1.5,
    };
    size_t match_cases = 0;
    for (int length = 1; length <= 96; ++length) {
        for (int iteration = 0; iteration < 40; ++iteration) {
            double *test = malloc((size_t)length * sizeof(*test));
            double *reference = malloc((size_t)length * sizeof(*reference));
            if (test == NULL || reference == NULL) {
                return 2;
            }
            for (int i = 0; i < length; ++i) {
                test[i] = random_double();
                reference[i] = random_double();
            }
            for (size_t i = 0; i < sizeof(thresholds) / sizeof(thresholds[0]); ++i) {
                int c_result = c_match(test, reference, length, thresholds[i]);
                int rust_result = rust_match(test, reference, length, thresholds[i]);
                if (c_result != rust_result) {
                    fprintf(stderr,
                            "match mismatch: length=%d iteration=%d threshold=%.17g C=%d Rust=%d\n",
                            length, iteration, thresholds[i], c_result, rust_result);
                    return 1;
                }
                ++match_cases;
            }
            free(test);
            free(reference);
        }
    }

    size_t contrast_cases = 0;
    for (int length = 0; length <= 96; ++length) {
        for (int iteration = 0; iteration < 100; ++iteration) {
            size_t doubles = (size_t)(length + 1) / 2 + 2;
            size_t bytes = doubles * sizeof(double);
            double *c_a = malloc(bytes);
            double *c_b = malloc(bytes);
            double *rust_a = malloc(bytes);
            double *rust_b = malloc(bytes);
            if (c_a == NULL || c_b == NULL || rust_a == NULL || rust_b == NULL) {
                return 2;
            }
            memset(c_a, 0xa5, bytes);
            memset(c_b, 0x5a, bytes);
            float *a_values = (float *)c_a;
            float *b_values = (float *)c_b;
            for (int i = 0; i < length; ++i) {
                a_values[i] = random_float();
                b_values[i] = random_float();
            }
            memcpy(rust_a, c_a, bytes);
            memcpy(rust_b, c_b, bytes);

            double c_result = c_contrast(c_a, c_b, length);
            double rust_result = rust_contrast(rust_a, rust_b, length);
            if (double_bits(c_result) != double_bits(rust_result) ||
                memcmp(c_a, rust_a, bytes) != 0 ||
                memcmp(c_b, rust_b, bytes) != 0) {
                fprintf(stderr,
                        "spectral_contrast mismatch: length=%d iteration=%d C=%016lx Rust=%016lx\n",
                        length, iteration, (unsigned long)double_bits(c_result),
                        (unsigned long)double_bits(rust_result));
                return 1;
            }
            ++contrast_cases;
            free(c_a);
            free(c_b);
            free(rust_a);
            free(rust_b);
        }
    }

    printf("matched %zu match cases and %zu spectral_contrast cases\n",
           match_cases, contrast_cases);
    dlclose(c_library);
    dlclose(rust_library);
    return 0;
}
