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

typedef struct {
    char *data;
    int capacity;
    int length;
} StringBuffer;

StringBuffer* create_buffer(int initial_capacity) {
    StringBuffer *buffer = (StringBuffer*)malloc(sizeof(StringBuffer));
    if (!buffer) {
        return NULL;
    }

    buffer->data = (char*)malloc(initial_capacity);
    if (!buffer->data) {
        free(buffer);
        return NULL;
    }

    buffer->capacity = initial_capacity;
    buffer->length = 0;
    buffer->data[0] = '\0';

    return buffer;
}

int append_to_buffer(StringBuffer *buffer, const char *str) {
    int str_len = strlen(str);
    int required_capacity = buffer->length + str_len + 1;

    if (required_capacity > buffer->capacity) {
        int new_capacity = required_capacity * 2;
        char *new_data = (char*)realloc(buffer->data, new_capacity);

        if (!new_data) {
            return -1;
        }

        buffer->data = new_data;
        buffer->capacity = new_capacity;
    }

    strcpy(buffer->data + buffer->length, str);
    buffer->length += str_len;

    return 0;
}

void destroy_buffer(StringBuffer *buffer) {
    if (buffer) {
        if (buffer->data) {
            free(buffer->data);
        }
        free(buffer);
    }
}

const char* get_operation_name(int op_code) {
    switch (op_code) {
        case 0: return "add";
        case 1: return "subtract";
        case 2: return "multiply";
        case 3: return "divide";
        default: return "unknown";
    }
}

int perform_operation(int a, int b, const char *operation) {
    if (strcmp(operation, "add") == 0) {
        return a + b;
    } else if (strcmp(operation, "subtract") == 0) {
        return a - b;
    } else if (strcmp(operation, "multiply") == 0) {
        return a * b;
    } else if (strcmp(operation, "divide") == 0) {
        if (b != 0) {
            return a / b;
        }
        return 0;
    }
    return 0;
}

// TODO: lib test stdout
int buffapp(int param1, int param2, int param3, int param4) {
    StringBuffer *log_buffer = create_buffer(32);
    int result = 0;
    char temp[64];

    log_buffer->length = 0;

    sprintf(temp, "Starting computation with %d parameters\n", 4);
    append_to_buffer(log_buffer, temp);

    const char *op1 = get_operation_name(param1 % 4);
    sprintf(temp, "Operation 1: %s(%d, %d)\n", op1, param1, param2);
    append_to_buffer(log_buffer, temp);

    int intermediate1 = perform_operation(param1, param2, op1);
    result += intermediate1;

    const char *op2 = get_operation_name(param3 % 4);
    sprintf(temp, "Operation 2: %s(%d, %d)\n", op2, param3, param4);
    append_to_buffer(log_buffer, temp);

    int intermediate2 = perform_operation(param3, param4, op2);
    result += intermediate2;

    const char *op3 = "multiply";
    sprintf(temp, "Operation 3: %s(%d, %d)\n", op3, intermediate1, intermediate2);
    append_to_buffer(log_buffer, temp);

    int intermediate3 = perform_operation(intermediate1, intermediate2, op3);

    if (intermediate3 != 0) {
        result = result / intermediate3;
    } else {
        result = param1 + param2 + param3 + param4;
    }

    sprintf(temp, "Final result: %d\n", result);
    append_to_buffer(log_buffer, temp);

    printf("Computation Log:\n%s\n", log_buffer->data);

    destroy_buffer(log_buffer);

    return result;
}
