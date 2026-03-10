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

void fma_array(int *restrict out, const int *mul1, const int *mul2, const int *add, int len) {
    for (int i = 0; i < len; i++) {
        out[i] = mul1[i] * mul2[i] + add[i];
    }
}

int call_fma(const int *data, int len) {
    if (len == 0) return 0;
    int out[len];
    int ones[len];
    int zeros[len];

    out[0] = 0;
    for (int i = 0; i < len; i++) {
        ones[i] = 1;
        zeros[i] = 0;
    }

    fma_array(out, ones, data, zeros, len);
    return out[len-1];
}

int main() {
    int data[100];
    int i;
    for (i = 0; i < 100; i++) {
        if (scanf("%d", &data[i]) != 1) {
            break;
        }
    }

    int result = call_fma(data, i);
    printf("%d\n", result);

    return 0;
}