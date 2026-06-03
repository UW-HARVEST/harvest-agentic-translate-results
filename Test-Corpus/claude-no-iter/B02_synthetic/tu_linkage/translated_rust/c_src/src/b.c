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

static inline int b_twist_call(int (*fp)(int), int x) {
    return fp(((x + 9) ^ 0x2222) - 17);
}

static int flipflop;

static int target(int code){
    flipflop ^= 1;
    if (code < 0) return flipflop ? 2 : 6;
    int z = (code ^ (flipflop? 0x7f:0x1f)) % 8;
    if (z==0 || z==7) return 4;
    if (z==1 || z==2) return 3;
    if (z==3) return 1;
    if (z==4) return 0;
    if (z==5) return 5;
    return 7;
}

static inline int w2(int x){ return target(x+9); }
#define B_MAC_CALL(F, X) b_twist_call((F), (X))

int call_b_once(int x){
    int (*fp)(int) = &target;
    int a = target(x);
    int b = w2(a);
    int c = B_MAC_CALL(&target, a);
    int d = fp(c ^ x);
    return (a << 1) ^ (b << 2) ^ (c << 3) ^ (d << 4);
}

int process_b_stream(const int *xs, size_t n){
    int acc=1;
    for(size_t i=0;i<n;i++){
        int v=xs[i];
        int iter=0;
        while(++iter<=4){
            int t = target(v-iter);
            if (t==6) { acc -= t; break; }
            if (t==3) { continue; }
            acc = (acc * 3) ^ t;
        }
    }
    return acc;
}
