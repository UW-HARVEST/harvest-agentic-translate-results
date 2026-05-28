// Rust crate library root.
// Mirrors the public C API exposed by c_src/, exporting every C-visible symbol
// from the corresponding `.so`. Internal helpers live in their own modules.

pub mod a;
pub mod b;
pub mod engine;
pub mod lib_target;
pub mod util;

use core::ffi::{c_char, c_int};

use crate::util::{
    iv_free_impl, iv_init_impl, iv_peek_impl, iv_pop_impl, iv_push_impl, iv_reserve_impl,
    prog_fetch_impl, prog_init_impl, vm_free_impl, vm_init_impl, vm_trace_impl, IntVec, Program,
    VM,
};

// ---------- IntVec ----------

#[no_mangle]
pub unsafe extern "C" fn iv_init(v: *mut IntVec) {
    iv_init_impl(v);
}

#[no_mangle]
pub unsafe extern "C" fn iv_free(v: *mut IntVec) {
    iv_free_impl(v);
}

#[no_mangle]
pub unsafe extern "C" fn iv_reserve(v: *mut IntVec, need: usize) -> bool {
    iv_reserve_impl(v, need)
}

#[no_mangle]
pub unsafe extern "C" fn iv_push(v: *mut IntVec, x: c_int) -> bool {
    iv_push_impl(v, x)
}

#[no_mangle]
pub unsafe extern "C" fn iv_pop(v: *mut IntVec, out: *mut c_int) -> bool {
    iv_pop_impl(v, out)
}

#[no_mangle]
pub unsafe extern "C" fn iv_peek(v: *const IntVec, def: c_int) -> c_int {
    iv_peek_impl(v, def)
}

// ---------- Program ----------

#[no_mangle]
pub unsafe extern "C" fn prog_init(p: *mut Program, code: *const c_int, n: usize) {
    prog_init_impl(p, code, n);
}

#[no_mangle]
pub unsafe extern "C" fn prog_fetch(p: *mut Program, out: *mut c_int) -> bool {
    prog_fetch_impl(p, out)
}

// ---------- VM ----------

#[no_mangle]
pub unsafe extern "C" fn vm_init(vm: *mut VM) {
    vm_init_impl(vm);
}

#[no_mangle]
pub unsafe extern "C" fn vm_free(vm: *mut VM) {
    vm_free_impl(vm);
}

#[no_mangle]
pub unsafe extern "C" fn vm_trace(vm: *mut VM, t: c_int) {
    vm_trace_impl(vm, t);
}

// vm_print writes to a FILE*; we shell out to fprintf/fputc so the output goes
// to the same stream the C binary would use.
#[no_mangle]
pub unsafe extern "C" fn vm_print(fp: *mut libc::FILE, label: *const c_char, vm: *const VM) {
    // "%sSTACK_TOP=%d STEPS=%d TRACE="
    let fmt = b"%sSTACK_TOP=%d STEPS=%d TRACE=\0".as_ptr() as *const c_char;
    let top = iv_peek_impl(&(*vm).stack as *const _, -777);
    libc::fprintf(fp, fmt, label, top, (*vm).steps);
    let alphabet = b"abcdefghijklmnopqrstuvwxyz";
    let trace_len = (*vm).trace.len;
    for i in 0..trace_len {
        let t = *(*vm).trace.data.add(i);
        let idx = ((t & 25) as u32 as usize) % 26;
        libc::fputc(alphabet[idx] as c_int, fp);
    }
    libc::fputc(b'\n' as c_int, fp);
}

// ---------- run_engine ----------

#[no_mangle]
pub unsafe extern "C" fn run_engine(
    impl_id: c_int,
    code: *const c_int,
    n: usize,
    vm: *mut VM,
) -> c_int {
    let slice = if code.is_null() || n == 0 {
        &[][..]
    } else {
        core::slice::from_raw_parts(code, n)
    };
    let vm_ref = &mut *vm;
    crate::engine::run_engine(impl_id, slice, vm_ref)
}

// ---------- a.c symbols ----------

#[no_mangle]
pub unsafe extern "C" fn call_a_once(x: c_int) -> c_int {
    crate::a::call_a_once(x)
}

#[no_mangle]
pub unsafe extern "C" fn process_a_stream(xs: *const c_int, n: usize) -> c_int {
    let slice = if xs.is_null() || n == 0 {
        &[][..]
    } else {
        core::slice::from_raw_parts(xs, n)
    };
    crate::a::process_a_stream(slice)
}

// ---------- b.c symbols ----------

#[no_mangle]
pub unsafe extern "C" fn call_b_once(x: c_int) -> c_int {
    crate::b::call_b_once(x)
}

#[no_mangle]
pub unsafe extern "C" fn process_b_stream(xs: *const c_int, n: usize) -> c_int {
    let slice = if xs.is_null() || n == 0 {
        &[][..]
    } else {
        core::slice::from_raw_parts(xs, n)
    };
    crate::b::process_b_stream(slice)
}

// ---------- lib.c (target) ----------

#[no_mangle]
pub unsafe extern "C" fn target(code: c_int) -> c_int {
    crate::lib_target::target(code)
}
