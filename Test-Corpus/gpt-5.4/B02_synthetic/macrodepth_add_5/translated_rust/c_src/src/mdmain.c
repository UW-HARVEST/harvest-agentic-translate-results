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
#include "mdmacros.h"

int main(int argc, char **argv) {
    if (argc < 3) {
        fprintf(stderr, "usage: %s A B\n", argv[0]);
        return 2;
    }
    int a = atoi(argv[1]);
    int b = atoi(argv[2]);

    int r_call = (OP_FN(OP))(a, b);
    int acc = INIT_FOR(OP);
    RUN_LOOP(OP, acc, REPEAT);

    int x1 = helper_call(a, b);
    int x2 = helper_ptr(a, b);
    int x3 = use_generated(REPEAT);
    int g  = G_OP(a, b);

    printf("op=%s call=%d acc=%d g.call=%d\n", G_OP_NAME, r_call, acc, g);
    printf("summary=%d\n", r_call + acc + x1 + x2 + x3 + g);
    return 0;
}
