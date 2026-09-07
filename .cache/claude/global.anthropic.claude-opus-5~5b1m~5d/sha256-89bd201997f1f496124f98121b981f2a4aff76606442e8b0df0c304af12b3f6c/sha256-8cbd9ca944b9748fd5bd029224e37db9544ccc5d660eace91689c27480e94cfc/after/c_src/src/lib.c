// Copyright 2025 MIT Lincoln Laboratory
// Permission is hereby granted, free of charge,
// to any person obtaining a copy of this software
// and associated documentation files (the "Software"),
// to deal in the Software without restriction,
// including without limitation the rights to use, copy,
// modify, merge, publish, distribute, sublicense,
// and/or sell copies of the Software,
// and to permit persons to whom the Software is furnished to do so,
// subject to the following conditions:

// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions of the Software.

// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
// EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
// THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
// IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE
// FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT,
// TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE
// OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

#include <stdio.h>
#include <stdlib.h>
#include <math.h>
#include <stdint.h>

#define FOREACH(item, array, count) \
    for(int keep = 1, \
            count_iter = 0, \
            size = (count); \
        keep && count_iter != size; \
        keep = !keep, count_iter++) \
      for(item = (array) + count_iter; keep; keep = !keep)

typedef int (*operation_func)(int a, int b, int unused1, int unused2);

typedef struct {
    int value;
    double scaled;
    int rank;
} Result;

typedef struct {
    Result data[10];
    int count;
} ResultArray;

int add_operation(int a, int b, int unused1, int unused2) {
    (void)unused1;
    (void)unused2;
    return a + b;
}

int multiply_operation(int a, int b, int unused1, int unused2) {
    (void)unused1;
    (void)unused2;
    return a * b;
}

int subtract_operation(int a, int b, int unused1, int unused2) {
    (void)unused1;
    (void)unused2;
    return a - b;
}

int modulo_operation(int a, int b, int unused1, int unused2) {
    (void)unused1;
    (void)unused2;
    if (b == 0) return 0;
    return a % b;
}

int safe_double_to_int(double d) {
    if (d >= (double)INT32_MAX) {
        return INT32_MAX;
    }
    if (d <= (double)INT32_MIN) {
        return INT32_MIN;
    }
    if (d != d) {
        return 0;
    }
    return (int)d;
}

int compute_scaled_value(int base, double scale_factor) {
    double scaled = (double)base * scale_factor;
    return safe_double_to_int(scaled);
}

int compare_results_in_array(ResultArray *arr, int idx1, int idx2) {
    if (idx1 >= arr->count || idx2 >= arr->count) {
        return 0;
    }

    Result *ptr1 = &arr->data[idx1];
    Result *ptr2 = &arr->data[idx2];

    if (ptr1 < ptr2) {
        return -1;
    } else if (ptr1 > ptr2) {
        return 1;
    }
    return 0;
}

void init_result_array(ResultArray *arr, int values[], int count) {
    arr->count = count < 10 ? count : 10;

    for (int i = 0; i < arr->count; i++) {
        arr->data[i] = (Result){
            .value = values[i],
            .scaled = (double)values[i] * 1.5,
            .rank = i
        };
    }
}

int process_with_foreach(ResultArray *arr, operation_func op) {
    int total = 0;
    Result *item;

    FOREACH(item, arr->data, arr->count) {
        int result = op(item->value, item->rank, 0, 0);
        total += result;

        double temp = (double)result * 0.75;
        item->scaled = temp;
        item->value = safe_double_to_int(temp);
    }

    return total;
}

int compute_weighted_sum(ResultArray *arr) {
    int sum = 0;

    for (int i = 0; i < arr->count; i++) {
        Result *current = &arr->data[i];
        Result *base = &arr->data[0];

        int weight = (current > base) ? (int)(current - base) : 1;

        double weighted = (double)current->value * (double)weight * 0.8;
        sum += safe_double_to_int(weighted);
    }

    return sum;
}

int arrayfunc(int param1, int param2, int param3, int param4) {
    operation_func operations[] = {
        add_operation,
        multiply_operation,
        subtract_operation,
        modulo_operation
    };

    int values[] = {param1, param2, param3, param4,
                    param1 + param2, param2 - param3,
                    param3 * 2, param4 / 2 + 1};

    ResultArray arr = {.count = 0};
    init_result_array(&arr, values, 8);

    int result = 0;

    for (int i = 0; i < 4; i++) {
        result += process_with_foreach(&arr, operations[i]);
    }

    result += compute_weighted_sum(&arr);

    for (int i = 0; i < arr.count - 1; i++) {
        int cmp = compare_results_in_array(&arr, i, i + 1);
        result += cmp;
    }

    double final_scale = (double)result * 0.333;
    result = safe_double_to_int(final_scale);

    return result;
}
