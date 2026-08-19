// Phase C — error-path differential tests.
//
// One test per row of ERRORS.md.  The C library has NO rejection path at all
// (no return value, no assert, no range/null check — see ERRORS.md for the
// mechanical derivation), so rows 1-4 prove that absence differentially: for
// inputs that would classically be rejected, BOTH implementations must accept
// the input, return normally, and emit the same bytes.  Rows 5-12 cover the
// generic C-API boundaries that this signature can actually express.

mod common;

use common::{assert_same, assert_same_transcript, assert_same_wide, impls};
use std::ffi::c_int;

// --- row 1 -----------------------------------------------------------------

/// There is no error return: the symbol is `void driver(int)`.  Sweeping a wide
/// spread of inputs, both libraries always produce exactly one line of output
/// and no error indication of any kind.
fn err_01_no_error_return_path_exists() {
    let probes = [
        0,
        -1,
        1,
        i32::MIN,
        i32::MAX,
        -150,
        0x5555_5555u32 as i32,
        0xAAAA_AAAAu32 as i32,
        0x7FFF_FFFF,
        0x8000_0000u32 as i32,
    ];
    for x in probes {
        let out = assert_same(x);
        assert_eq!(
            out.iter().filter(|&&b| b == b'\n').count(),
            1,
            "driver({x}) must emit exactly one line, never an error path"
        );
        assert!(!out.is_empty());
    }
}

// --- row 2 -----------------------------------------------------------------

/// No `assert` / `abort` / `exit` exists, so the call always returns to the
/// caller.  If either implementation aborted the process the test binary would
/// die; reaching the end of this test proves both returned normally for every
/// probe, including the signed-overflow inputs.
fn err_02_never_aborts_always_returns() {
    let im = impls();
    let probes = [i32::MIN, i32::MAX, i32::MIN + 1, i32::MAX - 1, 0];
    let mut returned = 0usize;
    let out = common::capture_stdout(|| {
        for x in probes {
            (im.c.driver)(x);
            (im.rust.driver)(x);
            returned += 1;
        }
    });
    assert_eq!(returned, probes.len(), "a call failed to return");
    assert_eq!(out.iter().filter(|&&b| b == b'\n').count(), probes.len() * 2);
    // and the two implementations agree line by line
    let lines: Vec<&str> = std::str::from_utf8(&out).unwrap().lines().collect();
    for pair in lines.chunks(2) {
        assert_eq!(pair[0], pair[1], "C and Rust lines differ: {pair:?}");
    }
}

// --- row 3 -----------------------------------------------------------------

/// A null pointer is not expressible through `void driver(int)`.  The closest
/// expressible inputs are the all-zero bit pattern (via the declared `int`
/// signature) and a pointer-sized null passed in the argument register.
fn err_03_null_pointer_not_expressible() {
    assert_eq!(assert_same(0), b"300\n");
    // pointer-sized zero in `rdi`
    assert_eq!(assert_same_wide(0i64), b"300\n");
    // a real null pointer value, reinterpreted the way the ABI requires
    let null: *const c_int = std::ptr::null();
    assert_eq!(assert_same_wide(null as i64), b"300\n");
}

// --- row 4 -----------------------------------------------------------------

/// No length/size parameter exists, so "zero length" and "oversized length"
/// map onto the numeric extremes of the single `int` argument.
fn err_04_no_length_parameter() {
    assert_eq!(assert_same(0), b"300\n"); // "zero"
    assert_same(i32::MAX); // "oversized"
    assert_same(u32::MAX as i32); // "(size_t)-1" truncated to int
    assert_same_wide(usize::MAX as i64); // SIZE_MAX in the argument register
    assert_same_wide(u32::MAX as i64); // UINT32_MAX, zero-extended
}

// --- row 5 -----------------------------------------------------------------

fn err_05_int_max() {
    let out = assert_same(i32::MAX);
    assert_eq!(out, b"298\n", "C ground truth for INT_MAX");
}

// --- row 6 -----------------------------------------------------------------

fn err_06_int_min() {
    let out = assert_same(i32::MIN);
    assert_eq!(out, b"300\n", "C ground truth for INT_MIN");
}

// --- row 7 -----------------------------------------------------------------

fn err_07_one_step_inside_range_ends() {
    assert_eq!(assert_same(i32::MAX - 1), b"296\n");
    assert_eq!(assert_same(i32::MIN + 1), b"302\n");
}

// --- row 8 -----------------------------------------------------------------

fn err_08_one_step_past_multiply_range() {
    // 2*x is exactly INT_MIN here (first representable overflow step)
    assert_eq!(assert_same(0x4000_0000), b"-2147483348\n");
    // one step further out on the negative side
    assert_eq!(assert_same(-0x4000_0001), b"-2147483350\n");
    // the last x for which 2*x still fits
    assert_eq!(assert_same(-0x4000_0000), b"-2147483348\n");
}

// --- row 9 -----------------------------------------------------------------

fn err_09_one_step_past_addition_range() {
    // 2*x = 2147483646 fits; + 300 overflows
    assert_eq!(assert_same(0x3FFF_FFFF), b"-2147483350\n");
    // the largest x for which 2*x + 300 still fits
    assert_eq!(assert_same(1_073_741_673), b"2147483646\n");
    // first x for which the addition overflows
    assert_eq!(assert_same(1_073_741_674), b"-2147483648\n");
}

// --- row 10 ----------------------------------------------------------------

/// The parameter is an `int`, so there is no "invalid enum variant"; the
/// equivalent cross-FFI hazard is a caller that puts a value wider than `int`
/// in the argument register.  Both implementations must ignore the upper 32
/// bits identically.
fn err_10_garbage_upper_32_bits_of_argument() {
    let los: [u32; 8] = [
        0,
        1,
        0xFFFF_FFFF,
        0x8000_0000,
        0x7FFF_FFFF,
        0x0000_012C,
        0xDEAD_BEEF,
        0x4000_0000,
    ];
    let his: [u32; 6] = [
        0,
        0xFFFF_FFFF,
        0x0000_0001,
        0x7FFF_FFFF,
        0x8000_0000,
        0xCAFE_F00D,
    ];
    for &lo in &los {
        let expected = format!("{}\n", (lo as i32).wrapping_mul(2).wrapping_add(300));
        for &hi in &his {
            let raw = (((hi as u64) << 32) | lo as u64) as i64;
            let out = assert_same_wide(raw);
            assert_eq!(
                String::from_utf8_lossy(&out),
                expected,
                "upper bits 0x{hi:08x} must not affect the result for lo=0x{lo:08x}"
            );
        }
    }
}

// --- row 11 ----------------------------------------------------------------

fn err_11_unsigned_bit_patterns() {
    assert_eq!(assert_same(0xFFFF_FFFFu32 as i32), b"298\n");
    assert_eq!(assert_same(0x8000_0000u32 as i32), b"300\n");
    assert_eq!(assert_same_wide(u64::MAX as i64), b"298\n");
    assert_eq!(assert_same_wide(0xFFFF_FFFF_8000_0000u64 as i64), b"300\n");
}

// --- row 12 ----------------------------------------------------------------

fn err_12_repeated_boundary_calls_no_state() {
    let boundary = [
        i32::MIN,
        i32::MIN + 1,
        -0x4000_0001,
        -0x4000_0000,
        -151,
        -150,
        -149,
        -1,
        0,
        1,
        0x3FFF_FFFF,
        0x4000_0000,
        1_073_741_673,
        1_073_741_674,
        i32::MAX - 1,
        i32::MAX,
    ];
    // three passes in one process: no accumulated state may change anything
    let mut all = Vec::new();
    for _ in 0..3 {
        all.extend_from_slice(&boundary);
    }
    let out = assert_same_transcript(&all);
    assert_eq!(out.iter().filter(|&&b| b == b'\n').count(), all.len());
    assert!(out.ends_with(b"\n"), "last line must be newline terminated");

    // repeating the identical call must give the identical bytes every time
    for x in boundary {
        let a = assert_same(x);
        let b = assert_same(x);
        assert_eq!(a, b, "driver({x}) is not idempotent");
    }
}

// ---------------------------------------------------------------------------
// Single entry point (see the note in tests/phase_b_configs.rs).
// ---------------------------------------------------------------------------

#[test]
fn phase_c_all_error_rows() {
    let mut r = common::RowRunner::new("ERRORS.md");
    r.row(
        "err_01_no_error_return_path_exists",
        err_01_no_error_return_path_exists,
    );
    r.row(
        "err_02_never_aborts_always_returns",
        err_02_never_aborts_always_returns,
    );
    r.row(
        "err_03_null_pointer_not_expressible",
        err_03_null_pointer_not_expressible,
    );
    r.row("err_04_no_length_parameter", err_04_no_length_parameter);
    r.row("err_05_int_max", err_05_int_max);
    r.row("err_06_int_min", err_06_int_min);
    r.row(
        "err_07_one_step_inside_range_ends",
        err_07_one_step_inside_range_ends,
    );
    r.row(
        "err_08_one_step_past_multiply_range",
        err_08_one_step_past_multiply_range,
    );
    r.row(
        "err_09_one_step_past_addition_range",
        err_09_one_step_past_addition_range,
    );
    r.row(
        "err_10_garbage_upper_32_bits_of_argument",
        err_10_garbage_upper_32_bits_of_argument,
    );
    r.row("err_11_unsigned_bit_patterns", err_11_unsigned_bit_patterns);
    r.row(
        "err_12_repeated_boundary_calls_no_state",
        err_12_repeated_boundary_calls_no_state,
    );
    r.finish();
}
