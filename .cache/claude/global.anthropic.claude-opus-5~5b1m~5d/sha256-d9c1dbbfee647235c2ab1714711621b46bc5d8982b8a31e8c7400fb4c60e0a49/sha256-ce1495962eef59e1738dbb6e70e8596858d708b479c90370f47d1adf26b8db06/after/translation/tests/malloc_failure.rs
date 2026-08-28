//! Phase C, row E2 — `malloc` returns NULL, so `if(!newstr) return NULL` fires.
//!
//! ```c
//!   newstr = malloc(len);
//!   if(!newstr)
//!     return (char *)NULL;      /* <-- the branch under test */
//! ```
//!
//! This is the single most important error-path row, because it is the one place
//! the two implementations could realistically diverge: a Rust translation that
//! allocated with the Rust global allocator (`alloc::alloc` / `Vec` / `String`)
//! would **abort the process** on allocation failure instead of returning
//! `NULL`. The translation must call libc `malloc` and hand back `NULL`.
//!
//! Reproducing allocator exhaustion deterministically:
//!   1. allocate a large source string *first*, while memory is still available;
//!   2. warm up both `.so`s so all lazy PLT binding is already resolved;
//!   3. lower `RLIMIT_AS` (soft only) to the process's current address-space
//!      size plus a tiny slack, so any further `mmap`-backed `malloc` fails;
//!   4. call both implementations and record the raw pointers — with **no**
//!      allocation of our own inside the window;
//!   5. restore the soft limit (the hard limit was never touched), then assert.
//!
//! This test lives in its own test binary and is the only test in it, so no
//! other test thread can allocate while the limit is lowered.

mod common;

use common::libs;
use std::ffi::c_char;

/// Current total address-space size of this process, in bytes (`/proc/self/statm`
/// field 1, in pages).
fn address_space_bytes() -> u64 {
    let statm = std::fs::read_to_string("/proc/self/statm").expect("read /proc/self/statm");
    let pages: u64 = statm
        .split_whitespace()
        .next()
        .expect("statm field 0")
        .parse()
        .expect("statm field 0 is a number");
    let page_size = unsafe { libc::sysconf(libc::_SC_PAGESIZE) } as u64;
    pages * page_size
}

fn get_rlimit_as() -> libc::rlimit {
    let mut rl = libc::rlimit {
        rlim_cur: 0,
        rlim_max: 0,
    };
    let rc = unsafe { libc::getrlimit(libc::RLIMIT_AS, &mut rl) };
    assert_eq!(rc, 0, "getrlimit(RLIMIT_AS) failed");
    rl
}

fn set_rlimit_as(rl: &libc::rlimit) -> i32 {
    unsafe { libc::setrlimit(libc::RLIMIT_AS, rl) }
}

#[test]
fn e2_malloc_returns_null() {
    let l = libs();
    let c_fn = l.c;
    let r_fn = l.rust;

    // ---- step 1: a source string far larger than the slack we will leave. ----
    const SRC_LEN: usize = 96 * 1024 * 1024; // 96 MiB payload
    let mut src_buf = vec![0x41u8; SRC_LEN];
    src_buf.push(0);
    // Touch every page so nothing is lazily faulted inside the window.
    for i in (0..src_buf.len()).step_by(4096) {
        std::hint::black_box(src_buf[i]);
    }
    let src = src_buf.as_ptr() as *const c_char;

    // ---- step 2: warm up both implementations (resolves lazy PLT entries). ---
    {
        let warm = b"warmup\0";
        let wsrc = warm.as_ptr() as *const c_char;
        let a = unsafe { c_fn(wsrc) };
        let b = unsafe { r_fn(wsrc) };
        assert!(!a.is_null() && !b.is_null(), "warmup allocation failed");
        unsafe {
            common::c_free(a);
            common::c_free(b);
        }
        // Also warm the NULL path.
        assert_eq!(unsafe { c_fn(std::ptr::null()) } as usize, 0);
        assert_eq!(unsafe { r_fn(std::ptr::null()) } as usize, 0);
    }

    let original = get_rlimit_as();

    // ---- step 3: lower the soft limit to "current usage + small slack". ----
    // 96 MiB is far beyond the slack, so malloc(96 MiB + 1) must fail while the
    // process itself keeps running normally.
    const SLACK: u64 = 512 * 1024;
    let target_soft = address_space_bytes() + SLACK;
    if original.rlim_max != libc::RLIM_INFINITY && target_soft > original.rlim_max {
        panic!("cannot lower RLIMIT_AS below the existing hard limit");
    }
    let tightened = libc::rlimit {
        rlim_cur: target_soft,
        rlim_max: original.rlim_max,
    };
    assert_eq!(
        set_rlimit_as(&tightened),
        0,
        "setrlimit(RLIMIT_AS) to {target_soft} failed"
    );

    // ---- step 4: the measurement window — no allocations of our own here. ----
    let c_res = unsafe { c_fn(src) };
    let r_res = unsafe { r_fn(src) };
    // Second round with the opposite call order, to rule out order dependence.
    let r_res2 = unsafe { r_fn(src) };
    let c_res2 = unsafe { c_fn(src) };
    // The NULL path must still work while the allocator is exhausted.
    let c_null = unsafe { c_fn(std::ptr::null()) };
    let r_null = unsafe { r_fn(std::ptr::null()) };

    // ---- step 5: restore the soft limit before doing anything that allocates.
    let restore_rc = set_rlimit_as(&original);

    assert_eq!(restore_rc, 0, "failed to restore RLIMIT_AS");

    // If any call unexpectedly succeeded, release it so we do not leak.
    for p in [c_res, r_res, c_res2, r_res2] {
        if !p.is_null() {
            unsafe { common::c_free(p) };
        }
    }

    // ---- assertions ----
    assert!(
        c_res.is_null(),
        "E2 precondition: expected malloc to fail for the C implementation \
         under RLIMIT_AS={target_soft}; it returned {c_res:p}. The test cannot \
         validate the allocation-failure branch unless malloc actually fails."
    );

    assert_eq!(
        c_res.is_null(),
        r_res.is_null(),
        "E2: rejection diverged — C returned {c_res:p}, Rust returned {r_res:p}. \
         On allocation failure both must return the NULL sentinel."
    );
    assert_eq!(c_res as usize, 0, "E2: C sentinel must be exactly 0");
    assert_eq!(
        r_res as usize, 0,
        "E2: Rust sentinel must be exactly 0 (a Rust-global-allocator \
         translation would have aborted instead of returning NULL)"
    );

    assert_eq!(
        c_res2.is_null(),
        r_res2.is_null(),
        "E2: rejection diverged on the reversed call order"
    );
    assert_eq!(c_res2 as usize, 0, "E2: C sentinel (reversed order)");
    assert_eq!(r_res2 as usize, 0, "E2: Rust sentinel (reversed order)");

    assert_eq!(c_null as usize, 0, "E2: C NULL path under exhaustion");
    assert_eq!(r_null as usize, 0, "E2: Rust NULL path under exhaustion");

    // ---- step 6: both implementations still work after the failure. ----
    // Proves the failed call performed no memcpy and left no broken state.
    let after = b"still-alive\0";
    let asrc = after.as_ptr() as *const c_char;
    let a = unsafe { c_fn(asrc) };
    let b = unsafe { r_fn(asrc) };
    assert!(!a.is_null(), "E2: C broken after allocation failure");
    assert!(!b.is_null(), "E2: Rust broken after allocation failure");
    unsafe {
        assert_eq!(common::bytes_with_nul(a), b"still-alive\0".to_vec());
        assert_eq!(common::bytes_with_nul(b), b"still-alive\0".to_vec());
        common::c_free(a);
        common::c_free(b);
    }

    drop(src_buf);
    println!("E2: both implementations returned NULL on malloc failure");
}
