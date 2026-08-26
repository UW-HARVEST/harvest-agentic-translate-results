use crate::util::{Program, VM};
use crate::api;
use crate::a::{call_a_once, process_a_stream};
use crate::b::{call_b_once, process_b_stream};

fn classify(impl_id: i32, x: i32) -> i32 {
    if impl_id == 0 {
        return call_a_once(x);
    }
    if impl_id == 1 {
        return call_b_once(x + 1);
    }
    api::target(api::target(x + 1))
}

fn process_stream(impl_id: i32, buf: &[i32]) -> i32 {
    if impl_id == 0 {
        return process_a_stream(buf);
    }
    if impl_id == 1 {
        return process_b_stream(buf);
    }
    let mut acc = 0i32;
    for &val in buf {
        let t = api::target(val);
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
        vm.steps += 1;
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
                let b = match vm.stack.pop() { Some(v) => v, None => return 2 };
                let a = match vm.stack.pop() { Some(v) => v, None => return 2 };
                vm.stack.push(a.wrapping_add(b));
                vm.trace(1);
            }
            2 => {
                let b = match vm.stack.pop() { Some(v) => v, None => return 3 };
                let a = match vm.stack.pop() { Some(v) => v, None => return 3 };
                vm.stack.push(a.wrapping_mul(b));
                vm.trace(2);
            }
            3 => {
                let a = vm.stack.last().copied().unwrap_or(0);
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
                let x = vm.stack.last().copied().unwrap_or(0);
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
                let k = match p.fetch() { Some(v) => v, None => return 5 };
                let cond = match vm.stack.pop() { Some(v) => v, None => return 6 };
                if cond != 0 {
                    if k < 0 || (k as usize) > p.code.len() - p.ip {
                        return 7;
                    }
                    p.ip += k as usize;
                    vm.trace(10);
                } else {
                    vm.trace(11);
                }
            }
            7 => {
                let times = match p.fetch() { Some(v) => v, None => return 8 };
                if p.ip >= p.code.len() {
                    return 9;
                }
                let saved_ip = p.ip;
                for _ in 0..times {
                    let inner_code = &p.code[saved_ip..saved_ip + 1];
                    let rc = run_engine(impl_id, inner_code, vm);
                    if rc != 0 {
                        p.ip = saved_ip + 1;
                        vm.trace(12);
                        break;
                    }
                }
                p.ip = saved_ip + 1;
            }
            8 => {
                let x = vm.stack.last().copied().unwrap_or(0);
                let y = classify(impl_id, x);
                vm.stack.push(y);
                vm.trace(13);
            }
            9 => {
                let m = match p.fetch() { Some(v) => v, None => return 10 };
                if m < 0 || (m as usize) > vm.stack.len() {
                    return 11;
                }
                let m_usize = m as usize;
                let mut tmp = vec![0; m_usize];
                for i in (0..m_usize).rev() {
                    if let Some(v) = vm.stack.pop() {
                        tmp[i] = v;
                    }
                }
                for i in (0..m_usize).rev() {
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
