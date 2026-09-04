// Translation of c_src/src/engine.c

use crate::a;
use crate::b;
use crate::libtarget::target;
use crate::stacklimit;
use crate::util::{iv_peek, iv_pop, Program, Vm};

/// C: `static int classify(int impl, int x)`
///
/// `inline_call(f, x)` is `f(x)`; `MAC_CALL(F, X)` is `((F)((X)+1))`.
/// The `impl == 0` / `impl == 1` arms call the *external* a.c / b.c entry
/// points; the fallback arm uses lib.c's global `target`.
fn classify(imp: i32, x: i32) -> i32 {
    if imp == 0 {
        // inline_call(call_a_once, x)
        return a::call_a_once(x);
    }
    if imp == 1 {
        // MAC_CALL(call_b_once, x) == call_b_once(x + 1)
        return b::call_b_once(x.wrapping_add(1));
    }
    // inline_call(&target, MAC_CALL(target, x)) == target(target(x + 1))
    target(target(x.wrapping_add(1)))
}

/// C: `static int process_stream(int impl, const int *buf, size_t n)`
fn process_stream(imp: i32, buf: &[i32]) -> i32 {
    if imp == 0 {
        return a::process_a_stream(buf);
    }
    if imp == 1 {
        return b::process_b_stream(buf);
    }
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

/// C: `int run_engine(int impl_id, const int *code, size_t n, VM *vm)`
pub fn run_engine(impl_id: i32, code: &[i32], n: usize, vm: &mut Vm) -> i32 {
    let mut p = Program::new(code, n);

    while let Some(op) = p.fetch() {
        vm.steps = vm.steps.wrapping_add(1);
        match op {
            // PUSH imm
            0 => {
                let imm = match p.fetch() {
                    Some(v) => v,
                    None => return 1,
                };
                let _ = vm.stack.push(imm);
                vm.trace(0);
            }
            // ADD
            1 => {
                // C short-circuits: if the first pop fails the second is never
                // attempted. Both failures yield rc 2.
                let bv = match iv_pop(&mut vm.stack) {
                    Some(v) => v,
                    None => return 2,
                };
                let av = match iv_pop(&mut vm.stack) {
                    Some(v) => v,
                    None => return 2,
                };
                let _ = vm.stack.push(av.wrapping_add(bv));
                vm.trace(1);
            }
            // MUL
            2 => {
                let bv = match iv_pop(&mut vm.stack) {
                    Some(v) => v,
                    None => return 3,
                };
                let av = match iv_pop(&mut vm.stack) {
                    Some(v) => v,
                    None => return 3,
                };
                let _ = vm.stack.push(av.wrapping_mul(bv));
                vm.trace(2);
            }
            // DUP
            3 => {
                let av = iv_peek(&vm.stack, 0);
                let _ = vm.stack.push(av);
                vm.trace(3);
            }
            // DROP
            4 => {
                if iv_pop(&mut vm.stack).is_none() {
                    return 4;
                }
                vm.trace(4);
            }
            // CLASSIFY (peek)
            5 => {
                let x = iv_peek(&vm.stack, 0);
                let bucket = classify(impl_id, x);
                let _ = vm.stack.push(bucket);

                match bucket {
                    0 => vm.trace(5),
                    1 => vm.trace(6),
                    2 => vm.trace(7),
                    // C: `case 3:;` falls through into `case 4:`
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
                    // C: `(size_t)k` sign-extends, so a negative k becomes a
                    // huge unsigned value and always trips this guard.
                    let ku = k as i64 as usize;
                    if ku > p.n - p.ip {
                        return 7;
                    }
                    p.ip += ku;
                    vm.trace(10);
                } else {
                    vm.trace(11);
                }
            }
            // REPEAT times
            7 => {
                let times = match p.fetch() {
                    Some(v) => v,
                    None => return 8,
                };
                if p.ip >= p.n {
                    return 9;
                }
                let saved_ip = p.ip;
                // `p.code` is a shared slice reference and is Copy, so this
                // sub-slice borrow is independent of `p` itself.
                let base: &[i32] = p.code;
                let mut i: i32 = 0;
                while i < times {
                    // C: `Program inner = p; inner.ip = saved_ip;`
                    //    `run_engine(impl_id, inner.code + inner.ip, 1, vm)`
                    let rc = run_engine(impl_id, &base[saved_ip..saved_ip + 1], 1, vm);
                    if rc != 0 {
                        p.ip = saved_ip + 1;
                        vm.trace(12);
                        break;
                    }
                    i += 1;
                }
                p.ip = saved_ip + 1;
            }
            // CLASSIFY (peek, unconditional trace)
            8 => {
                let x = iv_peek(&vm.stack, 0);
                let y = classify(impl_id, x);
                let _ = vm.stack.push(y);
                vm.trace(13);
            }
            // REDUCE m
            9 => {
                let m = match p.fetch() {
                    Some(v) => v,
                    None => return 10,
                };
                // The bound is checked against the stack length *before* any pops.
                if m < 0 || (m as i64 as usize) > vm.stack.len() {
                    return 11;
                }
                let mc = m as usize;
                // C: `int tmp[m];` -- a VLA in this stack frame, so an
                // oversized `m` dies with SIGSEGV. Reproduce that, then keep
                // the data on the heap (Rust has no VLA).
                let here = 0u8;
                let sp = &here as *const u8 as usize;
                let vla_bytes = mc.saturating_mul(4);
                if vla_bytes >= stacklimit::remaining(sp) {
                    stacklimit::raise_segv();
                }
                // The first loop below always fills every slot (the guard above
                // guarantees it), so the C version never reads an
                // uninitialized element.
                let mut tmp: Vec<i32> = vec![0; mc];

                for i in (0..mc).rev() {
                    if let Some(v) = iv_pop(&mut vm.stack) {
                        tmp[i] = v;
                    }
                }
                // Faithful reproduction of the original's duplicated pop loop:
                // it drains up to `m` *additional* values from the stack. Where
                // `iv_pop` fails it leaves `*out` alone, so `tmp[i]` keeps the
                // value written by the first loop.
                for i in (0..mc).rev() {
                    if let Some(v) = iv_pop(&mut vm.stack) {
                        tmp[i] = v;
                    }
                }

                let s = process_stream(impl_id, &tmp);
                let _ = vm.stack.push(s);
                vm.trace(14);
            }
            // HALT
            10 => return 0,
            _ => return 99,
        }
    }
    0
}
