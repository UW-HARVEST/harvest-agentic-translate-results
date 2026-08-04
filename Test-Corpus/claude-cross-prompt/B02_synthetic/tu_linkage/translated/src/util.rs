// Translated from c_src/src/util.c
// Provides IntVec, Program, VM and helpers.

use std::ffi::{c_char, c_int, c_void};
use std::mem;
use std::ptr;

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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn iv_init(v: *mut IntVec) {
    unsafe {
        (*v).data = ptr::null_mut();
        (*v).len = 0;
        (*v).cap = 0;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn iv_free(v: *mut IntVec) {
    unsafe {
        let cap = (*v).cap;
        let data = (*v).data;
        if !data.is_null() && cap > 0 {
            // Mirror C's free of malloc/realloc'd memory using libc.
            libc::free(data as *mut c_void);
        } else if !data.is_null() {
            // Defensive: still call free for non-null
            libc::free(data as *mut c_void);
        }
        (*v).data = ptr::null_mut();
        (*v).len = 0;
        (*v).cap = 0;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn iv_reserve(v: *mut IntVec, need: usize) -> bool {
    unsafe {
        if need <= (*v).cap {
            return true;
        }
        let mut nc: usize = if (*v).cap != 0 { (*v).cap } else { 8 };
        while nc < need {
            if nc > (usize::MAX / 2) {
                return false;
            }
            nc = nc.wrapping_mul(2);
        }
        let new_size = match nc.checked_mul(mem::size_of::<c_int>()) {
            Some(s) => s,
            None => return false,
        };
        let p = libc::realloc((*v).data as *mut c_void, new_size) as *mut c_int;
        if p.is_null() {
            return false;
        }
        (*v).data = p;
        (*v).cap = nc;
        true
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn iv_push(v: *mut IntVec, x: c_int) -> bool {
    unsafe {
        if (*v).len == (*v).cap {
            let need = if (*v).cap != 0 { (*v).cap.wrapping_mul(2) } else { 8 };
            if !iv_reserve(v, need) {
                return false;
            }
        }
        let len = (*v).len;
        *(*v).data.add(len) = x;
        (*v).len = len + 1;
        true
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn iv_pop(v: *mut IntVec, out: *mut c_int) -> bool {
    unsafe {
        if (*v).len == 0 {
            return false;
        }
        if !out.is_null() {
            *out = *(*v).data.add((*v).len - 1);
        }
        (*v).len -= 1;
        true
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn iv_peek(v: *const IntVec, def: c_int) -> c_int {
    unsafe {
        if (*v).len != 0 {
            *(*v).data.add((*v).len - 1)
        } else {
            def
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn prog_init(p: *mut Program, code: *const c_int, n: usize) {
    unsafe {
        (*p).code = code;
        (*p).n = n;
        (*p).ip = 0;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn prog_fetch(p: *mut Program, out: *mut c_int) -> bool {
    unsafe {
        if (*p).ip >= (*p).n {
            return false;
        }
        *out = *(*p).code.add((*p).ip);
        (*p).ip += 1;
        true
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn vm_init(vm: *mut VM) {
    unsafe {
        iv_init(&mut (*vm).stack as *mut IntVec);
        iv_init(&mut (*vm).trace as *mut IntVec);
        (*vm).steps = 0;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn vm_free(vm: *mut VM) {
    unsafe {
        iv_free(&mut (*vm).stack as *mut IntVec);
        iv_free(&mut (*vm).trace as *mut IntVec);
        (*vm).steps = 0;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn vm_trace(vm: *mut VM, t: c_int) {
    unsafe {
        iv_push(&mut (*vm).trace as *mut IntVec, t);
    }
}

// FILE * is opaque libc::FILE in our binding.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vm_print(
    fp: *mut libc::FILE,
    label: *const c_char,
    vm: *const VM,
) {
    unsafe {
        let stack_top = iv_peek(&(*vm).stack as *const IntVec, -777);
        let steps = (*vm).steps;
        let fmt = c"%sSTACK_TOP=%d STEPS=%d TRACE=";
        libc::fprintf(fp, fmt.as_ptr(), label, stack_top as c_int, steps);

        let alphabet = b"abcdefghijklmnopqrstuvwxyz";
        let trace_len = (*vm).trace.len;
        let trace_data = (*vm).trace.data;
        for i in 0..trace_len {
            let t = *trace_data.add(i);
            let idx = (t & 25) as usize;
            // alphabet has 26 entries; (t & 25) is at most 25 (since 25 = 0b11001 mask)
            let ch = alphabet[idx] as c_int;
            libc::fputc(ch, fp);
        }
        libc::fputc(b'\n' as c_int, fp);
    }
}
