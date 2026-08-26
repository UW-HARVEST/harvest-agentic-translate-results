#include <dlfcn.h>
#include <limits.h>
#include <math.h>
#include <stddef.h>
#include <stdio.h>
#include <stdlib.h>

typedef int (*convert_fn)(double);
typedef int (*find_fn)(const char *, size_t, int);
typedef int (*negate_fn)(int);
typedef void (*create_fn)(char *, int, int);
typedef double (*calculate_fn)(int, int, int);
typedef int (*doubleneg_fn)(int, int, int, int);

static void *load_symbol(void *library, const char *name) {
    void *symbol = dlsym(library, name);
    const char *error = dlerror();
    if (error != NULL) {
        fprintf(stderr, "%s\n", error);
        exit(2);
    }
    return symbol;
}

int main(int argc, char **argv) {
    if (argc != 2) {
        return 2;
    }

    void *library = dlopen(argv[1], RTLD_NOW | RTLD_LOCAL);
    if (library == NULL) {
        fprintf(stderr, "%s\n", dlerror());
        return 2;
    }

    convert_fn convert = (convert_fn)load_symbol(library, "convert_double_to_int");
    find_fn find = (find_fn)load_symbol(library, "find_value_in_buffer");
    negate_fn negate = (negate_fn)load_symbol(library, "process_negation");
    create_fn create = (create_fn)load_symbol(library, "create_numeric_buffer");
    calculate_fn calculate =
        (calculate_fn)load_symbol(library, "calculate_with_doubles");
    doubleneg_fn doubleneg = (doubleneg_fn)load_symbol(library, "doubleneg");

    const double conversions[] = {
        0.0,
        -0.0,
        3.75,
        -3.75,
        2147483647.0,
        2147483648.0,
        -2147483648.0,
        -2147483649.0,
        INFINITY,
        -INFINITY,
        NAN,
    };
    for (size_t i = 0; i < sizeof(conversions) / sizeof(conversions[0]); ++i) {
        printf("convert[%zu]=%d\n", i, convert(conversions[i]));
    }

    const int negations[] = {INT_MIN, -1, 0, 1, INT_MAX};
    for (size_t i = 0; i < sizeof(negations) / sizeof(negations[0]); ++i) {
        printf("negate[%zu]=%d\n", i, negate(negations[i]));
    }

    const int seeds[] = {0, -257, 255, INT_MAX};
    for (size_t seed_index = 0;
         seed_index < sizeof(seeds) / sizeof(seeds[0]);
         ++seed_index) {
        char buffer[32] = {0};
        create(buffer, 32, seeds[seed_index]);
        printf("buffer[%zu]=", seed_index);
        for (size_t i = 0; i < sizeof(buffer); ++i) {
            printf("%02x", (unsigned char)buffer[i]);
        }
        printf("\n");
        printf(
            "find[%zu]=%d,%d,%d\n",
            seed_index,
            find(buffer, sizeof(buffer), seeds[seed_index]),
            find(buffer, sizeof(buffer), 42),
            find(buffer, 0, 0));
    }

    char unchanged[4] = {1, 2, 3, 4};
    create(unchanged, -1, 99);
    printf(
        "negative-size=%d,%d,%d,%d\n",
        unchanged[0],
        unchanged[1],
        unchanged[2],
        unchanged[3]);

    const int calculations[][3] = {
        {1, 2, 3},
        {-7, 3, -4},
        {INT_MAX, 1, 9},
        {1, 0, 5},
        {0, 0, INT_MIN},
    };
    for (size_t i = 0; i < sizeof(calculations) / sizeof(calculations[0]); ++i) {
        double value =
            calculate(calculations[i][0], calculations[i][1], calculations[i][2]);
        printf("calculate[%zu]=%a\n", i, value);
    }

    const int calls[][4] = {
        {1, 2, 3, 4},
        {0, 0, 0, 0},
        {-17, 5, -3, 260},
        {INT_MAX, -1, INT_MIN, 42},
    };
    for (size_t i = 0; i < sizeof(calls) / sizeof(calls[0]); ++i) {
        int result = doubleneg(calls[i][0], calls[i][1], calls[i][2], calls[i][3]);
        printf("doubleneg-return[%zu]=%d\n", i, result);
    }

    return dlclose(library) == 0 ? 0 : 2;
}
