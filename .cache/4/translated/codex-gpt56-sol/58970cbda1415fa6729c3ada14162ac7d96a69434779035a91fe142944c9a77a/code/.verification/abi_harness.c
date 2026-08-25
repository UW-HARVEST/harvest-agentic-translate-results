#include <dlfcn.h>
#include <stdbool.h>
#include <stddef.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

typedef enum {
    OP_ADD = 1,
    OP_MULTIPLY = 2,
    OP_SUBTRACT = 3,
    OP_DIVIDE = 4,
    OP_MODULO = 5
} Operation;

typedef enum {
    STATUS_SUCCESS = 0,
    STATUS_ERROR = -1,
    STATUS_WARNING = 1
} StatusCode;

typedef struct {
    int value;
    time_t timestamp;
    StatusCode status;
} ComputationResult;

typedef int (*MathOperation)(int, int, int);

#define LOAD(name)                                                            \
    do {                                                                      \
        *(void **)(&name) = dlsym(handle, #name);                             \
        if (!(name)) {                                                        \
            fprintf(stderr, "missing %s\n", #name);                           \
            return 2;                                                         \
        }                                                                     \
    } while (0)

int main(int argc, char **argv) {
    if (argc != 2) {
        return 2;
    }

    void *handle = dlopen(argv[1], RTLD_NOW | RTLD_LOCAL);
    if (!handle) {
        fprintf(stderr, "%s\n", dlerror());
        return 2;
    }

    bool (*is_valid_operation)(char);
    int (*get_operation_priority)(Operation);
    int (*add_operation)(int, int, int);
    int (*multiply_operation)(int, int, int);
    int (*subtract_operation)(int, int, int);
    int (*divide_operation)(int, int, int);
    int (*modulo_operation)(int, int, int);
    MathOperation (*select_operation)(Operation);
    time_t (*get_computation_timestamp)(void);
    ComputationResult *(*allocate_results)(int);
    int (*perform_computation_with_history)(
        int, int, Operation, ComputationResult **, int *);
    int (*mathop)(int, int, int, int);

    LOAD(is_valid_operation);
    LOAD(get_operation_priority);
    LOAD(add_operation);
    LOAD(multiply_operation);
    LOAD(subtract_operation);
    LOAD(divide_operation);
    LOAD(modulo_operation);
    LOAD(select_operation);
    LOAD(get_computation_timestamp);
    LOAD(allocate_results);
    LOAD(perform_computation_with_history);
    LOAD(mathop);

    printf("layout=%zu,%zu,%zu,%zu\n",
           sizeof(ComputationResult),
           offsetof(ComputationResult, value),
           offsetof(ComputationResult, timestamp),
           offsetof(ComputationResult, status));

    for (int c = -128; c <= 127; ++c) {
        printf("v:%d=%d\n", c, is_valid_operation((char)c));
    }

    for (int op = -3; op <= 8; ++op) {
        MathOperation selected = select_operation((Operation)op);
        printf("op:%d=%d,%d\n",
               op,
               get_operation_priority((Operation)op),
               selected(17, 5, 91));
    }

    const int pairs[][2] = {
        {17, 5}, {-17, 5}, {17, -5}, {-17, -5}, {1, 0}, {0, 1}, {46340, 2},
    };
    for (size_t i = 0; i < sizeof(pairs) / sizeof(pairs[0]); ++i) {
        int a = pairs[i][0];
        int b = pairs[i][1];
        printf("arith:%d,%d=%d,%d,%d,%d,%d\n",
               a,
               b,
               add_operation(a, b, 7),
               multiply_operation(a, b, 7),
               subtract_operation(a, b, 7),
               divide_operation(a, b, 7),
               modulo_operation(a, b, 7));
    }

    ComputationResult *allocated = allocate_results(3);
    unsigned char zeros[3 * sizeof(ComputationResult)];
    memset(zeros, 0, sizeof(zeros));
    printf("calloc=%d\n",
           allocated != NULL &&
               memcmp(allocated, zeros, sizeof(zeros)) == 0);
    free(allocated);

    ComputationResult *history = NULL;
    int history_count = 777;
    for (int op = 1; op <= 5; ++op) {
        int result = perform_computation_with_history(
            40 + op, op + 1, (Operation)op, &history, &history_count);
        printf("history:%d=%d,%d,%d,%ld,%d\n",
               op,
               result,
               history_count,
               history[history_count - 1].value,
               (long)history[history_count - 1].timestamp,
               history[history_count - 1].status);
    }
    free(history);

    const int calls[][4] = {
        {49, 7, 0, 0},
        {50, 8, 1, 1},
        {51, 3, 2, 2},
        {52, 5, 3, 3},
        {53, 6, 4, 4},
        {-49, 7, -2, -2},
        {54, -7, 7, 7},
    };
    for (size_t i = 0; i < sizeof(calls) / sizeof(calls[0]); ++i) {
        printf("mathop-return=%d\n",
               mathop(calls[i][0], calls[i][1], calls[i][2], calls[i][3]));
    }
    printf("timestamp=%ld\n", (long)get_computation_timestamp());

    dlclose(handle);
    return 0;
}
