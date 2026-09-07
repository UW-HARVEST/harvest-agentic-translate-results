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
#include <stdint.h>
#include <math.h>
#include <limits.h> // ignore false positive "compiler builtins" hard blocker from here

#define FOREACH(item, array, count) \
    for(int keep = 1, \
            idx = 0, \
            size = (count); \
        keep && idx < size; \
        keep = !keep, idx++) \
      for(item = (array)[idx]; keep; keep = !keep)

#define OCTAL_MASK_1 0777
#define OCTAL_MASK_2 0100
#define OCTAL_FLAG   0200
#define OCTAL_BASE   010

typedef struct {
    int value;
    double coefficient;
} DataPoint;

int safe_double_to_int(double d) {
    if (isnan(d)) {
        return 0;
    }

    if (isinf(d)) {
        return d > 0 ? INT_MAX : INT_MIN;
    }

    if (d >= (double)INT_MAX) {
        return INT_MAX;
    }
    if (d <= (double)INT_MIN) {
        return INT_MIN;
    }

    return (int)d;
}

int process_array_reverse(int *end, int count) {
    int sum = 0;
    int *ptr = end;

    for (int i = 0; i < count; i++) {
        sum += *ptr;
        ptr--;
    }

    return sum;
}

int switch_fallthrough_calculator(int value, int operation) {
    int result = value;

    switch (operation) {
        case 0:
            result *= OCTAL_BASE;
        case 1:
            result += OCTAL_FLAG;
        case 2:
            result &= OCTAL_MASK_1;
            break;
        case 3:
            result *= 3;
        case 4:
            result += OCTAL_MASK_2;
            break;
        default:
            result = 0;
    }

    return result;
}

int allocate_and_compute(int size, double multiplier) {
    DataPoint *points = (DataPoint *)malloc(size * sizeof(DataPoint));

    if (points == NULL) {
        return -1;
    }

    for (int i = 0; i < size; i++) {
        points[i].value = i * OCTAL_BASE;
        points[i].coefficient = (double)i * multiplier;
    }

    double sum = 0.0;
    for (int i = 0; i < size; i++) {
        sum += points[i].value * points[i].coefficient;
    }

    int result = safe_double_to_int(sum);

    free(points);

    return result;
}

int foreach_sum(int *array, int count) {
    int total = 0;
    int element;

    FOREACH(element, array, count) {
        total += element;
    }

    return total;
}

int fallcalc(int param1, int param2, int param3, int param4) {
    int result = 0;

    int base_value = param1 * OCTAL_MASK_2 + param2;

    int array_size = 5;
    int *data_array = (int *)malloc(array_size * sizeof(int));

    if (data_array == NULL) {
        return -1;
    }

    for (int i = 0; i < array_size; i++) {
        data_array[i] = (i + 1) * OCTAL_BASE + param1;
    }

    int foreach_result = foreach_sum(data_array, array_size);

    int *last_element = data_array + array_size - 1;
    int reverse_sum = process_array_reverse(last_element, array_size);

    int switch_result = switch_fallthrough_calculator(param2, param3 % 5);

    double floating_calc = (double)param1 * 3.7 + (double)param2 * 2.3 - (double)param3 * 0.5;
    int converted = safe_double_to_int(floating_calc);

    int alloc_result = allocate_and_compute(param4 % 10 + 1, 1.5);

    result = base_value + foreach_result + reverse_sum + switch_result + converted + alloc_result;

    if (param3 > OCTAL_FLAG) {
        result |= OCTAL_FLAG;
    }

    free(data_array);

    result &= OCTAL_MASK_1;

    return result;
}
