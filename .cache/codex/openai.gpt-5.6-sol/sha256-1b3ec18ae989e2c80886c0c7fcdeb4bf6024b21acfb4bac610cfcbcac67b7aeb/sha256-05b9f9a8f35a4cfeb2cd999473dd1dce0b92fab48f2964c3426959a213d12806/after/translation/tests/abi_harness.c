#include <limits.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>

void shift_array(int *arr, int size, int positions);
int process_string(const char *str);
int apply_bitmask(int value, int operation);
void init_matrix(int matrix[3][4]);
int compare_allocations(int val1, int val2);
int arity2(int p1, int p2);
int arity3(int p1, int p2, int p3);
int arity4(int p1, int p2, int p3, int p4);
int arity(int len, int *params);

static uint64_t digest = UINT64_C(1469598103934665603);
static uint32_t state = UINT32_C(0xa17e1234);

static int next_int(void) {
    state = state * UINT32_C(1664525) + UINT32_C(1013904223);
    return (int)state;
}

static void record(int value) {
    uint32_t bits;
    memcpy(&bits, &value, sizeof(bits));
    digest ^= bits;
    digest *= UINT64_C(1099511628211);
}

int main(void) {
    static const int edges[] = {
        INT_MIN, INT_MIN + 1, -257, -256, -255, -1, 0,
        1, 15, 16, 255, 256, 257, INT_MAX,
    };

    for (int size = 1; size <= 8; ++size) {
        for (int positions = -2; positions <= size + 2; ++positions) {
            int values[10];
            for (int i = 0; i < 10; ++i) {
                values[i] = next_int();
            }
            shift_array(values, size, positions);
            for (int i = 0; i < 10; ++i) {
                record(values[i]);
            }
        }
    }
    printf("shift_array %016lx\n", (unsigned long)digest);

    static const char long_string[] =
        "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    record(process_string(""));
    record(process_string("Hello"));
    record(process_string(long_string));
    printf("process_string %016lx\n", (unsigned long)digest);

    for (size_t i = 0; i < sizeof(edges) / sizeof(edges[0]); ++i) {
        for (int operation = -3; operation <= 6; ++operation) {
            record(apply_bitmask(edges[i], operation));
        }
    }
    for (int i = 0; i < 500; ++i) {
        int value = next_int();
        for (int operation = -3; operation <= 6; ++operation) {
            record(apply_bitmask(value, operation));
        }
    }
    printf("apply_bitmask %016lx\n", (unsigned long)digest);

    int matrix[3][4];
    memset(matrix, 0xa5, sizeof(matrix));
    init_matrix(matrix);
    for (int i = 0; i < 12; ++i) {
        record((&matrix[0][0])[i]);
    }
    printf("init_matrix %016lx\n", (unsigned long)digest);

    for (int i = 0; i < 112; ++i) {
        int value = edges[i % (sizeof(edges) / sizeof(edges[0]))];
        record(compare_allocations(value, ~value));
    }
    printf("compare_allocations %016lx\n", (unsigned long)digest);

    for (int i = 0; i < 400; ++i) {
        record(arity2(next_int(), next_int()));
    }
    printf("arity2 %016lx\n", (unsigned long)digest);

    for (int i = 0; i < 400; ++i) {
        int p1 = next_int();
        int p2 = next_int();
        int p3 = next_int();
        record(arity3(p1, p2, p3));
    }
    printf("arity3 %016lx\n", (unsigned long)digest);

    for (int i = 0; i < 800; ++i) {
        int p1 = next_int();
        int p2 = next_int();
        int p3 = next_int();
        int p4 = next_int();
        record(arity4(p1, p2, p3, p4));
    }
    printf("arity4 %016lx\n", (unsigned long)digest);

    static const int lengths[] = {
        -513, -512, -511, -257, -256, -255, -1, 0, 1, 2, 3, 4,
        5, 254, 255, 256, 257, 258, 259, 260, 511, 512, 513,
    };
    for (size_t i = 0; i < sizeof(lengths) / sizeof(lengths[0]); ++i) {
        int params[4] = {next_int(), next_int(), next_int(), next_int()};
        record(arity(lengths[i], params));
    }
    printf("arity %016lx\n", (unsigned long)digest);
    return 0;
}
