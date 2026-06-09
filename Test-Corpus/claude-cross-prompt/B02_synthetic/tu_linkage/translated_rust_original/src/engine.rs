// Translated from c_src/src/engine.c

use std::ffi::c_int;

use crate::a::{call_a_once, process_a_stream};
use crate::b::{call_b_once, process_b_stream};
use crate::lib_target::target;
use crate::util::{
    iv_peek, iv_pop, iv_push, prog_fetch, prog_init, vm_trace, IntVec, Program, VM,
};

// Mirrors: static inline int inline_call(int (*f)(int), int x){ return f(x); }
fn inline_call(f: extern "C" fn(c_int) -> c_int, x: c_int) -> c_int {
    f(x)
}

// #define MAC_CALL(F,X) ((F)((X)+1))
// We inline this at call sites.

fn classify(impl_id: c_int, x: c_int) -> c_int {
    if impl_id == 0 {
        return inline_call(call_a_once, x);
    }
    if impl_id == 1 {
        // MAC_CALL(call_b_once, x) -> call_b_once(x + 1)
        return call_b_once(x.wrapping_add(1));
    }
    // inline_call(&target, MAC_CALL(target, x))
    // = inline_call(&target, target(x + 1))
    inline_call(target, target(x.wrapping_add(1)))
}

unsafe fn process_stream(impl_id: c_int, buf: *const c_int, n: usize) -> c_int {
    unsafe {
        if impl_id == 0 {
            return process_a_stream(buf, n);
        }
        if impl_id == 1 {
            return process_b_stream(buf, n);
        }
        let mut acc: c_int = 0;
        for i in 0..n {
            let t = target(*buf.add(i));
            if (t & 1) == 0 {
                acc = acc.wrapping_add(t.wrapping_mul(2));
            } else {
                acc ^= t.wrapping_add(7);
            }
        }
        acc
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn run_engine(
    impl_id: c_int,
    code: *const c_int,
    n: usize,
    vm: *mut VM,
) -> c_int {
    unsafe {
        let mut p = Program {
            code: std::ptr::null(),
            n: 0,
            ip: 0,
        };
        prog_init(&mut p as *mut Program, code, n);

        let mut op: c_int = 0;
        while prog_fetch(&mut p as *mut Program, &mut op as *mut c_int) {
            (*vm).steps = (*vm).steps.wrapping_add(1);
            match op {
                0 => {
                    let mut imm: c_int = 0;
                    if !prog_fetch(&mut p as *mut Program, &mut imm as *mut c_int) {
                        return 1;
                    }
                    iv_push(&mut (*vm).stack as *mut IntVec, imm);
                    vm_trace(vm, 0);
                }
                1 => {
                    let mut a: c_int = 0;
                    let mut b: c_int = 0;
                    if !iv_pop(&mut (*vm).stack as *mut IntVec, &mut b as *mut c_int)
                        || !iv_pop(&mut (*vm).stack as *mut IntVec, &mut a as *mut c_int)
                    {
                        return 2;
                    }
                    iv_push(&mut (*vm).stack as *mut IntVec, a.wrapping_add(b));
                    vm_trace(vm, 1);
                }
                2 => {
                    let mut a: c_int = 0;
                    let mut b: c_int = 0;
                    if !iv_pop(&mut (*vm).stack as *mut IntVec, &mut b as *mut c_int)
                        || !iv_pop(&mut (*vm).stack as *mut IntVec, &mut a as *mut c_int)
                    {
                        return 3;
                    }
                    iv_push(&mut (*vm).stack as *mut IntVec, a.wrapping_mul(b));
                    vm_trace(vm, 2);
                }
                3 => {
                    let a = iv_peek(&(*vm).stack as *const IntVec, 0);
                    iv_push(&mut (*vm).stack as *mut IntVec, a);
                    vm_trace(vm, 3);
                }
                4 => {
                    let mut tmp: c_int = 0;
                    if !iv_pop(&mut (*vm).stack as *mut IntVec, &mut tmp as *mut c_int) {
                        return 4;
                    }
                    vm_trace(vm, 4);
                }
                5 => {
                    let x = iv_peek(&(*vm).stack as *const IntVec, 0);
                    let bucket = classify(impl_id, x);
                    iv_push(&mut (*vm).stack as *mut IntVec, bucket);
                    match bucket {
                        0 => vm_trace(vm, 5),
                        1 => vm_trace(vm, 6),
                        2 => vm_trace(vm, 7),
                        3 | 4 => vm_trace(vm, 8),
                        _ => vm_trace(vm, 9),
                    }
                }
                6 => {
                    let mut k: c_int = 0;
                    if !prog_fetch(&mut p as *mut Program, &mut k as *mut c_int) {
                        return 5;
                    }
                    let mut cond: c_int = 0;
                    if !iv_pop(&mut (*vm).stack as *mut IntVec, &mut cond as *mut c_int) {
                        return 6;
                    }
                    if cond != 0 {
                        // (size_t)k > p.n - p.ip
                        // k may be negative; cast-to-size_t makes it huge.
                        let k_us = k as usize; // mirrors (size_t)k
                        if k_us > p.n.wrapping_sub(p.ip) {
                            return 7;
                        }
                        p.ip = p.ip.wrapping_add(k_us);
                        vm_trace(vm, 10);
                    } else {
                        vm_trace(vm, 11);
                    }
                }
                7 => {
                    let mut times: c_int = 0;
                    if !prog_fetch(&mut p as *mut Program, &mut times as *mut c_int) {
                        return 8;
                    }
                    if p.ip >= p.n {
                        return 9;
                    }
                    let saved_ip = p.ip;
                    let mut i: c_int = 0;
                    while i < times {
                        // Program inner = p; inner.ip = saved_ip;
                        // run_engine(impl_id, inner.code + inner.ip, 1, vm)
                        let rc = run_engine(
                            impl_id,
                            p.code.add(saved_ip),
                            1,
                            vm,
                        );
                        if rc != 0 {
                            p.ip = saved_ip + 1;
                            vm_trace(vm, 12);
                            break;
                        }
                        i = i.wrapping_add(1);
                    }
                    p.ip = saved_ip + 1;
                }
                8 => {
                    let x = iv_peek(&(*vm).stack as *const IntVec, 0);
                    let y = classify(impl_id, x);
                    iv_push(&mut (*vm).stack as *mut IntVec, y);
                    vm_trace(vm, 13);
                }
                9 => {
                    let mut m: c_int = 0;
                    if !prog_fetch(&mut p as *mut Program, &mut m as *mut c_int) {
                        return 10;
                    }
                    if m < 0 || (m as usize) > (*vm).stack.len {
                        return 11;
                    }
                    // int tmp[m]; — VLA with uninitialized contents in C.
                    // We allocate zeroed memory in Rust. The C code then pops
                    // 2*m elements (a bug); the second pop loop will be on an
                    // empty stack after the first loop, so iv_pop returns
                    // false and tmp[i] stays at its previous value (which in
                    // C is uninitialized). To reproduce exact behavior we use
                    // zero-initialized storage; tmp[i] remains 0 for all
                    // iterations of the second loop (since iv_pop won't write
                    // to tmp[i] when empty, but it WILL write the value from
                    // the first loop on the second iteration when stack is
                    // empty — so on the second loop, tmp[i] from the first
                    // loop is preserved by iv_pop returning false).
                    let m_us = m as usize;
                    let mut tmp: Vec<c_int> = vec![0; m_us];
                    // First pop loop: i = m-1 .. 0
                    for i in (0..m_us).rev() {
                        iv_pop(
                            &mut (*vm).stack as *mut IntVec,
                            tmp.as_mut_ptr().add(i),
                        );
                    }
                    // Second pop loop (bug — pops again):
                    for i in (0..m_us).rev() {
                        iv_pop(
                            &mut (*vm).stack as *mut IntVec,
                            tmp.as_mut_ptr().add(i),
                        );
                    }
                    let s = process_stream(impl_id, tmp.as_ptr(), m_us);
                    iv_push(&mut (*vm).stack as *mut IntVec, s);
                    vm_trace(vm, 14);
                }
                10 => return 0,
                _ => return 99,
            }
        }
        0
    }
}
