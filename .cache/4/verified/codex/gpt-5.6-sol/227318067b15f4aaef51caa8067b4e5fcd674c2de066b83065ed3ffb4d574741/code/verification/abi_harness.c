#include <dlfcn.h>
#include <limits.h>
#include <stddef.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

typedef int (*int_func)(int);
typedef int (*apply_func)(int_func, int);
typedef int (*charinbuf_func)(int, int, int, int);
typedef char *(*create_buffer_func)(const char *);
typedef char *(*find_char_func)(const char *, size_t, char);
typedef int (*string_empty_func)(const char *);

static int callback(int value) {
    return value * 3 + 1;
}

static void *load_symbol(void *library, const char *name) {
    dlerror();
    void *symbol = dlsym(library, name);
    const char *error = dlerror();
    if (error != NULL) {
        fprintf(stderr, "dlsym(%s): %s\n", name, error);
        exit(2);
    }
    return symbol;
}

#define LOAD(variable, library, name) \
    do { \
        void *symbol = load_symbol((library), (name)); \
        memcpy(&(variable), &symbol, sizeof(variable)); \
    } while (0)

int main(int argc, char **argv) {
    if (argc != 2) {
        fprintf(stderr, "usage: %s LIBRARY\n", argv[0]);
        return 2;
    }

    void *library = dlopen(argv[1], RTLD_NOW | RTLD_LOCAL);
    if (library == NULL) {
        fprintf(stderr, "dlopen: %s\n", dlerror());
        return 2;
    }

    int_func increment_counter;
    int_func decrement_counter;
    int_func multiply_counter;
    int_func reset_counter;
    int_func validate_uint16_range;
    apply_func apply_operation;
    charinbuf_func charinbuf;
    create_buffer_func create_buffer;
    find_char_func find_char_in_buffer;
    string_empty_func is_string_empty;

    LOAD(increment_counter, library, "increment_counter");
    LOAD(decrement_counter, library, "decrement_counter");
    LOAD(multiply_counter, library, "multiply_counter");
    LOAD(reset_counter, library, "reset_counter");
    LOAD(validate_uint16_range, library, "validate_uint16_range");
    LOAD(apply_operation, library, "apply_operation");
    LOAD(charinbuf, library, "charinbuf");
    LOAD(create_buffer, library, "create_buffer");
    LOAD(find_char_in_buffer, library, "find_char_in_buffer");
    LOAD(is_string_empty, library, "is_string_empty");

    printf("empty:%d,%d,%d\n",
           is_string_empty(NULL),
           is_string_empty(""),
           is_string_empty("x"));

    const char sample[] = {'a', '\0', 'b', 'X', 'z'};
    char *found_x = find_char_in_buffer(sample, sizeof(sample), 'X');
    char *found_nul = find_char_in_buffer(sample, sizeof(sample), '\0');
    char *not_found = find_char_in_buffer(sample, sizeof(sample), 'q');
    printf("find:%td,%td,%d,%d\n",
           found_x - sample,
           found_nul - sample,
           not_found == NULL,
           find_char_in_buffer(NULL, 5, 'x') == NULL);

    char *copy = create_buffer("copy me");
    printf("create:%d,%zu,%d\n",
           copy != NULL && strcmp(copy, "copy me") == 0,
           copy == NULL ? 0 : strlen(copy),
           create_buffer(NULL) == NULL);
    free(copy);

    const int range_values[] = {INT_MIN, -1, 0, 1, 65535, 65536, INT_MAX};
    printf("range:");
    for (size_t index = 0; index < sizeof(range_values) / sizeof(range_values[0]); ++index) {
        printf("%s%d", index == 0 ? "" : ",", validate_uint16_range(range_values[index]));
    }
    printf("\n");

    printf("apply:%d,%d\n", apply_operation(NULL, 9), apply_operation(callback, 9));

    int reset_result = reset_counter(12);
    int increment_result = increment_counter(5);
    int multiply_result = multiply_counter(-3);
    int decrement_result = decrement_counter(4);
    printf("counter:%d,%d,%d,%d\n",
           reset_result,
           increment_result,
           multiply_result,
           decrement_result);

    reset_result = reset_counter(INT_MAX);
    increment_result = increment_counter(1);
    multiply_result = multiply_counter(-1);
    decrement_result = decrement_counter(INT_MAX);
    printf("counter-extremes:%d,%d,%d,%d\n",
           reset_result,
           increment_result,
           multiply_result,
           decrement_result);

    const int cases[][4] = {
        {-7, 11, 22, 33},
        {0, -1, 0, 0},
        {0, INT_MIN, 0, 0},
        {0, 0, 0, 0},
        {0, 65535, 0, 0},
        {0, 65536, 0, 0},
        {0, INT_MAX, 0, 0},
        {1, 0, 0, 0},
        {2, 0, 0, 0},
        {3, 7, 4, 3},
        {3, -9, -2, 5},
        {3, INT_MAX, 1, -1},
        {4, 0, 0, 0},
    };
    for (size_t index = 0; index < sizeof(cases) / sizeof(cases[0]); ++index) {
        int result = charinbuf(cases[index][0], cases[index][1], cases[index][2], cases[index][3]);
        printf("charinbuf-result:%zu:%d\n", index, result);
    }

    if (dlclose(library) != 0) {
        fprintf(stderr, "dlclose: %s\n", dlerror());
        return 2;
    }
    return 0;
}
