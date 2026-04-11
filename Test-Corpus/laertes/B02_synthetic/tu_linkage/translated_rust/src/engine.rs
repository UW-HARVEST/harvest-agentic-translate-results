extern "C" {
    
    
    
    
    
    
    
    
    
    
    
}
pub use crate::src::a::call_a_once;
pub use crate::src::a::process_a_stream;
pub use crate::src::b::call_b_once;
pub use crate::src::b::process_b_stream;
pub use crate::src::lib::target;
pub use crate::src::util::iv_peek;
pub use crate::src::util::iv_pop;
pub use crate::src::util::iv_push;
pub use crate::src::util::prog_fetch;
pub use crate::src::util::prog_init;
pub use crate::src::util::vm_trace;
pub use crate::src::a::size_t;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct IntVec {
    pub data: *mut libc::c_int,
    pub len: size_t,
    pub cap: size_t,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct Program {
    pub code: *const libc::c_int,
    pub n: size_t,
    pub ip: size_t,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct VM {
    pub stack: IntVec,
    pub trace: IntVec,
    pub steps: libc::c_int,
}
#[inline]
unsafe extern "C" fn inline_call(
    mut f: Option<unsafe extern "C" fn(libc::c_int) -> libc::c_int>,
    mut x: libc::c_int,
) -> libc::c_int {
    return f.expect("non-null function pointer")(x);
}
unsafe extern "C" fn classify(
    mut impl_0: libc::c_int,
    mut x: libc::c_int,
) -> libc::c_int {
    if impl_0 == 0 as libc::c_int {
        return inline_call(
            Some(call_a_once as unsafe extern "C" fn(libc::c_int) -> libc::c_int),
            x,
        );
    }
    if impl_0 == 1 as libc::c_int {
        return call_b_once(x + 1 as libc::c_int);
    }
    return inline_call(
        Some(target as unsafe extern "C" fn(libc::c_int) -> libc::c_int),
        target(x + 1 as libc::c_int),
    );
}
unsafe extern "C" fn process_stream(
    mut impl_0: libc::c_int,
    mut buf: *const libc::c_int,
    mut n: size_t,
) -> libc::c_int {
    if impl_0 == 0 as libc::c_int {
        return process_a_stream(buf, n);
    }
    if impl_0 == 1 as libc::c_int {
        return process_b_stream(buf, n);
    }
    let mut acc: libc::c_int = 0 as libc::c_int;
    let mut i: size_t = 0 as size_t;
    while i < n {
        let mut t: libc::c_int = target(*buf.offset(i as isize));
        if t & 1 as libc::c_int == 0 as libc::c_int {
            acc += t * 2 as libc::c_int;
        } else {
            acc ^= t + 7 as libc::c_int;
        }
        i = i.wrapping_add(1);
    }
    return acc;
}
#[no_mangle]
pub unsafe extern "C" fn run_engine(
    mut impl_id: libc::c_int,
    mut code: *const libc::c_int,
    mut n: size_t,
    mut vm: *mut VM,
) -> libc::c_int {
    let mut p: Program = Program {
        code: std::ptr::null::<libc::c_int>(),
        n: 0,
        ip: 0,
    };
    prog_init(&raw mut p, code, n);
    let mut op: libc::c_int = 0;
    while prog_fetch(&raw mut p, &raw mut op) {
        (*vm).steps += 1;
        match op {
            0 => {
                let mut imm: libc::c_int = 0;
                if !prog_fetch(&raw mut p, &raw mut imm) {
                    return 1 as libc::c_int;
                }
                iv_push(&raw mut (*vm).stack, imm);
                vm_trace(vm, 0 as libc::c_int);
            }
            1 => {
                let mut a: libc::c_int = 0;
                let mut b: libc::c_int = 0;
                if !iv_pop(&raw mut (*vm).stack, &raw mut b)
                    || !iv_pop(&raw mut (*vm).stack, &raw mut a)
                {
                    return 2 as libc::c_int;
                }
                iv_push(&raw mut (*vm).stack, a + b);
                vm_trace(vm, 1 as libc::c_int);
            }
            2 => {
                let mut a_0: libc::c_int = 0;
                let mut b_0: libc::c_int = 0;
                if !iv_pop(&raw mut (*vm).stack, &raw mut b_0)
                    || !iv_pop(&raw mut (*vm).stack, &raw mut a_0)
                {
                    return 3 as libc::c_int;
                }
                iv_push(&raw mut (*vm).stack, a_0 * b_0);
                vm_trace(vm, 2 as libc::c_int);
            }
            3 => {
                let mut a_1: libc::c_int =
                    iv_peek(&raw mut (*vm).stack, 0 as libc::c_int);
                iv_push(&raw mut (*vm).stack, a_1);
                vm_trace(vm, 3 as libc::c_int);
            }
            4 => {
                let mut tmp: libc::c_int = 0;
                if !iv_pop(&raw mut (*vm).stack, &raw mut tmp) {
                    return 4 as libc::c_int;
                }
                vm_trace(vm, 4 as libc::c_int);
            }
            5 => {
                let mut x: libc::c_int =
                    iv_peek(&raw mut (*vm).stack, 0 as libc::c_int);
                let mut bucket: libc::c_int = classify(impl_id, x);
                iv_push(&raw mut (*vm).stack, bucket);
                match bucket {
                    0 => {
                        vm_trace(vm, 5 as libc::c_int);
                    }
                    1 => {
                        vm_trace(vm, 6 as libc::c_int);
                    }
                    2 => {
                        vm_trace(vm, 7 as libc::c_int);
                    }
                    3 | 4 => {
                        vm_trace(vm, 8 as libc::c_int);
                    }
                    _ => {
                        vm_trace(vm, 9 as libc::c_int);
                    }
                }
            }
            6 => {
                let mut k: libc::c_int = 0;
                if !prog_fetch(&raw mut p, &raw mut k) {
                    return 5 as libc::c_int;
                }
                let mut cond: libc::c_int = 0;
                if !iv_pop(&raw mut (*vm).stack, &raw mut cond) {
                    return 6 as libc::c_int;
                }
                if cond != 0 {
                    if k as size_t > p.n.wrapping_sub(p.ip) {
                        return 7 as libc::c_int;
                    }
                    p.ip = (p.ip as libc::c_ulong)
                        .wrapping_add(k as size_t as libc::c_ulong)
                        as size_t as size_t;
                    vm_trace(vm, 10 as libc::c_int);
                } else {
                    vm_trace(vm, 11 as libc::c_int);
                }
            }
            7 => {
                let mut times: libc::c_int = 0;
                if !prog_fetch(&raw mut p, &raw mut times) {
                    return 8 as libc::c_int;
                }
                if p.ip >= p.n {
                    return 9 as libc::c_int;
                }
                let mut saved_ip: size_t = p.ip;
                let mut i: libc::c_int = 0 as libc::c_int;
                while i < times {
                    let mut inner: Program = p;
                    inner.ip = saved_ip;
                    let mut rc: libc::c_int = run_engine(
                        impl_id,
                        inner.code.offset(inner.ip as isize),
                        1 as size_t,
                        vm,
                    );
                    if rc != 0 {
                        p.ip = saved_ip.wrapping_add(1 as size_t);
                        vm_trace(vm, 12 as libc::c_int);
                        break;
                    } else {
                        i += 1;
                    }
                }
                p.ip = saved_ip.wrapping_add(1 as size_t);
            }
            8 => {
                let mut x_0: libc::c_int =
                    iv_peek(&raw mut (*vm).stack, 0 as libc::c_int);
                let mut y: libc::c_int = classify(impl_id, x_0);
                iv_push(&raw mut (*vm).stack, y);
                vm_trace(vm, 13 as libc::c_int);
            }
            9 => {
                let mut m: libc::c_int = 0;
                if !prog_fetch(&raw mut p, &raw mut m) {
                    return 10 as libc::c_int;
                }
                if m < 0 as libc::c_int || m as size_t > (*vm).stack.len {
                    return 11 as libc::c_int;
                }
                let vla = m as usize;
                let mut tmp_0: Vec<libc::c_int> = ::std::vec::from_elem(0, vla);
                let mut i_0: libc::c_int = m - 1 as libc::c_int;
                while i_0 >= 0 as libc::c_int {
                    iv_pop(
                        &raw mut (*vm).stack,
                        tmp_0.as_mut_ptr().offset(i_0 as isize) as *mut libc::c_int,
                    );
                    i_0 -= 1;
                }
                let mut i_1: libc::c_int = m - 1 as libc::c_int;
                while i_1 >= 0 as libc::c_int {
                    iv_pop(
                        &raw mut (*vm).stack,
                        tmp_0.as_mut_ptr().offset(i_1 as isize) as *mut libc::c_int,
                    );
                    i_1 -= 1;
                }
                let mut s: libc::c_int =
                    process_stream(impl_id, tmp_0.as_mut_ptr(), m as size_t);
                iv_push(&raw mut (*vm).stack, s);
                vm_trace(vm, 14 as libc::c_int);
            }
            10 => return 0 as libc::c_int,
            _ => return 99 as libc::c_int,
        }
    }
    return 0 as libc::c_int;
}
