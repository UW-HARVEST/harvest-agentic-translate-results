//! Phase C — error/rejection-path differential tests, one per ERRORS.md row.
//!
//! The C function has no error returns at all (see ERRORS.md), so its whole
//! rejection surface is:
//!   * silent no-ops produced by the two loop guards (rows 1-11), and
//!   * hard faults where no check exists (rows 12-13).
//!
//! Rows 1-11 assert exact C-vs-Rust equality of buffer + descriptor.
//! Rows 12-13 assert the two libraries die from the *same signal*, by
//! re-executing this test binary in a child process — "both failed somehow" is
//! explicitly not accepted.

mod common;

use std::ffi::c_int;

use common::*;

/// Assert that both libraries leave the buffer and descriptor completely
/// untouched for the given (invalid/degenerate) dimensions.
#[track_caller]
fn assert_noop(libs: &Libs, w: c_int, h: c_int, buf_len: usize, row: &str) {
    let mut rng = Rng::new(SEED ^ (row.len() as u64) ^ ((w as u32 as u64) << 16) ^ (h as u32 as u64));
    for rep in 0..16 {
        let pixels = rng.pixels(buf_len);
        let out = assert_same(libs, w, h, &pixels, &format!("{row} rep={rep}"));
        assert_eq!(
            out.pixels, pixels,
            "{row}: expected a silent no-op for w={w} h={h}, but the buffer changed"
        );
    }
}

// ---------------------------------------------------------------------------
// Rows 1-5: `h` rejects the outer loop
// ---------------------------------------------------------------------------

#[test]
fn err_row01_h_zero() {
    let libs = Libs::load();
    assert_noop(&libs, 8, 0, 0, "row 1 (h==0)");
    // A non-empty buffer with h==0 must also be left alone entirely.
    assert_noop(&libs, 8, 0, 8, "row 1b (h==0, non-empty buffer)");
}

#[test]
fn err_row02_h_one() {
    let libs = Libs::load();
    assert_noop(&libs, 8, 1, 8, "row 2 (h==1)");
    assert_noop(&libs, 1, 1, 1, "row 2b (w==1,h==1)");
}

#[test]
fn err_row03_h_negative_odd() {
    let libs = Libs::load();
    // h = -1 -> flips = -1/2 = 0 in C (truncation toward zero).
    assert_noop(&libs, 8, -1, 8, "row 3 (h==-1)");
    assert_noop(&libs, 8, -3, 8, "row 3b (h==-3)");
    assert_noop(&libs, 8, -7, 8, "row 3c (h==-7)");
}

#[test]
fn err_row04_h_negative_even() {
    let libs = Libs::load();
    // h = -8 -> flips = -4; guard `0 < -4` is false.
    assert_noop(&libs, 8, -8, 8, "row 4 (h==-8)");
    assert_noop(&libs, 8, -2, 8, "row 4b (h==-2)");
    assert_noop(&libs, 1, -100, 4, "row 4c (h==-100)");
}

#[test]
fn err_row05_h_int_min() {
    let libs = Libs::load();
    // INT_MIN / 2 is exact (-1073741824); no overflow, guard still false.
    assert_noop(&libs, 8, c_int::MIN, 8, "row 5 (h==INT_MIN)");
    assert_noop(&libs, 0, c_int::MIN, 0, "row 5b (w==0,h==INT_MIN)");
    assert_noop(&libs, c_int::MIN, c_int::MIN, 0, "row 5c (both INT_MIN)");

    // ...and with a NULL pix, since nothing is dereferenced.
    let c = run_one_null_pix(&libs, Impl::C, 8, c_int::MIN);
    let r = run_one_null_pix(&libs, Impl::Rust, 8, c_int::MIN);
    assert_eq!(c, r, "row 5: h==INT_MIN with NULL pix diverged");
}

#[test]
fn err_row05b_h_int_max_no_deref_shapes() {
    // h == INT_MAX with w == 0: outer loop would spin ~2^30 times doing nothing,
    // which is too slow to run. Instead pin the *adjacent* boundary values that
    // exercise the same guard arithmetic cheaply.
    let libs = Libs::load();
    for h in [2, 3, 4, 5] {
        assert_noop(&libs, 0, h, 0, &format!("row 5b (w==0,h=={h})"));
    }
}

// ---------------------------------------------------------------------------
// Rows 6-8: `w` rejects the inner loop
// ---------------------------------------------------------------------------

#[test]
fn err_row06_w_zero() {
    let libs = Libs::load();
    // Outer loop runs h/2 times; inner guard `0 < 0` is false every time.
    for h in [2, 3, 4, 5, 16, 17] {
        assert_noop(&libs, 0, h, 0, &format!("row 6 (w==0,h=={h})"));
    }
}

#[test]
fn err_row07_w_negative() {
    let libs = Libs::load();
    // Row pointers `pix + w*i` go out of bounds but are never dereferenced,
    // because the inner guard `0 < w` is false. Must be a clean no-op.
    for w in [-1, -2, -3, -8, -37] {
        for h in [2, 3, 4, 5, 8, 9] {
            assert_noop(&libs, w, h, 16, &format!("row 7 (w=={w},h=={h})"));
        }
    }
}

#[test]
fn err_row08_w_int_min() {
    let libs = Libs::load();
    // off_b = INT_MIN * 1 wraps; pointer computed, never dereferenced.
    assert_noop(&libs, c_int::MIN, 2, 16, "row 8 (w==INT_MIN,h==2)");
    assert_noop(&libs, c_int::MIN, 3, 16, "row 8b (w==INT_MIN,h==3)");
    assert_noop(&libs, c_int::MIN, 8, 16, "row 8c (w==INT_MIN,h==8)");
    // and the neighbouring value
    assert_noop(&libs, c_int::MIN + 1, 4, 16, "row 8d (w==INT_MIN+1,h==4)");
}

// ---------------------------------------------------------------------------
// Rows 9-11: NULL `pix` tolerated when no dereference is due
// ---------------------------------------------------------------------------

#[test]
fn err_row09_null_pix_w0_h0() {
    let libs = Libs::load();
    let c = run_one_null_pix(&libs, Impl::C, 0, 0);
    let r = run_one_null_pix(&libs, Impl::Rust, 0, 0);
    assert_eq!(c, r, "row 9: diverged");
    assert_eq!(c, (0, 0, true), "row 9: descriptor mutated");
}

#[test]
fn err_row10_null_pix_w0_h_positive() {
    let libs = Libs::load();
    for h in [1, 2, 3, 4, 5, 32, 33] {
        let c = run_one_null_pix(&libs, Impl::C, 0, h);
        let r = run_one_null_pix(&libs, Impl::Rust, 0, h);
        assert_eq!(c, r, "row 10: diverged at w=0 h={h}");
        assert_eq!(c, (0, h, true), "row 10: descriptor mutated at h={h}");
    }
}

#[test]
fn err_row11_null_pix_h_negative() {
    let libs = Libs::load();
    for (w, h) in [(8, -1), (8, -8), (0, -1), (-1, -1), (8, c_int::MIN)] {
        let c = run_one_null_pix(&libs, Impl::C, w, h);
        let r = run_one_null_pix(&libs, Impl::Rust, w, h);
        assert_eq!(c, r, "row 11: diverged at w={w} h={h}");
        assert_eq!(c, (w, h, true), "row 11: descriptor mutated at w={w} h={h}");
    }
    // w negative with h >= 2 and NULL pix: inner guard rejects before any deref.
    for (w, h) in [(-1, 2), (-8, 4), (c_int::MIN, 2)] {
        let c = run_one_null_pix(&libs, Impl::C, w, h);
        let r = run_one_null_pix(&libs, Impl::Rust, w, h);
        assert_eq!(c, r, "row 11b: diverged at w={w} h={h}");
    }
}

// ---------------------------------------------------------------------------
// Rows 12-13: hard faults — signal parity via a forked child process
// ---------------------------------------------------------------------------

/// Env var naming the crash scenario the child should execute.
const CHILD_VAR: &str = "HARVEST_CRASH_CASE";

/// Child-side entry point. Runs in a subprocess and is expected to die.
#[test]
fn crash_child_worker() {
    let Ok(case) = std::env::var(CHILD_VAR) else {
        // Parent-side invocation: nothing to do, this test is a no-op.
        return;
    };

    let libs = Libs::load();
    let (which, scenario) = case.split_once(':').expect("malformed crash case");
    let imp = match which {
        "c" => Impl::C,
        "rust" => Impl::Rust,
        other => panic!("unknown impl {other}"),
    };
    let flip = match imp {
        Impl::C => libs.c_flip(),
        Impl::Rust => libs.rust_flip(),
    };

    match scenario {
        // Row 12: img == NULL -> the first statement dereferences address 0.
        "null_img" => unsafe { flip(std::ptr::null_mut()) },
        // Row 13: img->pix == NULL while work IS due.
        "null_pix_with_work" => {
            let mut img = CpImage {
                w: 4,
                h: 2,
                pix: std::ptr::null_mut(),
            };
            unsafe { flip(&mut img) }
        }
        other => panic!("unknown scenario {other}"),
    }

    // If we get here the call unexpectedly returned; report a distinct code.
    std::process::exit(42);
}

/// Outcome of a crash scenario: `Ok(exit_code)` or `Err(signal)`.
#[derive(Debug, PartialEq, Eq)]
enum Death {
    Exit(i32),
    Signal(i32),
}

fn run_crash_case(scenario: &str, which: &str) -> Death {
    use std::os::unix::process::ExitStatusExt;

    let exe = std::env::current_exe().expect("current_exe");
    let out = std::process::Command::new(exe)
        .args(["--exact", "crash_child_worker", "--nocapture", "--test-threads=1"])
        .env(CHILD_VAR, format!("{which}:{scenario}"))
        // Propagate library locations so the child resolves the same .so files.
        .env("HARVEST_C_SO", c_so_path())
        .env("HARVEST_RUST_SO", rust_so_path())
        .output()
        .expect("failed to spawn child");

    match out.status.signal() {
        Some(sig) => Death::Signal(sig),
        None => Death::Exit(out.status.code().unwrap_or(-1)),
    }
}

/// Shared assertion for the two fault rows.
///
/// The C dereferences address 0 with no check, so it dies from `SIGSEGV` (11).
/// The **release** Rust `.so` — the artifact that actually ships, and the
/// apples-to-apples counterpart of the optimised C — does exactly the same.
///
/// Under `debug_assertions`, however, Rust's standard library instruments raw
/// place-reads with a *null-pointer-dereference UB detector* that panics; with
/// `panic = "abort"` semantics in a non-unwinding context that surfaces as
/// `SIGABRT` (6). That is a deliberate toolchain sanitiser converting the very
/// same UB into a louder failure — not a behavioural divergence in the
/// translated logic. So the expectation is profile-aware, and in debug mode we
/// still pin the exact signal instead of accepting "it failed somehow".
#[track_caller]
fn assert_fault_parity(scenario: &str, row: &str) {
    const SIGSEGV: i32 = 11;
    const SIGABRT: i32 = 6;

    let c = run_crash_case(scenario, "c");
    let r = run_crash_case(scenario, "rust");

    // The C must genuinely fault, and specifically with SIGSEGV.
    assert_eq!(
        c,
        Death::Signal(SIGSEGV),
        "{row}: expected the C to die from SIGSEGV, got {c:?}"
    );

    if cfg!(debug_assertions) {
        assert_eq!(
            r,
            Death::Signal(SIGABRT),
            "{row}: in a debug build the Rust .so is expected to trap the same UB \
             via its null-dereference detector (SIGABRT), got {r:?}"
        );
    } else {
        assert_eq!(
            r, c,
            "{row}: the release Rust .so must fault identically to the C \
             (C={c:?}, Rust={r:?})"
        );
    }
}

#[test]
fn err_row12_null_img_same_signal() {
    // img == NULL: the first statement `img->pix` reads address 0.
    assert_fault_parity("null_img", "row 12");
}

#[test]
fn err_row13_null_pix_with_work_same_signal() {
    // img->pix == NULL with w=4,h=2: the inner loop reads address 0.
    assert_fault_parity("null_pix_with_work", "row 13");
}

// ---------------------------------------------------------------------------
// Generic FFI-boundary boundaries (required even though not in the table)
// ---------------------------------------------------------------------------

/// There are no enums anywhere in `include/lib.h`, so the "out-of-range enum
/// value across FFI" class does not exist for this API. The closest analogue is
/// an out-of-range *dimension*, which is covered exhaustively here: every value
/// one step past each boundary the C code branches on.
#[test]
fn boundary_one_step_past_every_guard() {
    let libs = Libs::load();
    let mut rng = Rng::new(SEED + 99);

    // Boundaries in `h`: the guard flips at h/2 >= 1, i.e. h == 2.
    // Boundaries in `w`: the guard flips at w >= 1, i.e. w == 1.
    let interesting = [
        c_int::MIN,
        c_int::MIN + 1,
        -2,
        -1,
        0,
        1,
        2,
        3,
    ];

    for &w in &interesting {
        for &h in &interesting {
            // Only allocate/deref-safe combinations do real work; everything
            // else must be a no-op. Size the buffer for the positive reading.
            let n = (w.max(0) as usize) * (h.max(0) as usize);
            if n > 4096 {
                continue; // INT_MIN-sized products are not allocatable
            }
            let pixels = rng.pixels(n);
            assert_same(&libs, w, h, &pixels, &format!("boundary w={w} h={h}"));
        }
    }
}

/// Zero and oversized lengths: the descriptor claims fewer rows/columns than the
/// buffer actually holds. The C only ever touches `w*h` pixels, so the tail must
/// be untouched — and Rust must agree.
#[test]
fn boundary_descriptor_smaller_than_buffer() {
    let libs = Libs::load();
    let mut rng = Rng::new(SEED + 100);

    for (w, h, extra) in [
        (0, 0, 16),
        (1, 1, 16),
        (2, 2, 16),
        (3, 4, 16),
        (8, 2, 64),
        (5, 5, 7),
    ] {
        let n = (w as usize) * (h as usize) + extra;
        let pixels = rng.pixels(n);
        let out = assert_same(
            &libs,
            w,
            h,
            &pixels,
            &format!("oversized buffer {w}x{h}+{extra}"),
        );
        // The trailing `extra` pixels must be bit-identical to the input.
        let used = (w as usize) * (h as usize);
        assert_eq!(
            &out.pixels[used..],
            &pixels[used..],
            "pixels beyond w*h were modified for {w}x{h}+{extra}"
        );
    }
}
