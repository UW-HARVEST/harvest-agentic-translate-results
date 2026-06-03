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

int matrix[3][4] = {
    {0x01, 0x02, 0x03, 0x04},
    {0x10, 0x20, 0x30, 0x40},
    {0xA1, 0xB2, 0xC3, 0xD4}
};

#define FLAG_READ    0b00000001
#define FLAG_WRITE   0b00000010
#define FLAG_EXECUTE 0b00000100
#define FLAG_DELETE  0b00001000

typedef struct {
    int *data;
    size_t size;
    size_t capacity;
} DynamicArray;

DynamicArray* init_array(size_t initial_capacity) {
    DynamicArray *arr = (DynamicArray*)malloc(sizeof(DynamicArray));
    if (!arr) return NULL;

    arr->data = (int*)malloc(initial_capacity * sizeof(int));
    if (!arr->data) {
        free(arr);
        return NULL;
    }

    arr->size = 0;
    arr->capacity = initial_capacity;
    return arr;
}

int expand_array(DynamicArray *arr) {
    if (!arr) return 0;

    size_t new_capacity = arr->capacity * 2;
    int *new_data = (int*)realloc(arr->data, new_capacity * sizeof(int));

    if (!new_data) {
        return 0;
    }

    arr->data = new_data;
    arr->capacity = new_capacity;
    return 1;
}

int add_element(DynamicArray *arr, int value) {
    if (!arr) return 0;

    if (arr->size >= arr->capacity) {
        if (!expand_array(arr)) {
            return 0;
        }
    }

    arr->data[arr->size++] = value;
    return 1;
}

void free_array(DynamicArray *arr) {
    if (arr) {
        free(arr->data);
        free(arr);
    }
}

int process_flags(int flags) {
    int count = 0;

    int has_read = flags & FLAG_READ;
    int read_enabled = !!has_read;

    int has_write = flags & FLAG_WRITE;
    int write_enabled = !!has_write;

    int has_execute = flags & FLAG_EXECUTE;
    int execute_enabled = !!has_execute;

    int has_delete = flags & FLAG_DELETE;
    int delete_enabled = !!has_delete;

    count = read_enabled + write_enabled + execute_enabled + delete_enabled;

    return count;
}

int calculate_matrix_checksum() {
    int sum = 0;
    int i, j;

    for (i = 0; i < 3; i++) {
        for (j = 0; j < 4; j++) {
            sum += matrix[i][j];
        }
    }

    return sum;
}

int matrixsum(int param1, int param2, int param3, int param4) {
    int result = 0;

    int hex_base = 0xFF;
    int hex_multiplier = 0x10;

    int permissions = 0b0000;

    int check1 = param1;
    int valid1 = !!check1;

    int check2 = param2;
    int valid2 = !!check2;

    int check3 = param3;
    int valid3 = !!check3;

    int check4 = param4;
    int valid4 = !!check4;

    if (valid1) permissions |= FLAG_READ;
    if (valid2) permissions |= FLAG_WRITE;
    if (valid3) permissions |= FLAG_EXECUTE;
    if (valid4) permissions |= FLAG_DELETE;

    DynamicArray *arr = init_array(2);
    if (!arr) {
        return -1;
    }

    add_element(arr, param1);
    add_element(arr, param2);
    add_element(arr, param3);
    add_element(arr, param4);

    int sum = 0;
    size_t i;
    for (i = 0; i < arr->size; i++) {
        sum += arr->data[i];
    }

    int flag_count = process_flags(permissions);

    int matrix_sum = calculate_matrix_checksum();

    result = (sum * hex_multiplier) + (flag_count * hex_base) + (matrix_sum & 0xFFF);

    free_array(arr);

    return result;
}
