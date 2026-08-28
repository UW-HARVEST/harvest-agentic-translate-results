// Translation of c_src/src/engine.c
//
// Copyright 2025 MIT Lincoln Laboratory  (see c_src for the full license text)

use crate::impl_a::{call_a_once, process_a_stream};
use crate::impl_b::{call_b_once, process_b_stream};
use crate::lib_target::target;
use crate::util::{Program, Vm};

/// `static int classify(int impl, int x)`
///
/// `inline_call(f, x)` is `f(x)` and `MAC_CALL(F,X)` is `F(X+1)`.
fn classify(imp: i32, x: i32) -> i32 {
    if imp == 0 {
        // inline_call(call_a_once, x)
        return call_a_once(x);
    }
    if imp == 1 {
        // MAC_CALL(call_b_once, x) == call_b_once(x + 1)
        return call_b_once(x.wrapping_add(1));
    }
    // inline_call(&target, MAC_CALL(target, x)) == target(target(x + 1))
    target(target(x.wrapping_add(1)))
}

/// `static int process_stream(int impl, const int *buf, size_t n)`
fn process_stream(imp: i32, buf: &[i32]) -> i32 {
    if imp == 0 {
        return process_a_stream(buf);
    }
    if imp == 1 {
        return process_b_stream(buf);
    }
    let mut acc: i32 = 0;
    for &b in buf.iter() {
        let t = target(b);
        if (t & 1) == 0 {
            acc = acc.wrapping_add(t.wrapping_mul(2));
        } else {
            acc ^= t.wrapping_add(7);
        }
    }
    acc
}

/// `int run_engine(int impl_id, const int *code, size_t n, VM *vm)`
pub fn run_engine(impl_id: i32, code: &[i32], vm: &mut Vm) -> i32 {
    let n = code.len();
    let mut p = Program::new(code, n);
    let mut op: i32 = 0;
    while p.fetch(&mut op) {
        vm.steps = vm.steps.wrapping_add(1);
        match op {
            0 => {
                let mut imm: i32 = 0;
                if !p.fetch(&mut imm) {
                    return 1;
                }
                vm.stack.push(imm);
                vm.trace(0);
            }
            1 => {
                let mut a: i32 = 0;
                let mut b: i32 = 0;
                if !vm.stack.pop_into(&mut b) || !vm.stack.pop_into(&mut a) {
                    return 2;
                }
                vm.stack.push(a.wrapping_add(b));
                vm.trace(1);
            }
            2 => {
                let mut a: i32 = 0;
                let mut b: i32 = 0;
                if !vm.stack.pop_into(&mut b) || !vm.stack.pop_into(&mut a) {
                    return 3;
                }
                vm.stack.push(a.wrapping_mul(b));
                vm.trace(2);
            }
            3 => {
                let a = vm.stack.peek(0);
                vm.stack.push(a);
                vm.trace(3);
            }
            4 => {
                if vm.stack.pop().is_none() {
                    return 4;
                }
                vm.trace(4);
            }
            5 => {
                let x = vm.stack.peek(0);
                let bucket = classify(impl_id, x);
                vm.stack.push(bucket);

                match bucket {
                    0 => vm.trace(5),
                    1 => vm.trace(6),
                    2 => vm.trace(7),
                    // `case 3:;` falls through to `case 4:`
                    3 | 4 => vm.trace(8),
                    _ => vm.trace(9),
                }
            }
            6 => {
                let mut k: i32 = 0;
                if !p.fetch(&mut k) {
                    return 5;
                }
                let mut cond: i32 = 0;
                if !vm.stack.pop_into(&mut cond) {
                    return 6;
                }
                if cond != 0 {
                    // (size_t)k sign-extends, so a negative k becomes huge and
                    // always trips this check.
                    if (k as usize) > p.n - p.ip {
                        return 7;
                    }
                    p.ip += k as usize;
                    vm.trace(10);
                } else {
                    vm.trace(11);
                }
            }
            7 => {
                let mut times: i32 = 0;
                if !p.fetch(&mut times) {
                    return 8;
                }
                if p.ip >= p.n {
                    return 9;
                }
                let saved_ip = p.ip;
                let mut i: i32 = 0;
                while i < times {
                    // `Program inner = p; inner.ip = saved_ip;` then
                    // run_engine(impl_id, inner.code + inner.ip, 1, vm)
                    let rc = run_engine(impl_id, &p.code[saved_ip..saved_ip + 1], vm);
                    if rc != 0 {
                        // (the C code assigns p.ip = saved_ip + 1 here too)
                        p.ip = saved_ip + 1;
                        vm.trace(12);
                        break;
                    }
                    i = i.wrapping_add(1);
                }
                p.ip = saved_ip + 1;
            }
            8 => {
                let x = vm.stack.peek(0);
                let y = classify(impl_id, x);
                vm.stack.push(y);
                vm.trace(13);
            }
            9 => {
                let mut m: i32 = 0;
                if !p.fetch(&mut m) {
                    return 10;
                }
                if m < 0 || (m as usize) > vm.stack.len() {
                    return 11;
                }
                let m = m as usize;
                let mut tmp = vec![0i32; m];
                // NOTE: the C source pops twice into the same buffer.  The
                // second round of pops silently fails once the stack runs dry,
                // leaving the values written by the first round in place.
                for i in (0..m).rev() {
                    vm.stack.pop_into(&mut tmp[i]);
                }
                for i in (0..m).rev() {
                    vm.stack.pop_into(&mut tmp[i]);
                }
                let s = process_stream(impl_id, &tmp);
                vm.stack.push(s);
                vm.trace(14);
            }
            10 => return 0,
            _ => return 99,
        }
    }
    0
}
