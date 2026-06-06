// Translated from c_src/src/engine.c

use crate::a::{call_a_once, process_a_stream};
use crate::b::{call_b_once, process_b_stream};
use crate::lib_target::target;
use crate::util::{iv_peek, Program, Vm};

fn classify(impl_id: i32, x: i32) -> i32 {
    if impl_id == 0 {
        return call_a_once(x);
    }
    if impl_id == 1 {
        // MAC_CALL(call_b_once, x) => call_b_once(x + 1)
        return call_b_once(x.wrapping_add(1));
    }
    // inline_call(&target, MAC_CALL(target, x)) => target(target(x + 1))
    target(target(x.wrapping_add(1)))
}

fn process_stream(impl_id: i32, buf: &[i32]) -> i32 {
    if impl_id == 0 {
        return process_a_stream(buf);
    }
    if impl_id == 1 {
        return process_b_stream(buf);
    }
    let mut acc: i32 = 0;
    for &v in buf {
        let t = target(v);
        if (t & 1) == 0 {
            acc = acc.wrapping_add(t.wrapping_mul(2));
        } else {
            acc ^= t.wrapping_add(7);
        }
    }
    acc
}

/// Pops the last element if available, otherwise returns the previous `out`
/// value (mirroring C's iv_pop which leaves *out unchanged on failure).
fn iv_pop_or(stack: &mut Vec<i32>, fallback: i32) -> (bool, i32) {
    if let Some(v) = stack.pop() {
        (true, v)
    } else {
        (false, fallback)
    }
}

pub fn run_engine(impl_id: i32, code: &[i32], vm: &mut Vm) -> i32 {
    let mut p = Program::new(code);
    while let Some(op) = p.fetch() {
        vm.steps += 1;
        match op {
            0 => {
                let imm = match p.fetch() {
                    Some(v) => v,
                    None => return 1,
                };
                vm.stack.push(imm);
                vm.trace_push(0);
            }
            1 => {
                let b = match vm.stack.pop() {
                    Some(v) => v,
                    None => return 2,
                };
                let a = match vm.stack.pop() {
                    Some(v) => v,
                    None => return 2,
                };
                vm.stack.push(a.wrapping_add(b));
                vm.trace_push(1);
            }
            2 => {
                let b = match vm.stack.pop() {
                    Some(v) => v,
                    None => return 3,
                };
                let a = match vm.stack.pop() {
                    Some(v) => v,
                    None => return 3,
                };
                vm.stack.push(a.wrapping_mul(b));
                vm.trace_push(2);
            }
            3 => {
                let a = iv_peek(&vm.stack, 0);
                vm.stack.push(a);
                vm.trace_push(3);
            }
            4 => {
                if vm.stack.pop().is_none() {
                    return 4;
                }
                vm.trace_push(4);
            }
            5 => {
                let x = iv_peek(&vm.stack, 0);
                let bucket = classify(impl_id, x);
                vm.stack.push(bucket);
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
                let cond = match vm.stack.pop() {
                    Some(v) => v,
                    None => return 6,
                };
                if cond != 0 {
                    // C: if ((size_t)k > p.n - p.ip) return 7;
                    // Negative k becomes a huge size_t and triggers return 7.
                    if k < 0 {
                        return 7;
                    }
                    let ku = k as usize;
                    if ku > p.n - p.ip {
                        return 7;
                    }
                    p.ip += ku;
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
                    // Pass single instruction starting at saved_ip
                    let slice = &p.code[saved_ip..saved_ip + 1];
                    let rc = run_engine(impl_id, slice, vm);
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
                vm.stack.push(y);
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
                let m_us = m as usize;
                // Reproduce C's exact (buggy) behavior: pop into tmp twice.
                // The second loop overwrites tmp[i] only when the pop succeeds;
                // failed pops leave the value from the first loop.
                let mut tmp = vec![0i32; m_us];
                for i in (0..m_us).rev() {
                    let (ok, v) = iv_pop_or(&mut vm.stack, tmp[i]);
                    let _ = ok;
                    tmp[i] = v;
                }
                for i in (0..m_us).rev() {
                    let (ok, v) = iv_pop_or(&mut vm.stack, tmp[i]);
                    let _ = ok;
                    tmp[i] = v;
                }
                let s = process_stream(impl_id, &tmp);
                vm.stack.push(s);
                vm.trace_push(14);
            }
            10 => return 0,
            _ => return 99,
        }
    }
    0
}
