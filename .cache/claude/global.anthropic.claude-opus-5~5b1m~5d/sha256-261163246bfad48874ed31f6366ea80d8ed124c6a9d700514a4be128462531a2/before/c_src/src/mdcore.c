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
#include "mdmacros.h"

/* Define operations */
int op_add(int a,int b){ return a + b; }
int op_sub(int a,int b){ return a - b; }
int op_mul(int a,int b){ return a * b; }

/* Define the macro-generated accumulator for the selected OP */
DEFINE_ACCUM(OP)

/* Global macro uses at file scope (exercises expansion at global init) */
int (*G_OP)(int,int) = OP_FN(OP);
const char *G_OP_NAME = STR(OP);

int helper_call(int a, int b) {
    int r = (OP_FN(OP))(a, b);
    int acc = INIT_FOR(OP);
    RUN_LOOP(OP, acc, REPEAT);
    printf("helper.call=%d helper.acc=%d\n", r, acc);
    return r + acc;
}

int helper_ptr(int a, int b) {
    int (*fp)(int,int) = OP_FN(OP);
    int r = fp(a, b);
    printf("helper.ptr=%d\n", r);
    return r;
}

int use_generated(int n) {
    int r = (ACCUM_FN(OP))(n);
    printf("gen.acc=%d\n", r);
    return r;
}
