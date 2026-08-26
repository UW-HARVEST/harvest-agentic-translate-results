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
#include <math.h> // ignore hard blocker "compiler builtins" from here

int convert_double_to_int(double value) {
    return (int)value;
}

int find_value_in_buffer(const char *buffer, size_t size, int search_val) {
    char target = (char)search_val;
    void *result = memchr(buffer, target, size);
    if (result != NULL) {
        return (int)((char*)result - buffer);
    }
    return -1;
}

int process_negation(int var1) {
    int var2;
    var2 = !!var1;
    return var2;
}

void create_numeric_buffer(char *buffer, int size, int seed) {
    for (int i = 0; i < size; i++) {
        buffer[i] = (char)((seed + i * 7) % 256);
    }
}

double calculate_with_doubles(int a, int b, int c) {
    double result = 0.0;

    if (b != 0) {
        result = (double)a / (double)b;
    }

    result *= pow(10.0, c % 10);

    return result;
}

int doubleneg(int param1, int param2, int param3, int param4) {
    int result = 0;
    char buffer[256];
    int i;

    printf("=== Starting foo() execution ===\n");
    printf("Parameters: %d, %d, %d, %d\n", param1, param2, param3, param4);

    printf("\n--- Integer Negation Test ---\n");
    int negation_test = param1;
    int negation_result = !!negation_test;
    printf("Original value: %d\n", negation_test);
    printf("After !!negation: %d\n", negation_result);
    result += negation_result * 10;

    int neg_p2 = !!param2;
    int neg_p3 = !!param3;
    int neg_p4 = !!param4;
    printf("Double negation results: %d, %d, %d\n", neg_p2, neg_p3, neg_p4);
    result += neg_p2 + neg_p3 + neg_p4;

    printf("\n--- Double to Int Conversion Test ---\n");

    double large_double = calculate_with_doubles(param1, param2, param3);
    printf("Calculated double value: %e\n", large_double);

    int converted_int = convert_double_to_int(large_double);
    printf("Converted to int (may be UB): %d\n", converted_int);

    double negative_large = -1.0 * pow(2.0, 40);
    printf("Very large negative double: %e\n", negative_large);
    int converted_neg = convert_double_to_int(negative_large);
    printf("Converted to int (UB likely): %d\n", converted_neg);

    result += (converted_int % 1000) + (converted_neg % 1000);

    printf("\n--- Memchr Search Test ---\n");

    create_numeric_buffer(buffer, 256, param1);

    int search_values[] = {param2 % 256, param3 % 256, param4 % 256, 42};
    int num_searches = sizeof(search_values) / sizeof(search_values[0]);

    printf("Searching buffer for values...\n");
    for (i = 0; i < num_searches; i++) {
        int pos = find_value_in_buffer(buffer, 256, search_values[i]);
        if (pos >= 0) {
            printf("Found value %d at position %d\n", search_values[i], pos);
            result += pos;
        } else {
            printf("Value %d not found\n", search_values[i]);
        }
    }

    char *direct_search = (char*)memchr(buffer, 100, 256);
    if (direct_search != NULL) {
        printf("Direct memchr found byte 100 at offset: %ld\n",
               (long)(direct_search - buffer));
        result += (int)(direct_search - buffer);
    }

    printf("\n--- Combined Feature Test ---\n");
    for (i = 0; i < 10; i++) {
        int search_byte = (param1 + i * param2) % 256;
        void *found = memchr(buffer, search_byte, 256);
        int found_flag = !!found;  // Double negation on pointer
        printf("Search %d: byte=%d, found=%d\n", i, search_byte, found_flag);
        result += found_flag;
    }

    double infinity_val = INFINITY;
    double nan_val = NAN;

    printf("\n--- Special Double Values ---\n");
    printf("Converting INFINITY to int: ");
    int inf_as_int = convert_double_to_int(infinity_val);
    printf("%d (undefined behavior)\n", inf_as_int);

    printf("Converting NAN to int: ");
    int nan_as_int = convert_double_to_int(nan_val);
    printf("%d (undefined behavior)\n", nan_as_int);

    printf("\n=== Final Result ===\n");
    printf("Accumulated result: %d\n", result);

    return result;
}
