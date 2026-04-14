use crate::a::{call_a_once, process_a_stream};
use crate::api::target;
use crate::b::{call_b_once, process_b_stream};
use crate::util::{iv_peek, iv_pop, iv_push, prog_fetch, prog_init, vm_trace, Program, VM};

fn inline_call(f: fn(i32) -> i32, x: i32) -> i32 {
    f(x)
}

fn classify(impl_id: i32, x: i32) -> i32 {
    if impl_id == 0 {
        return inline_call(call_a_once, x);
    }
    if impl_id == 1 {
        return call_b_once(x + 1);
    }
    inline_call(target, target(x + 1))
}

fn process_stream(impl_id: i32, buf: &[i32]) -> i32 {
    if impl_id == 0 {
        return process_a_stream(buf);
    }
    if impl_id == 1 {
        return process_b_stream(buf);
    }
    let mut acc = 0i32;
    for &item in buf {
        let t = target(item);
        if (t & 1) == 0 {
            acc += t * 2;
        } else {
            acc ^= t + 7;
        }
    }
    acc
}

pub fn run_engine(impl_id: i32, code: &[i32], vm: &mut VM) -> i32 {
    let mut p = Program { code: &[], n: 0, ip: 0 };
    prog_init(&mut p, code);
    let mut op = 0i32;
    while prog_fetch(&mut p, &mut op) {
        vm.steps += 1;
        match op {
            0 => {
                let mut imm = 0;
                if !prog_fetch(&mut p, &mut imm) {
                    return 1;
                }
                let _ = iv_push(&mut vm.stack, imm);
                vm_trace(vm, 0);
            }
            1 => {
                let mut a = 0;
                let mut b = 0;
                if !iv_pop(&mut vm.stack, Some(&mut b)) || !iv_pop(&mut vm.stack, Some(&mut a)) {
                    return 2;
                }
                let _ = iv_push(&mut vm.stack, a + b);
                vm_trace(vm, 1);
            }
            2 => {
                let mut a = 0;
                let mut b = 0;
                if !iv_pop(&mut vm.stack, Some(&mut b)) || !iv_pop(&mut vm.stack, Some(&mut a)) {
                    return 3;
                }
                let _ = iv_push(&mut vm.stack, a * b);
                vm_trace(vm, 2);
            }
            3 => {
                let a = iv_peek(&vm.stack, 0);
                let _ = iv_push(&mut vm.stack, a);
                vm_trace(vm, 3);
            }
            4 => {
                let mut tmp = 0;
                if !iv_pop(&mut vm.stack, Some(&mut tmp)) {
                    return 4;
                }
                vm_trace(vm, 4);
            }
            5 => {
                let x = iv_peek(&vm.stack, 0);
                let bucket = classify(impl_id, x);
                let _ = iv_push(&mut vm.stack, bucket);
                match bucket {
                    0 => vm_trace(vm, 5),
                    1 => vm_trace(vm, 6),
                    2 => vm_trace(vm, 7),
                    3 | 4 => vm_trace(vm, 8),
                    _ => vm_trace(vm, 9),
                }
            }
            6 => {
                let mut k = 0;
                if !prog_fetch(&mut p, &mut k) {
                    return 5;
                }
                let mut cond = 0;
                if !iv_pop(&mut vm.stack, Some(&mut cond)) {
                    return 6;
                }
                if cond != 0 {
                    if k < 0 || (k as usize) > p.n - p.ip {
                        return 7;
                    }
                    p.ip += k as usize;
                    vm_trace(vm, 10);
                } else {
                    vm_trace(vm, 11);
                }
            }
            7 => {
                let mut times = 0;
                if !prog_fetch(&mut p, &mut times) {
                    return 8;
                }
                if p.ip >= p.n {
                    return 9;
                }
                let saved_ip = p.ip;
                for _ in 0..times {
                    let inner_code = &p.code[saved_ip..saved_ip + 1];
                    let rc = run_engine(impl_id, inner_code, vm);
                    if rc != 0 {
                        p.ip = saved_ip + 1;
                        vm_trace(vm, 12);
                        break;
                    }
                }
                p.ip = saved_ip + 1;
            }
            8 => {
                let x = iv_peek(&vm.stack, 0);
                let y = classify(impl_id, x);
                let _ = iv_push(&mut vm.stack, y);
                vm_trace(vm, 13);
            }
            9 => {
                let mut m = 0;
                if !prog_fetch(&mut p, &mut m) {
                    return 10;
                }
                if m < 0 || (m as usize) > vm.stack.len {
                    return 11;
                }
                let mu = m as usize;
                let mut tmp = vec![0i32; mu];
                for i in (0..mu).rev() {
                    let _ = iv_pop(&mut vm.stack, Some(&mut tmp[i]));
                }
                for i in (0..mu).rev() {
                    let _ = iv_pop(&mut vm.stack, Some(&mut tmp[i]));
                }
                let s = process_stream(impl_id, &tmp);
                let _ = iv_push(&mut vm.stack, s);
                vm_trace(vm, 14);
            }
            10 => return 0,
            _ => return 99,
        }
    }
    0
}
