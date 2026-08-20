//! Phase C — error-path differential tests, one test per row of ERRORS.md.
//!
//! `half2float` has no explicit rejection path (see ERRORS.md for the
//! mechanical grep), so these tests cover the implicit boundaries: the exact
//! lower/upper index limits of all three lookup tables, the `uint32_t`
//! addition, the union type-pun, and the FFI-signature boundary (values pushed
//! across a widened prototype, which is how an "out of declared range" value
//! actually reaches this C API).

mod common;

use common::*;

// ---------------------------------------------------------------------- E1
#[test]
fn e1_lower_bound_index_zero() {
    // h = 0x0000 -> n = 0, m__offset[0] = 0, mantissa index 0 (lower bound).
    let bits = assert_same("E1", 0x0000);
    assert_eq!(bits, 0x0000_0000, "C returns +0.0 for the smallest input");
    assert_eq!(bits, oracle_bits(0x0000));
    // Neither library rejects it: no error sentinel, and the value is not NaN.
    assert!(!f32::from_bits(bits).is_nan());
}

// ---------------------------------------------------------------------- E2
#[test]
fn e2_upper_bound_all_tables() {
    // h = 0xFFFF -> n = 63 (last row of both 64-entry tables) and mantissa
    // index 1023 + 0x400 = 2047 (last element of the 2048-entry table).
    let bits = assert_same("E2", 0xFFFF);
    assert_eq!(bits, 0xFFFF_E000, "C returns 0xFFFFE000 for the largest input");
    assert_eq!(bits, oracle_bits(0xFFFF));
    // Also the neighbours, so an off-by-one is caught in either direction.
    for h in 0xFFFDu16..=0xFFFF {
        assert_eq!(assert_same("E2/neighbours", h), oracle_bits(h));
    }
}

// ---------------------------------------------------------------------- E3
#[test]
fn e3_exponent_offset_table_row_63() {
    // Every h with n = 63 reads m__offset[63] / m__exponent[63]; index 64
    // would be out of bounds for the C arrays.
    for h in 0xFC00u16..=0xFFFF {
        let bits = assert_same("E3", h);
        assert_eq!(bits, oracle_bits(h));
    }
    // The same for row 31, the other special exponent row.
    for h in 0x7C00u16..=0x7FFF {
        assert_eq!(assert_same("E3/row31", h), oracle_bits(h));
    }
}

// ---------------------------------------------------------------------- E4
#[test]
fn e4_mantissa_index_2047_every_row() {
    // (h & 0x3ff) == 0x3ff and m__offset[n] == 0x400 -> index 2047.
    let t = c_tables();
    let mut hit = 0;
    for n in 0..64u16 {
        if t.offset[n as usize] != 0x400 {
            continue;
        }
        let h = h_from(n, 0x3FF);
        assert_eq!(assert_same("E4", h), oracle_bits(h));
        hit += 1;
    }
    assert_eq!(hit, 62, "62 of the 64 rows use offset 0x400");
}

// ---------------------------------------------------------------------- E5
#[test]
fn e5_mantissa_index_1023_offset_zero_rows() {
    // The other index region: m__offset[n] == 0x0000 only for n = 0 and n = 32.
    let t = c_tables();
    let zero_rows: Vec<u16> = (0..64u16)
        .filter(|&n| t.offset[n as usize] == 0x0000)
        .collect();
    assert_eq!(zero_rows, vec![0, 32], "offset 0 rows, from the C source");
    for n in zero_rows {
        let h = h_from(n, 0x3FF); // index 1023
        assert_eq!(assert_same("E5", h), oracle_bits(h));
        let h = h_from(n, 0x3FE); // index 1022, guards an off-by-one
        assert_eq!(assert_same("E5", h), oracle_bits(h));
    }
}

// ---------------------------------------------------------------------- E6
#[test]
fn e6_uint32_addition_never_traps() {
    // C's unsigned addition wraps; Rust's `+` would panic in a debug build.
    // Prove over the whole domain that (a) no sum exceeds u32 and (b) both
    // libraries return the exact sum without trapping.
    let t = c_tables();
    let mut max_sum = 0u32;
    let mut wraps = 0;
    for h in 0..=u16::MAX {
        let n = (h >> 10) as usize;
        let idx = (h & 0x3ff) as usize + t.offset[n] as usize;
        let wide = t.mantissa[idx] as u64 + t.exponent[n] as u64;
        if wide > u32::MAX as u64 {
            wraps += 1;
        }
        max_sum = max_sum.max((wide & 0xFFFF_FFFF) as u32);
    }
    assert_eq!(wraps, 0, "no input makes the C addition wrap");
    assert_eq!(max_sum, 0xFFFF_E000);

    // The worst-case rows (largest exponent addends) must not trap in Rust.
    for n in [30u16, 31, 62, 63] {
        for m in [0u16, 1, 1022, 1023] {
            let h = h_from(n, m);
            assert_eq!(assert_same("E6", h), oracle_bits(h));
        }
    }
}

// ---------------------------------------------------------------------- E7
#[test]
fn e7_value_past_uint16_range_via_widened_prototype() {
    // A caller with a wider prototype -- C's uint16_t parameter truncates.
    let cases: &[u32] = &[
        0x0001_0000,
        0x0001_0001,
        0x0001_FFFF,
        0x0002_3C00,
        0x00FF_FFFF,
        0xDEAD_BEEF,
        0xFFFF_0000,
        0xFFFF_FFFF,
        u32::MAX - 1,
        0x8000_8000,
    ];
    for &v in cases {
        let bits = assert_same_wide("E7", v);
        // ... and the result must equal the in-range call on the low 16 bits.
        let truncated = (v & 0xFFFF) as u16;
        assert_eq!(
            bits,
            oracle_bits(truncated),
            "wide 0x{v:08X} should behave like half2float(0x{truncated:04X})"
        );
        assert_eq!(bits, assert_same("E7/truncated", truncated));
    }

    let mut rng = Rng::new(SEED ^ 7);
    for _ in 0..20_000 {
        let v = rng.next_u64() as u32;
        let bits = assert_same_wide("E7/rand", v);
        assert_eq!(bits, oracle_bits((v & 0xFFFF) as u16));
    }
}

// ---------------------------------------------------------------------- E8
#[test]
fn e8_negative_and_sign_extended_values() {
    // The "out-of-range enum passed as int" analogue: C accepts any int at a
    // uint16_t parameter, so negative values are a real input.
    let cases: &[i32] = &[-1, -2, -1024, -32768, -32769, -65536, -65537, i32::MIN, i32::MIN + 1];
    for &v in cases {
        let raw = v as u32;
        let bits = assert_same_wide("E8", raw);
        let truncated = (raw & 0xFFFF) as u16;
        assert_eq!(
            bits,
            oracle_bits(truncated),
            "int {v} (0x{raw:08X}) should behave like half2float(0x{truncated:04X})"
        );
    }

    let mut rng = Rng::new(SEED ^ 8);
    for _ in 0..20_000 {
        let v = -((rng.next_u64() % 0x1_0000_0000) as i64) as i32;
        let raw = v as u32;
        assert_eq!(assert_same_wide("E8/rand", raw), oracle_bits((raw & 0xFFFF) as u16));
    }
}

// ---------------------------------------------------------------------- E9
#[test]
fn e9_union_pun_preserves_nan_payload_and_sign() {
    // Rows 31 and 63 with a non-zero mantissa produce NaNs. A float-comparison
    // implementation would pass while corrupting the payload, so compare bits.
    let mut checked = 0;
    for n in [31u16, 63] {
        for m in 1..=1023u16 {
            let h = h_from(n, m);
            let bits = assert_same("E9", h);
            assert_eq!(bits, oracle_bits(h));
            let f = f32::from_bits(bits);
            assert!(f.is_nan(), "0x{h:04X} -> 0x{bits:08X} must be NaN");
            // NaN != NaN, so equality on floats can never be the check here.
            assert!(f != f);
            assert_eq!(
                bits >> 31,
                if n == 63 { 1 } else { 0 },
                "sign bit of 0x{h:04X} was not preserved"
            );
            assert_ne!(bits & 0x007F_FFFF, 0);
            checked += 1;
        }
    }
    assert_eq!(checked, 2046, "all 2046 NaN payloads");

    // Signalling-NaN boundary: payload with only the lowest bit set, and the
    // quiet-bit boundary payload.
    for h in [0x7C01u16, 0x7DFF, 0x7E00, 0xFC01, 0xFDFF, 0xFE00] {
        assert_eq!(assert_same("E9/snan", h), oracle_bits(h));
    }

    // Signed zeros: same class, different bits -- `==` would call them equal.
    let pz = assert_same("E9/+0", 0x0000);
    let nz = assert_same("E9/-0", 0x8000);
    assert_eq!(f32::from_bits(pz), f32::from_bits(nz), "+0.0 == -0.0 as floats");
    assert_ne!(pz, nz, "but the bit patterns must differ");
}

// ---------------------------------------------------------------------- E10
#[test]
fn e10_no_hidden_state_across_repeated_calls() {
    // The C tables are non-const `static` objects, so verify no call mutates
    // them: hammer a handful of inputs, interleaved, many times.
    let l = libs();
    let c = l.c_fn();
    let variants = l.rust_variants();

    let probes: Vec<u16> = vec![
        0x0000, 0x8000, 0x0001, 0x03FF, 0x0400, 0x3C00, 0x7BFF, 0x7C00, 0x7C01, 0x7FFF, 0x8001,
        0xBC00, 0xFBFF, 0xFC00, 0xFC01, 0xFFFF,
    ];
    let first: Vec<u32> = probes.iter().map(|&h| unsafe { c(h) }.to_bits()).collect();

    let mut rng = Rng::new(SEED ^ 10);
    for _ in 0..2000 {
        // Random traffic in between, to try to disturb any hidden state.
        let noise = rng.next_u16();
        let _ = unsafe { c(noise) };
        for (_, r) in &variants {
            let _ = unsafe { r(noise) };
        }
        for (k, &h) in probes.iter().enumerate() {
            let cb = unsafe { c(h) }.to_bits();
            assert_eq!(cb, first[k], "C mutated: half2float(0x{h:04X})");
            for (name, r) in &variants {
                let rb = unsafe { r(h) }.to_bits();
                assert_eq!(rb, first[k], "{name} diverged: half2float(0x{h:04X})");
            }
        }
    }
}

// ------------------------------------------------------- generic boundaries
#[test]
fn generic_boundaries_every_representable_input_is_accepted() {
    // There is no length, count or pointer argument, so the generic
    // null/zero-length/oversized-length boundaries collapse into "the whole
    // 16-bit domain is valid". Assert that literally: no input is rejected,
    // and C and Rust agree on all of them (also covered exhaustively in C18).
    let boundaries: &[u16] = &[
        0x0000, 0x0001, 0x03FF, 0x0400, 0x0401, 0x7BFF, 0x7C00, 0x7FFF, 0x8000, 0x8001, 0x83FF,
        0x8400, 0xFBFF, 0xFC00, 0xFFFE, 0xFFFF,
    ];
    for &h in boundaries {
        let bits = assert_same("generic", h);
        assert_eq!(bits, oracle_bits(h));
    }

    // One step past each table's declared extent, expressed as an input:
    // there is no such input -- n maxes out at 63 and the mantissa index at
    // 2047 for every u16. Prove it from the parsed C tables.
    let t = c_tables();
    for h in 0..=u16::MAX {
        let n = (h >> 10) as usize;
        assert!(n < t.exponent.len() && n < t.offset.len());
        let idx = (h & 0x3ff) as usize + t.offset[n] as usize;
        assert!(
            idx < t.mantissa.len(),
            "0x{h:04X} would index m__mantissa[{idx}] out of bounds"
        );
    }
}
