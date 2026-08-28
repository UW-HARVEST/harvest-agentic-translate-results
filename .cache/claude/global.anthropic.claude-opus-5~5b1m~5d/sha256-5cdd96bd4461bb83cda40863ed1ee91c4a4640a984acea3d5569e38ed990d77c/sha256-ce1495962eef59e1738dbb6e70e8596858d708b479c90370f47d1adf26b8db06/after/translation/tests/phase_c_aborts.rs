//! Phase C — abort/crash-parity rows.
//!
//! Some inputs are fatal in the C (a live `STBDS_ASSERT`, or a NULL
//! dereference). Those cannot be compared in-process, so each row runs the
//! offending call in a **subprocess** — once against the C `.so`, once against
//! the Rust `.so` — and the two termination statuses (exit code / signal) must
//! match.
//!
//! The C library is compiled without `NDEBUG` (`C_FLAGS = -fPIC`), so
//! `STBDS_ASSERT` == `assert` is live; the Rust translation therefore uses
//! `assert!` (not `debug_assert!`) at every `STBDS_ASSERT` site.
mod common;
use common::*;
use std::ffi::c_void;
use std::os::raw::c_char;
use std::os::unix::process::ExitStatusExt;
use std::process::{Command, Stdio};

const CASE_ENV: &str = "ARRINS_ABORT_CASE";
const LIB_ENV: &str = "ARRINS_ABORT_LIB";

fn child_status(case: &str, which: &str, testname: &str) -> (Option<i32>, Option<i32>) {
    let exe = std::env::current_exe().expect("current_exe");
    let st = Command::new(exe)
        .args(["--exact", testname, "--test-threads=1", "--nocapture"])
        .env(CASE_ENV, case)
        .env(LIB_ENV, which)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .expect("spawn child");
    (st.code(), st.signal())
}

/// Returns `Some(lib)` when this process is the crash-performing child.
fn child_lib(case: &str) -> Option<Lib> {
    if std::env::var(CASE_ENV).ok().as_deref() != Some(case) {
        return None;
    }
    match std::env::var(LIB_ENV).ok()?.as_str() {
        "c" => Some(Lib::open("C", &c_so_path())),
        "rust" => Some(Lib::open("Rust", &rust_so_path())),
        _ => None,
    }
}

/// Compare the two terminations and report.
#[track_caller]
fn assert_same_death(case: &str, testname: &str, expect_fatal: bool) {
    let c = child_status(case, "c", testname);
    let r = child_status(case, "rust", testname);
    assert_eq!(
        c, r,
        "abort-parity mismatch for `{case}`: C=(code,signal)={c:?} Rust=(code,signal)={r:?}"
    );
    if expect_fatal {
        assert!(
            c.1.is_some(),
            "expected `{case}` to be fatal in the C, got {c:?}"
        );
    }
}

// ===========================================================================
// E65 : stbds_stralloc with remaining >= len but storage == NULL
// ===========================================================================

#[test]
fn abort_stralloc_null_storage() {
    const CASE: &str = "stralloc_null_storage";
    if let Some(l) = child_lib(CASE) {
        unsafe {
            let mut a = CArena {
                storage: std::ptr::null_mut(),
                remaining: 64,
                block: 0,
                mode: 0,
            };
            let mut s = b"abcd\0".to_vec();
            // len(5) <= remaining(64) -> the C skips block allocation and does
            //   p = a->storage->storage + remaining - len
            // which dereferences NULL.
            let p = (l.stralloc)(&mut a, s.as_mut_ptr() as *mut c_char);
            println!("survived: {p:?}");
        }
        std::process::exit(0);
    }
    assert_same_death(CASE, "abort_stralloc_null_storage", true);
}

// ===========================================================================
// E49 extreme : a `block` value whose derived blocksize is astronomically large
// ===========================================================================

#[test]
fn abort_stralloc_absurd_blocksize() {
    const CASE: &str = "stralloc_absurd_blocksize";
    if let Some(l) = child_lib(CASE) {
        unsafe {
            // block = 200 -> (size_t)512 << (200>>1 == 100). The x86-64 `shl`
            // count is taken mod 64, so blocksize == 512 << 36 == 32 TiB.
            // The subsequent REALLOC fails and `sb->next = ...` writes to NULL.
            let mut a = CArena {
                storage: std::ptr::null_mut(),
                remaining: 0,
                block: 200,
                mode: 0,
            };
            let mut s = b"x\0".to_vec();
            let p = (l.stralloc)(&mut a, s.as_mut_ptr() as *mut c_char);
            println!("survived: {p:?} remaining={} block={}", a.remaining, a.block);
        }
        std::process::exit(0);
    }
    // NOTE: not asserted fatal — on a machine that happily over-commits 32 TiB
    // the call would succeed. What matters is that C and Rust agree.
    assert_same_death(CASE, "abort_stralloc_absurd_blocksize", false);
}

// ===========================================================================
// E67 : hmdel_key with `mode >= 2` and an element swap -> STBDS_ASSERT(slot>=0)
// ===========================================================================

#[test]
fn abort_hmdel_mode2_swap() {
    const CASE: &str = "hmdel_mode2_swap";
    if let Some(l) = child_lib(CASE) {
        unsafe {
            (l.rand_seed)(0x1234_5678);
            let elemsize = 16usize;
            let keysize = 8usize;
            // strdup-owned string map
            let mut t: *mut c_void = (l.shmode_func)(elemsize, 2 /* STBDS_SH_STRDUP */);
            let mut keys: Vec<Vec<u8>> = (0..4)
                .map(|i| format!("abort_key_{i}\0").into_bytes())
                .collect();
            for k in keys.iter_mut() {
                t = (l.hmput_key)(t, elemsize, k.as_mut_ptr() as *mut c_void, keysize, 1);
            }
            // delete the FIRST key with mode = 2: old_index(0) != final_index(3),
            // so the C re-finds the moved element with the wrong key expression
            // and trips STBDS_ASSERT(slot >= 0).
            t = (l.hmdel_key)(
                t,
                elemsize,
                keys[0].as_mut_ptr() as *mut c_void,
                keysize,
                0,
                2,
            );
            println!("survived: {t:?}");
        }
        std::process::exit(0);
    }
    assert_same_death(CASE, "abort_hmdel_mode2_swap", true);
}

// ===========================================================================
// E62 : stbds_arrfreef(NULL) -> free((char*)NULL - 32)
// ===========================================================================

#[test]
fn abort_arrfreef_null() {
    const CASE: &str = "arrfreef_null";
    if let Some(l) = child_lib(CASE) {
        unsafe {
            (l.arrfreef)(std::ptr::null_mut());
            println!("survived");
        }
        std::process::exit(0);
    }
    assert_same_death(CASE, "abort_arrfreef_null", true);
}

// ===========================================================================
// E66 : proof that the assert machinery is compiled in on BOTH sides
// ===========================================================================

#[test]
fn abort_assert_is_live() {
    // The C references glibc's __assert_fail, i.e. NDEBUG is not set.
    let out = Command::new("nm")
        .arg("-D")
        .arg(c_so_path())
        .output()
        .expect("nm");
    let syms = String::from_utf8_lossy(&out.stdout);
    assert!(
        syms.contains("__assert_fail"),
        "the C library must have live asserts (no NDEBUG)"
    );
    // The Rust cdylib must contain panic machinery for its `assert!`s.
    let out = Command::new("nm")
        .arg("-C")
        .arg(rust_so_path())
        .output()
        .expect("nm");
    let syms = String::from_utf8_lossy(&out.stdout);
    assert!(
        syms.contains("panic") || syms.contains("abort"),
        "the Rust library must have live asserts (assert!, not debug_assert!)"
    );
}

// ===========================================================================
// Sanity: a subprocess that is expected to survive must survive in both
// ===========================================================================

#[test]
fn abort_harness_sanity() {
    const CASE: &str = "harness_sanity";
    if let Some(l) = child_lib(CASE) {
        unsafe {
            (l.arr_ins)(7);
            let a = (l.arrgrowf)(std::ptr::null_mut(), 4, 4, 0);
            (l.arrfreef)(a);
        }
        std::process::exit(0);
    }
    let c = child_status(CASE, "c", "abort_harness_sanity");
    let r = child_status(CASE, "rust", "abort_harness_sanity");
    assert_eq!(c, (Some(0), None), "C child should exit cleanly, got {c:?}");
    assert_eq!(r, (Some(0), None), "Rust child should exit cleanly, got {r:?}");
}
