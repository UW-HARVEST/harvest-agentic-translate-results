#include <dlfcn.h>
#include <stdio.h>
#include <stdlib.h>

typedef char *(*create_result_string_fn)(const char *, int);
typedef int (*check_permissions_fn)(int, int);
typedef int (*safe_add_fn)(int, int, int);
typedef int (*multiply_with_log_fn)(int, int, char **);
typedef int (*copy_and_sum_fn)(int *, int);
typedef int (*compare_operations_fn)(const char *, const char *);
typedef int (*complexmode_fn)(int, int, int, int);

#define LOAD(handle, name) (*(void **)(&name) = dlsym(handle, #name))

int main(int argc, char **argv) {
    if (argc != 2) {
        return 2;
    }

    void *handle = dlopen(argv[1], RTLD_NOW | RTLD_LOCAL);
    if (handle == NULL) {
        fprintf(stderr, "%s\n", dlerror());
        return 3;
    }

    create_result_string_fn create_result_string;
    check_permissions_fn check_permissions;
    safe_add_fn safe_add;
    multiply_with_log_fn multiply_with_log;
    copy_and_sum_fn copy_and_sum;
    compare_operations_fn compare_operations;
    complexmode_fn complexmode;

    LOAD(handle, create_result_string);
    LOAD(handle, check_permissions);
    LOAD(handle, safe_add);
    LOAD(handle, multiply_with_log);
    LOAD(handle, copy_and_sum);
    LOAD(handle, compare_operations);
    LOAD(handle, complexmode);

    char *created = create_result_string("test", -17);
    printf("create_result_string=%s\n", created);
    free(created);

    printf("check_permissions=%d,%d,%d\n",
           check_permissions(0644, 0600),
           check_permissions(0644, 0100),
           check_permissions(0, 0));
    printf("safe_add_allowed=%d\n", safe_add(9, -4, 0600));
    printf("safe_add_denied=%d\n", safe_add(9, -4, 0400));

    char *log = NULL;
    int product = multiply_with_log(-7, 6, &log);
    printf("multiply_with_log=%d,%s\n", product, log);
    free(log);

    int values[] = {5, -2, 10, -4};
    printf("copy_and_sum=%d\n", copy_and_sum(values, 4));
    printf("copy_and_sum_empty=%d\n", copy_and_sum(values, 0));
    printf("copy_and_sum_null=%d\n", copy_and_sum(NULL, 4));

    printf("compare_equal=%d\n", compare_operations("addition", "addition"));
    printf("compare_less=%d\n", compare_operations("addition", "multiply"));
    printf("compare_null=%d\n", compare_operations(NULL, "multiply"));

    for (int mode = 0; mode <= 5; ++mode) {
        printf("complexmode_return[%d]=%d\n", mode, complexmode(mode, 7, -3, 11));
    }

    dlclose(handle);
    return 0;
}
