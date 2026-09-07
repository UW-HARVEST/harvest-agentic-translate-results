// Rust translation of c_src/src/driver.c (MIT Lincoln Laboratory, 2025).
//
// Public ABI reproduced (as exported by the C shared library):
//   void driver(const int *data, int len);
//   void fma_array(int *out, const int *mul1, const int *mul2, const int *add, int len);
//
// `inner` is `static` in the C source and therefore has no external linkage;
// it is translated as a private Rust function.

use core::ffi::{c_char, c_int, c_void};

// Use the platform C library directly so that stdout buffering, formatting and
// byte-level output are identical to the original C code.
unsafe extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
    fn memcpy(dest: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
}

/// `%d\n` format string used by the C `printf` call in `inner`.
static FMT_D_NL: [c_char; 4] = [b'%' as c_char, b'd' as c_char, b'\n' as c_char, 0];

/// void fma_array(int *out, const int *mul1, const int *mul2, const int *add, int len)
///
/// out[i] = mul1[i] * mul2[i] + add[i] for i in [0, len).
/// Signed overflow is UB in C; the generated code wraps, so `wrapping_*` is used
/// here to reproduce the same values without panicking.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fma_array(
    out: *mut c_int,
    mul1: *const c_int,
    mul2: *const c_int,
    add: *const c_int,
    len: c_int,
) {
    let mut i: c_int = 0;
    while i < len {
        let idx = i as isize;
        unsafe {
            let m1 = *mul1.offset(idx);
            let m2 = *mul2.offset(idx);
            let a = *add.offset(idx);
            *out.offset(idx) = m1.wrapping_mul(m2).wrapping_add(a);
        }
        i += 1;
    }
}

/// static void inner(int *out, int len)
fn inner(out: *mut c_int, len: c_int) {
    unsafe {
        fma_array(out, out, out, out, len);
    }
    let mut i: c_int = 0;
    while i < len {
        unsafe {
            printf(FMT_D_NL.as_ptr(), *out.offset(i as isize));
        }
        i += 1;
    }
}

/// void driver(const int *data, int len)
///
/// The C version declares a variable-length array `int out[len]` (an
/// UNINITIALIZED stack VLA) and copies `len * sizeof(int)` bytes into it.
///
/// Fidelity note on the backing storage: a `Vec`/`vec![0; elems]` is the wrong
/// model here. For a huge `len` (e.g. `INT_MAX`, ~8 GiB) the C code moves the
/// stack pointer past the stack guard and dies with SIGSEGV, whereas asking the
/// Rust global allocator for ~8 GiB fails and routes through
/// `handle_alloc_error`, which `abort()`s -> SIGABRT. To reproduce the C's
/// SIGSEGV we back the buffer with a RAW `std::alloc::alloc` (the uninitialized
/// variant, matching the uninitialized VLA) and, crucially, on allocation
/// failure we DO NOT call `handle_alloc_error`, panic, or abort: we carry on
/// with a null pointer so the subsequent `memcpy` faults with SIGSEGV, exactly
/// as the C stack-clash does.
///
/// The byte count is computed exactly as C does (the `int` `len` is converted to
/// `size_t`, i.e. sign-extended, before being multiplied), so non-positive
/// lengths behave as they do in the original.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(data: *const c_int, len: c_int) {
    // Transliterate gcc's emitted VLA arithmetic exactly.
    //
    //   movslq/cltq ; lea (,rax,4)  -> size_bytes = (u64)(i64)len * 4, WRAPPING
    let size_bytes: u64 = (len as i64 as u64).wrapping_mul(core::mem::size_of::<c_int>() as u64);
    //   add 15 ; unsigned div 16 ; imul 16 -> frame = ((size_bytes + 15) / 16) * 16
    // The WRAPPING add is the whole point: for len in {-1,-2,-3} it wraps small
    // and frame becomes 0 (valid stack address); for len <= -4 it does not wrap
    // and frame is astronomically large (wild pointer).
    let frame: u64 = size_bytes.wrapping_add(15) / 16 * 16;

    // Choose the destination pointer.
    let mut layout: Option<core::alloc::Layout> = None;
    let mut stack_probe: c_int = 0;
    let out: *mut c_int = if len > 0 {
        // Normal VLA: real uninitialized heap storage (matching the
        // uninitialized VLA). On any failure carry on with a NULL pointer so the
        // memcpy faults with SIGSEGV, mirroring the C stack clash. Do NOT call
        // `handle_alloc_error`, panic, or abort. (Preserves the first fix.)
        match core::alloc::Layout::array::<c_int>(len as usize) {
            Ok(l) => {
                layout = Some(l);
                unsafe { std::alloc::alloc(l) as *mut c_int }
            }
            Err(_) => core::ptr::null_mut(),
        }
    } else {
        // Model `out = align_up_4(rsp - frame)`. Use the address of a local as
        // the stand-in for rsp. frame == 0 for len in {0,-1,-2,-3} yields a
        // valid stack address; the huge frame for len <= -4 yields a wild
        // pointer. No allocation, no layout recorded.
        let base = &mut stack_probe as *mut c_int as usize;
        let addr = base.wrapping_sub(frame as usize).wrapping_add(3) >> 2 << 2;
        addr as *mut c_int
    };

    // Route the destination through `black_box` so LLVM cannot prove the
    // huge-count memcpy is UB and delete it (the actual bug being fixed).
    let dst = core::hint::black_box(out);
    // The count is `size_bytes` (the value C recomputes), NOT the rounded frame.
    unsafe {
        memcpy(dst as *mut c_void, data as *const c_void, size_bytes as usize);
    }

    inner(dst, len);

    // Free only in the len > 0 case, only when a layout was constructed and the
    // pointer is non-null. Control flow never reaches here in the crashing
    // cases, which matches C.
    if len > 0 {
        if let Some(l) = layout {
            if !dst.is_null() {
                unsafe {
                    std::alloc::dealloc(dst as *mut u8, l);
                }
            }
        }
    }
}
