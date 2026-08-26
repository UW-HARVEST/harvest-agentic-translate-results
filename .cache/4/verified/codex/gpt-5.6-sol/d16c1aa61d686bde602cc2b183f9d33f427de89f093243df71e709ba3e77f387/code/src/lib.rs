use std::ffi::{c_char, c_int, c_void};
use std::mem::size_of;
use std::ptr;

#[cfg(not(test))]
use std::ffi::c_long;

#[repr(C)]
pub struct IntVec {
    pub data: *mut c_int,
    pub len: usize,
    pub cap: usize,
}

#[repr(C)]
pub struct Program {
    pub code: *const c_int,
    pub n: usize,
    pub ip: usize,
}

#[repr(C)]
pub struct VM {
    pub stack: IntVec,
    pub trace: IntVec,
    pub steps: c_int,
}

unsafe extern "C" {
    fn realloc(ptr: *mut c_void, size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
    fn fprintf(stream: *mut c_void, format: *const c_char, ...) -> c_int;
    fn fputc(character: c_int, stream: *mut c_void) -> c_int;
    #[cfg(not(test))]
    fn fgets(buffer: *mut c_char, size: c_int, stream: *mut c_void) -> *mut c_char;
    #[cfg(not(test))]
    fn strcmp(left: *const c_char, right: *const c_char) -> c_int;
    #[cfg(not(test))]
    fn strtol(value: *const c_char, end: *mut *mut c_char, base: c_int) -> c_long;
    #[cfg(not(test))]
    static mut stdin: *mut c_void;
    #[cfg(not(test))]
    static mut stdout: *mut c_void;
    #[cfg(not(test))]
    static mut stderr: *mut c_void;
}

static mut STATE_A: c_int = 0;
static mut FLIPFLOP: c_int = 0;

unsafe fn segv_if_null<T>(pointer: *const T) {
    if pointer.is_null() {
        fputc(0, ptr::null_mut());
    }
}

unsafe fn target_a(code: c_int) -> c_int {
    if code < 0 {
        return if STATE_A & 1 != 0 { 6 } else { 5 };
    }
    STATE_A ^= code.wrapping_shl(1);
    match ((code >> 2) ^ STATE_A) & 7 {
        0 => 0,
        1 => 2,
        2 => 4,
        3 => 1,
        4 => 3,
        5 | 6 => 5,
        _ => 7,
    }
}

unsafe fn target_b(code: c_int) -> c_int {
    FLIPFLOP ^= 1;
    if code < 0 {
        return if FLIPFLOP != 0 { 2 } else { 6 };
    }
    let z = (code ^ if FLIPFLOP != 0 { 0x7f } else { 0x1f }) % 8;
    match z {
        0 | 7 => 4,
        1 | 2 => 3,
        3 => 1,
        4 => 0,
        5 => 5,
        _ => 7,
    }
}

unsafe fn classify(impl_id: c_int, x: c_int) -> c_int {
    if impl_id == 0 {
        call_a_once(x)
    } else if impl_id == 1 {
        call_b_once(x.wrapping_add(1))
    } else {
        target(target(x.wrapping_add(1)))
    }
}

unsafe fn process_stream(impl_id: c_int, buf: *const c_int, n: usize) -> c_int {
    if impl_id == 0 {
        return process_a_stream(buf, n);
    }
    if impl_id == 1 {
        return process_b_stream(buf, n);
    }

    let mut acc = 0i32;
    for i in 0..n {
        let t = target(*buf.add(i));
        if t & 1 == 0 {
            acc = acc.wrapping_add(t.wrapping_mul(2));
        } else {
            acc ^= t.wrapping_add(7);
        }
    }
    acc
}

#[unsafe(no_mangle)]
pub extern "C" fn target(code: c_int) -> c_int {
    if code < 0 {
        return 7;
    }
    match code % 10 {
        0 => 0,
        1..=3 => 1,
        4..=6 => 2,
        7 => 3,
        _ => 4,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn call_a_once(x: c_int) -> c_int {
    let a = target_a(x);
    let b = target_a(a.wrapping_sub(5));
    let c = target_a(b ^ 3);
    let d = target_a((b ^ 0x55).wrapping_add(7));
    a ^ b.wrapping_shl(1) ^ c.wrapping_shl(2) ^ d.wrapping_shl(3)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn process_a_stream(xs: *const c_int, n: usize) -> c_int {
    if n != 0 {
        segv_if_null(xs);
    }
    let mut acc = 0usize;
    for i in 0..n {
        let value = *xs.add(i);
        for j in 0..3 {
            let t = target_a(value.wrapping_add(j));
            if t & 1 == 0 {
                acc = acc.wrapping_add(t as usize);
                continue;
            }
            acc ^= t.wrapping_shl(j as u32) as usize;
            if t == 5 {
                break;
            }
        }
    }
    if acc > 0x7fff_ffff {
        acc = 0x7fff_ffff;
    }
    #[cfg(target_pointer_width = "64")]
    if acc < (-0x8000_0000i64) as usize {
        acc = (-0x8000_0000i64) as usize;
    }
    acc as c_int
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn call_b_once(x: c_int) -> c_int {
    let a = target_b(x);
    let b = target_b(a.wrapping_add(9));
    let c = target_b((a.wrapping_add(9) ^ 0x2222).wrapping_sub(17));
    let d = target_b(c ^ x);
    a.wrapping_shl(1) ^ b.wrapping_shl(2) ^ c.wrapping_shl(3) ^ d.wrapping_shl(4)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn process_b_stream(xs: *const c_int, n: usize) -> c_int {
    if n != 0 {
        segv_if_null(xs);
    }
    let mut acc = 1i32;
    for i in 0..n {
        let value = *xs.add(i);
        let mut iter = 0i32;
        loop {
            iter = iter.wrapping_add(1);
            if iter > 4 {
                break;
            }
            let t = target_b(value.wrapping_sub(iter));
            if t == 6 {
                acc = acc.wrapping_sub(t);
                break;
            }
            if t == 3 {
                continue;
            }
            acc = acc.wrapping_mul(3) ^ t;
        }
    }
    acc
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn iv_init(v: *mut IntVec) {
    segv_if_null(v);
    (*v).data = ptr::null_mut();
    (*v).len = 0;
    (*v).cap = 0;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn iv_free(v: *mut IntVec) {
    segv_if_null(v);
    free((*v).data.cast());
    (*v).data = ptr::null_mut();
    (*v).len = 0;
    (*v).cap = 0;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn iv_reserve(v: *mut IntVec, need: usize) -> bool {
    segv_if_null(v);
    if need <= (*v).cap {
        return true;
    }
    let mut new_cap = if (*v).cap != 0 { (*v).cap } else { 8 };
    while new_cap < need {
        if new_cap > usize::MAX / 2 {
            return false;
        }
        new_cap = new_cap.wrapping_mul(2);
    }
    let allocation =
        realloc((*v).data.cast(), new_cap.wrapping_mul(size_of::<c_int>())).cast::<c_int>();
    if allocation.is_null() {
        return false;
    }
    (*v).data = allocation;
    (*v).cap = new_cap;
    true
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn iv_push(v: *mut IntVec, x: c_int) -> bool {
    segv_if_null(v);
    if (*v).len == (*v).cap {
        let need = if (*v).cap != 0 {
            (*v).cap.wrapping_mul(2)
        } else {
            8
        };
        if !iv_reserve(v, need) {
            return false;
        }
    }
    *(*v).data.add((*v).len) = x;
    (*v).len += 1;
    true
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn iv_pop(v: *mut IntVec, out: *mut c_int) -> bool {
    segv_if_null(v);
    if (*v).len == 0 {
        return false;
    }
    if !out.is_null() {
        *out = *(*v).data.add((*v).len - 1);
    }
    (*v).len -= 1;
    true
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn iv_peek(v: *const IntVec, default_value: c_int) -> c_int {
    segv_if_null(v);
    if (*v).len != 0 {
        *(*v).data.add((*v).len - 1)
    } else {
        default_value
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn prog_init(p: *mut Program, code: *const c_int, n: usize) {
    segv_if_null(p);
    (*p).code = code;
    (*p).n = n;
    (*p).ip = 0;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn prog_fetch(p: *mut Program, out: *mut c_int) -> bool {
    segv_if_null(p);
    if (*p).ip >= (*p).n {
        return false;
    }
    segv_if_null((*p).code);
    segv_if_null(out);
    *out = *(*p).code.add((*p).ip);
    (*p).ip += 1;
    true
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn vm_init(vm: *mut VM) {
    segv_if_null(vm);
    iv_init(ptr::addr_of_mut!((*vm).stack));
    iv_init(ptr::addr_of_mut!((*vm).trace));
    (*vm).steps = 0;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn vm_free(vm: *mut VM) {
    segv_if_null(vm);
    iv_free(ptr::addr_of_mut!((*vm).stack));
    iv_free(ptr::addr_of_mut!((*vm).trace));
    (*vm).steps = 0;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn vm_trace(vm: *mut VM, value: c_int) {
    segv_if_null(vm);
    iv_push(ptr::addr_of_mut!((*vm).trace), value);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn vm_print(fp: *mut c_void, label: *const c_char, vm: *const VM) {
    segv_if_null(vm);
    fprintf(
        fp,
        c"%sSTACK_TOP=%d STEPS=%d TRACE=".as_ptr(),
        label,
        iv_peek(ptr::addr_of!((*vm).stack), -777),
        (*vm).steps,
    );
    for i in 0..(*vm).trace.len {
        let entry = *(*vm).trace.data.add(i);
        fputc(
            b"abcdefghijklmnopqrstuvwxyz"[(entry & 25) as usize] as c_int,
            fp,
        );
    }
    fputc(b'\n' as c_int, fp);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn run_engine(
    impl_id: c_int,
    code: *const c_int,
    n: usize,
    vm: *mut VM,
) -> c_int {
    let mut program = Program { code, n, ip: 0 };
    let mut op = 0;
    while prog_fetch(&mut program, &mut op) {
        segv_if_null(vm);
        (*vm).steps = (*vm).steps.wrapping_add(1);
        match op {
            0 => {
                let mut immediate = 0;
                if !prog_fetch(&mut program, &mut immediate) {
                    return 1;
                }
                iv_push(ptr::addr_of_mut!((*vm).stack), immediate);
                vm_trace(vm, 0);
            }
            1 => {
                let mut a = 0;
                let mut b = 0;
                if !iv_pop(ptr::addr_of_mut!((*vm).stack), &mut b)
                    || !iv_pop(ptr::addr_of_mut!((*vm).stack), &mut a)
                {
                    return 2;
                }
                iv_push(ptr::addr_of_mut!((*vm).stack), a.wrapping_add(b));
                vm_trace(vm, 1);
            }
            2 => {
                let mut a = 0;
                let mut b = 0;
                if !iv_pop(ptr::addr_of_mut!((*vm).stack), &mut b)
                    || !iv_pop(ptr::addr_of_mut!((*vm).stack), &mut a)
                {
                    return 3;
                }
                iv_push(ptr::addr_of_mut!((*vm).stack), a.wrapping_mul(b));
                vm_trace(vm, 2);
            }
            3 => {
                let value = iv_peek(ptr::addr_of!((*vm).stack), 0);
                iv_push(ptr::addr_of_mut!((*vm).stack), value);
                vm_trace(vm, 3);
            }
            4 => {
                let mut value = 0;
                if !iv_pop(ptr::addr_of_mut!((*vm).stack), &mut value) {
                    return 4;
                }
                vm_trace(vm, 4);
            }
            5 => {
                let x = iv_peek(ptr::addr_of!((*vm).stack), 0);
                let bucket = classify(impl_id, x);
                iv_push(ptr::addr_of_mut!((*vm).stack), bucket);
                vm_trace(
                    vm,
                    match bucket {
                        0 => 5,
                        1 => 6,
                        2 => 7,
                        3 | 4 => 8,
                        _ => 9,
                    },
                );
            }
            6 => {
                let mut k = 0;
                if !prog_fetch(&mut program, &mut k) {
                    return 5;
                }
                let mut condition = 0;
                if !iv_pop(ptr::addr_of_mut!((*vm).stack), &mut condition) {
                    return 6;
                }
                if condition != 0 {
                    if (k as usize) > program.n - program.ip {
                        return 7;
                    }
                    program.ip += k as usize;
                    vm_trace(vm, 10);
                } else {
                    vm_trace(vm, 11);
                }
            }
            7 => {
                let mut times = 0;
                if !prog_fetch(&mut program, &mut times) {
                    return 8;
                }
                if program.ip >= program.n {
                    return 9;
                }
                let saved_ip = program.ip;
                for _ in 0..times {
                    let rc = run_engine(impl_id, program.code.add(saved_ip), 1, vm);
                    if rc != 0 {
                        program.ip = saved_ip + 1;
                        vm_trace(vm, 12);
                        break;
                    }
                }
                program.ip = saved_ip + 1;
            }
            8 => {
                let x = iv_peek(ptr::addr_of!((*vm).stack), 0);
                let y = classify(impl_id, x);
                iv_push(ptr::addr_of_mut!((*vm).stack), y);
                vm_trace(vm, 13);
            }
            9 => {
                let mut m = 0;
                if !prog_fetch(&mut program, &mut m) {
                    return 10;
                }
                if m < 0 || (m as usize) > (*vm).stack.len {
                    return 11;
                }
                let m = m as usize;
                let mut temporary = vec![0i32; m];
                for i in (0..m).rev() {
                    iv_pop(
                        ptr::addr_of_mut!((*vm).stack),
                        ptr::addr_of_mut!(temporary[i]),
                    );
                }
                for i in (0..m).rev() {
                    iv_pop(
                        ptr::addr_of_mut!((*vm).stack),
                        ptr::addr_of_mut!(temporary[i]),
                    );
                }
                let result = process_stream(impl_id, temporary.as_ptr(), m);
                iv_push(ptr::addr_of_mut!((*vm).stack), result);
                vm_trace(vm, 14);
            }
            10 => return 0,
            _ => return 99,
        }
    }
    0
}

#[cfg(not(test))]
unsafe fn usage(program: *const c_char) {
    fprintf(
        stderr,
        c"Usage: %s [--stdin] [bytecodes...]\nBytecodes are integers forming a small VM program.\n"
            .as_ptr(),
        program,
    );
}

#[cfg(not(test))]
unsafe fn read_stdin(code: *mut IntVec) -> usize {
    let mut buffer = [0 as c_char; 4096];
    let mut count = 0usize;
    while !fgets(buffer.as_mut_ptr(), buffer.len() as c_int, stdin).is_null() {
        let mut p = buffer.as_mut_ptr();
        while *p != 0 {
            let mut q = p;
            while *q != 0
                && *q != b' ' as c_char
                && *q != b'\t' as c_char
                && *q != b'\n' as c_char
                && *q != b'\r' as c_char
            {
                q = q.add(1);
            }
            let saved = *q;
            *q = 0;
            if *p != 0 {
                let mut end = ptr::null_mut();
                let value = strtol(p, &mut end, 10);
                if !end.is_null() && *end == 0 {
                    iv_push(code, value as c_int);
                    count += 1;
                }
            }
            *q = saved;
            p = if *q != 0 { q.add(1) } else { q };
        }
    }
    count
}

#[cfg(not(test))]
#[unsafe(export_name = "main")]
pub unsafe extern "C" fn driver_main(argc: c_int, argv: *mut *mut c_char) -> c_int {
    let mut use_stdin = false;
    let mut code = IntVec {
        data: ptr::null_mut(),
        len: 0,
        cap: 0,
    };
    iv_init(&mut code);

    for i in 1..argc {
        let argument = *argv.add(i as usize);
        if strcmp(argument, c"--help".as_ptr()) == 0 {
            usage(*argv);
            iv_free(&mut code);
            return 0;
        } else if strcmp(argument, c"--stdin".as_ptr()) == 0 {
            use_stdin = true;
        } else {
            let mut end = ptr::null_mut();
            let value = strtol(argument, &mut end, 10);
            if !end.is_null() && *end == 0 {
                iv_push(&mut code, value as c_int);
            } else {
                fprintf(stderr, c"skip '%s'\n".as_ptr(), argument);
            }
        }
    }
    if use_stdin {
        read_stdin(&mut code);
    }
    if code.len == 0 {
        fprintf(stderr, c"no program\n".as_ptr());
        iv_free(&mut code);
        return 2;
    }

    let mut vm_a = VM {
        stack: IntVec {
            data: ptr::null_mut(),
            len: 0,
            cap: 0,
        },
        trace: IntVec {
            data: ptr::null_mut(),
            len: 0,
            cap: 0,
        },
        steps: 0,
    };
    let mut vm_b = VM {
        stack: IntVec {
            data: ptr::null_mut(),
            len: 0,
            cap: 0,
        },
        trace: IntVec {
            data: ptr::null_mut(),
            len: 0,
            cap: 0,
        },
        steps: 0,
    };
    let mut vm_external = VM {
        stack: IntVec {
            data: ptr::null_mut(),
            len: 0,
            cap: 0,
        },
        trace: IntVec {
            data: ptr::null_mut(),
            len: 0,
            cap: 0,
        },
        steps: 0,
    };
    vm_init(&mut vm_a);
    vm_init(&mut vm_b);
    vm_init(&mut vm_external);

    let result_a = run_engine(0, code.data, code.len, &mut vm_a);
    let result_b = run_engine(1, code.data, code.len, &mut vm_b);
    let result_external = run_engine(2, code.data, code.len, &mut vm_external);
    fprintf(
        stdout,
        c"RC:A=%d B=%d EXT=%d\n".as_ptr(),
        result_a,
        result_b,
        result_external,
    );
    vm_print(stdout, c"A:".as_ptr(), &vm_a);
    vm_print(stdout, c"B:".as_ptr(), &vm_b);
    vm_print(stdout, c"EXT:".as_ptr(), &vm_external);

    vm_free(&mut vm_a);
    vm_free(&mut vm_b);
    vm_free(&mut vm_external);
    iv_free(&mut code);
    0
}
