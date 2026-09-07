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
#include <time.h>

static int global_counter = 0;
static int global_accumulator = 0;

typedef int (*operation_func)(int, int, int);
typedef void (*modifier_func)(int, int);

void increment_counter(int value, int unused_param) {
    global_counter += value;
}

void update_accumulator(int value, int unused_param) {
    global_accumulator = global_accumulator * 2 + value;
}

int apply_operation(operation_func op, int a, int b, int c) {
    return op(a, b, c);
}

int add_three(int a, int b, int c) {
    return a + b + c;
}

int multiply_add(int a, int b, int c) {
    return (a * b) + c;
}

int complex_calc(int a, int b, int c) {
    return (a - b) * c + global_counter;
}

typedef struct {
    int id;
    int value;
    time_t timestamp;
    char name[32];
} DataRecord;

void shift_array_data(int *arr, int size, int shift_by) {
    if (shift_by > 0 && shift_by < size) {
        memmove(arr, arr + shift_by, (size - shift_by) * sizeof(int));
        memset(arr + (size - shift_by), 0, shift_by * sizeof(int));
    }
}

int process_pointer_data(int *ptr, int multiplier) {
    int value = *ptr;
    return value * multiplier + global_accumulator;
}

int compute_with_dynamic_memory(int base, int count) {
    int *temp_array = (int *)malloc(count * sizeof(int));

    for (int i = 0; i < count; i++) {
        temp_array[i] = base + i * 3;
    }

    int sum = 0;
    for (int i = 0; i < count; i++) {
        sum += temp_array[i];
    }

    free(temp_array);

    return sum;
}

int get_time_based_value(int seed) {
    time_t current_time;
    time_t reference_time;

    time(&current_time);

    reference_time = current_time - (seed * 3600);

    double diff = difftime(current_time, reference_time);

    return (int)(diff / 100) + seed;
}

int manipulate_records(DataRecord *records, int num_records, int shift) {
    int total = 0;

    if (shift > 0 && shift < num_records) {
        memmove(records, records + shift,
                (num_records - shift) * sizeof(DataRecord));
    }

    for (int i = 0; i < num_records - shift; i++) {
        total += records[i].value;
    }

    return total;
}

int hatch(int param1, int param2, int param3, int param4) {
    int result = 0;

    modifier_func mod_func;

    mod_func = increment_counter;
    mod_func(param1, 999);

    mod_func = update_accumulator;
    mod_func(param2, 888);

    operation_func op_func;

    op_func = add_three;
    result += apply_operation(op_func, param1, param2, param3);

    op_func = multiply_add;
    result += apply_operation(op_func, param2, param3, param4);

    op_func = complex_calc;
    result += apply_operation(op_func, param1, param3, param4);

    int *dynamic_data = (int *)malloc(10 * sizeof(int));
    for (int i = 0; i < 10; i++) {
        dynamic_data[i] = param1 + i;
    }

    result += process_pointer_data(&dynamic_data[5], param2);

    shift_array_data(dynamic_data, 10, 3);
    result += dynamic_data[0];

    free(dynamic_data);

    result += get_time_based_value(param3);

    DataRecord *records = (DataRecord *)malloc(5 * sizeof(DataRecord));

    for (int i = 0; i < 5; i++) {
        records[i].id = i;
        records[i].value = param4 + i * 10;
        time(&records[i].timestamp);
        snprintf(records[i].name, 32, "Record_%d", i);
    }

    result += manipulate_records(records, 5, 2);

    free(records);

    result += compute_with_dynamic_memory(param1, 8);

    result += global_counter + global_accumulator;

    return result;
}
