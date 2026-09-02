//! Phase C — error/rejection-path differential tests.
//!
//! One test per row of `ERRORS.md`. As documented there, `float2half` has an
//! **empty** error surface: no pointer, length, or enum parameter, no `if`,
//! no `assert`, and a single unconditional `return`. Rows 1–3 therefore *prove*
//! the absence of those parameter classes from the C header (so the
//! corresponding bug class cannot exist), and rows 4–10 attack the invariants
//! the C relies on **instead of** checking — OOB table index, oversized shift,
//! integer overflow/narrowing, out-of-range values, NaN/sNaN, signed zeros —
//! asserting that C and Rust agree bit-for-bit and that neither traps where the
//! other does not.

mod common;

use common::{bits_from, Pair, Rng, SEED};

// ---------------------------------------------------------------------------
// Row 1 — null pointer: no pointer parameter exists.
// ---------------------------------------------------------------------------

#[test]
fn err_01_no_pointer_parameter_documented() {
    let header = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("c_src/include/lib.h"),
    )
    .expect("read c_src/include/lib.h");

    // Strip the include line; what remains is the entire public API.
    let api: String = header
        .lines()
        .filter(|l| !l.trim_start().starts_with('#'))
        .collect::<Vec<_>>()
        .join("\n");

    assert!(
        api.contains("uint16_t float2half(float flt);"),
        "unexpected public API: {api:?}"
    );
    assert!(
        !api.contains('*'),
        "a pointer appeared in the public API; this row must be re-derived: {api:?}"
    );
    // Nothing to reject: there is no null-pointer input to construct. Confirm the
    // one and only entry point is callable and total on its whole domain edge.
    let pair = Pair::load();
    pair.check_bits(0x0000_0000, "row01 +0");
    pair.check_bits(0xFFFF_FFFF, "row01 all-ones");
}

// ---------------------------------------------------------------------------
// Row 2 — zero / oversized length: no length parameter exists.
// ---------------------------------------------------------------------------

#[test]
fn err_02_no_length_parameter_documented() {
    let header = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("c_src/include/lib.h"),
    )
    .unwrap();
    for banned in ["size_t", "len", "count", "n_", "num", "size"] {
        assert!(
            !header.contains(banned),
            "public header mentions {banned:?}; the length axis must be re-derived"
        );
    }
    // Degenerate-magnitude stand-ins for "zero" and "oversized".
    let pair = Pair::load();
    pair.check_value(0.0, "row02 zero");
    pair.check_value(f32::MAX, "row02 oversized");
    pair.check_value(f32::INFINITY, "row02 beyond-oversized");
}

// ---------------------------------------------------------------------------
// Row 3 — out-of-range enum across FFI: no enum parameter exists.
//
// The class of bug still gets probed as far as it can be: the parameter's full
// 32-bit domain is swept at its extremes, since a `float` (unlike a C enum)
// legitimately accepts every bit pattern.
// ---------------------------------------------------------------------------

#[test]
fn err_03_no_enum_parameter_documented() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap();
    let header = std::fs::read_to_string(root.join("c_src/include/lib.h")).unwrap();
    let source = std::fs::read_to_string(root.join("c_src/src/lib.c")).unwrap();
    assert!(
        !header.contains("enum"),
        "public header declares an enum; the enum axis must be re-derived"
    );
    assert!(
        !source.contains("enum"),
        "implementation declares an enum; the enum axis must be re-derived"
    );

    // Sweep every "one past the end" integer value in the parameter's domain,
    // reinterpreted as the float the ABI actually receives.
    let pair = Pair::load();
    for bits in [
        0u32,
        1,
        2,
        i32::MAX as u32,
        i32::MAX as u32 + 1,
        u32::MAX - 1,
        u32::MAX,
        0x8000_0000,
        0x7FFF_FFFF,
    ] {
        pair.check_bits(bits, "row03 domain extreme");
    }
    // Small integers cast to float, i.e. what an out-of-range enum value would
    // become if it reached this API.
    for v in [-1i32, 0, 1, 2, 3, 99, 1_000_000, i32::MIN, i32::MAX] {
        pair.check_value(v as f32, "row03 int-as-float");
    }
}

// ---------------------------------------------------------------------------
// Row 4 — table index: sweep ALL 512 reachable indices, incl. 0 and 511.
// ---------------------------------------------------------------------------

#[test]
fn err_04_all_512_table_indices_in_range() {
    let pair = Pair::load();
    // Every j is reachable and neither side may read out of bounds or differ.
    for j in 0..512u32 {
        for m in [0u32, 1, 0x007f_ffff, 0x0040_0000] {
            pair.check_bits(bits_from(j, m), &format!("row04 j={j} m={m:#08x}"));
        }
    }
    // The masking invariant itself: bits above bit 31 cannot exist, and j is
    // built from exactly the top 9 bits, so index 511 is the maximum. Confirm
    // the extreme index really is exercised by the all-ones input.
    // (`erasing_op` is expected here — spelling out the j=0 computation
    // literally, exactly as the C writes it, is the assertion's whole point.)
    #[allow(clippy::erasing_op)]
    {
        assert_eq!((0xFFFF_FFFFu32 >> 23) & 0x1ff, 511);
        assert_eq!((0x0000_0000u32 >> 23) & 0x1ff, 0);
    }
    pair.check_bits(0xFFFF_FFFF, "row04 j=511 max mantissa");
    pair.check_bits(0x0000_0000, "row04 j=0 zero mantissa");
}

// ---------------------------------------------------------------------------
// Row 5 — shift amount: max (24) and min (13) shifts with maximal mantissa.
// ---------------------------------------------------------------------------

#[test]
fn err_05_shift_amount_always_below_32() {
    let pair = Pair::load();

    // shift == 24 rows (underflow and overflow runs): the mantissa term must
    // vanish entirely, so the result equals the base for EVERY mantissa.
    for j in [0u32, 50, 102, 143, 200, 254, 256, 300, 358, 399, 450, 510] {
        let base = pair.c_of_bits(bits_from(j, 0));
        for m in [0u32, 1, 0x007f_ffff, 0x0040_0000, 0x0055_5555, 0x002a_aaaa] {
            let bits = bits_from(j, m);
            assert_eq!(
                pair.c_of_bits(bits), base,
                "C: shift-24 row j={j} must ignore mantissa {m:#08x}"
            );
            pair.check_bits(bits, &format!("row05 shift24 j={j} m={m:#08x}"));
        }
    }

    // shift == 13 rows (normal + inf/NaN): mantissa term maxes at 0x3ff.
    for j in [113u32, 128, 142, 255, 369, 384, 398, 511] {
        let base = pair.c_of_bits(bits_from(j, 0));
        let full = pair.c_of_bits(bits_from(j, 0x007f_ffff));
        assert_eq!(
            full.wrapping_sub(base),
            0x3ff,
            "C: shift-13 row j={j} mantissa term must max at 0x3ff"
        );
        pair.check_bits(bits_from(j, 0x007f_ffff), &format!("row05 shift13 j={j}"));
    }

    // The 10 singleton subnormal rows: shifts 23..14, again with max mantissa.
    for j in (103..=112u32).chain(359..=368u32) {
        pair.check_bits(bits_from(j, 0x007f_ffff), &format!("row05 singleton j={j}"));
        pair.check_bits(bits_from(j, 0x007f_fffe), &format!("row05 singleton j={j}"));
    }
}

// ---------------------------------------------------------------------------
// Row 6 — the uint32 sum never overflows, and narrowing to uint16 never loses.
// ---------------------------------------------------------------------------

#[test]
fn err_06_sum_never_overflows_u16() {
    let pair = Pair::load();
    // Maximal base (0xfc00) rows with maximal mantissa term.
    for j in [255u32, 511, 398, 142] {
        let base = pair.c_of_bits(bits_from(j, 0));
        let r = pair.c_of_bits(bits_from(j, 0x007f_ffff));
        // The sum stayed in range iff it is still >= the base it was built from.
        assert!(
            r >= base,
            "C result {r:#06x} wrapped below its base {base:#06x} at j={j}"
        );
        pair.check_bits(bits_from(j, 0x007f_ffff), &format!("row06 j={j}"));
    }
    // Brute-force the invariant over the entire index space with the mantissa
    // that maximises the added term, comparing C and Rust each time.
    for j in 0..512u32 {
        pair.check_bits(bits_from(j, 0x007f_ffff), &format!("row06 sweep j={j}"));
    }
    // Explicitly: 0xfc00 + 0x3ff == 0xffff, the largest reachable sum.
    assert_eq!(0xfc00u32 + 0x3ff, 0xffff);
    assert_eq!(pair.c_of_bits(bits_from(511, 0x007f_ffff)), 0xffff);
    assert_eq!(pair.rust_of_bits(bits_from(511, 0x007f_ffff)), 0xffff);
}

// ---------------------------------------------------------------------------
// Row 7 — one step past the representable half range (no rejection).
// ---------------------------------------------------------------------------

#[test]
fn err_07_past_half_range_no_rejection() {
    let pair = Pair::load();
    let mut vals = vec![
        65504.0f32, // half max finite
        -65504.0,
        65505.0,
        65519.0,
        65520.0, // first value that maps to inf under round-to-nearest
        -65520.0,
        65521.0,
        65536.0,
        -65536.0,
        131008.0,
        f32::MAX,
        f32::MIN,
        f32::INFINITY,
        f32::NEG_INFINITY,
    ];
    // One ULP either side of the half-max float, both signs.
    let hm = 65504.0f32.to_bits();
    for d in [-2i32, -1, 0, 1, 2] {
        let b = (hm as i32 + d) as u32;
        vals.push(f32::from_bits(b));
        vals.push(f32::from_bits(b | 0x8000_0000));
    }
    for v in vals {
        pair.check_value(v, "row07 past half range");
    }
    // Sweep every exponent in the overflow runs with random mantissas.
    let mut rng = Rng::new(SEED ^ 7);
    for j in (143..=254u32).chain(399..=510u32) {
        for _ in 0..64 {
            pair.check_bits(
                bits_from(j, rng.next_u32() & 0x007f_ffff),
                &format!("row07 overflow j={j}"),
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Row 8 — one step past the subnormal floor (underflow, no rejection).
// ---------------------------------------------------------------------------

#[test]
fn err_08_underflow_no_rejection() {
    let pair = Pair::load();
    let mut vals = vec![
        5.9604645e-8f32, // half min subnormal
        -5.9604645e-8,
        2.9802322e-8, // half of it -> underflow
        -2.9802322e-8,
        1.0e-10,
        -1.0e-10,
        f32::MIN_POSITIVE,
        -f32::MIN_POSITIVE,
        1.0e-45, // float subnormal
        -1.0e-45,
        0.0,
        -0.0,
    ];
    let s = 5.9604645e-8f32.to_bits();
    for d in [-2i32, -1, 0, 1, 2] {
        let b = (s as i32 + d) as u32;
        vals.push(f32::from_bits(b));
        vals.push(f32::from_bits(b | 0x8000_0000));
    }
    for v in vals {
        pair.check_value(v, "row08 underflow");
    }
    // Sweep every exponent in the underflow runs with random mantissas.
    let mut rng = Rng::new(SEED ^ 8);
    for j in (0..=102u32).chain(256..=358u32) {
        for _ in 0..64 {
            pair.check_bits(
                bits_from(j, rng.next_u32() & 0x007f_ffff),
                &format!("row08 underflow j={j}"),
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Row 9 — inf, quiet NaN, and signalling NaN with EVERY payload class.
// ---------------------------------------------------------------------------

#[test]
fn err_09_nan_inf_snan_bit_exact() {
    let pair = Pair::load();

    // Infinities.
    pair.check_bits(0x7F80_0000, "row09 +inf");
    pair.check_bits(0xFF80_0000, "row09 -inf");

    // Quiet NaNs (mantissa top bit set) and signalling NaNs (top bit clear,
    // mantissa non-zero). 0x7F80_0001 is the canonical minimal sNaN.
    for base in [0x7F80_0000u32, 0xFF80_0000u32] {
        for m in [
            0x0000_0001, // minimal sNaN
            0x0000_0002,
            0x0000_1FFF, // payload entirely inside the discarded low 13 bits
            0x0000_2000, // first payload bit that survives the >>13
            0x0020_0000,
            0x003F_FFFF, // max sNaN payload
            0x0040_0000, // minimal qNaN
            0x0040_0001,
            0x005F_FFFF,
            0x007F_FFFF, // max qNaN payload
        ] {
            pair.check_bits(base | m, &format!("row09 NaN payload {m:#08x}"));
        }
    }

    // Exhaustive over ALL 2^23 NaN/inf mantissas for both signs: this is the
    // class the C treats specially via shift 13 at j=255/511, and the class a
    // careless Rust translation would canonicalise.
    for m in 0..0x0080_0000u32 {
        let pos = 0x7F80_0000u32 | m;
        let neg = 0xFF80_0000u32 | m;
        let (cp, rp) = (pair.c_of_bits(pos), pair.rust_of_bits(pos));
        assert_eq!(cp, rp, "row09 +NaN mantissa {m:#08x}: C {cp:#06x} Rust {rp:#06x}");
        let (cn, rn) = (pair.c_of_bits(neg), pair.rust_of_bits(neg));
        assert_eq!(cn, rn, "row09 -NaN mantissa {m:#08x}: C {cn:#06x} Rust {rn:#06x}");
    }

    // Passing a NaN through must not be canonicalised by either side: an sNaN
    // whose payload survives the shift must still be distinguishable.
    let a = pair.c_of_bits(0x7F80_2000);
    let b = pair.c_of_bits(0x7FC0_0000);
    assert_ne!(a, b, "C must not canonicalise NaN payloads");
    assert_eq!(a, pair.rust_of_bits(0x7F80_2000));
    assert_eq!(b, pair.rust_of_bits(0x7FC0_0000));
}

// ---------------------------------------------------------------------------
// Row 10 — signed zeros.
// ---------------------------------------------------------------------------

#[test]
fn err_10_signed_zeros() {
    let pair = Pair::load();
    pair.check_value(0.0, "row10 +0.0");
    pair.check_value(-0.0, "row10 -0.0");
    pair.check_bits(0x0000_0000, "row10 +0 bits");
    pair.check_bits(0x8000_0000, "row10 -0 bits");
    assert_eq!(pair.c_of_bits(0x0000_0000), 0x0000);
    assert_eq!(pair.rust_of_bits(0x0000_0000), 0x0000);
    assert_eq!(pair.c_of_bits(0x8000_0000), 0x8000);
    assert_eq!(pair.rust_of_bits(0x8000_0000), 0x8000);
    // Every float subnormal is also a "zero" for binary16 purposes; sweep the
    // whole j=0 / j=256 rows exhaustively (all 2^23 mantissas each).
    for m in 0..0x0080_0000u32 {
        assert_eq!(
            pair.c_of_bits(m),
            pair.rust_of_bits(m),
            "row10 +subnormal mantissa {m:#08x}"
        );
        let n = 0x8000_0000u32 | m;
        assert_eq!(
            pair.c_of_bits(n),
            pair.rust_of_bits(n),
            "row10 -subnormal mantissa {m:#08x}"
        );
    }
}
