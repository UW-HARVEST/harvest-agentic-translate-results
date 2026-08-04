// Translation of c_src/src/engine.c

use core::ffi::c_int;

use crate::a::{call_a_once, process_a_stream};
use crate::b::{call_b_once, process_b_stream};
use crate::lib_target::target;
use crate::util::{Program, VM};

#[inline]
fn inline_call(f: fn(c_int) -> c_int, x: c_int) -> c_int {
    f(x)
}

// MAC_CALL(F,X) = (F)((X)+1)  -- in engine.c
#[inline]
fn mac_call(f: fn(c_int) -> c_int, x: c_int) -> c_int {
    f(x.wrapping_add(1))
}

fn classify(impl_id: c_int, x: c_int) -> c_int {
    if impl_id == 0 {
        return inline_call(call_a_once, x);
    }
    if impl_id == 1 {
        // MAC_CALL(call_b_once, x) = call_b_once(x+1)
        return mac_call(call_b_once, x);
    }
    inline_call(target, mac_call(target, x))
}

fn process_stream(impl_id: c_int, buf: &[c_int]) -> c_int {
    if impl_id == 0 {
        return process_a_stream(buf);
    }
    if impl_id == 1 {
        return process_b_stream(buf);
    }
    let mut acc: c_int = 0;
    for &b in buf {
        let t = target(b);
        if (t & 1) == 0 {
            acc = acc.wrapping_add(t.wrapping_mul(2));
        } else {
            acc ^= t.wrapping_add(7);
        }
    }
    acc
}

pub fn run_engine(impl_id: c_int, code: &[c_int], vm: &mut VM) -> c_int {
    let mut p = Program::new(code);
    while let Some(op) = p.fetch() {
        vm.steps = vm.steps.wrapping_add(1);
        match op {
            0 => {
                let imm = match p.fetch() {
                    Some(v) => v,
                    None => return 1,
                };
                vm.stack.push(imm);
                vm.trace(0);
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
                vm.trace(1);
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
                    3 | 4 => vm.trace(8),
                    _ => vm.trace(9),
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
                    if k < 0 {
                        return 7;
                    }
                    let k_us = k as usize;
                    if k_us > p.n - p.ip {
                        return 7;
                    }
                    p.ip += k_us;
                    vm.trace(10);
                } else {
                    vm.trace(11);
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
                let mut i: c_int = 0;
                while i < times {
                    // run_engine on a 1-element slice at saved_ip
                    let sub = unsafe { core::slice::from_raw_parts(p.code.add(saved_ip), 1) };
                    let rc = run_engine(impl_id, sub, vm);
                    if rc != 0 {
                        vm.trace(12);
                        break;
                    }
                    i += 1;
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
                let m = match p.fetch() {
                    Some(v) => v,
                    None => return 10,
                };
                if m < 0 || (m as usize) > vm.stack.len() {
                    return 11;
                }
                let m_us = m as usize;
                // C: int tmp[m]; -- VLA on stack (uninitialized)
                // Reproduce the bug: pop m elements, then pop AGAIN m elements (or fail silently).
                let mut tmp: Vec<c_int> = vec![0; m_us];
                for i in (0..m_us).rev() {
                    if let Some(v) = vm.stack.pop() {
                        tmp[i] = v;
                    }
                }
                for i in (0..m_us).rev() {
                    if let Some(v) = vm.stack.pop() {
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
