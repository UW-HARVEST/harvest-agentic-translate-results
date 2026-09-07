//! Phase C — error/rejection-path differential tests, one test per row of
//! `ERRORS.md`.
//!
//! `pow43` has *no* explicit error surface (no error codes, sentinels, asserts,
//! null checks or enums — see `ERRORS.md` for the exhaustive grep). Its only
//! rejection surface is the unchecked index into the 145-element table, so the
//! comparable observable for each row is:
//!
//!   * inside the defined domain  -> identical bits;
//!   * out-of-bounds but mapped   -> **both must return normally**; a
//!     bounds-checked Rust index would panic here, and that is precisely the
//!     divergence these rows exist to catch;
//!   * out-of-bounds and unmapped -> **both must fault identically**, checked
//!     in a subprocess so the crash is observable.

mod harness;

use harness::*;
use std::ffi::c_int;

/// Calls both exported symbols and asserts neither aborted/panicked. Returns
/// the two (UB, therefore not compared) values so the test can report them.
fn assert_both_return(x: c_int, row: &str) -> (f32, f32) {
    let l = libs();
    // A Rust panic here (e.g. from a bounds-checked index) unwinds out of the
    // `extern "C"` boundary and fails the test, which is the point.
    let (c, r) = unsafe { ((l.c_pow43)(x), (l.rust_pow43)(x)) };
    println!("[{row}] pow43({x}): C = {c:e} / Rust = {r:e} (out of bounds: values are UB, not compared)");
    (c, r)
}

// --- Row 1 ----------------------------------------------------------------
#[test]
fn err_row01_x_minus_17_returns_without_trap() {
    assert_both_return(-17, "row 1");
}

// --- Row 2 ----------------------------------------------------------------
#[test]
fn err_row02_moderately_negative_returns_without_trap() {
    for x in [-18, -32, -64, -100, -1000] {
        assert_both_return(x, "row 2");
    }
}

// --- Row 4 ----------------------------------------------------------------
#[test]
fn err_row04_x_8224_first_overrun_returns_without_trap() {
    // 8224: x & 32 != 0 => sign = 64, (8224 + 64) >> 6 = 129, index 145 = one
    // element past the end of g_pow43[145].
    assert_eq!(8224 & 32, 32);
    assert_eq!(16 + ((8224 + 64) >> 6), 145);
    assert_both_return(8224, "row 4");
}

// --- Row 5 ----------------------------------------------------------------
#[test]
fn err_row05_larger_overruns_return_without_trap() {
    for x in [8256, 8320, 9000, 16384, 65536] {
        assert_both_return(x, "row 5");
    }
}

// --- Row 7 ----------------------------------------------------------------
#[test]
fn err_row07_no_division_by_zero_in_domain() {
    // The divide is only reached for x >= 129; prove no in-domain input can
    // produce a zero denominator, and that both libs agree and stay finite.
    for x in DOMAIN_MIN..=DOMAIN_MAX {
        let mut xx = x;
        if xx >= BRANCH_A {
            if xx < BRANCH_B {
                xx <<= 3;
            }
            let sign = (2 * xx) & 64;
            let denom = (xx & !63) + sign;
            assert_ne!(denom, 0, "unexpected zero denominator at x = {x}");
        }
        let (c, r) = both(x);
        assert_eq!(c.to_bits(), r.to_bits(), "row 7: mismatch at x = {x}");
        assert!(c.is_finite(), "row 7: C produced a non-finite result at x = {x}");
        assert!(r.is_finite(), "row 7: Rust produced a non-finite result at x = {x}");
        assert!(!c.is_nan() && !r.is_nan(), "row 7: NaN at x = {x}");
    }
}

// --- Row 8 ----------------------------------------------------------------
#[test]
fn err_row08_full_int_domain_no_enum_surface() {
    // `float pow43(int)` has no enum parameter: every 32-bit value is a valid
    // argument, so the "out-of-range enum variant" analogue is an argument
    // outside the implicit domain (rows 1-6, 11, 12). What *is* assertable here
    // is that every in-domain int matches bit-for-bit.
    let n = assert_all_bit_eq(DOMAIN_MIN..=DOMAIN_MAX, "row 8");
    assert_eq!(n, 8240);
}

// --- Row 9 ----------------------------------------------------------------
#[test]
fn err_row09_no_pointer_or_length_surface() {
    // No pointer, buffer, size or length parameter exists in the ABI, so there
    // is no null / zero-length / oversized-length input to construct. The
    // return is by value; verify the by-value ABI agrees at the extremes of the
    // defined domain and at zero.
    for x in [DOMAIN_MIN, -1, 0, 1, 128, 129, 1023, 1024, DOMAIN_MAX] {
        assert_bit_eq(x, "row 9");
    }
}

// --- Row 10 ---------------------------------------------------------------
#[test]
fn err_row10_one_past_branch_constants() {
    // The only literals the C branches on are 129 and 1024.
    for x in [BRANCH_A - 1, BRANCH_A, BRANCH_A + 1, BRANCH_B - 1, BRANCH_B, BRANCH_B + 1] {
        assert_bit_eq(x, "row 10");
    }
}

// --- Row 11 ---------------------------------------------------------------
#[test]
fn err_row11_one_past_domain_max() {
    assert_bit_eq(DOMAIN_MAX, "row 11"); // last defined input
    assert_both_return(DOMAIN_MAX + 1, "row 11"); // first UB input
}

// --- Row 12 ---------------------------------------------------------------
#[test]
fn err_row12_one_past_domain_min() {
    assert_bit_eq(DOMAIN_MIN, "row 12"); // last defined input
    assert_both_return(DOMAIN_MIN - 1, "row 12"); // first UB input
}

// ---------------------------------------------------------------------------
// Rows 3 and 6: far out-of-range inputs. The read address is unmapped, so the
// only way to observe the behaviour is out-of-process. The child probe below
// loads ONE library and calls it; the parent compares the two exit statuses.
// ---------------------------------------------------------------------------

const ENV_LIB: &str = "POW43_PROBE_LIB";
const ENV_X: &str = "POW43_PROBE_X";

/// Not a test: the harness re-executes itself with `--ignored --exact` to reach
/// this, so a fault is observable as a child exit status.
#[test]
#[ignore = "internal subprocess probe; driven by the fault-comparison tests"]
fn probe_child() {
    let which = std::env::var(ENV_LIB).expect("POW43_PROBE_LIB not set");
    let x: i32 = std::env::var(ENV_X)
        .expect("POW43_PROBE_X not set")
        .parse()
        .expect("POW43_PROBE_X not an i32");

    let path = match which.as_str() {
        "c" => find_c_so(),
        "rust" => find_rust_so(),
        other => panic!("unknown POW43_PROBE_LIB {other}"),
    };
    unsafe {
        let lib = libloading::Library::new(&path).expect("dlopen");
        let sym: libloading::Symbol<Pow43> = lib.get(b"pow43\0").expect("dlsym pow43");
        let f = *sym;
        let v = f(x);
        // Keep the read from being optimised away and report it.
        println!("RESULT {}", v.to_bits());
    }
}

/// `(exited_ok, exit_code, signal)` for one probe run.
fn run_probe(which: &str, x: i32) -> (bool, Option<i32>, Option<i32>) {
    use std::os::unix::process::ExitStatusExt;
    let exe = std::env::current_exe().expect("current_exe");
    let out = std::process::Command::new(exe)
        .args(["--exact", "probe_child", "--ignored", "--nocapture", "--test-threads=1"])
        .env(ENV_LIB, which)
        .env(ENV_X, x.to_string())
        .output()
        .expect("spawn probe");
    let st = out.status;
    (st.success(), st.code(), st.signal())
}

fn assert_same_fault_behaviour(x: i32, row: &str) {
    let c = run_probe("c", x);
    let r = run_probe("rust", x);
    println!("[{row}] pow43({x}): C probe = {c:?}, Rust probe = {r:?}");
    assert_eq!(
        (c.0, c.2),
        (r.0, r.2),
        "[{row}] pow43({x}): C and Rust disagree on fault behaviour \
         (C: success={} code={:?} signal={:?}; Rust: success={} code={:?} signal={:?})",
        c.0, c.1, c.2, r.0, r.1, r.2
    );
}

// --- Row 3 ----------------------------------------------------------------
#[test]
fn err_row03_int_min_faults_in_both() {
    assert_same_fault_behaviour(i32::MIN, "row 3");
    assert_same_fault_behaviour(i32::MIN + 16, "row 3");
}

// --- Row 6 ----------------------------------------------------------------
#[test]
fn err_row06_int_max_faults_in_both() {
    assert_same_fault_behaviour(i32::MAX, "row 6");
    assert_same_fault_behaviour(i32::MAX - 63, "row 6");
}
