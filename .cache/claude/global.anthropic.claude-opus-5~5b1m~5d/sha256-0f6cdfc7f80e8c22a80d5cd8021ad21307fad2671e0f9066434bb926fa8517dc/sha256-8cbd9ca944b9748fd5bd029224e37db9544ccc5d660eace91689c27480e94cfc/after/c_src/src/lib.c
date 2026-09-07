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
#include <string.h>
#include <stdlib.h>

typedef int (*operation_func)(int, int);
typedef void (*string_processor)(char*, int);

static int accumulator = 0;
static int multiplier = 1;
static int operation_count = 0;

int add_to_accumulator(int a, int b) {
    accumulator += (a + b);
    operation_count++;
    return accumulator;
}

int multiply_with_multiplier(int a, int b) {
    multiplier *= (a * b);
    operation_count++;
    return multiplier;
}

int subtract_from_accumulator(int a, int b) {
    accumulator -= (a - b);
    operation_count++;
    return accumulator;
}

int divide_multiplier(int a, int b) {
    if (b != 0) {
        multiplier /= b;
    }
    operation_count++;
    return multiplier;
}

void process_octal_string(char* dest, int octal_val) {
    char buffer[50];
    sprintf(buffer, "Octal: 0%o, Decimal: %d", octal_val, octal_val);
    strcpy(dest, buffer);
}

void find_and_replace_char(char* str, int search_char) {
    char* found = (char*)memchr(str, search_char, strlen(str));
    if (found) {
        *found = 'X';
    }
}

int validate_and_normalize(int value) {
    int is_nonzero = !!value;
    int is_zero = !value;

    int lower_threshold = 0100;
    int upper_threshold = 0777;

    if (is_nonzero && value > 0) {
        if (value < lower_threshold) {
            return lower_threshold;
        } else if (value > upper_threshold) {
            return upper_threshold;
        }
    }

    return value;
}

static operation_func operations[4] = {
    add_to_accumulator,
    multiply_with_multiplier,
    subtract_from_accumulator,
    divide_multiplier
};

int findrep(int param1, int param2, int param3, int param4) {
    int result = 0;

    int p1_valid = !!param1;
    int p2_valid = !!param2;
    int p3_valid = !!param3;
    int p4_valid = !!param4;

    int active_params = p1_valid + p2_valid + p3_valid + p4_valid;

    int mode_add = 01;
    int mode_multiply = 02;
    int mode_subtract = 03;
    int mode_divide = 04;

    int normalized_p1 = validate_and_normalize(param1);
    int normalized_p2 = validate_and_normalize(param2);
    int normalized_p3 = validate_and_normalize(param3);
    int normalized_p4 = validate_and_normalize(param4);

    char message[100];
    char search_buffer[100];

    process_octal_string(message, 0123);
    strcpy(search_buffer, "Function pointer example with static vars");

    char* found_char = (char*)memchr(search_buffer, 'p', strlen(search_buffer));
    if (found_char) {
        result += (int)(found_char - search_buffer);
    }

    operation_func selected_op;

    if (active_params >= mode_add) {
        selected_op = operations[0];
        result += selected_op(normalized_p1, normalized_p2);
    }

    if (active_params >= mode_multiply) {
        selected_op = operations[1];
        result += selected_op(normalized_p3, normalized_p4);
    }

    if (accumulator > 0150) {
        selected_op = operations[2];
        int subtract_result = selected_op(normalized_p1, normalized_p3);
        result += subtract_result;
    }

    find_and_replace_char(message, 'O');

    char final_message[100];
    strcpy(final_message, message);

    int has_accumulator = !!accumulator;
    int has_multiplier = !!multiplier;
    int both_active = has_accumulator && has_multiplier;

    if (both_active) {
        result += accumulator + multiplier;
    }

    if (multiplier > 0100) {
        selected_op = operations[3];
        selected_op(multiplier, 2);
    }

    result += operation_count * 010;

    int result_exists = !!result;
    if (!result_exists) {
        result = 0777;
    }

    return result;
}
