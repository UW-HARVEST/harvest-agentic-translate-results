// Translation of engine.c

use crate::a::{call_a_once, process_a_stream};
use crate::b::{call_b_once, process_b_stream};
use crate::lib_target::target;
use crate::util::{Program, VM};

fn classify(impl_id: i32, x: i32) -> i32 {
    if impl_id == 0 {
        // inline_call(call_a_once, x)
        return call_a_once(x);
    }
    if impl_id == 1 {
        // MAC_CALL(call_b_once, x) = call_b_once(x + 1)
        return call_b_once(x.wrapping_add(1));
    }
    // Default: inline_call(&target, MAC_CALL(target, x))
    //   = target(target(x + 1))
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

pub fn run_engine(impl_id: i32, code: &[i32], n: usize, vm: &mut VM) -> i32 {
    let mut p = Program::init(code, n);
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
                    // case 3: case 4: -> trace 8 (case 3 falls through to case 4)
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
                    // size_t cast: for negative k, it becomes a huge value; p.n - p.ip is non-negative.
                    let k_us: usize = k as isize as usize; // sign-extend then reinterpret
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
                let mut i = 0i32;
                while i < times {
                    // Inner program runs a single instruction at code[saved_ip..saved_ip+1].
                    // Note: due to the structure of run_engine, calling run_engine on a
                    // 1-instruction slice may consume only that instruction - if the
                    // instruction itself fetches an operand it'll fail (return non-zero
                    // because n=1).
                    let slice = &p.code[saved_ip..saved_ip + 1];
                    let rc = run_engine(impl_id, slice, 1, vm);
                    if rc != 0 {
                        p.ip = saved_ip + 1;
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
                // C code: int tmp[m]; then:
                //   for (i=m-1;i>=0;i--) iv_pop(&vm->stack, &tmp[i]);
                //   for (i=m-1;i>=0;i--) iv_pop(&vm->stack, &tmp[i]);
                // (Note the duplicate loop - bug-for-bug reproduction.)
                let mlen = m as usize;
                let mut tmp: Vec<i32> = vec![0i32; mlen];
                if mlen > 0 {
                    for i in (0..mlen).rev() {
                        if let Some(v) = vm.stack.pop() {
                            tmp[i] = v;
                        }
                        // If pop fails, tmp[i] stays at the previously written
                        // value (0 on the first pass; the value popped earlier
                        // on the second pass). The C code doesn't check the
                        // return value either.
                    }
                    for i in (0..mlen).rev() {
                        if let Some(v) = vm.stack.pop() {
                            tmp[i] = v;
                        }
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
