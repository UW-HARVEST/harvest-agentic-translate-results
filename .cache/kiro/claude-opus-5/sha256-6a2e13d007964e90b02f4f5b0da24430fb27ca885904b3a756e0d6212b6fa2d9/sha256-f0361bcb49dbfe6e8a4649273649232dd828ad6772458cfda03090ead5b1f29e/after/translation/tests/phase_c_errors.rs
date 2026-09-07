//! Phase C — error/rejection-path differential tests.
//!
//! One test per row of `ERRORS.md`. The C library has no error channel at all
//! (`void` return, zero `if`/`return`/`assert`), so a "rejection" is observable
//! only as either
//!   * OK/NOP — returns cleanly, buffer untouched, or
//!   * FAULT  — the process dies from a signal.
//!
//! Both outcomes are compared between the two `.so`s. FAULT rows are run in
//! child processes so the *termination signal* itself can be compared, rather
//! than settling for "both failed somehow".

mod common;

use common::*;
use std::os::unix::process::ExitStatusExt;
use std::process::Command;

/// Call both implementations with a caller-supplied `pix` (possibly null) and
/// assert both return normally and agree on the buffer + struct.
#[track_caller]
fn assert_both_nop_with_pix(impls: &Impls, w: i32, h: i32, pix: *mut u8, label: &str) {
    let mut c_img = CpImage { w, h, pix };
    let mut rust_img = CpImage { w, h, pix };
    unsafe {
        (impls.c)(&mut c_img);
        (impls.rust)(&mut rust_img);
    }
    assert_eq!(c_img.w, rust_img.w, "{label}: w diverges");
    assert_eq!(c_img.h, rust_img.h, "{label}: h diverges");
    assert!(
        std::ptr::eq(c_img.pix, rust_img.pix),
        "{label}: pix diverges"
    );
    assert_eq!(c_img.w, w, "{label}: C mutated w");
    assert_eq!(c_img.h, h, "{label}: C mutated h");
    assert!(std::ptr::eq(c_img.pix, pix), "{label}: C mutated pix");
    assert_eq!(rust_img.w, w, "{label}: Rust mutated w");
    assert_eq!(rust_img.h, h, "{label}: Rust mutated h");
    assert!(std::ptr::eq(rust_img.pix, pix), "{label}: Rust mutated pix");
}

/// Assert both implementations leave a real buffer bit-identical and untouched.
#[track_caller]
fn assert_both_untouched(impls: &Impls, w: i32, h: i32, label: &str) {
    let mut rng = Rng::new(SEED ^ 0xC0DE ^ (w as u32 as u64) ^ ((h as u32 as u64) << 32));
    for _ in 0..16 {
        let original = Buffer::randomized(pixel_count(w, h), &mut rng);
        let r = run_both(impls, w, h, &original);
        assert_eq!(r.c_bytes, r.rust_bytes, "{label}: C vs Rust buffer diverges");
        assert_eq!(
            r.c_bytes, original.bytes,
            "{label}: C did NOT leave the buffer untouched (ground truth changed)"
        );
        assert_eq!(
            r.rust_bytes, original.bytes,
            "{label}: Rust did not leave the buffer untouched"
        );
    }
}

// ---------------------------------------------------------------------------
// Child-process plumbing for the FAULT rows.
// ---------------------------------------------------------------------------

const ENV_CASE: &str = "HARVEST_FAULT_CASE";
const ENV_IMPL: &str = "HARVEST_FAULT_IMPL";
const CHILD_TEST: &str = "phase_c_fault_child_entry";

#[derive(Debug, PartialEq, Eq)]
struct Outcome {
    signal: Option<i32>,
    code: Option<i32>,
}

fn run_case_in_child(case: &str, which: &str) -> Outcome {
    let exe = std::env::current_exe().expect("current_exe");
    let out = Command::new(exe)
        .arg(CHILD_TEST)
        .arg("--exact")
        .arg("--test-threads=1")
        .env(ENV_CASE, case)
        .env(ENV_IMPL, which)
        .env("RUST_BACKTRACE", "0")
        .output()
        .expect("spawn child");
    Outcome {
        signal: out.status.signal(),
        code: out.status.code(),
    }
}

/// Run one FAULT case under both implementations and require identical
/// termination: same signal, or same exit code.
///
/// When the harness has been pointed at a non-default (debug) Rust artifact via
/// `HARVEST_RUST_SO`, Rust's null-pointer / "unsafe precondition" UB checks are
/// compiled in and convert a null dereference into a controlled abort (SIGABRT)
/// instead of a fault (SIGSEGV). That is a Rust debugging feature rather than
/// translated behavior, so it is accepted there — and only there — as a known,
/// explicitly named difference. The default (release, shipped) artifact is held
/// to exact signal equality.
#[track_caller]
fn assert_same_fault(case: &str, label: &str) {
    let c = run_case_in_child(case, "c");
    let r = run_case_in_child(case, "rust");

    if c != r {
        let ub_checked_build = std::env::var("HARVEST_RUST_SO").is_ok();
        let is_ub_check_abort = c.signal == Some(11) && r.signal == Some(6);
        assert!(
            ub_checked_build && is_ub_check_abort,
            "{label}: case '{case}' terminated differently — C={c:?} Rust={r:?}"
        );
        eprintln!(
            "{label}: case '{case}': C SIGSEGV vs Rust SIGABRT under a \
             UB-checked (debug) artifact — accepted, see ERRORS.md"
        );
        return;
    }

    assert!(
        c.signal.is_some() || c.code == Some(0),
        "{label}: case '{case}' gave an inconclusive outcome {c:?}"
    );
}

/// The child side. A no-op unless the parent set the environment variables.
#[test]
fn phase_c_fault_child_entry() {
    let (case, which) = match (std::env::var(ENV_CASE), std::env::var(ENV_IMPL)) {
        (Ok(c), Ok(i)) => (c, i),
        _ => return, // ordinary parent-side run: nothing to do
    };

    let impls = load_impls();
    let f = match which.as_str() {
        "c" => impls.c,
        "rust" => impls.rust,
        other => panic!("unknown impl '{other}'"),
    };

    match case.as_str() {
        // ERRORS.md row 1: img == NULL.
        "null_img" => unsafe {
            f(std::ptr::null_mut());
        },
        // ERRORS.md row 2: pix == NULL with a shape that actually swaps.
        "null_pix_active" => unsafe {
            let mut img = CpImage {
                w: 1,
                h: 2,
                pix: std::ptr::null_mut(),
            };
            f(&mut img);
        },
        // ERRORS.md row 2 variant: wider rows.
        "null_pix_active_wide" => unsafe {
            let mut img = CpImage {
                w: 16,
                h: 4,
                pix: std::ptr::null_mut(),
            };
            f(&mut img);
        },
        // ERRORS.md row 11: `w * (h - i - 1)` overflows `int`. The wrapped
        // (negative) offset is dereferenced, exactly as the C does.
        "offset_overflow" => unsafe {
            let mut buf = vec![0u8; 4096];
            let mut img = CpImage {
                w: 65536,
                h: 65536,
                pix: buf.as_mut_ptr(),
            };
            f(&mut img);
            std::hint::black_box(&mut buf);
        },
        other => panic!("unknown case '{other}'"),
    }

    // Survived: report success unambiguously without the harness's own output.
    std::process::exit(0);
}

// ---------------------------------------------------------------------------
// ERRORS.md rows
// ---------------------------------------------------------------------------

/// Row 1: `img == NULL` — unconditional `img->pix` load, no null check.
#[test]
fn phase_c_row01_null_img_faults_identically() {
    assert_same_fault("null_img", "ERRORS row 1 (img == NULL)");
}

/// Row 2: `img->pix == NULL` with `h >= 2 && w >= 1` — the swap loop runs and
/// dereferences NULL.
#[test]
fn phase_c_row02_null_pix_active_faults_identically() {
    assert_same_fault("null_pix_active", "ERRORS row 2 (pix == NULL, w=1 h=2)");
    assert_same_fault(
        "null_pix_active_wide",
        "ERRORS row 2 (pix == NULL, w=16 h=4)",
    );
}

/// Row 3: `img->pix == NULL` but `h < 2` — `pix` is loaded and never
/// dereferenced, so this must NOT fault in either implementation.
#[test]
fn phase_c_row03_null_pix_inactive_h_lt_2() {
    let impls = load_impls();
    for h in [i32::MIN, -3, -1, 0, 1] {
        for w in [i32::MIN, -1, 0, 1, 7, i32::MAX] {
            assert_both_nop_with_pix(
                &impls,
                w,
                h,
                std::ptr::null_mut(),
                "ERRORS row 3 (pix == NULL, h < 2)",
            );
        }
    }
}

/// Row 4: `img->pix == NULL` with `w == 0` — the outer loop runs but the inner
/// loop body never executes, so no dereference occurs for any `h`.
#[test]
fn phase_c_row04_null_pix_w_zero() {
    let impls = load_impls();
    for h in [0i32, 1, 2, 3, 8, 9, 1024, 1025] {
        assert_both_nop_with_pix(
            &impls,
            0,
            h,
            std::ptr::null_mut(),
            "ERRORS row 4 (pix == NULL, w == 0)",
        );
    }
    // Same for negative `w`, which also skips the inner loop.
    for w in [-1i32, -9, i32::MIN] {
        for h in [2i32, 3, 64] {
            assert_both_nop_with_pix(
                &impls,
                w,
                h,
                std::ptr::null_mut(),
                "ERRORS row 4 (pix == NULL, w < 0)",
            );
        }
    }
}

/// Row 5: `h == 0`.
#[test]
fn phase_c_row05_h_zero() {
    let impls = load_impls();
    for w in [0i32, 1, 3, 64] {
        assert_both_untouched(&impls, w, 0, "ERRORS row 5 (h == 0)");
    }
}

/// Row 6: `h == 1` — one step below the smallest `h` that does any work.
#[test]
fn phase_c_row06_h_one() {
    let impls = load_impls();
    for w in [0i32, 1, 3, 64] {
        assert_both_untouched(&impls, w, 1, "ERRORS row 6 (h == 1)");
    }
}

/// Row 7: `h < 0` — C division truncates toward zero, so `flips <= 0`.
#[test]
fn phase_c_row07_h_negative() {
    let impls = load_impls();
    for h in [-1i32, -2, -3, -7, -64, -1023, i32::MIN, i32::MIN + 1] {
        for w in [0i32, 1, 4] {
            assert_both_untouched(&impls, w, h, "ERRORS row 7 (h < 0)");
        }
    }
}

/// Row 8: `w < 0` with `h >= 2` — pointers are formed but never dereferenced.
#[test]
fn phase_c_row08_w_negative_active_h() {
    let impls = load_impls();
    for w in [-1i32, -5, -64, -1023, i32::MIN, i32::MIN + 1] {
        for h in [2i32, 3, 9, 64] {
            assert_both_untouched(&impls, w, h, "ERRORS row 8 (w < 0, h >= 2)");
        }
    }
}

/// Row 9: `w == 0` with `h >= 2`.
#[test]
fn phase_c_row09_w_zero_active_h() {
    let impls = load_impls();
    for h in [2i32, 3, 4, 17, 512] {
        assert_both_untouched(&impls, 0, h, "ERRORS row 9 (w == 0, h >= 2)");
    }
}

/// Row 10: both dimensions zero.
#[test]
fn phase_c_row10_both_zero() {
    let impls = load_impls();
    assert_both_untouched(&impls, 0, 0, "ERRORS row 10 (w == 0 && h == 0)");
}

/// Row 11: signed overflow of the row-offset expression, dereferenced. Both
/// implementations must terminate the same way.
#[test]
fn phase_c_row11_offset_overflow() {
    assert_same_fault("offset_overflow", "ERRORS row 11 (int overflow in w*(h-i-1))");
}

/// Row 12: `h == INT_MIN`.
#[test]
fn phase_c_row12_h_int_min() {
    let impls = load_impls();
    for w in [0i32, 1, 8, i32::MAX] {
        assert_both_untouched(&impls, w, i32::MIN, "ERRORS row 12 (h == INT_MIN)");
    }
}

/// Row 13: `w == INT_MIN`.
#[test]
fn phase_c_row13_w_int_min() {
    let impls = load_impls();
    for h in [0i32, 1, 2, 7, 64] {
        assert_both_untouched(&impls, i32::MIN, h, "ERRORS row 13 (w == INT_MIN)");
    }
}

/// Row 14: `h == INT_MAX` with `w == 0`.
///
/// `flips` is 1_073_741_823 and the inner loop is empty, so this is a
/// billion-iteration no-op in both implementations — a runtime hazard, not a
/// behavioral difference. It is verified for the largest `h` that still runs
/// promptly, plus `h == INT_MAX` paired with `h`-values that short-circuit, so
/// the `INT_MAX` value itself is still exercised through the FFI boundary.
#[test]
fn phase_c_row14_h_int_max() {
    let impls = load_impls();
    // INT_MAX as `w` (inner loop gate) is cheap and exercises the extreme.
    for h in [0i32, 1, -1, i32::MIN] {
        assert_both_untouched(&impls, i32::MAX, h, "ERRORS row 14 (w == INT_MAX)");
    }
    // Large-but-bounded stand-in for `h == INT_MAX, w == 0`.
    for h in [1_000_001i32, 1_000_000] {
        assert_both_untouched(&impls, 0, h, "ERRORS row 14 (w == 0, huge h)");
    }
}

/// Row 15: there is no enum / flag / mode anywhere in the API, so there is no
/// invalid-discriminant input class. Asserted structurally against the C source
/// so the claim cannot go stale, and the adjacent class (arbitrary `int` values
/// in `w`/`h`) is fuzzed here across the full signed range.
#[test]
fn phase_c_row15_no_enum_surface_and_arbitrary_ints() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for f in ["../c_src/include/lib.h", "../c_src/src/lib.c"] {
        let src = std::fs::read_to_string(root.join(f)).unwrap();
        assert!(
            !src.contains("enum"),
            "{f} now contains an enum: ERRORS.md row 15 must be replaced with \
             real out-of-range-discriminant tests"
        );
    }

    // Arbitrary 32-bit garbage in `w`/`h`, which C accepts for any `int`.
    // Only shapes that provably touch no memory are dereference-safe, so the
    // sign/magnitude is chosen to keep `flips <= 0` or the inner loop empty.
    let impls = load_impls();
    let mut rng = Rng::new(SEED ^ 0x15);
    for _ in 0..2000 {
        let garbage = rng.next_u64() as u32 as i32;
        // Negative-or-tiny `h`: no iterations regardless of `w`.
        let h = if garbage > 1 { -garbage } else { garbage };
        assert_both_nop_with_pix(
            &impls,
            garbage,
            h,
            std::ptr::null_mut(),
            "ERRORS row 15 (arbitrary int w, non-positive h)",
        );
        // Non-positive `w`: inner loop empty regardless of `h` (kept small so
        // the outer no-op loop terminates promptly).
        let w = if garbage > 0 { -garbage } else { garbage };
        let h_small = (rng.next_u64() % 65) as i32;
        assert_both_nop_with_pix(
            &impls,
            w,
            h_small,
            std::ptr::null_mut(),
            "ERRORS row 15 (non-positive w, arbitrary small h)",
        );
    }
}

/// Generic boundary sweep required by Phase C independently of the table:
/// null pointers, zero lengths, and one step past every interesting boundary.
#[test]
fn phase_c_generic_boundaries() {
    let impls = load_impls();

    // One step either side of the `flips > 0` boundary (h = 1 / 2) and of the
    // inner-loop boundary (w = 0 / 1), with real memory so bytes are compared.
    for h in [-1i32, 0, 1, 2, 3] {
        for w in [-1i32, 0, 1, 2] {
            let mut rng = Rng::new(SEED ^ 0xB0 ^ (w as u32 as u64) ^ ((h as u32 as u64) << 16));
            for _ in 0..16 {
                let buf = Buffer::randomized(pixel_count(w, h), &mut rng);
                assert_same(&impls, w, h, &buf, "Phase C generic boundary");
            }
        }
    }

    // Null `pix` on every shape that cannot dereference.
    for &(w, h) in &[
        (0i32, 0i32),
        (0, 1),
        (1, 0),
        (1, 1),
        (0, 2),
        (-1, 2),
        (i32::MAX, 1),
        (i32::MIN, 2),
        (7, i32::MIN),
    ] {
        assert_both_nop_with_pix(
            &impls,
            w,
            h,
            std::ptr::null_mut(),
            "Phase C generic boundary (pix == NULL)",
        );
    }

    // Dangling-but-unread pointer: the C loads `img->pix` unconditionally, so a
    // non-null garbage pointer must also be tolerated when nothing is read.
    for &(w, h) in &[(0i32, 8i32), (-4, 8), (9, 1), (9, 0), (9, -9)] {
        assert_both_nop_with_pix(
            &impls,
            w,
            h,
            0xDEAD_BEEF_usize as *mut u8,
            "Phase C generic boundary (garbage pix, never read)",
        );
    }
}
