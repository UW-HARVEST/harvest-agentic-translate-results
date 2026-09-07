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
#include <math.h> // ignore hard blocker "compiler builtins" from here
#include <limits.h>

#define MAKE_VAR_NAME(prefix, suffix) prefix##suffix
#define PRINT_VAR(name) printf(#name " = %d\n", name)

typedef struct {
    int id;
    double value;
    char label[20];
} DataBlock;

int safe_double_to_int(double d) {
    if (d > (double)INT_MAX) {
        return INT_MAX;
    } else if (d < (double)INT_MIN) {
        return INT_MIN;
    } else if (isnan(d)) {
        return 0;
    } else {
        return (int)d;
    }
}

int process_with_fallthrough(int code, int base_value) {
    int result = base_value;

    switch (code) {
        case 5:
            result += 50;
        case 4:
            result += 40;
        case 3:
            result += 30;
        case 2:
            result += 20;
        case 1:
            result += 10;
            break;
        case 0:
            result = 0;
            break;
        default:
            result = -1;
            break;
    }

    return result;
}

void copy_data_block(DataBlock *dest, const DataBlock *src) {
    memcpy(dest, src, sizeof(DataBlock));
}

int handle_pointer_operations(int value) {
    int *ptr;
    int local_value = value * 2;

    ptr = &local_value;

    int result = *ptr + 100;

    return result;
}

int overunder(int a, int b, int c, int d) {
    int total = 0;

    int MAKE_VAR_NAME(result, _1) = a;
    int MAKE_VAR_NAME(result, _2) = b;
    int MAKE_VAR_NAME(result, _3) = c;
    int MAKE_VAR_NAME(result, _4) = d;

    PRINT_VAR(result_1);
    PRINT_VAR(result_2);

    double temp1 = (double)a * 1.5;
    double temp2 = (double)b * 2.7;
    double temp3 = (double)c / 3.3;
    double temp4 = sqrt((double)(d * d + a * a));

    int conv1 = safe_double_to_int(temp1);
    int conv2 = safe_double_to_int(temp2);
    int conv3 = safe_double_to_int(temp3);
    int conv4 = safe_double_to_int(temp4);

    printf("Converted values: %d, %d, %d, %d\n", conv1, conv2, conv3, conv4);

    int switch_result = process_with_fallthrough(a % 6, b);
    printf("Switch fall-through result: %d\n", switch_result);

    DataBlock source_block;
    source_block.id = a;
    source_block.value = temp1;
    strncpy(source_block.label, "Source", sizeof(source_block.label) - 1);
    source_block.label[sizeof(source_block.label) - 1] = '\0';

    DataBlock dest_block;
    copy_data_block(&dest_block, &source_block);

    printf("Copied block: id=%d, value=%.2f, label=%s\n",
           dest_block.id, dest_block.value, dest_block.label);

    int ptr_result = handle_pointer_operations(c);
    printf("Pointer operation result: %d\n", ptr_result);

    total = conv1 + conv2 + conv3 + conv4 + switch_result + ptr_result;
    total += dest_block.id;

    double overflow_test = 1e15;
    int safe_conv = safe_double_to_int(overflow_test);
    printf("Overflow protected conversion: %d\n", safe_conv);

    double underflow_test = -1e15;
    int safe_conv2 = safe_double_to_int(underflow_test);
    printf("Underflow protected conversion: %d\n", safe_conv2);

    int array1[5] = {a, b, c, d, a+b};
    int array2[5];

    memcpy(array2, array1, sizeof(array1));

    printf("Array copied via memcpy: ");
    for (int i = 0; i < 5; i++) {
        printf("%d ", array2[i]);
        total += array2[i];
    }
    printf("\n");

    return total;
}
