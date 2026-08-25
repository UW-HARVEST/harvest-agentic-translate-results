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

#define STRINGIFY(x) #x
#define DEBUG_VAR(var) printf("Debug: " STRINGIFY(var) " = %d\n", var)
#define LOG_OPERATION(op, val) printf("Operation: " STRINGIFY(op) " with value %d\n", val)

typedef struct {
    unsigned int flag1 : 1;
    unsigned int flag2 : 1;
    unsigned int flag3 : 1;
    unsigned int counter : 5;
    unsigned int mode : 3;
    unsigned int status : 5;
    unsigned int reserved : 16;
} PackedFlags;

typedef union {
    int int_val;
    float float_val;
    unsigned int uint_val;
    char bytes[4];
} TypeConfusion;

typedef struct {
    PackedFlags flags;
    TypeConfusion data;
    char* buffer;
    int capacity;
} ProcessState;

ProcessState* create_state(int initial_val, int capacity) {
    ProcessState* state = (ProcessState*)malloc(sizeof(ProcessState));

    if (state == NULL) {
        printf("Error: Failed to allocate memory for state\n");
        return NULL;
    }

    state->flags.flag1 = 1;
    state->flags.flag2 = 0;
    state->flags.flag3 = 1;
    state->flags.counter = 0;
    state->flags.mode = 3;
    state->flags.status = 15;
    state->flags.reserved = 0;

    state->data.int_val = initial_val;

    state->capacity = capacity;
    state->buffer = (char*)malloc(capacity);

    if (state->buffer == NULL) {
        printf("Error: Failed to allocate buffer\n");
        free(state);
        return NULL;
    }

    snprintf(state->buffer, capacity, "State:%d:Mode:%d",
             initial_val, state->flags.mode);

    return state;
}

void destroy_state(ProcessState* state) {
    if (state != NULL) {
        if (state->buffer != NULL) {
            free(state->buffer);
        }
        free(state);
    }
}

int process_buffer(ProcessState* state, char target) {
    if (state == NULL || state->buffer == NULL) {
        printf("Error: Null pointer in process_buffer\n");
        return -1;
    }

    int count = 0;
    char* ptr = state->buffer;
    size_t remaining = strlen(state->buffer);

    while (remaining > 0) {
        char* found = (char*)memchr(ptr, target, remaining);

        if (found == NULL) {
            break;
        }

        count++;
        LOG_OPERATION(memchr_found, count);

        remaining -= (found - ptr + 1);
        ptr = found + 1;
    }

    return count;
}

void update_flags(ProcessState* state, int param) {
    if (state == NULL) {
        return;
    }

    state->flags.counter = (state->flags.counter + 1) & 0x1F; // 5-bit counter
    state->flags.flag1 = (param & 1);
    state->flags.flag2 = (param & 2) >> 1;
    state->flags.flag3 = (param & 4) >> 2;
    state->flags.mode = (param >> 3) & 0x7;

    DEBUG_VAR(state->flags.counter);
    printf("Bit fields - flag1:%d flag2:%d flag3:%d mode:%d\n",
           state->flags.flag1, state->flags.flag2,
           state->flags.flag3, state->flags.mode);
}

int confuse_types(ProcessState* state, int operation) {
    if (state == NULL) {
        return 0;
    }

    int result = 0;

    switch (operation) {
        case 0:
            state->data.int_val = 1078530011;
            printf("Set as int: %d\n", state->data.int_val);
            break;

        case 1:
            printf("Read as float: %f\n", state->data.float_val);
            result = (int)(state->data.float_val * 100);
            break;

        case 2:
            printf("Read as uint: %u\n", state->data.uint_val);
            result = state->data.uint_val & 0xFF;
            break;

        case 3:
            printf("Read as bytes: [%d, %d, %d, %d]\n",
                   state->data.bytes[0], state->data.bytes[1],
                   state->data.bytes[2], state->data.bytes[3]);
            result = state->data.bytes[0] + state->data.bytes[1];
            break;
    }

    return result;
}

int confusion(int param1, int param2, int param3, int param4) {
    DEBUG_VAR(param1);
    DEBUG_VAR(param2);
    DEBUG_VAR(param3);
    DEBUG_VAR(param4);

    int result = 0;

    ProcessState* state = create_state(param1, 128);

    if (state == NULL) {
        return -1;
    }

    update_flags(state, param2);

    char search_char = '0' + (param3 % 10);
    int found_count = process_buffer(state, search_char);
    result += found_count * 10;

    int confusion_result = confuse_types(state, param4 % 4);
    result += confusion_result;

    result += state->flags.counter * 5;
    result += state->flags.mode * 3;

    printf("Final result: %d\n", result);

    destroy_state(state);

    return result;
}
