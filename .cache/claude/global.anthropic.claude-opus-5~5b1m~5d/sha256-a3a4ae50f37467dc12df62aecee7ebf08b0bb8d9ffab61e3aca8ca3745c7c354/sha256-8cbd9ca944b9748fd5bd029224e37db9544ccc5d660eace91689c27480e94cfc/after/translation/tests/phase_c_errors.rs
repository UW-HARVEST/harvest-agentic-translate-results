//! Phase C — error/rejection-path differential tests, one per ERRORS.md row.
//!
//! The C `colourblind` returns `void` and has no error codes, so the only
//! "rejection" it performs is the silent no-op fallthrough of a `switch` with
//! no `default:` label. Each test therefore asserts the *sentinel* behaviour
//! exactly: the three slots come back BIT-FOR-BIT unmodified, in both
//! libraries, and both agree.

mod common;
use common::*;

/// A distinguishable sentinel triple: if either library writes anything, the
/// bits change.
const SENTINEL: [f32; 3] = [1.25f32, -7.5f32, 1234.5f32];

fn sentinels() -> Vec<[f32; 3]> {
    vec![
        SENTINEL,
        [0.0, -0.0, 1.0],
        [f32::NAN, f32::INFINITY, f32::NEG_INFINITY],
        [f32::from_bits(0xDEAD_BEEF), f32::from_bits(0x0000_0001), f32::MAX],
    ]
}

// ------------------------------------------------------------------ E1
// Impairment == 3: exactly one past the last valid enumerator.
#[test]
fn err_e1_one_past_end() {
    for s in sentinels() {
        assert_noop_both("E1 imp==3 (one past cbTritanopia)", 3, s);
    }
}

// ------------------------------------------------------------------ E2
// Small out-of-range values 4..=255.
#[test]
fn err_e2_small_out_of_range() {
    for imp in 4u32..=255 {
        for s in sentinels() {
            assert_noop_both("E2 small out-of-range", imp, s);
        }
    }
}

// ------------------------------------------------------------------ E3
// (cb_impairment)-1 == 0xFFFFFFFF. gcc gives this enum the compatible type
// `unsigned int`, so a "negative" int arriving over the FFI is a huge unsigned
// value and must still be a no-op.
#[test]
fn err_e3_negative_as_unsigned() {
    let imp = (-1i32) as u32;
    assert_eq!(imp, 0xFFFF_FFFF);
    for s in sentinels() {
        assert_noop_both("E3 imp==(cb_impairment)-1", imp, s);
    }
}

// ------------------------------------------------------------------ E4
// -2 ..= -256 reinterpreted, plus INT_MIN.
#[test]
fn err_e4_negative_sweep() {
    for n in 2i64..=256 {
        let imp = (-n as i32) as u32;
        for s in sentinels() {
            assert_noop_both("E4 negative sweep", imp, s);
        }
    }
    for &imp in &[i32::MIN as u32, (i32::MIN + 1) as u32] {
        for s in sentinels() {
            assert_noop_both("E4 INT_MIN", imp, s);
        }
    }
}

// ------------------------------------------------------------------ E5
// Extreme / boundary out-of-range enum values.
#[test]
fn err_e5_extreme_enum_values() {
    let extremes: [u32; 12] = [
        u32::MAX,
        u32::MAX - 1,
        0x7FFF_FFFF,
        0x8000_0000,
        1 << 31,
        1 << 16,
        256,
        65536,
        0x0000_0100,
        0x0001_0000,
        0x5555_5555,
        0xAAAA_AAAA,
    ];
    for &imp in &extremes {
        for s in sentinels() {
            assert_noop_both("E5 extreme enum value", imp, s);
        }
    }
}

// ------------------------------------------------------------------ E6
// Whole-domain agreement sweep: for EVERY impairment value tested, C and Rust
// must agree on *whether* a transform happened and on the resulting bits.
// 0..=1024 exhaustively, plus random u32 values.
#[test]
fn err_e6_enum_domain_sweep() {
    let input = [0.25f32, 0.75f32, -1.5f32];

    for imp in 0u32..=1024 {
        // differential: identical result either way (transform for 0/1/2,
        // no-op for everything else)
        assert_same("E6 exhaustive 0..=1024", imp, input);
        if imp > 2 {
            assert_noop_both("E6 exhaustive no-op", imp, input);
        }
    }

    let mut rng = Rng::new(0xE6);
    for _ in 0..20_000 {
        let imp = rng.next_u32();
        let t = rng.triple(|r| r.any_f32());
        assert_same("E6 random enum value", imp, t);
        if imp > 2 {
            assert_noop_both("E6 random no-op", imp, t);
        }
    }
}

// ------------------------------------------------------------------ E7
// Non-finite channel values are NOT rejected by the C (there is no validation);
// they are accepted and propagate. Bits, including NaN sign and payload, must
// match.
#[test]
fn err_e7_nonfinite_accepted() {
    let nonfinite: [f32; 8] = [
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::from_bits(0x7FC0_0000), // +qNaN
        f32::from_bits(0xFFC0_0000), // -qNaN
        f32::from_bits(0x7F80_0001), // +sNaN
        f32::from_bits(0xFF80_0001), // -sNaN
        f32::from_bits(0x7FFF_FFFF),
        f32::from_bits(0xFF81_2345),
    ];
    for &imp in &VALID {
        for &r in &nonfinite {
            for &g in &nonfinite {
                for &b in &nonfinite {
                    assert_same("E7 non-finite accepted", imp, [r, g, b]);
                }
            }
        }
        // one non-finite mixed with finite values, in every position
        for &v in &nonfinite {
            for pos in 0..3 {
                let mut t = [0.5f32, -0.25, 3.0];
                t[pos] = v;
                assert_same("E7 non-finite mixed", imp, t);
            }
        }
    }
    // and under aliasing
    for alias in ALL_ALIASES {
        for &imp in &VALID {
            for &v in &nonfinite {
                assert_same_aliased("E7 non-finite aliased", imp, [v, -v, 1.0], alias);
            }
        }
    }
}

// ------------------------------------------------------------------ E8
// Subnormals and signed zeros are accepted, not rejected. No FTZ/DAZ.
#[test]
fn err_e8_subnormal_and_zero() {
    let vals: [f32; 8] = [
        0.0,
        -0.0,
        f32::from_bits(0x0000_0001),
        f32::from_bits(0x8000_0001),
        f32::from_bits(0x007F_FFFF),
        f32::from_bits(0x807F_FFFF),
        f32::MIN_POSITIVE,
        -f32::MIN_POSITIVE,
    ];
    for &imp in &VALID {
        for &r in &vals {
            for &g in &vals {
                for &b in &vals {
                    assert_same("E8 subnormal/zero", imp, [r, g, b]);
                }
            }
        }
    }
}

// ------------------------------------------------------------------ B1
// NULL pointers with an OUT-OF-RANGE impairment. The switch falls through
// without dereferencing, so this is well-defined in C and must not crash in
// either library. (NULL with a VALID impairment is UB in C — see ERRORS.md B2 —
// and is deliberately not exercised.)
#[test]
fn err_b1_null_ptrs_invalid_enum() {
    let c = c_fn();
    let rust = rust_fn();
    let bad: [u32; 8] = [3, 4, 7, 255, 256, 0x8000_0000, u32::MAX, (-1i32) as u32];
    for &imp in &bad {
        unsafe {
            c(imp, std::ptr::null_mut(), std::ptr::null_mut(), std::ptr::null_mut());
            rust(imp, std::ptr::null_mut(), std::ptr::null_mut(), std::ptr::null_mut());
        }
    }
    // Also mixed: some NULL, some valid — still never dereferenced.
    let mut v = [1.0f32, 2.0, 3.0];
    for &imp in &bad {
        unsafe {
            c(imp, std::ptr::null_mut(), &mut v[1], std::ptr::null_mut());
            rust(imp, std::ptr::null_mut(), &mut v[1], std::ptr::null_mut());
        }
    }
    assert_eq!(
        v.map(f32::to_bits),
        [1.0f32, 2.0, 3.0].map(f32::to_bits),
        "B1: a non-NULL slot was written despite an out-of-range impairment"
    );
}

// ------------------------------------------------------------------ B4
// The API takes no length/count/size argument, so "zero and oversized lengths"
// has no representation. Documented here so the row is accounted for.
#[test]
fn err_b4_no_length_arguments_exist() {
    // colourblind(cb_impairment, float*, float*, float*) — 4 args, none a size.
    // Nothing to test; the test exists so the ERRORS.md row is traceable.
}

// ------------------------------------------------------------------ misaligned
// One more generic FFI boundary: unaligned float pointers. Both libraries use
// plain scalar loads/stores, so behaviour must be identical.
#[test]
fn err_misaligned_pointers() {
    let c = c_fn();
    let rust = rust_fn();
    for &imp in &VALID {
        for off in 1..=3usize {
            let src: [f32; 3] = [0.75, -2.5, 1.0e-3];
            let mut bytes = [0u8; 12];
            for (i, v) in src.iter().enumerate() {
                bytes[i * 4..i * 4 + 4].copy_from_slice(&v.to_le_bytes());
            }
            let mut raw_c = [0u8; 16];
            let mut raw_rust = [0u8; 16];
            raw_c[off..off + 12].copy_from_slice(&bytes);
            raw_rust[off..off + 12].copy_from_slice(&bytes);
            unsafe {
                let pc = raw_c.as_mut_ptr().add(off) as *mut f32;
                let pr = raw_rust.as_mut_ptr().add(off) as *mut f32;
                c(imp, pc, pc.byte_add(4), pc.byte_add(8));
                rust(imp, pr, pr.byte_add(4), pr.byte_add(8));
            }
            assert_eq!(
                raw_c, raw_rust,
                "misaligned (offset {off}) divergence for imp={imp}"
            );
        }
    }
}
