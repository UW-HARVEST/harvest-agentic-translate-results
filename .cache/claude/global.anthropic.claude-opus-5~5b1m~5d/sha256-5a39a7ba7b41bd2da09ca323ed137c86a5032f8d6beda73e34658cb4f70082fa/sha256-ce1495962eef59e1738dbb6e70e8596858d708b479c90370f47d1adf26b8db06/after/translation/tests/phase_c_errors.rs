//! Phase C — error/rejection-path differential tests.
//!
//! One `#[test]` per row of `ERRORS.md` (rows 1..=11; row 12, the exhaustive
//! statement, lives in `phase_d_exhaustive.rs`).
//!
//! `float2half` is a total, branchless function: the C source contains zero
//! error returns, zero asserts, zero range checks, zero pointers, zero length
//! parameters and zero enums (see `ERRORS.md` for the mechanical grep counts).
//! So "the same error/rejection" here means: for every input that *would* be an
//! error case in a less total API, the two implementations must agree on the
//! exact returned `u16` and neither may trap, panic or abort.
//!
//! These tests also pin the *specific* value C returns for each condition, so
//! they cannot pass by "both failed somehow" — nor by both returning some
//! unrelated value.

mod common;

use common::*;

// ---------------------------------------------------------------------------
// Row 1: table index domain - the extreme indices j == 0 and j == 511.
// C masks with & 0x1ff so an out-of-bounds read is impossible; Rust indexes
// fixed 512-element arrays with the same masked value and must not panic.
// ---------------------------------------------------------------------------

#[test]
fn err_row01_index_domain_never_out_of_bounds() {
    let libs = Libs::load();

    // j == 511: every bit set. base=0xFC00, shift=13, mantissa=0x7FFFFF.
    let all_ones = 0xFFFF_FFFFu32;
    assert_eq!(index_of(all_ones), 511, "sanity: 0xFFFFFFFF maps to j=511");
    let c = libs.c(f32::from_bits(all_ones));
    let r = libs.rust(f32::from_bits(all_ones));
    assert_eq!(
        c, 0xFFFF,
        "C is expected to return exactly 0xFFFF for 0xFFFFFFFF (0xFC00 + 0x3FF)"
    );
    assert_eq!(r, c, "Rust must match C at j=511");

    // j == 0: all bits clear.
    let zero = 0x0000_0000u32;
    assert_eq!(index_of(zero), 0, "sanity: 0x00000000 maps to j=0");
    let c = libs.c(f32::from_bits(zero));
    let r = libs.rust(f32::from_bits(zero));
    assert_eq!(c, 0x0000, "C is expected to return exactly 0x0000 for +0.0");
    assert_eq!(r, c, "Rust must match C at j=0");

    // The neighbours of both extremes.
    for bits in [0x0000_0001u32, 0x0080_0000, 0xFFFF_FFFE, 0xFF7F_FFFF] {
        check_bits(&libs, bits, "row01 index-domain neighbour");
    }
}

// ---------------------------------------------------------------------------
// Row 2: all 512 indices are reachable and safe, driven from the input side.
// ---------------------------------------------------------------------------

#[test]
fn err_row02_all_512_indices_reachable_and_safe() {
    let libs = Libs::load();
    let mut seen = [false; 512];
    for sign in 0..2u32 {
        for exp in 0..256u32 {
            let bits = (sign << 31) | (exp << 23);
            let j = index_of(bits) as usize;
            seen[j] = true;
            // mantissa 0, 1, max: three probes per index, no trap allowed.
            for m in [0u32, 1, 0x7F_FFFF] {
                check_fields(&libs, sign, exp, m, "row02 index reachable");
            }
        }
    }
    assert!(
        seen.iter().all(|&b| b),
        "not all 512 table indices were reached from the input side"
    );
}

// ---------------------------------------------------------------------------
// Row 3: maximum shift amount (24). Well-defined in C (24 < 32); a Rust shift
// at/over the bit width would panic. Result must be exactly `base`.
// ---------------------------------------------------------------------------

#[test]
fn err_row03_max_shift_amount_24() {
    let libs = Libs::load();
    let (base, shift) = read_c_tables();

    let mut probed = 0usize;
    let mut rng = Rng::new(SEED ^ 0x0303);
    for j in 0..512usize {
        if shift[j] != 24 {
            continue;
        }
        probed += 1;
        let (sign, exp) = ((j >> 8) as u32, (j & 0xFF) as u32);
        // With shift 24 the whole 23-bit mantissa is discarded, so EVERY
        // mantissa must give exactly base[j].
        let mut mantissas = vec![0u32, 1, 0x7F_FFFF, 0x40_0000, 0x00_0FFF];
        for _ in 0..16 {
            mantissas.push(rng.next_u32() & 0x7F_FFFF);
        }
        for m in mantissas {
            let x = make_f32(sign, exp, m);
            let c = libs.c(x);
            let r = libs.rust(x);
            assert_eq!(
                c, base[j],
                "shift==24 must discard the mantissa: j={j} mantissa=0x{m:06X} \
                 -> C gave 0x{c:04X}, expected base 0x{:04X}",
                base[j]
            );
            assert_eq!(
                r, c,
                "Rust must match C at shift==24: j={j} mantissa=0x{m:06X} \
                 -> C 0x{c:04X}, Rust 0x{r:04X}"
            );
        }
    }
    assert_eq!(
        probed, 430,
        "expected 430 table indices with shift==24, probed {probed}"
    );
}

// ---------------------------------------------------------------------------
// Row 4: the sum never overflows u16. Max attainable is exactly 0xFFFF.
// A plain `u16 + u16` in Rust would panic on overflow in a debug build; this
// asserts the exact maximum is produced rather than a wrap.
// ---------------------------------------------------------------------------

#[test]
fn err_row04_sum_never_overflows_u16() {
    let libs = Libs::load();
    let (base, shift) = read_c_tables();

    // Mechanically confirm the worst case over all 512 indices.
    let mut worst = 0u32;
    let mut worst_j = 0usize;
    for j in 0..512usize {
        let s = base[j] as u32 + (0x7F_FFFFu32 >> shift[j]);
        if s > worst {
            worst = s;
            worst_j = j;
        }
    }
    assert_eq!(
        worst, 0xFFFF,
        "worst-case sum should be exactly 0xFFFF (at j={worst_j}), got 0x{worst:X}"
    );
    assert_eq!(worst_j, 511, "worst case should occur at j=511");

    // Now exercise the worst case and the runner-up per index through the FFI.
    for j in 0..512usize {
        let (sign, exp) = ((j >> 8) as u32, (j & 0xFF) as u32);
        for m in [0x7F_FFFFu32, 0x7F_FFFE, 0x7F_E000] {
            let x = make_f32(sign, exp, m);
            let c = libs.c(x);
            let r = libs.rust(x);
            let expect = (base[j] as u32).wrapping_add(m >> shift[j]) as u16;
            assert_eq!(
                c, expect,
                "C deviated from the table model at j={j} mantissa=0x{m:06X}"
            );
            assert_eq!(
                r, c,
                "Rust must match C at the arithmetic maximum: j={j} \
                 mantissa=0x{m:06X} -> C 0x{c:04X}, Rust 0x{r:04X}"
            );
        }
    }

    // The single global maximum, spelled out.
    assert_eq!(libs.c(f32::from_bits(0xFFFF_FFFF)), 0xFFFF);
    assert_eq!(libs.rust(f32::from_bits(0xFFFF_FFFF)), 0xFFFF);
    // ...and the positive mirror: 0x7C00 + 0x3FF.
    assert_eq!(libs.c(f32::from_bits(0x7FFF_FFFF)), 0x7FFF);
    assert_eq!(libs.rust(f32::from_bits(0x7FFF_FFFF)), 0x7FFF);
}

// ---------------------------------------------------------------------------
// Row 5: signalling NaN. The C only type-puns the bits; it never does FP
// arithmetic, so the payload is read verbatim and no FP exception is raised.
// ---------------------------------------------------------------------------

#[test]
fn err_row05_signalling_nan_payload_not_quieted() {
    let libs = Libs::load();

    // +sNaN 0x7FA00000: mantissa 0x200000 -> 0x200000 >> 13 == 0x100.
    let c = libs.c(f32::from_bits(0x7FA0_0000));
    let r = libs.rust(f32::from_bits(0x7FA0_0000));
    assert_eq!(
        c, 0x7D00,
        "C should read the sNaN payload verbatim: expected 0x7C00 + 0x100"
    );
    assert_eq!(r, c, "Rust must not quiet the sNaN payload (+sNaN)");

    // -sNaN 0xFFA00000.
    let c = libs.c(f32::from_bits(0xFFA0_0000));
    let r = libs.rust(f32::from_bits(0xFFA0_0000));
    assert_eq!(c, 0xFD00, "C: 0xFC00 + 0x100");
    assert_eq!(r, c, "Rust must not quiet the sNaN payload (-sNaN)");

    // A spread of signalling payloads: exponent 255 with bit 22 CLEAR and a
    // non-zero payload is signalling. Sweep them all densely.
    let mut rng = Rng::new(SEED ^ 0x0505_0505);
    for _ in 0..20_000 {
        let payload = (rng.next_u32() & 0x3F_FFFF).max(1); // bit22 clear, non-zero
        for sign in 0..2u32 {
            check_fields(&libs, sign, 255, payload, "row05 sNaN sweep");
        }
    }
    // Every sNaN payload that is a power of two, plus its neighbours.
    for k in 0..22u32 {
        let p = 1u32 << k;
        for m in [p - 1, p, p + 1] {
            let m = m.max(1);
            for sign in 0..2u32 {
                check_fields(&libs, sign, 255, m, "row05 sNaN power-of-two");
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 6: NaN with a payload small enough to be shifted away degenerates to
// Infinity. This is "wrong" but it is what the C does, so it must be matched.
// ---------------------------------------------------------------------------

#[test]
fn err_row06_nan_degenerates_to_infinity() {
    let libs = Libs::load();

    // +NaN with mantissa 1 -> 1 >> 13 == 0 -> 0x7C00 == +Inf.
    let c = libs.c(f32::from_bits(0x7F80_0001));
    let r = libs.rust(f32::from_bits(0x7F80_0001));
    assert_eq!(c, 0x7C00, "C maps a tiny-payload +NaN to +Inf (0x7C00)");
    assert_eq!(r, c, "Rust must replicate NaN -> +Inf");
    assert!(
        f32::from_bits(0x7F80_0001).is_nan(),
        "sanity: the input really is a NaN"
    );

    // -NaN with mantissa 1 -> 0xFC00 == -Inf.
    let c = libs.c(f32::from_bits(0xFF80_0001));
    let r = libs.rust(f32::from_bits(0xFF80_0001));
    assert_eq!(c, 0xFC00, "C maps a tiny-payload -NaN to -Inf (0xFC00)");
    assert_eq!(r, c, "Rust must replicate NaN -> -Inf");

    // EVERY payload in 1..=0x1FFF is shifted away, so all of them must give
    // exactly Inf - and the Rust must agree on every one.
    for m in 1..=0x1FFFu32 {
        let cp = libs.c(make_f32(0, 255, m));
        let rp = libs.rust(make_f32(0, 255, m));
        assert_eq!(cp, 0x7C00, "payload 0x{m:04X} should shift away to +Inf");
        assert_eq!(rp, cp, "Rust diverged for +NaN payload 0x{m:04X}");
        let cn = libs.c(make_f32(1, 255, m));
        let rn = libs.rust(make_f32(1, 255, m));
        assert_eq!(cn, 0xFC00, "payload 0x{m:04X} should shift away to -Inf");
        assert_eq!(rn, cn, "Rust diverged for -NaN payload 0x{m:04X}");
    }
    // 0x2000 is the first payload that survives.
    assert_eq!(libs.c(make_f32(0, 255, 0x2000)), 0x7C01);
    assert_eq!(libs.rust(make_f32(0, 255, 0x2000)), 0x7C01);
}

// ---------------------------------------------------------------------------
// Row 7: ALL NaN payloads, both signs. Exhaustive over the only
// value-dependent special path (j == 255 / 511, shift == 13).
// ---------------------------------------------------------------------------

#[test]
fn err_row07_all_nan_payloads_both_signs() {
    let libs = Libs::load();
    for sign in 0..2u32 {
        let base = if sign == 0 { 0x7C00u16 } else { 0xFC00u16 };
        for m in 0..=0x7F_FFFFu32 {
            let x = make_f32(sign, 255, m);
            let c = libs.c(x);
            let r = libs.rust(x);
            let expect = base + (m >> 13) as u16;
            assert_eq!(
                c, expect,
                "C deviated from base+(mant>>13) at sign={sign} payload=0x{m:06X}"
            );
            assert_eq!(
                r, c,
                "Rust diverged at sign={sign} payload=0x{m:06X}: C 0x{c:04X}, Rust 0x{r:04X}"
            );
        }
    }
    // The full span is covered: 0x7C00..=0x7FFF and 0xFC00..=0xFFFF.
    assert_eq!(libs.rust(make_f32(0, 255, 0)), 0x7C00);
    assert_eq!(libs.rust(make_f32(0, 255, 0x7F_FFFF)), 0x7FFF);
    assert_eq!(libs.rust(make_f32(1, 255, 0)), 0xFC00);
    assert_eq!(libs.rust(make_f32(1, 255, 0x7F_FFFF)), 0xFFFF);
}

// ---------------------------------------------------------------------------
// Row 8: one step past the finite-binary16 range (j 142 -> 143, shift 13 -> 24)
// ---------------------------------------------------------------------------

#[test]
fn err_row08_one_past_finite_half_range() {
    let libs = Libs::load();

    // Last exponent that produces a finite half (142) vs. the first that
    // saturates (143).
    for sign in 0..2u32 {
        for m in [0u32, 1, 0x1FFF, 0x2000, 0x7F_FFFF] {
            check_fields(&libs, sign, 142, m, "row08 last finite exponent");
            check_fields(&libs, sign, 143, m, "row08 first saturating exponent");
        }
    }

    // 65504.0 is the largest finite binary16; 65520.0 is the first f32 that
    // rounds/truncates past it in this scheme.
    check_bits(&libs, 65504.0f32.to_bits(), "row08 half max");
    check_bits(&libs, 65505.0f32.to_bits(), "row08 half max + eps");
    check_bits(&libs, 65519.0f32.to_bits(), "row08 just below overflow");
    check_bits(&libs, 65520.0f32.to_bits(), "row08 overflow point");
    check_bits(&libs, (-65504.0f32).to_bits(), "row08 -half max");
    check_bits(&libs, (-65520.0f32).to_bits(), "row08 -overflow point");

    // Across the whole saturating region 143..=254 the mantissa is discarded,
    // so the answer must be exactly Inf for every mantissa.
    let mut rng = Rng::new(SEED ^ 0x0808);
    for exp in 143..=254u32 {
        for sign in 0..2u32 {
            let want = if sign == 0 { 0x7C00u16 } else { 0xFC00u16 };
            for _ in 0..8 {
                let m = rng.next_u32() & 0x7F_FFFF;
                let x = make_f32(sign, exp, m);
                let c = libs.c(x);
                let r = libs.rust(x);
                assert_eq!(
                    c, want,
                    "C should saturate to Inf at exp={exp} sign={sign} mant=0x{m:06X}"
                );
                assert_eq!(r, c, "Rust diverged in the saturating region");
            }
        }
    }
    // Exponent 255 is NOT in the saturating region: it uses shift 13.
    assert_ne!(
        libs.c(make_f32(0, 255, 0x7F_FFFF)),
        0x7C00,
        "exp 255 must NOT discard the mantissa (shift is 13, not 24)"
    );
    assert_eq!(
        libs.rust(make_f32(0, 255, 0x7F_FFFF)),
        libs.c(make_f32(0, 255, 0x7F_FFFF))
    );
}

// ---------------------------------------------------------------------------
// Row 9: one step past representable - underflow (j 103 -> 102), and the fact
// that negative underflow yields -0 (0x8000), not 0.
// ---------------------------------------------------------------------------

#[test]
fn err_row09_one_past_representable_underflow() {
    let libs = Libs::load();

    // exp 103 still produces a non-zero half subnormal at large mantissas;
    // exp 102 and below flush to zero for EVERY mantissa.
    let mut rng = Rng::new(SEED ^ 0x0909);
    for exp in 0..=102u32 {
        for sign in 0..2u32 {
            let want = if sign == 0 { 0x0000u16 } else { 0x8000u16 };
            let mut mantissas = vec![0u32, 1, 0x7F_FFFF, 0x40_0000];
            for _ in 0..8 {
                mantissas.push(rng.next_u32() & 0x7F_FFFF);
            }
            for m in mantissas {
                let x = make_f32(sign, exp, m);
                let c = libs.c(x);
                let r = libs.rust(x);
                assert_eq!(
                    c, want,
                    "C should flush to {}0 at exp={exp} mant=0x{m:06X}, got 0x{c:04X}",
                    if sign == 1 { "-" } else { "+" }
                );
                assert_eq!(r, c, "Rust diverged in the underflow region");
            }
        }
    }

    // exp 103 boundary: shift is 23, so only mantissa >= 0x800000 would add -
    // impossible - hence base 0x0001 exactly, for every mantissa.
    for sign in 0..2u32 {
        for m in [0u32, 1, 0x7F_FFFF] {
            let c = libs.c(make_f32(sign, 103, m));
            let r = libs.rust(make_f32(sign, 103, m));
            let want = if sign == 0 { 0x0001u16 } else { 0x8001u16 };
            assert_eq!(c, want, "exp 103 should give the smallest half subnormal");
            assert_eq!(r, c, "Rust diverged at the underflow boundary");
        }
    }

    // Negative underflow really is -0, not +0.
    assert_eq!(libs.c(f32::from_bits(0x8000_0000)), 0x8000);
    assert_eq!(libs.rust(f32::from_bits(0x8000_0000)), 0x8000);
    assert_eq!(libs.c(-1e-45f32), 0x8000);
    assert_eq!(libs.rust(-1e-45f32), 0x8000);
}

// ---------------------------------------------------------------------------
// Row 10: zeros and infinities
// ---------------------------------------------------------------------------

#[test]
fn err_row10_zeros_and_infinities() {
    let libs = Libs::load();
    let cases: &[(u32, u16, &str)] = &[
        (0x0000_0000, 0x0000, "+0.0"),
        (0x8000_0000, 0x8000, "-0.0"),
        (0x7F80_0000, 0x7C00, "+Inf"),
        (0xFF80_0000, 0xFC00, "-Inf"),
    ];
    for &(bits, want, name) in cases {
        let x = f32::from_bits(bits);
        let c = libs.c(x);
        let r = libs.rust(x);
        assert_eq!(c, want, "C({name}) should be 0x{want:04X}, got 0x{c:04X}");
        assert_eq!(r, c, "Rust({name}) 0x{r:04X} != C 0x{c:04X}");
    }
}

// ---------------------------------------------------------------------------
// Row 11: float subnormal inputs are not special-cased; all flush to +/-0.
// ---------------------------------------------------------------------------

#[test]
fn err_row11_float_subnormal_inputs() {
    let libs = Libs::load();
    let mut rng = Rng::new(SEED ^ 0x1111);

    // Exhaustive over the interesting structure plus dense random coverage.
    let mut mantissas: Vec<u32> = vec![1, 2, 3, 0x7F_FFFF, 0x7F_FFFE, 0x40_0000];
    for k in 0..23u32 {
        mantissas.push(1u32 << k);
    }
    for _ in 0..50_000 {
        mantissas.push((rng.next_u32() & 0x7F_FFFF).max(1));
    }
    for m in mantissas {
        for sign in 0..2u32 {
            let x = make_f32(sign, 0, m);
            assert!(
                x.is_subnormal(),
                "sanity: mantissa 0x{m:06X} at exp 0 should be subnormal"
            );
            let c = libs.c(x);
            let r = libs.rust(x);
            let want = if sign == 0 { 0x0000u16 } else { 0x8000u16 };
            assert_eq!(
                c, want,
                "C should flush the float subnormal 0x{m:06X} to {}0",
                if sign == 1 { "-" } else { "+" }
            );
            assert_eq!(r, c, "Rust diverged on a float subnormal input");
        }
    }
}

// ---------------------------------------------------------------------------
// Generic-boundary coverage that the table does not enumerate, restated
// explicitly so the record shows it was considered rather than skipped.
// ---------------------------------------------------------------------------

/// `float2half` takes no pointer, no length and no enum, so the classic
/// generic boundary cases are *inapplicable by construction*. This test
/// documents and mechanically re-verifies that fact against the C header, so
/// the claim cannot silently rot if the header ever grows a parameter.
#[test]
fn err_generic_boundaries_are_inapplicable_by_construction() {
    let header = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("c_src/include/lib.h"),
    )
    .expect("read c_src/include/lib.h");

    // Exactly one declaration, exactly one `float` parameter.
    let decls: Vec<&str> = header
        .lines()
        .filter(|l| l.contains('(') && l.contains(';'))
        .collect();
    assert_eq!(
        decls.len(),
        1,
        "the public header should declare exactly one function, found: {decls:?}"
    );
    let d = decls[0];
    assert!(
        d.contains("uint16_t float2half(float"),
        "unexpected public signature: {d}"
    );

    // No pointers => no null-pointer case. No size/len => no zero/oversized
    // length case. No enum => no out-of-range enum case.
    assert!(!d.contains('*'), "signature grew a pointer parameter: {d}");
    assert!(
        !d.contains("size") && !d.contains("len") && !d.contains("count"),
        "signature grew a length parameter: {d}"
    );
    assert!(
        !header.contains("enum"),
        "header grew an enum; out-of-range enum values would need testing"
    );
    assert!(
        !header.contains("struct"),
        "header grew a struct; its field combinations would need testing"
    );
}

/// The one "out-of-range value crossing the FFI boundary" that *is* meaningful
/// here: because the parameter is a `float`, the analogue of an invalid enum
/// value is a bit pattern that is not a number at all (NaN / sNaN) or that sits
/// one step past every documented range endpoint. Those are covered by rows
/// 5-9; this test additionally hammers the immediate neighbourhood of every
/// power-of-two exponent boundary from both directions.
#[test]
fn err_one_step_past_every_exponent_boundary() {
    let libs = Libs::load();
    for exp in 0..256u32 {
        for sign in 0..2u32 {
            // Last mantissa of this exponent and first of the next: adjacent
            // f32 values that straddle an exponent (hence table-index) change.
            check_fields(&libs, sign, exp, 0x7F_FFFF, "boundary last-of-exp");
            check_fields(&libs, sign, exp, 0, "boundary first-of-exp");
            // And the raw bit-pattern neighbours across the boundary.
            let bits = (sign << 31) | (exp << 23);
            check_bits(&libs, bits.wrapping_sub(1), "boundary bits-1");
            check_bits(&libs, bits.wrapping_add(1), "boundary bits+1");
        }
    }
}
