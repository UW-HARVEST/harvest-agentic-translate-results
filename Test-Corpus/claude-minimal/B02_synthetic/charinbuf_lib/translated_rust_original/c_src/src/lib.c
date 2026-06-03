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
#include <stdint.h>

static int counter = 0;

typedef int (*operation_func)(int);

int increment_counter(int value) {
    counter += value;
    return counter;
}

int decrement_counter(int value) {
    counter -= value;
    return counter;
}

int multiply_counter(int value) {
    counter *= value;
    return counter;
}

int reset_counter(int value) {
    counter = value;
    return counter;
}

int is_string_empty(const char *str) {
    if (!str) return 1;
    if (*str) {
        return 0;
    }
    return 1;
}

char* find_char_in_buffer(const char *buffer, size_t size, char target) {
    if (!buffer) return NULL;
    return (char*)memchr(buffer, target, size);
}

char* create_buffer(const char *initial) {
    if (!initial) return NULL;

    size_t len = strlen(initial);
    char *buffer = (char*)malloc(len + 1);

    if (buffer) {
        strcpy(buffer, initial);
    }

    return buffer;
}

int validate_uint16_range(int value) {
    if (value < 0) return 0;
    if (value > UINT16_MAX) return 0;
    return 1;
}

int apply_operation(operation_func op, int value) {
    if (!op) return -1;
    return op(value);
}

// TODO: lib test stdout
int charinbuf(int mode, int value, int opt1, int opt2) {
    int result = 0;
    char *buffer = NULL;
    char *found_pos = NULL;
    const char *test_string = "";
    const char *non_empty_string = "Hello, World!";

    operation_func current_op = NULL;

    counter = 0;

    switch (mode) {
        case 0:
            printf("Mode 0: UINT16_MAX validation\n");
            printf("Checking if value %d is within uint16_t range...\n", value);

            if (validate_uint16_range(value)) {
                printf("Value %d is valid (0 <= value <= %u)\n", value, UINT16_MAX);
                result = value;
            } else {
                printf("Value %d is out of range for uint16_t\n", value);
                result = -1;
            }

            printf("UINT16_MAX constant value: %u\n", UINT16_MAX);
            break;

        case 1:
            printf("Mode 1: String empty check by dereference\n");

            if (is_string_empty(test_string)) {
                printf("Test string is empty (checked with *string)\n");
                result = 0;
            } else {
                printf("Test string is not empty\n");
                result = 1;
            }

            if (is_string_empty(non_empty_string)) {
                printf("Non-empty string check failed!\n");
            } else {
                printf("Non-empty string correctly identified\n");
                result += 10;
            }
            break;

        case 2:
            printf("Mode 2: Dynamic memory allocation and free\n");

            buffer = create_buffer("Testing malloc and free");

            if (buffer) {
                printf("Buffer allocated: '%s'\n", buffer);
                printf("Buffer length: %zu\n", strlen(buffer));
                result = (int)strlen(buffer);

                free(buffer);
                printf("Buffer freed successfully\n");
                buffer = NULL;
            } else {
                printf("Failed to allocate buffer\n");
                result = -1;
            }
            break;

        case 3:
            printf("Mode 3: Function pointers with static counter\n");

            current_op = reset_counter;
            result = apply_operation(current_op, value);
            printf("Counter reset to: %d\n", result);

            current_op = increment_counter;
            result = apply_operation(current_op, opt1);
            printf("Counter after increment by %d: %d\n", opt1, result);

            current_op = multiply_counter;
            result = apply_operation(current_op, opt2);
            printf("Counter after multiply by %d: %d\n", opt2, result);

            current_op = decrement_counter;
            result = apply_operation(current_op, 5);
            printf("Counter after decrement by 5: %d\n", result);

            printf("Final static counter value: %d\n", counter);
            break;

        case 4:
            printf("Mode 4: Using memchr to find character\n");

            buffer = create_buffer("Search for character X in this buffer");

            if (buffer) {
                size_t buf_size = strlen(buffer);
                char search_char = 'X';

                printf("Searching for '%c' in: '%s'\n", search_char, buffer);
                found_pos = find_char_in_buffer(buffer, buf_size, search_char);

                if (found_pos) {
                    result = (int)(found_pos - buffer);
                    printf("Found '%c' at position: %d\n", search_char, result);
                } else {
                    printf("Character '%c' not found\n", search_char);
                    result = -1;
                }

                free(buffer);
                buffer = NULL;
            }
            break;

        default:
            printf("Invalid mode: %d\n", mode);
            result = -1;
    }

    return result;
}
