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

#ifndef UTIL_H
#define UTIL_H
#include <stddef.h>
#include <stdbool.h>
#include <stdio.h>

typedef struct {
    int *data; size_t len, cap;
} IntVec;

void iv_init(IntVec *v);
void iv_free(IntVec *v);
bool iv_reserve(IntVec *v, size_t need);
bool iv_push(IntVec *v, int x);
bool iv_pop(IntVec *v, int *out);
int  iv_peek(const IntVec *v, int def);

typedef struct {
    const int *code; size_t n; size_t ip;
} Program;

void prog_init(Program *p, const int *code, size_t n);
bool prog_fetch(Program *p, int *out);

typedef struct {
    IntVec stack;
    IntVec trace;
    int    steps;
} VM;

void vm_init(VM *vm);
void vm_free(VM *vm);
void vm_trace(VM *vm, int t);
void vm_print(FILE *fp, const char *label, const VM *vm);

int run_engine(int impl_id, const int *code, size_t n, VM *vm);

#endif
