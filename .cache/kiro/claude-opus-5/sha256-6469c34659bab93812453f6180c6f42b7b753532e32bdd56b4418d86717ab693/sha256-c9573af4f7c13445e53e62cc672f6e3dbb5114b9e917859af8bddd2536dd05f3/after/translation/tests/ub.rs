//! Phase C (continued) — the rows of `ERRORS.md` whose trigger is *undefined
//! behaviour* in the C.
//!
//! Each case is executed in a forked child so the crash can be observed without
//! taking the test process down. The child does nothing but call the exported
//! symbol and `_exit`, so it never touches the allocator on the success path.
//!
//! What is asserted:
//!   * for the C `.so`: the documented outcome (fatal signal, or clean exit)
//!     actually happens — the "UB" label in `ERRORS.md` is measured, not assumed;
//!   * for the Rust `.so`: the outcome recorded in `ERRORS.md`. Where the C
//!     executes UB the Rust deliberately does not fault; see `ERRORS.md`
//!     § "Deliberate non-reproduction of UB".

mod common;

use std::ffi::c_int;

use common::*;

unsafe extern "C" {
    fn fork() -> i32;
    fn waitpid(pid: i32, status: *mut i32, options: i32) -> i32;
    fn _exit(code: i32) -> !;
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Outcome {
    Exited(i32),
    Signalled(i32),
}

impl Outcome {
    fn crashed(self) -> bool {
        matches!(self, Outcome::Signalled(_))
    }
}

/// Run `f` in a forked child and report how the child terminated.
fn probe(f: impl FnOnce()) -> Outcome {
    // Flush stdout/stderr before forking so buffered output is not duplicated.
    use std::io::Write;
    let _ = std::io::stdout().flush();
    let _ = std::io::stderr().flush();
    let pid = unsafe { fork() };
    assert!(pid >= 0, "fork() failed");
    if pid == 0 {
        f();
        unsafe { _exit(0) };
    }
    let mut status: i32 = 0;
    let r = unsafe { waitpid(pid, &mut status, 0) };
    assert_eq!(r, pid, "waitpid failed");
    // WIFEXITED / WEXITSTATUS / WTERMSIG, per <bits/waitstatus.h>.
    if status & 0x7f == 0x7f {
        Outcome::Signalled(status & 0x7f) // stopped; treat as signalled
    } else if status & 0x7f == 0 {
        Outcome::Exited((status >> 8) & 0xff)
    } else {
        Outcome::Signalled(status & 0x7f)
    }
}

#[track_caller]
fn assert_c_crashes(name: &str, o: Outcome) {
    assert!(
        o.crashed(),
        "{name}: expected the C .so to die on a fatal signal (UB), got {o:?}"
    );
}

#[track_caller]
fn assert_no_crash(name: &str, o: Outcome) {
    assert!(
        !o.crashed(),
        "{name}: expected a clean exit, got {o:?}"
    );
}

// ---------------------------------------------------------------------------
// Rows 3, 4 — `match` with `bins == 0`.
//
// MEASURED, not assumed: the zero-length VLA makes `differentiate` store
// `v[length-1]` == `v[-1]`, which lands exactly on the stack slot holding
// `preprocess`'s return address into `match` (the VLA base is `match`'s own
// `%rsp`, and `call preprocess` pushed the return address at `%rsp-8`).
// `preprocess`'s `ret` therefore jumps to address 0 and the C `.so` segfaults.
// ---------------------------------------------------------------------------

#[test]
fn ub_row03_match_bins_zero_crashes_in_c() {
    let c = c_lib();
    for &thr in &[0.0f64, 1.0, -1.0, f64::NAN] {
        let o = probe(|| {
            let mut t = vec![1.0f64; 8];
            let mut r = vec![2.0f64; 8];
            let v = unsafe { (c.r#match)(t.as_mut_ptr(), r.as_mut_ptr(), 0, thr) };
            std::hint::black_box(v);
        });
        assert_c_crashes(&format!("row03 C match(bins=0, thr={thr:?})"), o);
    }
}

#[test]
fn ub_row04_match_bins_zero_null_crashes_in_c() {
    let c = c_lib();
    let o = probe(|| {
        let v = unsafe { (c.r#match)(std::ptr::null_mut(), std::ptr::null_mut(), 0, 1.0) };
        std::hint::black_box(v);
    });
    assert_c_crashes("row04 C match(NULL, NULL, 0, 1.0)", o);
}

#[test]
fn ub_row03_04_match_bins_zero_rust_is_safe() {
    // The Rust translation degenerates to empty buffers and returns
    // `(0.0 >= threshold)` instead of corrupting its own return address.
    let rust = rust_lib();
    for &thr in SPECIAL_THRESHOLDS {
        let expected = (0.0f64 >= thr) as c_int;
        let o = probe(|| {
            let got =
                unsafe { (rust.r#match)(std::ptr::null_mut(), std::ptr::null_mut(), 0, thr) };
            unsafe { _exit(if got == expected { 0 } else { 42 }) };
        });
        assert_eq!(
            o,
            Outcome::Exited(0),
            "row03/04 Rust match(bins=0, thr={thr:?}) should return {expected} without faulting"
        );
    }
}

// ---------------------------------------------------------------------------
// Row 5 — `match` with `bins < 0`: negative-size VLA, then
// `memcpy(v, source, (size_t)length * 8)` ≈ 2^64 bytes.
// ---------------------------------------------------------------------------

#[test]
fn ub_row05_match_negative_bins_crashes_in_c() {
    let c = c_lib();
    for bins in [-1 as c_int, -2, -17, -100000, c_int::MIN] {
        let o = probe(|| {
            let mut t = vec![1.0f64; 64];
            let mut r = vec![2.0f64; 64];
            let v = unsafe { (c.r#match)(t.as_mut_ptr(), r.as_mut_ptr(), bins, 0.5) };
            std::hint::black_box(v);
        });
        assert_c_crashes(&format!("row05 C match(bins={bins})"), o);
    }
}

#[test]
fn ub_row05_match_negative_bins_rust_is_safe() {
    let rust = rust_lib();
    for bins in [-1 as c_int, -2, -17, -100000, c_int::MIN] {
        let o = probe(|| {
            let mut t = vec![1.0f64; 64];
            let mut r = vec![2.0f64; 64];
            let got = unsafe { (rust.r#match)(t.as_mut_ptr(), r.as_mut_ptr(), bins, 0.5) };
            unsafe { _exit(if got == 0 { 0 } else { 42 }) };
        });
        assert_eq!(
            o,
            Outcome::Exited(0),
            "row05 Rust match(bins={bins}) should return 0 without faulting"
        );
    }
}

// ---------------------------------------------------------------------------
// Rows 6, 7 — `match` with a NULL buffer and `bins > 0`.
// ---------------------------------------------------------------------------

#[test]
fn ub_row06_07_match_null_buffers_crash_in_c() {
    let c = c_lib();
    let cases: [(&str, bool, bool); 3] = [
        ("test=NULL", true, false),
        ("reference=NULL", false, true),
        ("both=NULL", true, true),
    ];
    for (name, tnull, rnull) in cases {
        let o = probe(|| {
            let mut t = vec![1.0f64; 32];
            let mut r = vec![2.0f64; 32];
            let tp = if tnull { std::ptr::null_mut() } else { t.as_mut_ptr() };
            let rp = if rnull { std::ptr::null_mut() } else { r.as_mut_ptr() };
            let v = unsafe { (c.r#match)(tp, rp, 32, 0.5) };
            std::hint::black_box(v);
        });
        assert_c_crashes(&format!("row06/07 C match({name}, bins=32)"), o);
    }
}

// ---------------------------------------------------------------------------
// Row 8 — `match` with an oversized `bins`: the VLA blows the stack rlimit and
// there is no probe or check anywhere in the C.
// ---------------------------------------------------------------------------

#[test]
fn ub_row08_match_oversized_bins_crashes_in_c() {
    let c = c_lib();
    for bins in [1 << 24 as c_int, 1 << 28, c_int::MAX] {
        let o = probe(|| {
            // The buffers only need to be readable; the crash happens while
            // reserving the VLAs / walking off the stack.
            let mut t = vec![1.0f64; 1024];
            let mut r = vec![2.0f64; 1024];
            let v = unsafe { (c.r#match)(t.as_mut_ptr(), r.as_mut_ptr(), bins, 0.5) };
            std::hint::black_box(v);
        });
        assert_c_crashes(&format!("row08 C match(bins={bins})"), o);
    }
}

// ---------------------------------------------------------------------------
// Row 14 — `spectral_contrast` with a NULL buffer and `length > 0`, and with a
// length that runs off the end of a valid buffer.
// ---------------------------------------------------------------------------

#[test]
fn ub_row14_spectral_null_or_oversized_crashes_in_c() {
    let c = c_lib();
    for len in [1 as c_int, 2, 1024] {
        let o = probe(|| {
            let v = unsafe {
                (c.spectral_contrast)(std::ptr::null_mut(), std::ptr::null_mut(), len)
            };
            std::hint::black_box(v);
        });
        assert_c_crashes(&format!("row14 C spectral_contrast(NULL, NULL, {len})"), o);
    }
    for len in [1 << 24 as c_int, c_int::MAX] {
        let o = probe(|| {
            let mut a = vec![1.0f32; 16];
            let mut b = vec![2.0f32; 16];
            let v = unsafe { (c.spectral_contrast)(a.as_mut_ptr(), b.as_mut_ptr(), len) };
            std::hint::black_box(v);
        });
        assert_c_crashes(&format!("row14 C spectral_contrast(len={len} past end)"), o);
    }
}

// ---------------------------------------------------------------------------
// Control: the *defined* boundary cases must NOT crash either library, so the
// probe harness is proven able to distinguish the two outcomes.
// ---------------------------------------------------------------------------

#[test]
fn ub_control_defined_cases_do_not_crash() {
    let p = pair();
    for len in [0 as c_int, -1, c_int::MIN] {
        assert_no_crash(
            &format!("control C spectral_contrast(NULL, NULL, {len})"),
            probe(|| {
                let v = unsafe {
                    (p.c.spectral_contrast)(std::ptr::null_mut(), std::ptr::null_mut(), len)
                };
                std::hint::black_box(v);
            }),
        );
        assert_no_crash(
            &format!("control Rust spectral_contrast(NULL, NULL, {len})"),
            probe(|| {
                let v = unsafe {
                    (p.rust.spectral_contrast)(std::ptr::null_mut(), std::ptr::null_mut(), len)
                };
                std::hint::black_box(v);
            }),
        );
    }
    for bins in [1 as c_int, 16, 17, 1024] {
        assert_no_crash(
            &format!("control C match(bins={bins})"),
            probe(|| {
                let mut t = vec![1.0f64; bins as usize];
                let mut r = vec![2.0f64; bins as usize];
                let v = unsafe { (p.c.r#match)(t.as_mut_ptr(), r.as_mut_ptr(), bins, 0.5) };
                std::hint::black_box(v);
            }),
        );
        assert_no_crash(
            &format!("control Rust match(bins={bins})"),
            probe(|| {
                let mut t = vec![1.0f64; bins as usize];
                let mut r = vec![2.0f64; bins as usize];
                let v = unsafe { (p.rust.r#match)(t.as_mut_ptr(), r.as_mut_ptr(), bins, 0.5) };
                std::hint::black_box(v);
            }),
        );
    }
}
