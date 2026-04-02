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

#define READ_PERM 0400
#define WRITE_PERM 0200
#define EXEC_PERM 0100

typedef struct {
    int value;
    char operation[32];
    int permissions;
} Result;

char* create_result_string(const char* op, int val) {
    char* str = (char*)malloc(64 * sizeof(char));
    if (str == NULL) {
        return NULL;
    }
    snprintf(str, 64, "Operation: %s, Value: %d", op, val);
    return str;
}

int check_permissions(int perms, int required) {
    return (perms & required) == required;
}

int safe_add(int a, int b, int perms) {
    if (!check_permissions(perms, READ_PERM | WRITE_PERM)) {
        printf("Insufficient permissions for addition\n");
        return 0;
    }
    return a + b;
}

int multiply_with_log(int a, int b, char** log_msg) {
    *log_msg = create_result_string("multiply", a * b);
    if (*log_msg == NULL) {
        return 0;
    }
    return a * b;
}

int copy_and_sum(int* src, int count) {
    if (src == NULL) {
        printf("Source pointer is NULL\n");
        return -1;
    }

    int* dest = (int*)malloc(count * sizeof(int));
    if (dest == NULL) {
        printf("Memory allocation failed\n");
        return -1;
    }

    memcpy(dest, src, count * sizeof(int));

    int sum = 0;
    for (int i = 0; i < count; i++) {
        sum += dest[i];
    }

    free(dest);
    return sum;
}

int compare_operations(const char* op1, const char* op2) {
    if (op1 == NULL || op2 == NULL) {
        printf("One or both operation strings are NULL\n");
        return -1;
    }

    return strcmp(op1, op2);
}

int complexmode(int mode, int value1, int value2, int value3) {
    int result = 0;
    char* log_message = NULL;

    int permissions = 0644;  // rw-r--r--

    Result* res_tracker = (Result*)malloc(sizeof(Result));
    if (res_tracker == NULL) {
        printf("Failed to allocate result tracker\n");
        return -1;
    }

    res_tracker->value = 0;
    res_tracker->permissions = permissions;
    strcpy(res_tracker->operation, "none");

    switch (mode) {
        case 1: {
            strcpy(res_tracker->operation, "addition");
            result = safe_add(value1, value2, permissions);
            res_tracker->value = result;

            printf("Mode 1: Addition\n");
            printf("Result: %d\n", result);
            break;
        }

        case 2: {
            strcpy(res_tracker->operation, "multiplication");
            result = multiply_with_log(value1, value2, &log_message);
            res_tracker->value = result;

            if (log_message == NULL || strcmp(log_message, "") == 0) {
                printf("Log message creation failed\n");
            } else {
                printf("Mode 2: %s\n", log_message);
                free(log_message);
            }
            break;
        }

        case 3: {
            strcpy(res_tracker->operation, "array_sum");
            int values[3] = {value1, value2, value3};
            result = copy_and_sum(values, 3);
            res_tracker->value = result;

            printf("Mode 3: Array Sum\n");
            printf("Result: %d\n", result);
            break;
        }

        case 4: {
            strcpy(res_tracker->operation, "complex");

            if (check_permissions(permissions, 0100)) {
                result = (value1 * value2) + value3;
            } else {
                result = value1 + value2 + value3;
            }

            res_tracker->value = result;
            printf("Mode 4: Complex Calculation\n");
            printf("Result: %d\n", result);
            break;
        }

        default: {
            printf("Invalid mode\n");
            result = -1;
            break;
        }
    }

    if (strcmp(res_tracker->operation, "none") != 0) {
        printf("Operation performed: %s\n", res_tracker->operation);
    }

    free(res_tracker);

    return result;
}
