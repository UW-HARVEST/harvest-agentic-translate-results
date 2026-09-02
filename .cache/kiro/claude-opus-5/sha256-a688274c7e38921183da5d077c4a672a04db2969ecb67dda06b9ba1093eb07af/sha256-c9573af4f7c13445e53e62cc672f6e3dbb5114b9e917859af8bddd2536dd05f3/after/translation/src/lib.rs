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

    // Used only to learn the current thread's stack extent, so that the
    // variable-length array in `driver` can fail exactly like the C one.
    fn pthread_self() -> usize;
    fn pthread_getattr_np(th: usize, attr: *mut c_void) -> c_int;
    fn pthread_attr_getstack(
        attr: *const c_void,
        stackaddr: *mut *mut c_void,
        stacksize: *mut usize,
    ) -> c_int;
    fn pthread_attr_destroy(attr: *mut c_void) -> c_int;
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
        i = i.wrapping_add(1);
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
        i = i.wrapping_add(1);
    }
}

/// Number of bytes still available below `sp` on the current thread's stack,
/// or `usize::MAX` if it cannot be determined (in which case no VLA check is
/// performed, so no fault is ever invented).
fn stack_bytes_below(sp: usize) -> usize {
    // glibc's `pthread_attr_t` is 56 bytes on x86-64; over-allocate and keep
    // 8-byte alignment.
    let mut attr = [0u64; 16];
    let attr_ptr = attr.as_mut_ptr() as *mut c_void;
    unsafe {
        if pthread_getattr_np(pthread_self(), attr_ptr) != 0 {
            return usize::MAX;
        }
        let mut base: *mut c_void = core::ptr::null_mut();
        let mut size: usize = 0;
        let rc = pthread_attr_getstack(attr_ptr, &mut base, &mut size);
        pthread_attr_destroy(attr_ptr);
        if rc != 0 || base.is_null() || size == 0 {
            return usize::MAX;
        }
        let low = base as usize;
        if sp <= low { 0 } else { sp - low }
    }
}

/// Reproduce the memory behaviour of the C `int out[len]` variable-length array.
///
/// gcc lowers a VLA to a stack-pointer decrement, and the `memcpy` that follows
/// is the first access to that memory. When the VLA does not fit in the
/// remaining stack, the C process therefore dies with `SIGSEGV`. A Rust `Vec`
/// of the same size would instead report a heap allocation failure and abort
/// with `SIGABRT`, so the rejection would not match. Perform the same
/// out-of-stack access the C makes, which faults identically.
#[inline(never)]
fn vla_stack_probe(n_bytes: usize) {
    // Sizes this small cannot exhaust a stack; skip the check entirely so no
    // stack memory is touched on the ordinary paths.
    const MIN_INTERESTING: usize = 64 * 1024;
    if n_bytes < MIN_INTERESTING {
        return;
    }
    let anchor: usize = 0;
    let sp = &anchor as *const usize as usize;
    core::hint::black_box(&anchor);

    // `len * sizeof(int)` can exceed the address space (e.g. a negative `len`).
    // The C's stack-pointer arithmetic then wraps around and it faults inside
    // `memcpy` instead; leave that path to the `memcpy` below.
    if n_bytes > sp {
        return;
    }
    let available = stack_bytes_below(sp);
    if n_bytes <= available {
        return; // The VLA fits, exactly as it does in C.
    }
    // The VLA base lies below the stack mapping. This is the address the C
    // memcpy writes to first.
    unsafe { core::ptr::write_volatile((sp - n_bytes) as *mut u8, 0) };
}

/// void driver(const int *data, int len)
///
/// The C version declares a variable-length array `int out[len]` and copies
/// `len * sizeof(int)` bytes into it. The byte count is computed exactly as C
/// does (the `int` `len` is converted to `size_t`, i.e. sign-extended, before
/// being multiplied), so non-positive lengths behave as they do in the original.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(data: *const c_int, len: c_int) {
    let n_bytes = (len as isize as usize).wrapping_mul(core::mem::size_of::<c_int>());

    // `int out[len];` — match the stack-exhaustion behaviour of the VLA before
    // allocating anything.
    vla_stack_probe(n_bytes);

    let elems = if len > 0 { len as usize } else { 0 };
    let mut out: Vec<c_int> = vec![0; elems];

    unsafe {
        memcpy(
            out.as_mut_ptr() as *mut c_void,
            data as *const c_void,
            n_bytes,
        );
    }

    inner(out.as_mut_ptr(), len);
}
