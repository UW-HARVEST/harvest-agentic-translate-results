//! Port of `c_src/src/engine.c`.

use crate::a;
use crate::b;
use crate::lib_target;
use crate::util::{iv_peek, iv_pop, Program, Vm};

/// `classify`
///
/// impl 0 -> `inline_call(call_a_once, x)`      == call_a_once(x)
/// impl 1 -> `MAC_CALL(call_b_once, x)`         == call_b_once(x + 1)
/// else   -> `inline_call(&target, MAC_CALL(target, x))` == target(target(x + 1))
fn classify(imp: i32, x: i32) -> i32 {
    if imp == 0 {
        return a::call_a_once(x);
    }
    if imp == 1 {
        return b::call_b_once(x.wrapping_add(1));
    }
    lib_target::target(lib_target::target(x.wrapping_add(1)))
}

/// `process_stream`
fn process_stream(imp: i32, buf: &[i32]) -> i32 {
    if imp == 0 {
        return a::process_a_stream(buf);
    }
    if imp == 1 {
        return b::process_b_stream(buf);
    }
    let mut acc: i32 = 0;
    for &x in buf {
        let t = lib_target::target(x);
        if (t & 1) == 0 {
            acc = acc.wrapping_add(t.wrapping_mul(2));
        } else {
            acc ^= t.wrapping_add(7);
        }
    }
    acc
}

/// `run_engine`
pub fn run_engine(impl_id: i32, code: &[i32], vm: &mut Vm) -> i32 {
    let mut p = Program::new(code);

    while let Some(op) = p.fetch() {
        vm.steps = vm.steps.wrapping_add(1);
        match op {
            // PUSH imm
            0 => {
                let imm = match p.fetch() {
                    Some(v) => v,
                    None => return 1,
                };
                vm.stack.push(imm);
                vm.trace(0);
            }
            // ADD
            1 => {
                let b_val = match iv_pop(&mut vm.stack) {
                    Some(v) => v,
                    None => return 2,
                };
                let a_val = match iv_pop(&mut vm.stack) {
                    Some(v) => v,
                    None => return 2,
                };
                vm.stack.push(a_val.wrapping_add(b_val));
                vm.trace(1);
            }
            // MUL
            2 => {
                let b_val = match iv_pop(&mut vm.stack) {
                    Some(v) => v,
                    None => return 3,
                };
                let a_val = match iv_pop(&mut vm.stack) {
                    Some(v) => v,
                    None => return 3,
                };
                vm.stack.push(a_val.wrapping_mul(b_val));
                vm.trace(2);
            }
            // DUP (peeks with default 0, so it pushes 0 on an empty stack)
            3 => {
                let a_val = iv_peek(&vm.stack, 0);
                vm.stack.push(a_val);
                vm.trace(3);
            }
            // DROP
            4 => {
                if iv_pop(&mut vm.stack).is_none() {
                    return 4;
                }
                vm.trace(4);
            }
            // CLASSIFY top-of-stack (peek, not pop) and push the bucket
            5 => {
                let x = iv_peek(&vm.stack, 0);
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
            // JMP-IF k
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
                    // `(size_t)k` sign-extends, so a negative k is a huge
                    // unsigned value and always fails this bound check.
                    if (k as usize) > p.n() - p.ip {
                        return 7;
                    }
                    p.ip += k as usize;
                    vm.trace(10);
                } else {
                    vm.trace(11);
                }
            }
            // REPEAT times: re-run the single next instruction `times` times
            7 => {
                let times = match p.fetch() {
                    Some(v) => v,
                    None => return 8,
                };
                if p.ip >= p.n() {
                    return 9;
                }
                let saved_ip = p.ip;
                let mut i: i32 = 0;
                while i < times {
                    let sub = &p.code[saved_ip..saved_ip + 1];
                    let rc = run_engine(impl_id, sub, vm);
                    if rc != 0 {
                        // The C assigns p.ip here and then again after the
                        // loop; both assignments are identical.
                        p.ip = saved_ip + 1;
                        vm.trace(12);
                        break;
                    }
                    i += 1;
                }
                p.ip = saved_ip + 1;
            }
            // CLASSIFY (no bucket-dependent trace)
            8 => {
                let x = iv_peek(&vm.stack, 0);
                let y = classify(impl_id, x);
                vm.stack.push(y);
                vm.trace(13);
            }
            // REDUCE m
            9 => {
                let m = match p.fetch() {
                    Some(v) => v,
                    None => return 10,
                };
                if m < 0 || (m as usize) > vm.stack.len() {
                    return 11;
                }
                let mu = m as usize;
                // `int tmp[m];` -- the first loop always fills every slot
                // because m <= stack.len was just validated.
                let mut tmp = vec![0i32; mu];

                // The original pops the operands TWICE. The second pass may
                // run the stack dry, in which case iv_pop leaves the slot
                // holding the value from the first pass. Reproduced as-is.
                let mut i = m - 1;
                while i >= 0 {
                    if let Some(v) = iv_pop(&mut vm.stack) {
                        tmp[i as usize] = v;
                    }
                    i -= 1;
                }
                let mut i = m - 1;
                while i >= 0 {
                    if let Some(v) = iv_pop(&mut vm.stack) {
                        tmp[i as usize] = v;
                    }
                    i -= 1;
                }

                let s = process_stream(impl_id, &tmp);
                vm.stack.push(s);
                vm.trace(14);
            }
            // HALT
            10 => return 0,
            _ => return 99,
        }
    }
    0
}
