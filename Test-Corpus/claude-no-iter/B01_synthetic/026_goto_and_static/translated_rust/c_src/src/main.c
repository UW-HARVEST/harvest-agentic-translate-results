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

static int y = 123;

static int multi_stage(int x, int z) {
    int result = 0;
    if (x != 1) {
        printf("Error: x != 1\n");
        result = 1;
        goto fail;
    }

    if (y != 2) {
        printf("Error: x == 1 but y != 2\n");
        result = 2;
        goto fail;
    }

    if (z != 3) {
        printf("Error: x == 1 and y == 2, but z != 3\n");
        result = 3;
        goto fail;
    }

    printf("Ok!\n");
    return result;

fail:
    printf("Operation failed\n");
    return result;
}

int main() {
    int x = 0, z = 0;
    scanf("%d %d %d", &x, &y, &z);
    int result = multi_stage(x, z);
    printf("Result: %d\n", result);
    return 0;
}