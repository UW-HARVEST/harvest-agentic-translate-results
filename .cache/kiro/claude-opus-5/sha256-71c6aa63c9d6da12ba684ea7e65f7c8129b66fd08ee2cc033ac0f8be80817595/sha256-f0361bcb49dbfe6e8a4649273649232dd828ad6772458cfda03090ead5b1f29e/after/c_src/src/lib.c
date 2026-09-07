// Copyright 2025 MIT Lincoln Laboratory
// Permission is hereby granted, free of charge,
// to any person obtaining a copy of this software
// and associated documentation files (the “Software”),
// to deal in the Software without restriction,
// including without limitation the rights to use, copy,
// modify, merge, publish, distribute, sublicense,
// and/or sell copies of the Software,
// and to permit persons to whom the Software is furnished to do so,
// subject to the following conditions:
//
// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED “AS IS”, WITHOUT WARRANTY OF ANY KIND,
// EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
// THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
// IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE
// FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT,
// TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE
// OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define STRINGIFY(x) #x
#define LOG_VALUE(var) printf("Variable " STRINGIFY(var) " = %d\n", var)

#define OP_ADD 0x01
#define OP_MULTIPLY 0x02
#define OP_XOR 0x03
#define OP_SHIFT 0x04
#define MAGIC_NUMBER 0xDEADBEEF
#define MASK_LOWER 0x0000FFFF

typedef struct {
    int accumulator;
    int operation_count;
    unsigned int checksum;
} ComputeState;

typedef int (*operation_func)(int, int);

static int static_multiplier = 3;
static int static_addend = 100;
static int static_shift_amount = 2;

int multiply_with_static(int a, int b) {
    return (a * b) * static_multiplier;
}

int add_with_static(int a, int b) {
    return (a + b) + static_addend;
}

int xor_operation(int a, int b) {
    return a ^ b ^ 0xABCD;
}

int shift_with_static(int a, int b) {
    return (a << static_shift_amount) | (b >> static_shift_amount);
}

operation_func get_operation(int opcode) {
    static operation_func ops[4] = {NULL, NULL, NULL, NULL};

    if (ops[0] == NULL) {
        ops[0] = multiply_with_static;
        ops[1] = add_with_static;
        ops[2] = xor_operation;
        ops[3] = shift_with_static;
    }

    if (opcode >= 0 && opcode < 4) {
        return ops[opcode];
    }

    return NULL;
}

int execute_operation(operation_func func, int a, int b, const char* op_name) {
    if (func == NULL) {
        printf("Error: Operation function pointer is NULL for %s\n", op_name);
        return 0;
    }

    LOG_VALUE(a);
    LOG_VALUE(b);

    int result = func(a, b);
    printf("Result of %s: %d\n", op_name, result);

    return result;
}

unsigned int compute_checksum(int* values, int count) {
    unsigned int checksum = 0;
    unsigned char buffer[sizeof(int) * 4];

    if (values != NULL && count > 0) {
        int copy_count = (count > 4) ? 4 : count;
        memcpy(buffer, values, sizeof(int) * copy_count);

        for (int i = 0; i < sizeof(int) * copy_count; i++) {
            checksum = (checksum << 1) ^ buffer[i];
        }

        checksum ^= MAGIC_NUMBER;
    }

    return checksum & MASK_LOWER;
}

void init_state(ComputeState* state, int initial_value) {
    if (state == NULL) {
        printf("Error: state pointer is NULL in init_state\n");
        return;
    }

    ComputeState template = {initial_value, 0, 0x0000};

    memcpy(state, &template, sizeof(ComputeState));

    printf("State initialized with accumulator = %d\n", state->accumulator);
}

void apply_operation(ComputeState* state, int value, operation_func func) {
    if (state == NULL) {
        printf("Error: state pointer is NULL in apply_operation\n");
        return;
    }

    if (func == NULL) {
        printf("Error: operation function pointer is NULL in apply_operation\n");
        return;
    }

    state->accumulator = func(state->accumulator, value);
    state->operation_count++;
}

int checkshift(int param1, int param2, int param3, int param4) {
    printf("\n=== Starting foo function ===\n");
    printf("Parameters: %d, %d, %d, %d\n", param1, param2, param3, param4);

    ComputeState* state = (ComputeState*)malloc(sizeof(ComputeState));

    if (state == NULL) {
        printf("Error: Failed to allocate memory for state\n");
        return -1;
    }

    init_state(state, param1);

    int params[4] = {param1, param2, param3, param4};

    operation_func mult_op = get_operation(0);
    operation_func add_op = get_operation(1);
    operation_func xor_op = get_operation(2);
    operation_func shift_op = get_operation(3);

    printf("\n--- Operation 1: Multiply ---\n");
    apply_operation(state, param2, mult_op);

    printf("\n--- Operation 2: Add ---\n");
    apply_operation(state, param3, add_op);

    printf("\n--- Operation 3: XOR ---\n");
    int xor_result = execute_operation(xor_op, state->accumulator, param4, "XOR");

    printf("\n--- Operation 4: Shift ---\n");
    int shift_result = execute_operation(shift_op, xor_result, param2, "SHIFT");

    state->checksum = compute_checksum(params, 4);
    printf("\nComputed checksum: 0x%04X\n", state->checksum);

    int final_result = (state->accumulator + shift_result) ^ state->checksum;

    printf("\nFinal accumulator: %d\n", state->accumulator);
    printf("Operation count: %d\n", state->operation_count);
    printf("Final result: %d\n", final_result);

    free(state);
    state = NULL;

    printf("=== Ending foo function ===\n\n");

    return final_result;
}
