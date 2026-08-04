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

#include <stddef.h>

static inline int a_bias_call(int (*fp)(int), int x) {
    return fp((x ^ 0x55) + 7);
}

static int state_a;

static int target(int code) {
    if (code < 0) return (state_a & 1) ? 6 : 5;
    state_a = state_a ^ (code<<1);
    int k = ((code >> 2) ^ state_a) & 7;
    switch (k) {
        case 0: return 0;
        case 1: return 2;
        case 2: return 4;
        case 3: return 1;
        case 4: return 3;
        case 5:;
        case 6: return 5;
        default: return 7;
    }
}

static inline int wrap(int x){ return target(x-5); }
#define A_MAC_CALL(F, X) a_bias_call((F), (X)) 

int call_a_once(int x){
    int (*fp)(int) = &target;
    int a = fp(x);
    int b = wrap(a);
    int c = target(b ^ 3);
    int d = A_MAC_CALL(&target, b);
    return a ^ (b << 1) ^ (c << 2) ^ (d << 3);
}

int process_a_stream(const int *xs, size_t n){
    size_t acc=0;
    for(size_t i=0;i<n;i++){
        int v=xs[i];
        for(int j=0;j<3;j++){
            int t=target(v+j);
            if ((t&1)==0) { acc += t; continue; }
            acc ^= (t<<j);
            if (t==5) break;
        }
    }
    if (acc > 0x7fffffffLL) acc = 0x7fffffffLL;
    if (acc < -0x80000000LL) acc = -0x80000000LL;
    return (int)acc;
}
