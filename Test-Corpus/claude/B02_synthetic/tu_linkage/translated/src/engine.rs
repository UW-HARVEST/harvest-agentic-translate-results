// Translation of c_src/src/engine.c

use crate::a::{call_a_once, process_a_stream};
use crate::b::{call_b_once, process_b_stream};
use crate::lib_target::target;
use crate::util::{Program, VM};

#[inline]
fn inline_call(f: fn(i32) -> i32, x: i32) -> i32 {
    f(x)
}

// MAC_CALL(F,X) = (F)((X)+1)  -- in engine.c
#[inline]
fn mac_call(f: fn(i32) -> i32, x: i32) -> i32 {
    f(x.wrapping_add(1))
}

fn classify(impl_id: i32, x: i32) -> i32 {
    if impl_id == 0 {
        return inline_call(call_a_once, x);
    }
    if impl_id == 1 {
        // MAC_CALL(call_b_once, x) = call_b_once(x+1)
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

pub fn run_engine(impl_id: i32, code: &[i32], vm: &mut VM) -> i32 {
    let n = code.len();
    let mut p = Program::new(code, n);
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
                    // (size_t)k for negative k becomes huge unsigned -> condition true -> return 7
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
                let mut i: i32 = 0;
                while i < times {
                    // inner.code = p.code, inner.ip = saved_ip
                    // run_engine(impl_id, inner.code + inner.ip, 1, vm)
                    // = run_engine on a 1-element slice starting at saved_ip
                    let sub = &p.code[saved_ip..saved_ip + 1];
                    let rc = run_engine(impl_id, sub, vm);
                    if rc != 0 {
                        // p.ip = saved_ip + 1; vm_trace(vm, 12); break;
                        // (we'll set p.ip below as well, but C does it before break too)
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
                // C: int tmp[m]; -- VLA on stack
                // Reproduce the bug: pop m elements, then pop AGAIN m elements (or fail silently).
                let mut tmp: Vec<i32> = vec![0; m_us];
                // First loop: pop m values into tmp[m-1..0]
                for i in (0..m_us).rev() {
                    if let Some(v) = vm.stack.pop() {
                        tmp[i] = v;
                    }
                    // C: iv_pop returns false on empty, but doesn't modify *out.
                    // tmp[i] keeps its previous value (uninitialized in C — UB).
                    // We've initialized to 0; if pop fails we leave tmp[i] alone.
                }
                // Second loop: pop AGAIN (this is a bug in the C code, reproduce exactly)
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
