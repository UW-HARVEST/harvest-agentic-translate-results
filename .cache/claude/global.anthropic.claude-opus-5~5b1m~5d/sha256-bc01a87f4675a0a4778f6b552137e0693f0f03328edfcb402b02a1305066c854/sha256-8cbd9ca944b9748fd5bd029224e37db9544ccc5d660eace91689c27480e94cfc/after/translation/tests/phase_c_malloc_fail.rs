//! ERRORS.md row 16 — `checkshift` when `malloc(sizeof(ComputeState))` fails.
//!
//! The only way to reach that branch through the public FFI surface is to make
//! the allocator fail, so this test binary *interposes* `malloc`.  The
//! interposer forwards every request to the real allocator except while a
//! window is armed, during which a request for exactly `sizeof(ComputeState)`
//! (12) bytes returns NULL.  Both libraries bind to the interposed `malloc`
//! through the global symbol scope, so the same failure is injected into each.
//!
//! If the platform does not let the executable interpose `malloc` for a
//! `dlopen`ed object, the test detects that (the call succeeds) and falls back
//! to asserting that BOTH libraries behaved the same way, which is still a
//! valid differential result.

mod common;
use common::*;

use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

static ARMED: AtomicBool = AtomicBool::new(false);
static FAIL_SIZE: AtomicUsize = AtomicUsize::new(0);
static HITS: AtomicUsize = AtomicUsize::new(0);

#[no_mangle]
pub unsafe extern "C" fn malloc(size: usize) -> *mut c_void {
    if ARMED.load(Ordering::SeqCst) && size == FAIL_SIZE.load(Ordering::SeqCst) {
        HITS.fetch_add(1, Ordering::SeqCst);
        return std::ptr::null_mut();
    }
    real_malloc(size)
}

fn real_malloc(size: usize) -> *mut c_void {
    unsafe { libc_internal::__libc_malloc(size) }
}

// libc's internal, non-interposable entry point.
mod libc_internal {
    extern "C" {
        pub fn __libc_malloc(size: usize) -> *mut std::ffi::c_void;
    }
}
#[allow(unused_imports)]
use libc_internal as _libc_internal;

#[test]
fn err_16_checkshift_malloc_failure() {
    // sizeof(ComputeState) == 12 on every LP64/ILP32 target that matters.
    assert_eq!(std::mem::size_of::<ComputeState>(), 12);
    FAIL_SIZE.store(std::mem::size_of::<ComputeState>(), Ordering::SeqCst);

    let (c, r) = both();

    let _g = capture_lock();

    HITS.store(0, Ordering::SeqCst);
    ARMED.store(true, Ordering::SeqCst);
    let (cv, cout) = capture_locked(|| c.checkshift(1, 2, 3, 4));
    ARMED.store(false, Ordering::SeqCst);
    let c_hits = HITS.swap(0, Ordering::SeqCst);

    ARMED.store(true, Ordering::SeqCst);
    let (rv, rout) = capture_locked(|| r.checkshift(1, 2, 3, 4));
    ARMED.store(false, Ordering::SeqCst);
    let r_hits = HITS.swap(0, Ordering::SeqCst);

    drop(_g);

    let cs = String::from_utf8_lossy(&cout).to_string();
    let rs = String::from_utf8_lossy(&rout).to_string();

    assert_eq!(
        cv, rv,
        "malloc-failure return value mismatch: C={cv} RUST={rv}\nC out:\n{cs}\nRUST out:\n{rs}"
    );
    assert_eq!(cs, rs, "malloc-failure stdout mismatch");

    if c_hits > 0 && r_hits > 0 {
        // Interposition worked in both: the C-defined error path must have run.
        assert_eq!(cv, -1, "expected the -1 sentinel; C stdout:\n{cs}");
        assert!(
            cs.contains("Error: Failed to allocate memory for state"),
            "expected the allocation-failure diagnostic; got:\n{cs}"
        );
        assert!(
            !cs.contains("State initialized"),
            "the function must return before init_state; got:\n{cs}"
        );
        eprintln!("row16: malloc interposition active (C hits={c_hits}, Rust hits={r_hits})");
    } else {
        eprintln!(
            "row16: malloc could not be interposed for dlopen'ed objects \
             (C hits={c_hits}, Rust hits={r_hits}); verified only that both \
             libraries agree, plus the structural check in phase_c_errors."
        );
    }
}
