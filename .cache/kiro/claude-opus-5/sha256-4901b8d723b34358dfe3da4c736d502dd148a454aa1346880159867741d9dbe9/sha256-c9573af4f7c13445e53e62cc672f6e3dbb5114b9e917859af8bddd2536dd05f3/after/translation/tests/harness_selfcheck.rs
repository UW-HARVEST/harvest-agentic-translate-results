//! Negative controls for the differential harness itself.
//!
//! A harness that cannot fail proves nothing, so these tests assert that the
//! stdout/stderr capture really captures, and that `diff` really panics on a
//! divergence.

mod common;

use common::*;
use std::ffi::CString;

#[test]
fn capture_really_captures_stdout_from_the_libraries() {
    let _g = lock();
    let (c, r) = libs();
    env_clear_all();
    env_set("PROG_VERBOSE", "1");

    let (rc, out_c, err_c) = capture(|| unsafe { (c.envy)(1, 2, 3, 4) });
    let (rr, out_r, err_r) = capture(|| unsafe { (r.envy)(1, 2, 3, 4) });
    env_clear_all();

    assert_eq!(rc, rr);
    assert!(
        !out_c.is_empty(),
        "capture() returned no stdout for a verbose call — the harness is blind"
    );
    let text = String::from_utf8_lossy(&out_c);
    for expected in [
        "Verbose mode enabled",
        "Base offset: 64 (from octal 0100)",
        "Multiplier: 10 (from octal 012)",
        "Found colon at position: 6",
        "Final result:",
        "Configuration - Debug:",
    ] {
        assert!(text.contains(expected), "C stdout missing {expected:?}: {text}");
    }
    assert_eq!(out_c, out_r, "verbose stdout must match byte for byte");
    assert!(err_c.is_empty() && err_r.is_empty());
}

#[test]
fn capture_really_captures_stderr_from_the_libraries() {
    let _g = lock();
    let (c, r) = libs();
    env_clear_all();
    env_set("PROG_MULTIPLIER", "1,2");
    let name = CString::new("PROG_MULTIPLIER").unwrap();

    let (vc, out_c, err_c) = capture(|| unsafe { (c.parse_env_numeric)(name.as_ptr(), 99) });
    let (vr, out_r, err_r) = capture(|| unsafe { (r.parse_env_numeric)(name.as_ptr(), 99) });
    env_clear_all();

    assert_eq!(vc, 99);
    assert_eq!(vc, vr);
    assert_eq!(
        String::from_utf8_lossy(&err_c),
        "Warning: Invalid character in PROG_MULTIPLIER\n"
    );
    assert_eq!(err_c, err_r, "stderr must match byte for byte");
    assert!(out_c.is_empty() && out_r.is_empty());
}

#[test]
fn diff_detects_a_return_value_divergence() {
    let _g = lock();
    env_clear_all();
    // Deliberately make the two runs disagree: the C library is called first,
    // so the counter makes the second (Rust) run return different values. The
    // bump must be large enough to survive the `| 0x0F` in the pipeline.
    let seen = std::sync::atomic::AtomicUsize::new(0);
    let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        diff("negative control", |lib| {
            let bump = seen.fetch_add(1, std::sync::atomic::Ordering::SeqCst) as i32;
            vec![unsafe { (lib.envy)(1 + bump * 100_000, 2, 3, 4) } as i64]
        });
    }));
    assert!(
        res.is_err(),
        "diff() accepted diverging return values — the harness is broken"
    );
}

#[test]
fn diff_detects_a_stdout_divergence() {
    let _g = lock();
    env_clear_all();
    env_set("PROG_VERBOSE", "1");
    let seen = std::sync::atomic::AtomicUsize::new(0);
    let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        diff("negative control", |lib| {
            // Same return value both times, but a different number of verbose
            // lines printed, so only the stream comparison can catch it.
            let n = 1 + seen.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            let mut v = 0i64;
            for _ in 0..n {
                v = unsafe { (lib.envy)(1, 2, 3, 4) } as i64;
            }
            vec![v]
        });
    }));
    env_clear_all();
    assert!(
        res.is_err(),
        "diff() accepted diverging stdout — the stream comparison is broken"
    );
}

/// The flag-word encoding the tests use must be the same layout the compiled C
/// actually uses, otherwise every `perform_operation` / `apply_bit_operations`
/// row would be testing the wrong bits. Pin it against the C library itself:
/// `init_config_from_env` is documented to produce
/// verbose/debug/optimize from the environment, `cache_enabled = 1`,
/// `log_level = 03`, `reserved = 0`.
#[test]
fn bitfield_layout_matches_the_compiled_c() {
    let _g = lock();
    let (c, _r) = libs();

    env_clear_all();
    let mut storage: u32 = 0;
    unsafe { (c.init_config_from_env)(&mut storage) };
    assert_eq!(
        storage,
        flags_word(0, 0, 0, 1, 3, 0, 0),
        "cache_enabled=1, log_level=3 must land in bits 3 and 4..6"
    );

    env_set("PROG_VERBOSE", "1");
    let mut storage: u32 = 0;
    unsafe { (c.init_config_from_env)(&mut storage) };
    assert_eq!(storage, flags_word(1, 0, 0, 1, 3, 0, 0), "verbose is bit 0");

    env_set("PROG_DEBUG", "1");
    let mut storage: u32 = 0;
    unsafe { (c.init_config_from_env)(&mut storage) };
    assert_eq!(storage, flags_word(1, 1, 0, 1, 3, 0, 0), "debug is bit 1");

    env_set("PROG_OPTIMIZE", "x");
    let mut storage: u32 = 0;
    unsafe { (c.init_config_from_env)(&mut storage) };
    assert_eq!(storage, flags_word(1, 1, 1, 1, 3, 0, 0), "optimize is bit 2");

    // And the padding really is untouched.
    let mut storage: u32 = 0xFFFF_FFFF;
    unsafe { (c.init_config_from_env)(&mut storage) };
    assert_eq!(
        storage,
        0xFFFF_FF00 | flags_word(1, 1, 1, 1, 3, 0, 0),
        "the upper 24 bits must survive init_config_from_env"
    );
    env_clear_all();
}
