//! Phase C — error/boundary-path differential tests, one per row of `ERRORS.md`.
//!
//! `driver`'s error surface is provably EMPTY (see `ERRORS.md`: the C source has
//! zero `return`s, zero `assert`s, zero null/range checks and zero branches).
//! It has no pointer, length or enum parameters, so rows E1 (null pointer) and
//! E6 (out-of-range enum across FFI) are not expressible for this signature and
//! are documented rather than tested.
//!
//! What remains is the boundary surface that DOES exist: the `int` domain is
//! total, so every one of the 2^32 inputs is "valid", and the interesting
//! boundaries are the signed-overflow discontinuities of `2*x + 300` — which
//! are undefined behaviour in ISO C and therefore the single most likely place
//! for a translation to diverge (plain `*`/`+` in Rust would panic in debug
//! instead of wrapping).

mod harness;
use harness::*;

// --- E2: "zero length" analogue --------------------------------------------

#[test]
fn err_zero() {
    assert_same("E2", &[0]);
}

// --- E3/E4/E5: extremes and one-step-past-the-range ------------------------

#[test]
fn err_extremes() {
    assert_same(
        "E3/E4/E5",
        &[
            i32::MIN,
            i32::MIN + 1,
            i32::MIN + 2,
            -1,
            0,
            1,
            i32::MAX - 2,
            i32::MAX - 1,
            i32::MAX,
        ],
    );
}

// --- E7: signed overflow of the multiply 2*x ------------------------------

#[test]
fn err_mul_overflow_boundary() {
    // The multiply overflows exactly when |x| > 2^30. Walk both thresholds
    // densely, then sample the whole overflowing region randomly.
    let mut xs = around(1 << 30, 64);
    xs.extend(around(-(1 << 30), 64));
    xs.extend(around((1 << 30) - 1, 4));
    xs.extend(around(-(1 << 30) - 1, 4));

    let mut rng = Rng::with_seed(0xE7);
    for _ in 0..2_000 {
        // Strictly inside the overflowing regions.
        xs.push(rng.range_i32((1 << 30) + 1, i32::MAX));
        xs.push(rng.range_i32(i32::MIN, -(1 << 30) - 1));
    }
    xs.sort_unstable();
    xs.dedup();
    assert_same("E7", &xs);
}

// --- E8: signed overflow of the add y += 300 ------------------------------

#[test]
fn err_add_overflow_boundary() {
    // 2*x lands within 300 of INT_MAX, so the ADD is what overflows.
    // 2*x is always even; y = 2x + 300 overflows when 2x > INT_MAX - 300.
    let mut xs = Vec::new();
    let threshold = ((i32::MAX as i64 - 300) / 2) as i32; // largest x with no add overflow
    for d in -200..=200i64 {
        let v = threshold as i64 + d;
        if v >= i32::MIN as i64 && v <= i32::MAX as i64 {
            xs.push(v as i32);
        }
    }
    // Mirror case: 2*x just above INT_MIN, so subtracting nothing but the
    // negative side of the wrap is exercised via the lower threshold.
    let low = ((i32::MIN as i64 + 300) / 2) as i32;
    for d in -200..=200i64 {
        let v = low as i64 + d;
        if v >= i32::MIN as i64 && v <= i32::MAX as i64 {
            xs.push(v as i32);
        }
    }
    xs.sort_unstable();
    xs.dedup();
    assert_same("E8", &xs);
}

// --- E9: exact sign transition of the result ------------------------------

#[test]
fn err_sign_transition() {
    // y == 0 at x == -150; x == -151 is the first negative result.
    assert_same("E9", &around(-150, 32));
}

// --- E10: printf field-width transitions, incl. the 11-char INT_MIN -------

#[test]
fn err_digit_widths() {
    let mut xs = Vec::new();
    // Every decimal-width boundary reachable by an even 2*x + 300.
    let mut p: i64 = 1;
    while p <= 1_000_000_000 {
        for sign in [1i64, -1] {
            for d in [-2i64, -1, 0, 1, 2] {
                let y = sign * (p + d);
                let num = y - 300;
                if num % 2 == 0 {
                    let x = num / 2;
                    if x >= i32::MIN as i64 && x <= i32::MAX as i64 {
                        xs.push(x as i32);
                    }
                }
            }
        }
        p *= 10;
    }
    // The widest possible output, "-2147483648" (11 chars): y == INT_MIN
    // requires 2x == INT_MIN - 300 which is even, so it is reachable.
    let num = i32::MIN as i64 - 300;
    assert_eq!(num % 2, 0);
    let x_for_int_min = num / 2;
    if x_for_int_min >= i32::MIN as i64 {
        xs.push(x_for_int_min as i32);
    } else {
        // Not reachable directly; reach INT_MIN through the wrap instead.
        let wrapped = (num + (1i64 << 32)) / 2;
        xs.push(wrapped as i32);
    }
    xs.sort_unstable();
    xs.dedup();
    assert_same("E10", &xs);

    // Confirm an 11-character line is actually produced somewhere in the row,
    // otherwise this test would silently not cover the widest format.
    let widest = xs
        .iter()
        .copied()
        .map(|x| expected_line(x).trim_end().len())
        .max()
        .unwrap();
    assert!(
        widest >= 11,
        "E10 never produced an 11-char result; widest was {widest}"
    );
}

// --- E1 / E6: documented as inexpressible for this signature --------------

#[test]
fn err_e1_null_pointer_not_applicable() {
    // `void driver(int)` has no pointer parameter, so there is no null pointer
    // to pass. Asserted structurally against the header so that this test
    // starts failing if the C API ever grows a pointer argument.
    let hdr = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../c_src/include/driver.h"),
    )
    .expect("read driver.h");
    let decls: Vec<&str> = hdr
        .lines()
        .filter(|l| !l.trim_start().starts_with("//") && l.contains('('))
        .collect();
    assert_eq!(decls.len(), 1, "expected exactly one declaration, got {decls:?}");
    assert!(
        !decls[0].contains('*'),
        "driver.h now declares a pointer parameter ({:?}) — E1 must become a real test",
        decls[0]
    );
}

#[test]
fn err_e6_out_of_range_enum_not_applicable() {
    // No enum parameters exist, and the `int` parameter's domain is total:
    // every 32-bit pattern is a valid input with defined printed output.
    // The closest analogue to "a value with no valid variant" is simply an
    // arbitrary bit pattern, so we hammer the FFI boundary with the kinds of
    // values that break naive enum/int marshalling: all-ones, sign bit only,
    // alternating bits, and values that only differ above the low byte.
    let xs = vec![
        0x0000_0000u32 as i32,
        0xFFFF_FFFFu32 as i32, // -1, all ones
        0x8000_0000u32 as i32, // sign bit only == INT_MIN
        0x7FFF_FFFFu32 as i32, // INT_MAX
        0xAAAA_AAAAu32 as i32,
        0x5555_5555u32 as i32,
        0xDEAD_BEEFu32 as i32,
        0xCAFE_BABEu32 as i32,
        0x0000_00FFu32 as i32,
        0xFFFF_FF00u32 as i32,
        0x0000_0100u32 as i32,
        0x0100_0000u32 as i32,
        0xFF00_0000u32 as i32,
        // Same low byte, different high bytes: catches an accidental
        // narrowing of the argument to 8/16 bits across the FFI boundary.
        0x0000_0001u32 as i32,
        0x0001_0001u32 as i32,
        0x0100_0001u32 as i32,
        0x7F00_0001u32 as i32,
        0xFF00_0001u32 as i32,
    ];
    assert_same("E6", &xs);
}

// --- Extra: exhaustive low-16-bit sweep to catch argument narrowing -------

#[test]
fn err_argument_width_sweep() {
    // If either side narrowed or sign-extended the argument incorrectly, a
    // dense sweep across a sign boundary would expose it.
    let mut xs = inclusive(-2048, 2048);
    xs.extend(inclusive(32_760, 32_775)); // around 2^15
    xs.extend(inclusive(-32_775, -32_760));
    xs.extend(inclusive(65_530, 65_540)); // around 2^16
    xs.extend(inclusive(-65_540, -65_530));
    assert_same("width-sweep", &xs);
}
