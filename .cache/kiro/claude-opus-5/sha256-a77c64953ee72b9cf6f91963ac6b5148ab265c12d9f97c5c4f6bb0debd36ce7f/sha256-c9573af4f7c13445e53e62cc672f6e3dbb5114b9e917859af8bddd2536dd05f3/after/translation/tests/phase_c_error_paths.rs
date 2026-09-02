//! Phase C — error-path / boundary differential tests, one per `ERRORS.md` row.
//!
//! The C function has no error channel (it returns `void` and performs no
//! validation), so the "rejection" surface reduces to the overflow boundaries
//! and to explicitly documenting the ABI-level boundaries that do not apply.
//! Every assertion is differential against the compiled C `.so`.

mod common;

use common::{assert_same, assert_same_all, capture, driver_symbol, Impl, Rng, SEED};

// ---------------------------------------------------------------- E1
/// No error can be observed: the symbol's ABI return type is `void`, so the
/// only channel is stdout, and *every* input produces exactly one line.
#[test]
fn e1_no_error_channel_return_type() {
    let mut rng = Rng::new(SEED ^ 101);
    let xs: Vec<i32> = vec![i32::MIN, -1, 0, 1, i32::MAX]
        .into_iter()
        .chain((0..200).map(|_| rng.next_i32()))
        .collect();
    for x in xs {
        let out = assert_same(x, "E1");
        assert_eq!(
            out.iter().filter(|&&b| b == b'\n').count(),
            1,
            "E1: driver({x}) must emit exactly one line, got {:?}",
            String::from_utf8_lossy(&out)
        );
        assert!(
            !out.is_empty() && out.ends_with(b"\n"),
            "E1: driver({x}) output must be newline-terminated"
        );
        // Never an error string / sentinel: the payload is a decimal integer.
        let body = &out[..out.len() - 1];
        assert!(
            body.iter()
                .enumerate()
                .all(|(i, &b)| b.is_ascii_digit() || (i == 0 && b == b'-')),
            "E1: unexpected non-numeric output {:?}",
            String::from_utf8_lossy(&out)
        );
    }
}

// ---------------------------------------------------------------- E2
#[test]
fn e2_int_max_mul_overflow() {
    let out = assert_same(i32::MAX, "E2");
    assert_eq!(out, b"298\n", "E2: C reference for driver(INT_MAX)");
    // The whole top of the range, where 2*x always overflows.
    assert_same_all((i32::MAX - 40)..=i32::MAX, "E2-band");
}

// ---------------------------------------------------------------- E3
#[test]
fn e3_int_min_mul_overflow() {
    let out = assert_same(i32::MIN, "E3");
    assert_eq!(out, b"300\n", "E3: C reference for driver(INT_MIN)");
    // The whole bottom of the range.
    assert_same_all(i32::MIN..=(i32::MIN + 40), "E3-band");
}

// ---------------------------------------------------------------- E4
#[test]
fn e4_add_overflow_boundary() {
    // x = INT_MAX/2: 2*x fits exactly, but +300 overflows.
    let x = i32::MAX / 2; // 1073741823
    let out = assert_same(x, "E4");
    assert_eq!(out, b"-2147483350\n", "E4: C reference for driver(INT_MAX/2)");
}

// ---------------------------------------------------------------- E5 / E6
#[test]
fn e5_add_overflow_first_input() {
    // E6: last input whose y += 300 does NOT overflow.
    let ok = 1_073_741_673i32;
    assert_eq!(
        assert_same(ok, "E6"),
        b"2147483646\n",
        "E6: C reference for the largest non-overflowing input"
    );
    // E5: one step past it — the add overflows.
    let first_ovf = 1_073_741_674i32;
    assert_eq!(
        assert_same(first_ovf, "E5"),
        b"-2147483648\n",
        "E5: C reference for the first add-overflowing input"
    );
    // Sweep straight across the boundary.
    assert_same_all((ok - 20)..=(first_ovf + 20), "E5-crossing");
}

// ---------------------------------------------------------------- E7 / E8
#[test]
fn e7_int_min_half_boundary() {
    // E7: 2*x == INT_MIN exactly, no multiply overflow.
    let x = i32::MIN / 2; // -1073741824
    assert_eq!(
        assert_same(x, "E7"),
        b"-2147483348\n",
        "E7: C reference for driver(INT_MIN/2)"
    );
    // E8: one step further down — the multiply overflows. C is ground truth, so
    // the expected bytes come from the C .so, not from a hand computation.
    let past = i32::MIN / 2 - 1; // -1073741825
    let c_out = common::run_one(Impl::C, past);
    let rust_out = common::run_one(Impl::Rust, past);
    assert_eq!(
        c_out, rust_out,
        "E8: divergence at INT_MIN/2 - 1 ({past})"
    );
    assert_same_all((i32::MIN / 2 - 20)..=(i32::MIN / 2 + 20), "E7-crossing");
}

// ---------------------------------------------------------------- E9
/// Out-of-range enum / arbitrary bit patterns across the FFI boundary. The
/// parameter is a plain `int`, so *every* 32-bit pattern is a valid argument and
/// none may be rejected — including the ones that look like sentinel error
/// values (-1, 0xFFFFFFFF, 0x80000000, 0xDEADBEEF, …).
#[test]
fn e9_all_bit_patterns_are_valid() {
    let patterns: [u32; 20] = [
        0x0000_0000,
        0x0000_0001,
        0x0000_00FF,
        0x0000_7FFF,
        0x0000_8000,
        0x0000_FFFF,
        0x7FFF_FFFF,
        0x8000_0000,
        0x8000_0001,
        0xFFFF_FFFF,
        0xFFFF_FFFE,
        0xDEAD_BEEF,
        0xCAFE_BABE,
        0xA5A5_A5A5,
        0x5A5A_5A5A,
        0xFFFF_0000,
        0x0000_FFFE,
        0x7FFF_FFFE,
        0x4000_0000,
        0xC000_0000,
    ];
    for p in patterns {
        let x = p as i32; // reinterpret, exactly what a C caller passing an
                          // out-of-range enum value would put in the register
        let out = assert_same(x, "E9");
        assert!(
            !out.is_empty(),
            "E9: pattern 0x{p:08X} must still produce output"
        );
    }
    // Plus randomized full-width bit patterns.
    let mut rng = Rng::new(SEED ^ 109);
    assert_same_all((0..600).map(|_| rng.next_u64() as u32 as i32), "E9-random");
}

// ---------------------------------------------------------------- E10
/// Null pointers / zero and oversized lengths are inapplicable: the exported
/// signature is `void driver(int)` — no pointer, no length. This test pins that
/// fact so the omission is deliberate and re-checked, and additionally confirms
/// that a garbage value in the *upper* half of the 64-bit argument register is
/// truncated identically by both libraries (the ABI-level analogue of passing a
/// too-wide value).
#[test]
fn e10_no_pointer_or_length_params() {
    // Call through a deliberately widened signature: pass a 64-bit value whose
    // low 32 bits are the intended int and whose high bits are garbage. A
    // correct `int` callee ignores the high half; both must agree.
    type WideFn = unsafe extern "C" fn(u64);
    let c = driver_symbol(Impl::C);
    let rust = driver_symbol(Impl::Rust);
    let c_wide: WideFn = unsafe { std::mem::transmute(*c) };
    let rust_wide: WideFn = unsafe { std::mem::transmute(*rust) };

    for &(hi, lo) in &[
        (0xFFFF_FFFFu64, 0x0000_0000u64),
        (0xDEAD_BEEFu64, 0x0000_0001u64),
        (0x0000_0001u64, 0x8000_0000u64),
        (0x7FFF_FFFFu64, 0x7FFF_FFFFu64),
    ] {
        let arg = (hi << 32) | lo;
        let c_out = capture(|| unsafe { c_wide(arg) }).1;
        let rust_out = capture(|| unsafe { rust_wide(arg) }).1;
        assert_eq!(
            c_out, rust_out,
            "E10: divergence for widened arg 0x{arg:016X}"
        );
        // And it must equal the plain 32-bit call with the low half.
        let narrow = common::run_one(Impl::C, lo as u32 as i32);
        assert_eq!(
            c_out, narrow,
            "E10: high 32 bits must be ignored (arg 0x{arg:016X})"
        );
    }

    // There is exactly one exported function and it takes no pointers, so there
    // is no null-pointer path to test. Assert the surface really is that small
    // so this row cannot silently become stale.
    assert!(
        std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../c_src/include/driver.h"))
            .expect("read driver.h")
            .contains("void driver(int x);"),
        "E10: public header changed — re-derive ERRORS.md"
    );
}

// ---------------------------------------------------------------- extra
/// Exhaustive check of a dense contiguous window (every single value), which
/// catches off-by-one divergences that sampling could miss.
#[test]
fn extra_exhaustive_dense_window() {
    assert_same_all(-1500..=1500, "dense");
}
