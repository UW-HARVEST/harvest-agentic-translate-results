#include <dlfcn.h>
#include <limits.h>
#include <stdio.h>
#include <stdlib.h>

typedef char *(*create_result_string_fn)(const char *, int);
typedef int (*check_permissions_fn)(int, int);
typedef int (*safe_add_fn)(int, int, int);
typedef int (*multiply_with_log_fn)(int, int, char **);
typedef int (*copy_and_sum_fn)(int *, int);
typedef int (*compare_operations_fn)(const char *, const char *);
typedef int (*complexmode_fn)(int, int, int, int);

#define LOAD(name)                                                             \
    name##_fn name = (name##_fn)dlsym(handle, #name);                          \
    if (name == NULL) {                                                        \
        fprintf(stderr, "missing symbol: %s\n", #name);                        \
        return 2;                                                              \
    }

int main(int argc, char **argv) {
    if (argc != 2) {
        return 2;
    }

    void *handle = dlopen(argv[1], RTLD_NOW | RTLD_LOCAL);
    if (handle == NULL) {
        fprintf(stderr, "%s\n", dlerror());
        return 2;
    }

    LOAD(create_result_string);
    LOAD(check_permissions);
    LOAD(safe_add);
    LOAD(multiply_with_log);
    LOAD(copy_and_sum);
    LOAD(compare_operations);
    LOAD(complexmode);

    printf("permissions:%d,%d,%d\n",
           check_permissions(0600, 0600),
           check_permissions(0400, 0600),
           check_permissions(-1, 0100));

    printf("safe:%d,%d,%d\n",
           safe_add(12, -5, 0600),
           safe_add(12, -5, 0400),
           safe_add(INT_MAX, 1, 0600));

    char *created = create_result_string("addition", -12345);
    printf("created:%s\n", created);
    free(created);

    created = create_result_string(
        "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789", 77);
    printf("truncated:%s\n", created);
    free(created);

    char *log = NULL;
    int product = multiply_with_log(-123, 456, &log);
    printf("multiply:%d:%s\n", product, log);
    free(log);

    log = NULL;
    product = multiply_with_log(INT_MAX, 2, &log);
    printf("multiply-overflow:%d:%s\n", product, log);
    free(log);

    int values[] = {1, -2, 3, 1000000};
    int overflow_values[] = {INT_MAX, 1};
    printf("sum:%d,%d,%d,%d\n",
           copy_and_sum(values, 4),
           copy_and_sum(values, 0),
           copy_and_sum(overflow_values, 2),
           copy_and_sum(values, -1));
    printf("sum-null:%d\n", copy_and_sum(NULL, 3));

    printf("compare:%d,%d,%d\n",
           compare_operations("same", "same"),
           compare_operations("alpha", "beta"),
           compare_operations("beta", "alpha"));
    printf("compare-null:%d,%d\n",
           compare_operations(NULL, "x"),
           compare_operations("x", NULL));

    for (int mode = 0; mode <= 5; ++mode) {
        printf("complex-return-%d:%d\n",
               mode,
               complexmode(mode, 7, -3, 11));
    }
    printf("complex-overflow:%d\n", complexmode(1, INT_MAX, 1, 0));

    dlclose(handle);
    return 0;
}
