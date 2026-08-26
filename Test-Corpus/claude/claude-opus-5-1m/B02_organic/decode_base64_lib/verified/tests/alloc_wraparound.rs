//! Phase C — the allocation-failure rejection branches reached *deterministically*
//! through the C's own integer arithmetic (`ERRORS.md` rows 3, 3b and 4).
//!
//! `int l = strlen(src) + 1;` (`lib.c:49`) truncates to `int`. For
//! `strlen(src) == 4294967282` that makes `l == -13`, so
//!
//!   * `calloc(sizeof(char), l + 13)` becomes `calloc(1, 0)` — which SUCCEEDS,
//!   * and the following `malloc(l)` converts `-13` to `(size_t)0xFFFF…F3` —
//!     which ALWAYS fails.
//!
//! i.e. row 4 (`free(dest); return NULL`) is reached with no environment
//! trickery at all. `l <= -14` instead makes `l + 13` negative, so `calloc`
//! itself gets a huge `size_t` and fails: row 3.
//!
//! These tests need multi-GiB buffers, which is why they live in their own test
//! binary: freeing such a buffer leaves a huge reusable hole in the heap that
//! would perturb the `RLIMIT_AS` tests in `alloc_failure.rs`.

mod common;

use common::diff_null;
use std::ffi::c_char;

/// The tests in this file allocate multi-GiB buffers, so they must not run
/// concurrently with each other (the host's `RLIMIT_DATA` is 6 GiB). Cargo runs
/// test *binaries* sequentially but the tests *inside* one binary in parallel,
/// hence this guard: the suite stays correct at any `--test-threads` setting.
static BIG_ALLOC: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn big_alloc_guard() -> std::sync::MutexGuard<'static, ()> {
    BIG_ALLOC.lock().unwrap_or_else(|e| e.into_inner())
}

fn mem_available_bytes() -> u64 {
    let s = std::fs::read_to_string("/proc/meminfo").unwrap_or_default();
    for line in s.lines() {
        if let Some(rest) = line.strip_prefix("MemAvailable:") {
            if let Some(kb) = rest.split_whitespace().next() {
                if let Ok(v) = kb.parse::<u64>() {
                    return v * 1024;
                }
            }
        }
    }
    0
}


/// Allocate a buffer big enough to give `strlen(src)` values around 2^32, then
/// walk the interesting NUL positions. One buffer is reused for all of them.
fn with_wraparound_buffer(f: impl Fn(&mut Vec<u8>)) {
    let _serialize = big_alloc_guard();
    let want = 4_294_967_296u64; // 4 GiB
    let avail = mem_available_bytes();
    assert!(
        avail > want + (2 << 30),
        "not enough free memory for the 4 GiB integer-wraparound test \
         ({} MiB available, need ~{} MiB)",
        avail >> 20,
        (want + (2 << 30)) >> 20
    );
    let mut buf = vec![b'A'; want as usize - 1]; // len == 4294967295
    f(&mut buf);
}

/// ERRORS.md row 4 (deterministic) — `strlen(src) == 4294967282` makes
/// `l == -13`, so `calloc(1, l + 13) == calloc(1, 0)` succeeds and
/// `malloc((size_t)-13)` fails: the C takes the `free(dest); return NULL`
/// branch. Also covers `l == -12 .. -1` (`calloc` sizes 1..12).
#[test]
fn e4_malloc_fails_after_calloc_succeeds_deterministic() {
    with_wraparound_buffer(|buf| {
        // l == -13 .. -1  =>  calloc(1, 0 .. 12) succeeds, malloc(l) is huge
        for l in -13i64..=-1 {
            let strlen = (l - 1).rem_euclid(4_294_967_296) as usize;
            assert!(strlen < buf.len(), "buffer too small for strlen {strlen}");
            let saved = buf[strlen];
            buf[strlen] = 0;
            let label = format!(
                "ERRORS row 4 deterministic (strlen={strlen} -> l={l}, calloc(1,{}), malloc({l}))",
                l + 13
            );
            diff_null(buf.as_ptr() as *const c_char, &label);
            buf[strlen] = saved;
        }
    });
}

/// ERRORS.md row 3 (deterministic) — the same wrap-around, but with `l + 13`
/// still negative (`l <= -14`), so `calloc` itself gets a huge `size_t` and
/// fails before `malloc` is ever reached.
#[test]
fn e3_calloc_fails_deterministic() {
    with_wraparound_buffer(|buf| {
        for l in [-14i64, -15, -20, -100, -4096] {
            let strlen = (l - 1).rem_euclid(4_294_967_296) as usize;
            assert!(strlen < buf.len(), "buffer too small for strlen {strlen}");
            let saved = buf[strlen];
            buf[strlen] = 0;
            let label =
                format!("ERRORS row 3 deterministic (strlen={strlen} -> l={l}, calloc size huge)");
            diff_null(buf.as_ptr() as *const c_char, &label);
            buf[strlen] = saved;
        }
    });
}

/// ERRORS.md row 3b — `strlen` at/near `INT_MAX`, where `l` or `l + 13`
/// overflows `int` and the implicit conversion to `size_t` yields a huge value.
#[test]
fn e3b_int_overflow_makes_calloc_fail() {
    let _serialize = big_alloc_guard();
    let int_max = i32::MAX as usize; // 2147483647
    let need = int_max as u64 + 4096;
    assert!(
        mem_available_bytes() >= need + (1 << 30),
        "not enough free memory ({} MiB available) for the 2 GiB oversized-input test",
        mem_available_bytes() >> 20
    );
    let mut buf = vec![b'A'; int_max + 1];
    for (name, len) in [
        ("strlen == INT_MAX (l wraps to INT_MIN)", int_max),
        ("strlen == INT_MAX - 12 (l + 13 overflows)", int_max - 12),
        ("strlen == INT_MAX - 1", int_max - 1),
        ("strlen == INT_MAX - 11", int_max - 11),
    ] {
        buf[len] = 0;
        let label = format!("ERRORS row 3b ({name})");
        diff_null(buf.as_ptr() as *const c_char, &label);
        buf[len] = b'A';
    }
}

