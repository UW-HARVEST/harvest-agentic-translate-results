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

#include "../include/util.h"
#include <stdlib.h>
#include <string.h>
#include <limits.h>
#include <stdint.h>

void iv_init(IntVec *v){ v->data=NULL; v->len=v->cap=0; }
void iv_free(IntVec *v){ free(v->data); v->data=NULL; v->len=v->cap=0; }

bool iv_reserve(IntVec *v, size_t need){
    if (need <= v->cap) return true;
    size_t nc = v->cap? v->cap:8;
    while (nc < need) {
        if (nc > (SIZE_MAX/2)) return false;
        nc *= 2;
    }
    int *p = (int*)realloc(v->data, nc*sizeof(int));
    if (!p) return false;
    v->data = p; v->cap = nc; return true;
}
bool iv_push(IntVec *v,int x){ if(v->len==v->cap && !iv_reserve(v, v->cap? v->cap*2:8)) return false; v->data[v->len++]=x; return true; }
bool iv_pop (IntVec *v,int *out){ if(!v->len) return false; if(out) *out=v->data[v->len-1]; v->len--; return true; }
int  iv_peek(const IntVec *v,int def){ return v->len? v->data[v->len-1]: def; }

void prog_init(Program *p, const int *code, size_t n){ p->code=code; p->n=n; p->ip=0; }
bool prog_fetch(Program *p, int *out){ if(p->ip>=p->n) return false; *out=p->code[p->ip++]; return true; }

void vm_init(VM *vm){ iv_init(&vm->stack); iv_init(&vm->trace); vm->steps=0; }
void vm_free(VM *vm){ iv_free(&vm->stack); iv_free(&vm->trace); vm->steps=0; }
void vm_trace(VM *vm, int t){ iv_push(&vm->trace, t); }

void vm_print(FILE *fp, const char *label, const VM *vm){
    fprintf(fp, "%sSTACK_TOP=%d STEPS=%d TRACE=", label, iv_peek(&vm->stack, -777), vm->steps);
    for (size_t i=0;i<vm->trace.len;i++) fputc("abcdefghijklmnopqrstuvwxyz"[(vm->trace.data[i])&25], fp);
    fputc('\n', fp);
}
