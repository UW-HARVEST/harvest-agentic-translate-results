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

static int memchra(const char *str, int c, size_t n) {
    int count = 0;
    for (size_t i = 0; i < n; i++) {
        if (str[i] == (char)c) {
            count++;
        }
    }
    return count;
}

static int process_buffer(char *buffer, size_t len) {
    if (buffer == NULL || *buffer == '\0') {
        return -1;
    }

    int result = 0;
    for (char *i = buffer; i < buffer + len && *i != '\0'; i++) {
        result += (int)(*i);
    }
    return result;
}

static float int_to_float_bits(int value) {
    union {
        int i;
        float f;
    } converter;

    converter.i = value;
    return converter.f;
}

static int process_strings(char **strings, int count, const char *target) {
    if (strings == NULL || count <= 0) {
        return 0;
    }

    int matches = 0;

    for (char **i = strings; i < strings + count; i++) {
        if (*i == NULL || **i == '\0') {
            continue;
        }

        if (strncmp(*i, target, strlen(target)) == 0) {
            matches++;
        }
    }

    return matches;
}

static int safe_sum_array(int *arr, size_t size) {
    if (arr == NULL || size == 0) {
        return 0;
    }

    int sum = 0;

    for (int *i = arr; i < arr + size; i++) {
        sum += *i;
    }

    return sum;
}

static int interpret_as_int(unsigned char *bytes, size_t len) {
    if (bytes == NULL || len < sizeof(int)) {
        return 0;
    }

    int *int_ptr = (int *)bytes;
    return *int_ptr;
}

static int count_occurrences(const char *text, char ch) {
    if (text == NULL || *text == '\0') {
        return 0;
    }

    size_t len = strlen(text);
    return memchra(text, ch, len);
}

static int complex_iteration(int *data, size_t count) {
    if (data == NULL || count == 0) {
        return -1;
    }

    int result = 0;

    for (int *i = data; i < data + count; i++) {
        unsigned int u = (unsigned int)(*i);
        result ^= (int)(u & 0xFF);
    }

    return result;
}

int memchra2(int a, int b, int c, int d) {
    int result = 0;

    char buffer[64];
    snprintf(buffer, sizeof(buffer), "test%d-%d-%d-%d", a, b, c, d);

    int dash_count = count_occurrences(buffer, '-');
    result += dash_count * 10;

    int values[] = {a, b, c, d};
    int sum = safe_sum_array(values, 4);
    result += sum;

    char *test_strings[] = {
        "test1",
        "test2",
        "testing",
        "other"
    };

    int matches = process_strings(test_strings, 4, "test");
    result += matches * 5;

    float f = int_to_float_bits(a);
    if (f > 0.0f && f < 1000.0f) {
        result += (int)f;
    }

    int buf_sum = process_buffer(buffer, strlen(buffer));
    if (buf_sum > 0) {
        result += buf_sum % 256;
    }

    unsigned char bytes[4];
    bytes[0] = (unsigned char)(b & 0xFF);
    bytes[1] = (unsigned char)(c & 0xFF);
    bytes[2] = (unsigned char)(d & 0xFF);
    bytes[3] = 0;

    int interpreted = interpret_as_int(bytes, 4);
    result ^= interpreted;

    int complex_result = complex_iteration(values, 4);
    result += complex_result;

    return result;
}
