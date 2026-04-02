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
#include <limits.h>
#include <errno.h>

#define ARRAY_SIZE (256 * 1024) // 1MB assuming sizeof(int) = 4
#define ITERATIONS 2000

// Global array
int array[ARRAY_SIZE];

// Perform expensive arithmetic on each element
void perform_expensive_operations() {
    for (size_t i = 0; i < ARRAY_SIZE; i++) {
        int x = array[i];
        for (int j = 0; j < 100; j++) {
            x = x * 3 + 7;
            x = x ^ (x >> 3);
            x = x - (x << 1);
            x = x / 2 + x % 7;
        }
        array[i] = x;
    }
}

int main(int argc, char *argv[]) {
    if (argc != 2) {
        fprintf(stderr, "Usage: %s <seed>\n", argv[0]);
        return 1;
    }

    errno = 0;
    char *endptr;
    unsigned long temp_seed = strtoul(argv[1], &endptr, 10);
    if (*endptr != '\0' || errno != 0 || temp_seed > UINT_MAX) {
        fprintf(stderr, "Invalid seed: '%s'\n", argv[1]);
        return 1;
    }

    unsigned int seed = (unsigned int)temp_seed;
    srand(seed);

    for (size_t i = 0; i < ARRAY_SIZE; i++) {
        array[i] = rand();
    }

    for (int i = 0; i < ITERATIONS; i++) {
        perform_expensive_operations();
    }

    int xor_result = 0;
    for (size_t i = 0; i < ARRAY_SIZE; i++) {
        xor_result ^= array[i];
    }

    printf("%d\n", xor_result);
    return 0;
}

