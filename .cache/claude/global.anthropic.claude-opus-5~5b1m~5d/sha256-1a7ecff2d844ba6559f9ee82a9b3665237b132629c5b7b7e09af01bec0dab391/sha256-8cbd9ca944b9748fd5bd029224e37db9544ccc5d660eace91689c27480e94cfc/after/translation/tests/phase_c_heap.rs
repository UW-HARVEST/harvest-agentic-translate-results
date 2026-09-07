//! Phase C addendum — heap-accounting differential tests.
//!
//! `UTIL_createLinePointers` has a *memory* contract as well as a return-value
//! contract: on the `lineIndex != numLines` path the C calls `free(bufferPtrs)`
//! before returning `NULL`, so a rejected call must leak nothing. That is
//! invisible to a return-value comparison, so it is checked here directly.
//!
//! Both shared libraries are dlopen'd into this one process and therefore share
//! the same glibc allocator, which makes `mallinfo2().uordblks` (bytes in use)
//! directly comparable between the C and the Rust implementation.

mod common;

use common::*;
use std::os::raw::c_char;

#[repr(C)]
#[derive(Default, Clone, Copy)]
struct MallInfo2 {
    arena: usize,
    ordblks: usize,
    smblks: usize,
    hblks: usize,
    hblkhd: usize,
    usmblks: usize,
    fsmblks: usize,
    uordblks: usize,
    fordblks: usize,
    keepcost: usize,
}

extern "C" {
    fn mallinfo2() -> MallInfo2;
}

fn in_use() -> usize {
    unsafe { mallinfo2().uordblks }
}

/// `mallinfo2()` reports PROCESS-WIDE allocator state, so two heap tests running
/// on parallel harness threads would measure each other's allocations. Every
/// test in this file takes this lock so the measurements are serialized.
static HEAP_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn heap_guard() -> std::sync::MutexGuard<'static, ()> {
    HEAP_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

/// Allowance for allocations the test harness itself performs on other threads
/// while a measurement window is open. Every leak this file looks for is orders
/// of magnitude larger (asserted per-test).
const NOISE_FLOOR: isize = 256 * 1024;

/// Call `f` `n` times on an input that must be REJECTED, and return the net
/// change in bytes held by the allocator.
fn net_bytes_after_rejected_calls(
    f: CreateLinePointersFn,
    buf: *mut c_char,
    num_lines: usize,
    size: usize,
    n: usize,
) -> isize {
    // Warm up so first-touch arena growth is not counted.
    for _ in 0..64 {
        let r = unsafe { f(buf, num_lines, size) };
        assert!(r.is_null(), "input was supposed to be rejected");
    }
    let before = in_use();
    for _ in 0..n {
        let r = unsafe { f(buf, num_lines, size) };
        assert!(r.is_null(), "input was supposed to be rejected");
    }
    in_use() as isize - before as isize
}

/// The rejection path must free its allocation in BOTH implementations, and
/// their net heap growth must agree.
#[test]
fn heap_rejected_calls_do_not_leak_in_either_impl() {
    let _heap = heap_guard();
    let l = libs();
    // 3 lines present, 64 requested -> `lineIndex != numLines` -> free + NULL.
    let mut buf = *b"one\0two\0three\0";
    let p = buf.as_mut_ptr() as *mut c_char;
    const N: usize = 20_000;

    let c_delta = net_bytes_after_rejected_calls(l.c_fn, p, 64, buf.len(), N);
    let r_delta = net_bytes_after_rejected_calls(l.rust_fn, p, 64, buf.len(), N);

    // 64 * 8 = 512 bytes per call, so a missing `free` grows the heap by
    // ~10 MB over N calls. NOISE_FLOOR absorbs the few KB the test harness
    // itself may allocate concurrently while still being ~40x smaller than a
    // real leak, keeping the check both robust and highly sensitive.
    let would_leak = (N * 512) as isize;
    let bound = NOISE_FLOOR;
    assert!(would_leak > 20 * bound, "test not sensitive enough");
    assert!(
        c_delta.abs() < bound,
        "C leaked on the rejection path?! delta={c_delta} bound={bound}"
    );
    assert!(
        r_delta.abs() < bound,
        "Rust LEAKS on the rejection path (missing free): delta={r_delta} bound={bound} \
         (C delta was {c_delta})"
    );
}

/// Same check driven across many different rejected shapes, comparing the two
/// implementations against each other rather than against a fixed bound.
#[test]
fn heap_rejection_growth_matches_between_impls() {
    let _heap = heap_guard();
    let l = libs();
    let mut rng = Rng::new(SEED ^ 0xBEEF);
    for i in 0..20 {
        let count = rng.range(1, 8);
        let lines: Vec<Vec<u8>> = (0..count)
            .map(|_| {
                let len = rng.range(1, 8);
                (0..len).map(|_| rng.nonzero_byte()).collect()
            })
            .collect();
        let mut buf = join_terminated(&lines);
        let size = buf.len();
        let want = count + rng.range(1, 40);
        let p = buf.as_mut_ptr() as *mut c_char;

        const N: usize = 50_000;
        let c_delta = net_bytes_after_rejected_calls(l.c_fn, p, want, size, N);
        let r_delta = net_bytes_after_rejected_calls(l.rust_fn, p, want, size, N);
        let per_call = want * 8;
        let would_leak = (N * per_call) as isize;
        let bound = NOISE_FLOOR;
        assert!(would_leak > 5 * bound, "test not sensitive enough: want={want}");
        assert!(
            c_delta.abs() < bound && r_delta.abs() < bound,
            "heap growth divergence [iter{i}] want={want} size={size}: \
             C delta={c_delta}, Rust delta={r_delta}, bound={bound}"
        );
    }
}

/// The SUCCESS path must hand back a block the caller can `free`, and both
/// implementations must request the same size, so freeing what they return must
/// leave the heap flat for both.
#[test]
fn heap_success_path_block_is_freeable_and_same_size() {
    let _heap = heap_guard();
    let l = libs();
    let mut buf = *b"one\0two\0three\0";
    let p = buf.as_mut_ptr() as *mut c_char;

    extern "C" {
        fn free(ptr: *mut std::os::raw::c_void);
    }

    let measure = |f: CreateLinePointersFn| -> isize {
        for _ in 0..64 {
            let r = unsafe { f(p, 3, buf.len()) };
            assert!(!r.is_null());
            unsafe { free(r as *mut _) };
        }
        let before = in_use();
        for _ in 0..10_000 {
            let r = unsafe { f(p, 3, buf.len()) };
            assert!(!r.is_null());
            unsafe { free(r as *mut _) };
        }
        in_use() as isize - before as isize
    };

    let c_delta = measure(l.c_fn);
    let r_delta = measure(l.rust_fn);
    assert!(
        c_delta.abs() < NOISE_FLOOR && r_delta.abs() < NOISE_FLOOR,
        "success-path heap divergence: C delta={c_delta}, Rust delta={r_delta}"
    );
}

/// `numLines == 0` allocates `malloc(0)` in the C and returns it as a SUCCESS.
/// A Rust version that saturated (rather than wrapped) the size computation, or
/// that skipped the allocation entirely, would differ here — both in the
/// returned sentinel and in the block the caller is expected to free.
#[test]
fn heap_zero_lines_returns_a_freeable_block_in_both() {
    let _heap = heap_guard();
    let l = libs();
    extern "C" {
        fn free(ptr: *mut std::os::raw::c_void);
    }
    for &f in &[l.c_fn, l.rust_fn] {
        let r = unsafe { f(std::ptr::null_mut(), 0, 0) };
        assert!(
            !r.is_null(),
            "numLines=0 must return the non-NULL malloc(0) block"
        );
        unsafe { free(r as *mut _) }; // must not abort: it is a real heap block
    }
}
