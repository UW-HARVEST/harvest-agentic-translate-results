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

#define STRINGIZE(x) #x
#define TO_STRING(x) STRINGIZE(x)

int cleanup(int a, int b, int c, int d);
void print_result(const char *label, int result);
void cleanup_resources(char *dynamic_str);

int cleanup(int a, int b, int c, int d) {
    int numbers[] = {a, b, c, d};
    char *dynamic_str = NULL;
    int result = 0;

    const char *expected_str = "VALID";
    const char *input_str = "VALID";
    if (strncmp(input_str, expected_str, strlen(expected_str)) != 0) {
        printf("Input string validation failed.\n");
        goto cleanup;
    }

    for (int i = 0; i < 4; i++) {
        switch (numbers[i]) {
            case 10:
                result += 10;
            case 20:
                result += 20;
                break;
            case 30:
                result += 30;
            case 40:
                result += 40;
                break;
            default:
                result += numbers[i];
                break;
        }
    }

    dynamic_str = (char *)malloc(50 * sizeof(char));
    if (!dynamic_str) {
        printf("Memory allocation failed.\n");
        goto cleanup;
    }

    snprintf(dynamic_str, 50, "Processed numbers: %s", TO_STRING(numbers));
    printf("%s\n", dynamic_str);

cleanup:
    cleanup_resources(dynamic_str);
    return result;
}

void print_result(const char *label, int result) {
    printf("%s: %d\n", label, result);
}

void cleanup_resources(char *dynamic_str) {
    if (dynamic_str) {
        free(dynamic_str);
        dynamic_str = NULL;
    }
}
