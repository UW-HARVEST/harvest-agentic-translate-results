//! Sound verification that both libraries issue the *identical* `malloc`
//! request, i.e. that the Rust reproduces
//! `malloc(numLines * sizeof(const char**))` exactly.
//!
//! `malloc_usable_size` cannot be used for this: glibc may serve a request
//! from a larger free chunk, so the usable size is a function of heap *state*,
//! not just of the request (observed: a 248-byte request reporting 248 in one
//! call and 264 in another). Instead this test **interposes `malloc`** in the
//! test executable. Because the executable comes first in the dynamic symbol
//! lookup scope, the `malloc@plt` calls inside *both* dlopened `.so`s resolve
//! here, and the requested byte count is recorded exactly. The real allocator
//! is reached through glibc's `__libc_malloc` alias, so there is no recursion
//! and blocks remain `free()`-able as usual.
//!
//! This file deliberately contains a single `#[test]` so that no other test
//! thread can allocate while the recorder is armed.

mod common;

use common::*;
use std::ffi::c_void;
use std::os::raw::c_char;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

extern "C" {
    fn __libc_malloc(n: usize) -> *mut c_void;
    fn free(p: *mut c_void);
}

static ARMED: AtomicBool = AtomicBool::new(false);
static LAST_SIZE: AtomicUsize = AtomicUsize::new(usize::MAX);
static CALL_COUNT: AtomicUsize = AtomicUsize::new(0);

/// Interposed `malloc` — records the request size while armed, then forwards.
#[no_mangle]
pub unsafe extern "C" fn malloc(n: usize) -> *mut c_void {
    if ARMED.load(Ordering::Relaxed) {
        LAST_SIZE.store(n, Ordering::Relaxed);
        CALL_COUNT.fetch_add(1, Ordering::Relaxed);
    }
    __libc_malloc(n)
}

/// Record the exact `malloc` request size made by one invocation.
/// Returns `(requested_size, malloc_call_count, returned_ptr)`.
unsafe fn measure(
    imp: &Impl,
    buffer: *mut c_char,
    num_lines: usize,
    buffer_size: usize,
) -> (usize, usize, *const *const c_char) {
    for _attempt in 0..32 {
        LAST_SIZE.store(usize::MAX, Ordering::Relaxed);
        CALL_COUNT.store(0, Ordering::Relaxed);
        ARMED.store(true, Ordering::SeqCst);
        let ret = imp.create_raw(buffer, num_lines, buffer_size);
        ARMED.store(false, Ordering::SeqCst);
        let n = CALL_COUNT.load(Ordering::Relaxed);
        let size = LAST_SIZE.load(Ordering::Relaxed);
        if n == 1 {
            return (size, n, ret);
        }
        // Another thread allocated inside the window; discard and retry.
        if !ret.is_null() {
            free(ret as *mut c_void);
        }
    }
    panic!("could not obtain an uncontended malloc measurement for {}", imp.name);
}

#[test]
fn malloc_request_size_is_identical() {
    // Prove the interposition is actually in effect before relying on it.
    {
        LAST_SIZE.store(usize::MAX, Ordering::Relaxed);
        CALL_COUNT.store(0, Ordering::Relaxed);
        ARMED.store(true, Ordering::SeqCst);
        let p = unsafe { __libc_malloc(1) };
        ARMED.store(false, Ordering::SeqCst);
        unsafe { free(p) };
        assert_eq!(
            CALL_COUNT.load(Ordering::Relaxed),
            0,
            "__libc_malloc must bypass the recorder"
        );

        let pair = pair();
        // A call that is guaranteed to reach `malloc` in both libraries.
        let mut buf = vec![0u8; 5];
        let base = buf.as_mut_ptr() as *mut c_char;
        let (sz, n, ret) = unsafe { measure(&pair.c, base, 5, 5) };
        assert_eq!(n, 1, "interposition did not observe the C library's malloc");
        assert_eq!(sz, 5 * 8, "C requested {sz} bytes for numLines=5, expected 40");
        assert!(!ret.is_null());
        unsafe { free(ret as *mut c_void) };
    }

    let p = pair();

    // ---- successful calls: numLines lines really are present -------------
    for k in [
        0usize, 1, 2, 3, 4, 5, 7, 8, 9, 15, 16, 17, 31, 32, 33, 63, 64, 100, 127, 128, 255, 256,
        1000, 1023, 1024, 4096, 65535, 65536,
    ] {
        let mut buf = vec![0u8; k.max(1)]; // all-NUL -> exactly k empty lines
        let base = buf.as_mut_ptr() as *mut c_char;

        let (sc, _, rc) = unsafe { measure(&p.c, base, k, k) };
        let (sr, _, rr) = unsafe { measure(&p.rust, base, k, k) };
        assert_eq!(
            sc,
            k.wrapping_mul(8),
            "C's own request for numLines={k} was {sc}, expected {}",
            k.wrapping_mul(8)
        );
        assert_eq!(
            sc, sr,
            "malloc request size diverges for numLines={k}: C asked for {sc} bytes, \
             Rust asked for {sr}"
        );
        assert_eq!(rc.is_null(), rr.is_null(), "NULL-ness differs at k={k}");
        unsafe {
            if !rc.is_null() {
                free(rc as *mut c_void);
            }
            if !rr.is_null() {
                free(rr as *mut c_void);
            }
        }
    }

    // ---- failing calls: the malloc request still has to match -------------
    // (bufferSize = 0, so nothing is dereferenced and no OOB write occurs)
    for k in [
        1usize,
        2,
        1 << 10,
        1 << 20,
        1 << 58,
        1 << 60,
        1 << 61, // *8 wraps to 0
        (1 << 61) + 1, // *8 wraps to 8
        (1 << 61) + 3, // *8 wraps to 24
        1 << 62,
        1 << 63,
        usize::MAX / 8,
        usize::MAX / 8 + 1,
        usize::MAX - 1,
        usize::MAX,
    ] {
        let (sc, _, rc) = unsafe { measure(&p.c, std::ptr::null_mut(), k, 0) };
        let (sr, _, rr) = unsafe { measure(&p.rust, std::ptr::null_mut(), k, 0) };
        assert_eq!(
            sc,
            k.wrapping_mul(8),
            "C's own request for numLines={k} was {sc}, expected wrapping {}",
            k.wrapping_mul(8)
        );
        assert_eq!(
            sc, sr,
            "malloc request size diverges for numLines={k}: C asked for {sc} bytes, \
             Rust asked for {sr} (wrapping multiply not reproduced?)"
        );
        assert_eq!(rc.is_null(), rr.is_null(), "NULL-ness differs at k={k}");
        unsafe {
            if !rc.is_null() {
                free(rc as *mut c_void);
            }
            if !rr.is_null() {
                free(rr as *mut c_void);
            }
        }
    }

    // ---- randomized numLines, including the wrap region -------------------
    let mut rng = Rng::new(SEED ^ 0xA110C);
    for _ in 0..500 {
        let k = match rng.below(3) {
            0 => rng.below(4096),
            1 => (1usize << 61) + rng.below(4096),
            _ => usize::MAX - rng.below(4096),
        };
        let (sc, _, rc) = unsafe { measure(&p.c, std::ptr::null_mut(), k, 0) };
        let (sr, _, rr) = unsafe { measure(&p.rust, std::ptr::null_mut(), k, 0) };
        assert_eq!(sc, sr, "malloc request size diverges for numLines={k}");
        assert_eq!(
            sc,
            k.wrapping_mul(8),
            "C request {sc} != wrapping k*8 for k={k}"
        );
        assert_eq!(rc.is_null(), rr.is_null(), "NULL-ness differs at k={k}");
        unsafe {
            if !rc.is_null() {
                free(rc as *mut c_void);
            }
            if !rr.is_null() {
                free(rr as *mut c_void);
            }
        }
    }

    // ---- exactly one malloc per call, and exactly one free on failure -----
    // (the C makes a single malloc; a translation that allocates twice, or that
    //  routes through Rust's allocator, is rejected here)
    let mut buf = vec![0u8; 8];
    let base = buf.as_mut_ptr() as *mut c_char;
    for imp in [&p.c, &p.rust] {
        let (_, n, ret) = unsafe { measure(imp, base, 8, 8) };
        assert_eq!(n, 1, "{} performed {n} mallocs, expected exactly 1", imp.name);
        unsafe {
            if !ret.is_null() {
                free(ret as *mut c_void);
            }
        }
    }
}
