//! Allocator-level differential tests: exact allocation sizes, allocation
//! counts, free counts, and REAL allocation-failure fault injection.
//!
//! Why interposition: the C allocates the returned buffer with
//! `calloc(sizeof(char), l + 13)` and a scratch buffer with `malloc(l)`, where
//! `l == strlen(src) + 1`. Neither size is visible in the returned bytes, and
//! `malloc_usable_size` is NOT a usable proxy for a requested size (glibc reuses
//! binned chunks and hands a chunk over whole when the remainder is too small to
//! split, so the value depends on heap state).
//!
//! So `calloc`, `malloc` and `free` are defined in the TEST EXECUTABLE. The
//! executable is searched first in the global symbol scope (`-rdynamic` is set
//! in `.cargo/config.toml`), so both dlopened `.so`s bind their PLT slots here.
//! Forwarding uses glibc's `__libc_calloc` / `__libc_malloc` / `__libc_free`
//! rather than `dlsym(RTLD_NEXT, ...)`, which would itself allocate and recurse.
//!
//! This also makes `ERRORS.md` rows 3 and 4 (calloc/malloc failure) genuinely
//! testable rather than "verified by construction": the interposed allocators
//! return NULL on demand for one exact size.
//!
//! The whole file is a SINGLE `#[test]` because the recorder and the
//! fault-injection arming are process-global and must not be touched by a
//! concurrently running sibling test.

mod common;

use common::*;
use std::ffi::{c_char, c_void};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

extern "C" {
    fn __libc_calloc(nmemb: usize, size: usize) -> *mut c_void;
    fn __libc_malloc(size: usize) -> *mut c_void;
    fn __libc_free(ptr: *mut c_void);
}

const DISARMED: usize = usize::MAX;

static RECORDING: AtomicBool = AtomicBool::new(false);
static CALLOC_COUNT: AtomicUsize = AtomicUsize::new(0);
static CALLOC_LAST: AtomicUsize = AtomicUsize::new(DISARMED);
static MALLOC_COUNT: AtomicUsize = AtomicUsize::new(0);
static MALLOC_LAST: AtomicUsize = AtomicUsize::new(DISARMED);
static FREE_COUNT: AtomicUsize = AtomicUsize::new(0);

/// When armed, an allocation request of EXACTLY this size returns NULL.
static CALLOC_FAIL_SIZE: AtomicUsize = AtomicUsize::new(DISARMED);
static MALLOC_FAIL_SIZE: AtomicUsize = AtomicUsize::new(DISARMED);

#[no_mangle]
pub unsafe extern "C" fn calloc(nmemb: usize, size: usize) -> *mut c_void {
    let total = match nmemb.checked_mul(size) {
        Some(t) => t,
        None => return std::ptr::null_mut(),
    };
    if RECORDING.load(Ordering::SeqCst) {
        CALLOC_LAST.store(total, Ordering::SeqCst);
        CALLOC_COUNT.fetch_add(1, Ordering::SeqCst);
    }
    if total == CALLOC_FAIL_SIZE.load(Ordering::SeqCst) {
        return std::ptr::null_mut();
    }
    __libc_calloc(nmemb, size)
}

#[no_mangle]
pub unsafe extern "C" fn malloc(size: usize) -> *mut c_void {
    if RECORDING.load(Ordering::SeqCst) {
        MALLOC_LAST.store(size, Ordering::SeqCst);
        MALLOC_COUNT.fetch_add(1, Ordering::SeqCst);
    }
    if size == MALLOC_FAIL_SIZE.load(Ordering::SeqCst) {
        return std::ptr::null_mut();
    }
    __libc_malloc(size)
}

#[no_mangle]
pub unsafe extern "C" fn free(ptr: *mut c_void) {
    if RECORDING.load(Ordering::SeqCst) && !ptr.is_null() {
        FREE_COUNT.fetch_add(1, Ordering::SeqCst);
    }
    __libc_free(ptr)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Trace {
    calloc_count: usize,
    calloc_size: usize,
    malloc_count: usize,
    malloc_size: usize,
    free_count: usize,
    returned_null: bool,
}

/// Call `f` with the recorder on. The arming window is exactly the call, so an
/// unrelated Rust allocation cannot be hit by the fault injection.
unsafe fn trace(f: impl FnOnce() -> *mut c_char) -> (*mut c_char, Trace) {
    CALLOC_COUNT.store(0, Ordering::SeqCst);
    MALLOC_COUNT.store(0, Ordering::SeqCst);
    FREE_COUNT.store(0, Ordering::SeqCst);
    CALLOC_LAST.store(DISARMED, Ordering::SeqCst);
    MALLOC_LAST.store(DISARMED, Ordering::SeqCst);
    RECORDING.store(true, Ordering::SeqCst);
    let p = f();
    RECORDING.store(false, Ordering::SeqCst);
    (
        p,
        Trace {
            calloc_count: CALLOC_COUNT.load(Ordering::SeqCst),
            calloc_size: CALLOC_LAST.load(Ordering::SeqCst),
            malloc_count: MALLOC_COUNT.load(Ordering::SeqCst),
            malloc_size: MALLOC_LAST.load(Ordering::SeqCst),
            free_count: FREE_COUNT.load(Ordering::SeqCst),
            returned_null: p.is_null(),
        },
    )
}

fn input_of(len: usize) -> Vec<u8> {
    let mut rng = Rng::new(SEED ^ (len as u64).wrapping_mul(31));
    let mut v = from_set(&mut rng, ALPHABET_EQ, len);
    v.push(0);
    v
}

/// Sanity: prove the interposition is actually in effect, otherwise every
/// assertion in this file would pass vacuously.
fn part0_interposition_is_live() {
    unsafe {
        let (p, t) = trace(|| calloc(1, 4242) as *mut c_char);
        __libc_free(p as *mut c_void);
        assert!(
            t.calloc_count >= 1 && t.calloc_size == 4242,
            "calloc interposition is not in effect ({t:?}); is -rdynamic applied \
             via .cargo/config.toml?"
        );
        let q = malloc(1234);
        assert!(!q.is_null());
        __libc_free(q);
    }
}

/// Exact allocation sizes and counts must match the C for every length.
fn part1_exact_alloc_sizes_and_counts() {
    let i = impls();
    let mut checked = 0usize;
    for len in (1usize..=300).chain([511, 512, 513, 1000, 4096, 65536, 1 << 20]) {
        let cstr = input_of(len);
        let ptr = cstr.as_ptr() as *const c_char;
        unsafe {
            let (pc, tc) = trace(|| (i.c_decode)(ptr));
            __libc_free(pc as *mut c_void);
            let (pr, tr) = trace(|| (i.rust_decode)(ptr));
            __libc_free(pr as *mut c_void);

            // Pin the C's contract explicitly, then require Rust to match it.
            assert_eq!(tc.calloc_count, 1, "len={len}: C calloc count");
            assert_eq!(tc.calloc_size, len + 1 + 13, "len={len}: C calloc size");
            assert_eq!(tc.malloc_count, 1, "len={len}: C malloc count");
            assert_eq!(tc.malloc_size, len + 1, "len={len}: C malloc size");
            assert_eq!(tc.free_count, 1, "len={len}: C free count (scratch buf)");
            assert!(!tc.returned_null, "len={len}: C returned NULL");

            assert_eq!(tr, tc, "len={len}: Rust allocator trace differs from C");
        }
        checked += 1;
    }
    assert!(checked >= 300, "expected >= 300 lengths, checked {checked}");
}

/// ERRORS.md rows 1 and 2: NULL / empty input must allocate nothing at all.
fn part2_null_and_empty_allocate_nothing() {
    let i = impls();
    unsafe {
        for (label, ptr) in [
            ("NULL", std::ptr::null::<c_char>()),
            ("empty", b"\0".as_ptr() as *const c_char),
        ] {
            let (_, tc) = trace(|| (i.c_decode)(ptr));
            let (_, tr) = trace(|| (i.rust_decode)(ptr));
            assert!(tc.returned_null, "{label}: C must return NULL");
            assert_eq!(tc.calloc_count, 0, "{label}: C must not calloc");
            assert_eq!(tc.malloc_count, 0, "{label}: C must not malloc");
            assert_eq!(tc.free_count, 0, "{label}: C must not free");
            assert_eq!(tr, tc, "{label}: Rust allocator trace differs from C");
        }
    }
}

/// ERRORS.md row 3: `calloc` fails => return NULL, having allocated nothing else.
fn part3_calloc_failure() {
    let i = impls();
    for len in [1usize, 2, 3, 4, 17, 63, 64, 255, 1000] {
        let cstr = input_of(len);
        let ptr = cstr.as_ptr() as *const c_char;
        let fail_size = len + 1 + 13;
        unsafe {
            CALLOC_FAIL_SIZE.store(fail_size, Ordering::SeqCst);
            let (pc, tc) = trace(|| (i.c_decode)(ptr));
            let (pr, tr) = trace(|| (i.rust_decode)(ptr));
            CALLOC_FAIL_SIZE.store(DISARMED, Ordering::SeqCst);

            assert!(pc.is_null(), "len={len}: C must return NULL when calloc fails");
            assert!(pr.is_null(), "len={len}: Rust must return NULL when calloc fails");
            assert_eq!(tc.calloc_count, 1, "len={len}: C calloc attempts");
            assert_eq!(
                tc.malloc_count, 0,
                "len={len}: C must not malloc after calloc failed"
            );
            assert_eq!(tc.free_count, 0, "len={len}: C must not free (nothing owned)");
            assert_eq!(
                tr, tc,
                "len={len}: Rust allocator trace differs from C on calloc failure"
            );
        }
    }
}

/// ERRORS.md row 4: `malloc` fails after `calloc` succeeded => `free(dest)` then
/// return NULL. The free count is what proves the cleanup actually happens.
fn part4_malloc_failure_frees_dest() {
    let i = impls();
    for len in [1usize, 2, 3, 4, 17, 63, 64, 255, 1000] {
        let cstr = input_of(len);
        let ptr = cstr.as_ptr() as *const c_char;
        let fail_size = len + 1; // malloc(l), l == strlen(src) + 1
        unsafe {
            MALLOC_FAIL_SIZE.store(fail_size, Ordering::SeqCst);
            let (pc, tc) = trace(|| (i.c_decode)(ptr));
            let (pr, tr) = trace(|| (i.rust_decode)(ptr));
            MALLOC_FAIL_SIZE.store(DISARMED, Ordering::SeqCst);

            assert!(pc.is_null(), "len={len}: C must return NULL when malloc fails");
            assert!(pr.is_null(), "len={len}: Rust must return NULL when malloc fails");
            assert_eq!(tc.calloc_count, 1, "len={len}: C calloc attempts");
            assert_eq!(tc.malloc_count, 1, "len={len}: C malloc attempts");
            assert_eq!(
                tc.free_count, 1,
                "len={len}: C must free(dest) before returning NULL"
            );
            assert_eq!(
                tr, tc,
                "len={len}: Rust allocator trace differs from C on malloc failure \
                 (a missing free(dest) here would be a leak)"
            );
        }
    }
}

#[test]
fn alloc_contract() {
    part0_interposition_is_live();
    part1_exact_alloc_sizes_and_counts();
    part2_null_and_empty_allocate_nothing();
    part3_calloc_failure();
    part4_malloc_failure_frees_dest();
}
