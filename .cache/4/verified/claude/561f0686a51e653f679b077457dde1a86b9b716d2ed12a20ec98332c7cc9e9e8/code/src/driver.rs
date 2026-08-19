// Rust translation of c_src/src/driver.c
//
// Public ABI (matches `nm -D` on the C shared library):
//   * fma_array
//   * driver
//
// `inner` is `static` in the C source and therefore is NOT exported here.

use core::ffi::{c_char, c_int, c_void};

unsafe extern "C" {
    /// C standard library `printf`, used so that stdout buffering behavior
    /// (and therefore the exact byte stream produced) matches the C library.
    fn printf(fmt: *const c_char, ...) -> c_int;

    /// C standard library `memcpy`, used so that `driver`'s copy has exactly
    /// the same semantics (including for degenerate lengths) as the C source.
    fn memcpy(dest: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;

    /// `mmap`/`munmap` provide `driver`'s stand-in for the C variable-length
    /// array. A heap (`malloc`) allocation must NOT be used for it: the C VLA
    /// lives on the stack, so allocating it from the same heap that the caller's
    /// `data` buffer lives in perturbs the bytes immediately after `data` and
    /// changes what an out-of-range `len` reads there. `mmap` leaves the malloc
    /// heap untouched, exactly like the C's stack allocation does.
    fn mmap(
        addr: *mut c_void,
        len: usize,
        prot: c_int,
        flags: c_int,
        fd: c_int,
        off: i64,
    ) -> *mut c_void;
    fn munmap(addr: *mut c_void, len: usize) -> c_int;
}

// <sys/mman.h> constants (linux/x86-64).
const PROT_READ: c_int = 0x1;
const PROT_WRITE: c_int = 0x2;
const MAP_PRIVATE: c_int = 0x02;
const MAP_ANONYMOUS: c_int = 0x20;
const MAP_FAILED: *mut c_void = usize::MAX as *mut c_void;

/// Format string used by the `printf("%d\n", ...)` call in the C source.
const FMT_D_NL: &[u8; 4] = b"%d\n\0";

/// ```c
/// void fma_array(int *out, const int *mul1, const int *mul2, const int *add, int len) {
///     for (int i = 0; i < len; i++) {
///         out[i] = mul1[i] * mul2[i] + add[i];
///     }
/// }
/// ```
///
/// The C callers pass fully aliasing pointers (`out == mul1 == mul2 == add`),
/// so raw pointer accesses are used here instead of slices.
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
        let a = unsafe { *mul1.offset(idx) };
        let b = unsafe { *mul2.offset(idx) };
        let c = unsafe { *add.offset(idx) };
        // C signed overflow is UB; reproduce the two's-complement wrapping
        // behavior emitted by the C compiler on the target hardware.
        let v = a.wrapping_mul(b).wrapping_add(c);
        unsafe { *out.offset(idx) = v };
        i += 1;
    }
}

/// ```c
/// static void inner(int *out, int len) {
///     fma_array(out, out, out, out, len);
///     for (int i = 0; i < len; i++) {
///         printf("%d\n", out[i]);
///     }
/// }
/// ```
unsafe fn inner(out: *mut c_int, len: c_int) {
    unsafe { fma_array(out, out, out, out, len) };
    let mut i: c_int = 0;
    while i < len {
        let v = unsafe { *out.offset(i as isize) };
        unsafe { printf(FMT_D_NL.as_ptr() as *const c_char, v) };
        i += 1;
    }
}

/// ```c
/// void driver(const int *data, int len) {
///     int out[len];
///     memcpy(out, data, len * sizeof(int));
///     inner(out, len);
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(data: *const c_int, len: c_int) {
    // `len * sizeof(int)` in C: `int` is converted to `size_t`, so a negative
    // length becomes an enormous unsigned byte count (the original UB is
    // reproduced rather than "fixed"). `black_box` keeps the optimizer from
    // reasoning about (and deleting) that out-of-range copy.
    let nbytes = core::hint::black_box((len as usize).wrapping_mul(core::mem::size_of::<c_int>()));

    // ---- `int out[len];` ---------------------------------------------------
    //
    // gcc lowers a variable-length array to a bare `rsp -= nbytes`: there is no
    // stack probe, no size validation and no failure path, so the VLA simply
    // *is* the `nbytes` bytes below the current stack pointer whether or not
    // that memory is usable. As a result the C library dies with `SIGSEGV` --
    // faulting on the VLA's *first* byte, which is where the `memcpy` below
    // starts writing -- as soon as `nbytes` exceeds the stack space left in the
    // running thread. That happens for oversized positive lengths (e.g.
    // `len == INT_MAX` asks for 8 GiB) and for every negative length (whose
    // wrapped byte count can never fit).
    //
    // Rust has no VLA. `vla_base` below is the address the C VLA's first byte
    // would live at, computed off a local so that it tracks the real stack
    // pointer. gcc reserves the size rounded *up* to a 16-byte multiple and then
    // rounds the resulting pointer up to a 4-byte boundary, i.e. exactly
    //
    //     rax = (int64)len * 4;  rax = (rax + 15) / 16 * 16;  rsp -= rax;
    //     out = ((rsp + 3) >> 2) << 2;
    //
    // (all unsigned, all wrapping - see the `driver` disassembly of the C `.so`).
    let vla_size = core::hint::black_box(nbytes.wrapping_add(15) / 16 * 16);
    let mut anchor: c_int = 0;
    let reserved = core::hint::black_box(&mut anchor as *mut c_int)
        .cast::<u8>()
        .wrapping_sub(vla_size) as usize;
    let vla_base: *mut u8 = core::hint::black_box((((reserved + 3) >> 2) << 2) as *mut u8);

    // Touching that byte reproduces the C VLA's *fault behaviour* exactly: where
    // the C would have got away with the allocation this is a harmless read of
    // stack scratch space, and where the C would not (an oversized `nbytes`, or
    // any negative `len` whose wrapped `nbytes` puts the VLA outside the stack)
    // it faults with the same signal at the same fault address, at the same point
    // in the call sequence -- before anything has been printed.
    core::hint::black_box(unsafe { core::ptr::read_volatile(vla_base as *const u8) });

    if len > 0 {
        // Stand-in for the C variable length array `int out[len]`.
        //
        // The storage cannot be placed below the stack pointer the way the C VLA
        // is, because the `printf` frames that `inner` creates would clobber it.
        // It also must not come from `malloc` (i.e. not a `Vec`): the C VLA lives
        // on the stack and therefore leaves the malloc heap alone, so taking the
        // buffer from the same heap the caller's `data` lives in would perturb
        // the bytes immediately after `data` and change what an out-of-range
        // `len` reads there. A fresh anonymous mapping satisfies both.
        //
        // The `read_volatile` probe above has already faulted for every `nbytes`
        // that does not fit in the remaining stack, so anything reaching this
        // point is at most a few megabytes and the mapping always succeeds.
        let map = unsafe {
            mmap(
                core::ptr::null_mut(),
                nbytes,
                PROT_READ | PROT_WRITE,
                MAP_PRIVATE | MAP_ANONYMOUS,
                -1,
                0,
            )
        };
        if map == MAP_FAILED {
            // The C has no failure path at all -- a VLA it cannot back simply
            // faults on first use. Do the same rather than invent an error.
            unsafe { core::ptr::read_volatile(core::ptr::null::<u8>()) };
            return;
        }
        let out = map.cast::<c_int>();
        unsafe {
            memcpy(map, data.cast::<c_void>(), nbytes);
            inner(out, len);
            munmap(map, nbytes);
        }
    } else if len == 0 {
        // `int out[0]` followed by `memcpy(out, data, 0)`: nothing is copied and
        // neither pointer is dereferenced (so a NULL `data` is fine), then both
        // loops in `inner` are skipped. `vla_base` is where the C's `out` points.
        let out = vla_base.cast::<c_int>();
        unsafe {
            memcpy(out.cast::<c_void>(), data.cast::<c_void>(), 0);
            inner(out, 0);
        }
    } else {
        // Negative length. `(size_t)(len * sizeof(int))` is enormous, so
        // `rsp -= nbytes` moves the stack pointer *up*: the C VLA's base address
        // is `rsp + |len| * sizeof(int)`, i.e. it points into the caller's frames,
        // and the `memcpy` then writes there with the same enormous count. Which
        // memory that clobbers -- and therefore whether the process survives the
        // return -- depends on the exact address, so the copy is performed at the
        // very same address the C uses (`vla_base`) rather than into a local.
        //
        // Nothing else runs on the corrupted stack: `inner` skips both of its
        // loops for a negative `len`, so it prints nothing, exactly as in the C.
        let dst = vla_base.cast::<c_int>();
        unsafe {
            memcpy(dst.cast::<c_void>(), data.cast::<c_void>(), nbytes);
            inner(dst, len);
        }
    }
}
