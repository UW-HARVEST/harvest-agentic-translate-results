#include <dlfcn.h>
#include <math.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

typedef int (*match_fn)(double *, double *, int, double);
typedef double (*spectral_contrast_fn)(double *, double *, int);

struct library {
    void *handle;
    match_fn match;
    spectral_contrast_fn spectral_contrast;
};

static uint64_t state = UINT64_C(0xd1b54a32d192ed03);

static uint64_t random_u64(void) {
    state ^= state >> 12;
    state ^= state << 25;
    state ^= state >> 27;
    return state * UINT64_C(0x2545f4914f6cdd1d);
}

static double random_value(void) {
    int64_t value = (int64_t)(random_u64() % 2000001) - 1000000;
    return (double)value / 8192.0;
}

static uint64_t double_bits(double value) {
    uint64_t bits;
    memcpy(&bits, &value, sizeof(bits));
    return bits;
}

static void *load_symbol(void *handle, const char *name) {
    void *symbol;
    const char *error;

    dlerror();
    symbol = dlsym(handle, name);
    error = dlerror();
    if (error != NULL) {
        fprintf(stderr, "dlsym(%s): %s\n", name, error);
        exit(2);
    }
    return symbol;
}

static struct library load_library(const char *path) {
    struct library library;
    void *symbol;

    library.handle = dlopen(path, RTLD_NOW | RTLD_LOCAL);
    if (library.handle == NULL) {
        fprintf(stderr, "dlopen(%s): %s\n", path, dlerror());
        exit(2);
    }

    symbol = load_symbol(library.handle, "match");
    memcpy(&library.match, &symbol, sizeof(library.match));
    symbol = load_symbol(library.handle, "spectral_contrast");
    memcpy(&library.spectral_contrast, &symbol, sizeof(library.spectral_contrast));
    return library;
}

static int check_spectral(
    const struct library *c_library,
    const struct library *rust_library,
    const double *a,
    const double *b,
    int length,
    int alias
) {
    size_t bytes = (size_t)length * sizeof(double);
    double *c_a = malloc(bytes);
    double *c_b = alias ? c_a : malloc(bytes);
    double *rust_a = malloc(bytes);
    double *rust_b = alias ? rust_a : malloc(bytes);
    double c_result;
    double rust_result;
    int failed;

    if (c_a == NULL || c_b == NULL || rust_a == NULL || rust_b == NULL) {
        fputs("allocation failed\n", stderr);
        exit(2);
    }

    memcpy(c_a, a, bytes);
    memcpy(rust_a, a, bytes);
    if (!alias) {
        memcpy(c_b, b, bytes);
        memcpy(rust_b, b, bytes);
    }

    c_result = c_library->spectral_contrast(c_a, c_b, length);
    rust_result = rust_library->spectral_contrast(rust_a, rust_b, length);
    failed = double_bits(c_result) != double_bits(rust_result)
        || memcmp(c_a, rust_a, bytes) != 0
        || (!alias && memcmp(c_b, rust_b, bytes) != 0);
    if (failed) {
        fprintf(
            stderr,
            "spectral mismatch: length=%d alias=%d C=%016lx Rust=%016lx\n",
            length,
            alias,
            (unsigned long)double_bits(c_result),
            (unsigned long)double_bits(rust_result)
        );
    }

    free(c_a);
    if (!alias) {
        free(c_b);
    }
    free(rust_a);
    if (!alias) {
        free(rust_b);
    }
    return failed;
}

static int check_match(
    const struct library *c_library,
    const struct library *rust_library,
    const double *test,
    const double *reference,
    int bins,
    double threshold
) {
    size_t bytes = (size_t)bins * sizeof(double);
    double *c_test = malloc(bytes);
    double *c_reference = malloc(bytes);
    double *rust_test = malloc(bytes);
    double *rust_reference = malloc(bytes);
    int c_result;
    int rust_result;
    int failed;

    if (c_test == NULL || c_reference == NULL
        || rust_test == NULL || rust_reference == NULL) {
        fputs("allocation failed\n", stderr);
        exit(2);
    }

    memcpy(c_test, test, bytes);
    memcpy(c_reference, reference, bytes);
    memcpy(rust_test, test, bytes);
    memcpy(rust_reference, reference, bytes);
    c_result = c_library->match(c_test, c_reference, bins, threshold);
    rust_result = rust_library->match(
        rust_test,
        rust_reference,
        bins,
        threshold
    );
    failed = c_result != rust_result
        || memcmp(c_test, rust_test, bytes) != 0
        || memcmp(c_reference, rust_reference, bytes) != 0;
    if (failed) {
        fprintf(
            stderr,
            "match mismatch: bins=%d threshold=%a C=%d Rust=%d\n",
            bins,
            threshold,
            c_result,
            rust_result
        );
    }

    free(c_test);
    free(c_reference);
    free(rust_test);
    free(rust_reference);
    return failed;
}

int main(int argc, char **argv) {
    static const double thresholds[] = {
        -1.0, 0.0, 0.125, 0.5, 0.9, 1.0, 2.0, NAN
    };
    struct library c_library;
    struct library rust_library;
    int checks = 0;
    int length;

    if (argc != 3) {
        fprintf(stderr, "usage: %s C_LIBRARY RUST_LIBRARY\n", argv[0]);
        return 2;
    }
    c_library = load_library(argv[1]);
    rust_library = load_library(argv[2]);

    for (length = 1; length <= 256; ++length) {
        size_t bytes = (size_t)length * sizeof(double);
        double *a = malloc(bytes);
        double *b = malloc(bytes);
        int sample;

        if (a == NULL || b == NULL) {
            fputs("allocation failed\n", stderr);
            return 2;
        }
        for (sample = 0; sample < 32; ++sample) {
            size_t i;
            size_t threshold_index;

            for (i = 0; i < (size_t)length; ++i) {
                a[i] = random_value();
                b[i] = random_value();
            }
            if (check_spectral(&c_library, &rust_library, a, b, length, 0)
                || check_spectral(&c_library, &rust_library, a, b, length, 1)) {
                return 1;
            }
            checks += 2;

            for (threshold_index = 0;
                 threshold_index < sizeof(thresholds) / sizeof(thresholds[0]);
                 ++threshold_index) {
                if (check_match(
                        &c_library,
                        &rust_library,
                        a,
                        b,
                        length,
                        thresholds[threshold_index])) {
                    return 1;
                }
                ++checks;
            }
        }

        memset(a, 0, bytes);
        memset(b, 0, bytes);
        if (check_spectral(&c_library, &rust_library, a, b, length, 0)
            || check_match(&c_library, &rust_library, a, b, length, 0.5)) {
            return 1;
        }
        checks += 2;
        free(a);
        free(b);
    }

    printf("all %d differential checks passed\n", checks);
    dlclose(rust_library.handle);
    dlclose(c_library.handle);
    return 0;
}
