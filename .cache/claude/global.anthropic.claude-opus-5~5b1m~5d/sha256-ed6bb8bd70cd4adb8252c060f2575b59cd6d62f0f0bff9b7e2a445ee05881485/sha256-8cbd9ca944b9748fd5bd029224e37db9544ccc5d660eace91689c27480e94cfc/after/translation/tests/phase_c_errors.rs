//! Phase C — error-path differential tests.
//!
//! `ERRORS.md` has ZERO rows: the C library has no error surface at all
//! (`void driver(int)` — no return channel, no pointer/length/enum parameter,
//! no `if`, no `assert`, no `return`, no error macro anywhere in the 40-line
//! translation unit). See `ERRORS.md` for the mechanical grep that establishes
//! this.
//!
//! What remains is the "generic boundaries every C API has" obligation. For
//! this signature the only expressible boundary is the argument's numeric
//! domain, which is *total*: every one of the 2^32 bit patterns is a valid
//! `int`. The tests below therefore assert that for the inputs a caller would
//! use to probe for rejection, BOTH sides agree that there is no rejection and
//! emit the identical bytes.

mod harness;

use harness::*;

/// The full valid range and one step past each end (which wraps, since `int`
/// arithmetic at the call site is the caller's problem, not `driver`'s).
/// Asserts identical behaviour, i.e. identical output and no abort on either
/// side.
fn phase_c_no_error_surface_extremes(libs: &Libs) {    let xs = [
        i32::MIN,
        i32::MIN + 1,
        i32::MIN / 2,
        -65_537,
        -65_536,
        -65_535,
        -257,
        -256,
        -255,
        -129,
        -128,
        -127,
        -2,
        -1,
        0,
        1,
        2,
        127,
        128,
        129,
        255,
        256,
        257,
        65_535,
        65_536,
        65_537,
        i32::MAX / 2,
        i32::MAX - 1,
        i32::MAX,
    ];
    for x in xs {
        assert_same(libs, x, "phase C extremes");
    }
    assert_same_batch(libs, &xs, "phase C extremes (batched)");
}

/// A C `enum` parameter accepts any `int`, so the canonical "out-of-range enum
/// value across the FFI boundary" bug class is probed here by passing the
/// integer values such a stray enum would carry. `driver` has no enum
/// parameter, so every one of these is simply a valid `int` and both sides must
/// treat it as data, not as an invalid mode.
fn phase_c_stray_enum_like_values(libs: &Libs) {    let xs: Vec<i32> = vec![
        -2147483648, -1000000, -1000, -42, -3, -1, 0, 1, 2, 3, 4, 5, 6, 7, 8, 16, 32, 64, 99, 100,
        255, 256, 999, 1000, 0x7fff, 0x8000, 0xffff, 0x1_0000, 1000000, 2147483647,
    ];
    for &x in &xs {
        assert_same(libs, x, "phase C stray-enum ints");
    }
    assert_same_batch(libs, &xs, "phase C stray-enum ints (batched)");
}

/// "Zero and oversized lengths": no length parameter exists, but the values a
/// caller would pass *as* a length (0, negative, huge) are exercised as inputs
/// so the surrogate boundary is still covered differentially.
fn phase_c_length_surrogate_values(libs: &Libs) {    let xs: Vec<i32> = vec![
        0,
        -1,
        -4,
        -8,
        4,
        8,
        i32::MAX,
        i32::MIN,
        (usize::MAX as u32) as i32, // 0xffffffff
        0x7fff_ffff,
    ];
    for &x in &xs {
        assert_same(libs, x, "phase C length surrogates");
    }
    assert_same_batch(libs, &xs, "phase C length surrogates (batched)");
}

/// "Null pointer": no pointer parameter exists, so the closest expressible
/// probe is the all-zero-bits and all-one-bits arguments (what a truncated
/// `NULL` and an invalid sentinel pointer would look like as an `int`).
/// Neither side may crash and both must print the same bytes.
fn phase_c_null_pointer_surrogates(libs: &Libs) {    for x in [0i32, -1i32, 0xffff_ffffu32 as i32] {
        assert_same(libs, x, "phase C null surrogates");
    }
}

/// Repeated invocation with the same "invalid-looking" input must be
/// idempotent on both sides (no latched error state, since there is none).
fn phase_c_repeat_idempotent(libs: &Libs) {    for x in [0i32, -1, i32::MIN, i32::MAX] {
        let first_c = c_out(libs, x);
        let first_r = rust_out(libs, x);
        assert_eq!(first_c, first_r, "phase C idempotence, first call, x={x}");
        for _ in 0..5 {
            assert_eq!(c_out(libs, x), first_c, "C not idempotent for x={x}");
            assert_eq!(rust_out(libs, x), first_r, "Rust not idempotent for x={x}");
        }
    }
}

/// Aggregate entry point — see `harness::run_rows` for why every row runs
/// inside a single `#[test]` (process-wide stdout redirection must be serial).
#[test]
fn all_rows() {
    run_rows(&[
        ("phase_c_no_error_surface_extremes", phase_c_no_error_surface_extremes),
        ("phase_c_stray_enum_like_values", phase_c_stray_enum_like_values),
        ("phase_c_length_surrogate_values", phase_c_length_surrogate_values),
        ("phase_c_null_pointer_surrogates", phase_c_null_pointer_surrogates),
        ("phase_c_repeat_idempotent", phase_c_repeat_idempotent),
    ]);
}
