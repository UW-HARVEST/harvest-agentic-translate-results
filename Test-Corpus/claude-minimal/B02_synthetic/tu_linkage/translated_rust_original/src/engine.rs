// Translated from c_src/src/engine.c

use crate::a::{call_a_once, process_a_stream};
use crate::b::{call_b_once, process_b_stream};
use crate::lib_target::target;
use crate::util::{Program, VM};

#[inline]
fn inline_call(f: fn(i32) -> i32, x: i32) -> i32 {
    f(x)
}

// MAC_CALL(F, X) -> ((F)((X)+1))
#[inline]
fn mac_call(f: fn(i32) -> i32, x: i32) -> i32 {
    f(x.wrapping_add(1))
}

fn classify(impl_id: i32, x: i32) -> i32 {
    if impl_id == 0 {
        return inline_call(call_a_once, x);
    }
    if impl_id == 1 {
        return mac_call(call_b_once, x);
    }
    inline_call(target, mac_call(target, x))
}

fn process_stream(impl_id: i32, buf: &[i32]) -> i32 {
    if impl_id == 0 {
        return process_a_stream(buf);
    }
    if impl_id == 1 {
        return process_b_stream(buf);
    }
    let mut acc: i32 = 0;
    for &v in buf.iter() {
        let t = target(v);
        if (t & 1) == 0 {
            acc = acc.wrapping_add(t.wrapping_mul(2));
        } else {
            acc ^= t.wrapping_add(7);
        }
    }
    acc
}

pub fn run_engine(impl_id: i32, code: &[i32], vm: &mut VM) -> i32 {
    let mut p = Program::new(code);
    let mut op: i32 = 0;
    while p.fetch(&mut op) {
        vm.steps += 1;
        match op {
            0 => {
                let mut imm = 0;
                if !p.fetch(&mut imm) {
                    return 1;
                }
                vm.stack.push(imm);
                vm.trace(0);
            }
            1 => {
                let mut a = 0;
                let mut b = 0;
                if !vm.stack.pop(Some(&mut b)) || !vm.stack.pop(Some(&mut a)) {
                    return 2;
                }
                vm.stack.push(a.wrapping_add(b));
                vm.trace(1);
            }
            2 => {
                let mut a = 0;
                let mut b = 0;
                if !vm.stack.pop(Some(&mut b)) || !vm.stack.pop(Some(&mut a)) {
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
                let mut tmp = 0;
                if !vm.stack.pop(Some(&mut tmp)) {
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
                    3 | 4 => vm.trace(8),
                    _ => vm.trace(9),
                }
            }
            6 => {
                let mut k = 0;
                if !p.fetch(&mut k) {
                    return 5;
                }
                let mut cond = 0;
                if !vm.stack.pop(Some(&mut cond)) {
                    return 6;
                }
                if cond != 0 {
                    // C: if ((size_t)k > p.n - p.ip) return 7;
                    // Note: cast to size_t makes negative k a huge number, hence treated as out-of-range.
                    if k < 0 {
                        return 7;
                    }
                    let k_u = k as usize;
                    let remaining = p.code.len() - p.ip;
                    if k_u > remaining {
                        return 7;
                    }
                    p.ip += k_u;
                    vm.trace(10);
                } else {
                    vm.trace(11);
                }
            }
            7 => {
                let mut times = 0;
                if !p.fetch(&mut times) {
                    return 8;
                }
                if p.ip >= p.code.len() {
                    return 9;
                }
                let saved_ip = p.ip;
                for _ in 0..times {
                    // inner.code + inner.ip is a sub-slice of length 1
                    let inner_slice = &p.code[saved_ip..saved_ip + 1];
                    let rc = run_engine(impl_id, inner_slice, vm);
                    if rc != 0 {
                        p.ip = saved_ip + 1;
                        vm.trace(12);
                        break;
                    }
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
                let mut m = 0;
                if !p.fetch(&mut m) {
                    return 10;
                }
                if m < 0 || (m as usize) > vm.stack.len() {
                    return 11;
                }
                let m_u = m as usize;
                let mut tmp = vec![0i32; m_u];
                // Mirror the (buggy) C: two consecutive pop loops; second loop pops past prior pops.
                // The C code pops m elements twice into tmp, but stops when stack is empty (pop returns false).
                for i in (0..m_u).rev() {
                    let mut v = 0;
                    if vm.stack.pop(Some(&mut v)) {
                        tmp[i] = v;
                    } else {
                        // pop returns false but tmp[i] left as 0 (matches C: tmp uninitialized but
                        // here we initialize to 0 which is safer than UB in C).
                    }
                }
                for i in (0..m_u).rev() {
                    let mut v = 0;
                    if vm.stack.pop(Some(&mut v)) {
                        tmp[i] = v;
                    }
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
