#include <dlfcn.h>
#include <limits.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>

typedef int (*operation_func)(int, int);

typedef struct {
    int accumulator;
    int operation_count;
    unsigned int checksum;
} ComputeState;

typedef int (*binary_fn)(int, int);
typedef operation_func (*get_operation_fn)(int);
typedef int (*execute_operation_fn)(operation_func, int, int, const char *);
typedef unsigned int (*compute_checksum_fn)(int *, int);
typedef void (*init_state_fn)(ComputeState *, int);
typedef void (*apply_operation_fn)(ComputeState *, int, operation_func);
typedef int (*checkshift_fn)(int, int, int, int);

static void *load_symbol(void *handle, const char *name) {
    void *symbol = dlsym(handle, name);
    if (symbol == NULL) {
        fprintf(stderr, "missing symbol %s: %s\n", name, dlerror());
        exit(2);
    }
    return symbol;
}

static int subtract_operation(int a, int b) {
    return a - b;
}

int main(int argc, char **argv) {
    if (argc != 2) {
        fprintf(stderr, "usage: %s LIBRARY\n", argv[0]);
        return 2;
    }

    void *handle = dlopen(argv[1], RTLD_NOW | RTLD_LOCAL);
    if (handle == NULL) {
        fprintf(stderr, "dlopen: %s\n", dlerror());
        return 2;
    }

    binary_fn multiply_with_static = (binary_fn)load_symbol(handle, "multiply_with_static");
    binary_fn add_with_static = (binary_fn)load_symbol(handle, "add_with_static");
    binary_fn xor_operation = (binary_fn)load_symbol(handle, "xor_operation");
    binary_fn shift_with_static = (binary_fn)load_symbol(handle, "shift_with_static");
    get_operation_fn get_operation =
        (get_operation_fn)load_symbol(handle, "get_operation");
    execute_operation_fn execute_operation =
        (execute_operation_fn)load_symbol(handle, "execute_operation");
    compute_checksum_fn compute_checksum =
        (compute_checksum_fn)load_symbol(handle, "compute_checksum");
    init_state_fn init_state = (init_state_fn)load_symbol(handle, "init_state");
    apply_operation_fn apply_operation =
        (apply_operation_fn)load_symbol(handle, "apply_operation");
    checkshift_fn checkshift = (checkshift_fn)load_symbol(handle, "checkshift");

    const int pairs[][2] = {
        {2, 3},
        {-7, 11},
        {INT_MAX, 2},
        {INT_MIN, -1},
        {0x12345678, -98765},
    };
    const size_t pair_count = sizeof(pairs) / sizeof(pairs[0]);

    for (size_t i = 0; i < pair_count; ++i) {
        int a = pairs[i][0];
        int b = pairs[i][1];
        printf(
            "binary[%zu]=%d,%d,%d,%d\n",
            i,
            multiply_with_static(a, b),
            add_with_static(a, b),
            xor_operation(a, b),
            shift_with_static(a, b));
    }

    for (int opcode = -1; opcode <= 4; ++opcode) {
        operation_func operation = get_operation(opcode);
        printf(
            "get_operation[%d]=%s",
            opcode,
            operation == NULL ? "null" : "set");
        if (operation != NULL) {
            printf(",%d", operation(17, -9));
        }
        putchar('\n');
    }

    printf(
        "execute valid return=%d\n",
        execute_operation(get_operation(2), 123, -456, "CUSTOM_XOR"));
    printf(
        "execute custom return=%d\n",
        execute_operation(subtract_operation, INT_MIN, 7, "SUBTRACT"));
    printf(
        "execute null return=%d\n",
        execute_operation(NULL, 1, 2, "NULL_OP"));

    int values[] = {0x01020304, -1, INT_MIN, INT_MAX, 0x55667788};
    for (int count = 0; count <= 6; ++count) {
        printf("checksum[%d]=%u\n", count, compute_checksum(values, count));
    }
    printf("checksum[negative]=%u\n", compute_checksum(values, -3));
    printf("checksum[null]=%u\n", compute_checksum(NULL, 4));

    ComputeState state;
    init_state(NULL, 10);
    init_state(&state, -1000);
    printf(
        "state initialized=%d,%d,%u\n",
        state.accumulator,
        state.operation_count,
        state.checksum);
    apply_operation(NULL, 5, get_operation(0));
    apply_operation(&state, 5, NULL);
    apply_operation(&state, 5, get_operation(1));
    printf(
        "state add=%d,%d,%u\n",
        state.accumulator,
        state.operation_count,
        state.checksum);
    apply_operation(&state, -7, subtract_operation);
    printf(
        "state custom=%d,%d,%u\n",
        state.accumulator,
        state.operation_count,
        state.checksum);

    const int checkshift_cases[][4] = {
        {1, 2, 3, 4},
        {-7, 11, -13, 17},
        {INT_MAX, 2, INT_MIN, -1},
        {0x12345678, -98765, 0x0F0E0D0C, INT_MIN},
    };
    const size_t case_count = sizeof(checkshift_cases) / sizeof(checkshift_cases[0]);
    for (size_t i = 0; i < case_count; ++i) {
        const int *p = checkshift_cases[i];
        int result = checkshift(p[0], p[1], p[2], p[3]);
        printf("checkshift[%zu] return=%d\n", i, result);
    }

    dlclose(handle);
    return 0;
}
