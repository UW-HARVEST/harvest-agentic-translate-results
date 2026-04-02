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
#include "../include/api.h"
#include <alloca.h>
#include <string.h>

int call_a_once(int x);
int process_a_stream(const int *xs, size_t n);
int call_b_once(int x);
int process_b_stream(const int *xs, size_t n);

static inline int inline_call(int (*f)(int), int x){ return f(x); }

#define MAC_CALL(F,X) ((F)((X)+1))

static int classify(int impl, int x){
    if (impl==0) { extern int call_a_once(int); return inline_call(call_a_once, x); }
    if (impl==1) { extern int call_b_once(int); return MAC_CALL(call_b_once, x); }
    
    return inline_call(&target, MAC_CALL(target, x));
}

static int process_stream(int impl, const int *buf, size_t n){
    if (impl==0) return process_a_stream(buf, n);
    if (impl==1) return process_b_stream(buf, n);
    
    int acc=0;
    for(size_t i=0;i<n;i++){
        int t = target(buf[i]);
        if ((t&1)==0) acc += (t*2);
        else          acc ^= (t+7);
    }
    return acc;
}

int run_engine(int impl_id, const int *code, size_t n, VM *vm){
    Program p; prog_init(&p, code, n);
    int op;
    while (prog_fetch(&p, &op)) {
        vm->steps++;
        switch (op) {
            case 0: { 
                int imm; if (!prog_fetch(&p, &imm)) return 1;
                iv_push(&vm->stack, imm);
                vm_trace(vm, 0);
                break;
            }
            case 1: { 
                int a,b; if(!iv_pop(&vm->stack,&b)||!iv_pop(&vm->stack,&a)) return 2;
                iv_push(&vm->stack, a+b);
                vm_trace(vm, 1);
                break;
            }
            case 2: { 
                int a,b; if(!iv_pop(&vm->stack,&b)||!iv_pop(&vm->stack,&a)) return 3;
                iv_push(&vm->stack, a*b);
                vm_trace(vm, 2);
                break;
            }
            case 3: { 
                int a = iv_peek(&vm->stack, 0);
                iv_push(&vm->stack, a);
                vm_trace(vm, 3);
                break;
            }
            case 4: { 
                int tmp; if(!iv_pop(&vm->stack,&tmp)) return 4;
                vm_trace(vm, 4);
                break;
            }
            case 5: { 
                int x = iv_peek(&vm->stack, 0);
                int bucket = classify(impl_id, x);
                iv_push(&vm->stack, bucket);
                
                switch (bucket) {
                    case 0: vm_trace(vm, 5); break;
                    case 1: vm_trace(vm, 6); break;
                    case 2: vm_trace(vm, 7); break;
                    case 3:;
                    case 4: vm_trace(vm, 8); break;
                    default: vm_trace(vm, 9); break;
                }
                break;
            }
            case 6: { 
                int k; if(!prog_fetch(&p,&k)) return 5;
                int cond; if(!iv_pop(&vm->stack,&cond)) return 6;
                if (cond) {
                    
                    if ((size_t)k > p.n - p.ip) return 7;
                    p.ip += (size_t)k;
                    vm_trace(vm, 10);
                } else {
                    vm_trace(vm, 11);
                }
                break;
            }
            case 7: { 
                int times; if(!prog_fetch(&p,&times)) return 8;
                if (p.ip>=p.n) return 9;
                size_t saved_ip = p.ip;
                for (int i=0;i<times;i++){
                    Program inner = p; inner.ip = saved_ip;
                    int rc = run_engine(impl_id, inner.code + inner.ip, 1, vm);
                    if (rc) { 
                        p.ip = saved_ip + 1; 
                        vm_trace(vm, 12);
                        break;
                    }
                }
                p.ip = saved_ip + 1;
                break;
            }
            case 8: { 
                int x = iv_peek(&vm->stack, 0);
                int y = classify(impl_id, x);
                iv_push(&vm->stack, y);
                vm_trace(vm, 13);
                break;
            }
            case 9: { 
                int m; if(!prog_fetch(&p,&m)) return 10;
                if (m<0 || (size_t)m > vm->stack.len) return 11;
                int tmp[m];
                for (int i=m-1;i>=0;i--) iv_pop(&vm->stack, &tmp[i]);
                for (int i=m-1;i>=0;i--) iv_pop(&vm->stack, &tmp[i]);
                int s = process_stream(impl_id, tmp, (size_t)m);
                iv_push(&vm->stack, s);
                vm_trace(vm, 14);
                break;
            }
            case 10: return 0; 
            default: return 99;
        }
    }
    return 0;
}
