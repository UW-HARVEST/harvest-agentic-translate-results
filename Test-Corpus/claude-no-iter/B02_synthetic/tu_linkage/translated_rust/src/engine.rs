// Translated from c_src/src/engine.c

use crate::a::{call_a_once, process_a_stream};
use crate::b::{call_b_once, process_b_stream};
use crate::lib_target::target;
use crate::util::{iv_peek, iv_pop, iv_push, Program, VM};

// MAC_CALL(F,X) = F(X + 1)
fn mac_call<F: Fn(i32) -> i32>(f: F, x: i32) -> i32 {
    f(x.wrapping_add(1))
}

fn classify(impl_id: i32, x: i32) -> i32 {
    if impl_id == 0 {
        // inline_call(call_a_once, x) = call_a_once(x)
        call_a_once(x)
    } else if impl_id == 1 {
        // MAC_CALL(call_b_once, x) = call_b_once(x+1)
        mac_call(call_b_once, x)
    } else {
        // inline_call(&target, MAC_CALL(target, x)) = target(target(x+1))
        target(mac_call(target, x))
    }
}

fn process_stream(impl_id: i32, buf: &[i32]) -> i32 {
    if impl_id == 0 {
        return process_a_stream(buf);
    }
    if impl_id == 1 {
        return process_b_stream(buf);
    }
    // EXT impl
    let mut acc: i32 = 0;
    for &x in buf {
        let t = target(x);
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
    while let Some(op) = p.fetch() {
        vm.steps = vm.steps.wrapping_add(1);
        match op {
            0 => {
                let imm = match p.fetch() {
                    Some(v) => v,
                    None => return 1,
                };
                iv_push(&mut vm.stack, imm);
                vm.trace_push(0);
            }
            1 => {
                let b = match iv_pop(&mut vm.stack) {
                    Some(v) => v,
                    None => return 2,
                };
                let a = match iv_pop(&mut vm.stack) {
                    Some(v) => v,
                    None => return 2,
                };
                iv_push(&mut vm.stack, a.wrapping_add(b));
                vm.trace_push(1);
            }
            2 => {
                let b = match iv_pop(&mut vm.stack) {
                    Some(v) => v,
                    None => return 3,
                };
                let a = match iv_pop(&mut vm.stack) {
                    Some(v) => v,
                    None => return 3,
                };
                iv_push(&mut vm.stack, a.wrapping_mul(b));
                vm.trace_push(2);
            }
            3 => {
                let a = iv_peek(&vm.stack, 0);
                iv_push(&mut vm.stack, a);
                vm.trace_push(3);
            }
            4 => {
                if iv_pop(&mut vm.stack).is_none() {
                    return 4;
                }
                vm.trace_push(4);
            }
            5 => {
                let x = iv_peek(&vm.stack, 0);
                let bucket = classify(impl_id, x);
                iv_push(&mut vm.stack, bucket);
                match bucket {
                    0 => vm.trace_push(5),
                    1 => vm.trace_push(6),
                    2 => vm.trace_push(7),
                    3 | 4 => vm.trace_push(8),
                    _ => vm.trace_push(9),
                }
            }
            6 => {
                let k = match p.fetch() {
                    Some(v) => v,
                    None => return 5,
                };
                let cond = match iv_pop(&mut vm.stack) {
                    Some(v) => v,
                    None => return 6,
                };
                if cond != 0 {
                    // C: if ((size_t)k > p.n - p.ip) return 7;
                    // Note: this casts k to size_t, so negative k becomes huge unsigned
                    // and would make this true. Reproduce that.
                    let k_us = k as usize; // truncation/reinterpretation matches (size_t)k for 64-bit
                    let remaining = p.n.wrapping_sub(p.ip);
                    if k_us > remaining {
                        return 7;
                    }
                    p.ip = p.ip.wrapping_add(k_us);
                    vm.trace_push(10);
                } else {
                    vm.trace_push(11);
                }
            }
            7 => {
                let times = match p.fetch() {
                    Some(v) => v,
                    None => return 8,
                };
                if p.ip >= p.n {
                    return 9;
                }
                let saved_ip = p.ip;
                for _i in 0..times {
                    // C: Program inner = p; inner.ip = saved_ip;
                    //    int rc = run_engine(impl_id, inner.code + inner.ip, 1, vm);
                    let inner_slice = &p.code[saved_ip..saved_ip + 1];
                    let rc = run_engine(impl_id, inner_slice, vm);
                    if rc != 0 {
                        p.ip = saved_ip + 1;
                        vm.trace_push(12);
                        break;
                    }
                }
                p.ip = saved_ip + 1;
            }
            8 => {
                let x = iv_peek(&vm.stack, 0);
                let y = classify(impl_id, x);
                iv_push(&mut vm.stack, y);
                vm.trace_push(13);
            }
            9 => {
                let m = match p.fetch() {
                    Some(v) => v,
                    None => return 10,
                };
                if m < 0 || (m as usize) > vm.stack.len() {
                    return 11;
                }
                let m_usz = m as usize;
                // Allocate tmp of length m, init to 0
                let mut tmp: Vec<i32> = vec![0; m_usz];
                // First loop: pop m values into tmp[m-1] ... tmp[0]
                for i in (0..m_usz).rev() {
                    if let Some(v) = iv_pop(&mut vm.stack) {
                        tmp[i] = v;
                    }
                }
                // Second loop: same again (BUG in C: pops twice). If stack runs out,
                // iv_pop returns false and tmp[i] is unchanged.
                for i in (0..m_usz).rev() {
                    if let Some(v) = iv_pop(&mut vm.stack) {
                        tmp[i] = v;
                    }
                }
                let s = process_stream(impl_id, &tmp);
                iv_push(&mut vm.stack, s);
                vm.trace_push(14);
            }
            10 => return 0,
            _ => return 99,
        }
    }
    0
}
