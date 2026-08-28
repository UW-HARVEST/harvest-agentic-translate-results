//! Phase C — error-path / edge-condition differential tests, gated on `ERRORS.md`.
//!
//! `tritanopia` has no error returns, no asserts, no null checks and no enums
//! (see `ERRORS.md` for the mechanical grep proving this): it is a total function
//! over a 3-byte struct. The "error surface" is therefore the set of implicit
//! edges — the undefined-behaviour float→`unsigned char` conversions whose
//! compiled behaviour must be reproduced exactly, the branch-threshold
//! boundaries, and the FFI register-level boundary conditions.
//!
//! One test per `ERRORS.md` row.

mod common;

use common::*;

/// Inputs whose R output channel lands in the given `cbDenorm` bucket
/// (`0` = negative → wraps, `1` = in range, `2` = `>= 256` → wraps).
/// Classification comes from the C itself, via `tests/data/signatures.txt`.
fn witnesses_with_r_bucket(digit: char) -> Vec<Rgb255> {
    let raw = include_str!("data/signatures.txt");
    let mut out = Vec::new();
    let mut take = false;
    for line in raw.lines() {
        let f: Vec<&str> = line.split_whitespace().collect();
        match f.first().copied() {
            Some("SIG") => take = f[3].starts_with(digit),
            Some("W") if take => out.push(Rgb255::new(
                f[1].parse().unwrap(),
                f[2].parse().unwrap(),
                f[3].parse().unwrap(),
            )),
            _ => {}
        }
    }
    out
}

// ---------------------------------------------------------------------------
// ERRORS.md row 1 — float -> unsigned char with a NEGATIVE value (C UB).
// ---------------------------------------------------------------------------
//
// `cbDenorm` does `(unsigned char)(x * 255.f + 0.5f)`. GCC lowers this to
// `cvttss2si %xmm0,%eax` + `mov %al`, i.e. truncate-toward-zero into a 32-bit
// register and keep the low byte, so negatives WRAP. The tritanopia matrix
// really does produce them: the red row is `R + 0.1274*G - 0.1274*B`, so strong
// blue pushes red to about -1.65 (-419.23 after scaling).
//
// A translation that clamped to 0 (or used Rust's saturating `as u8`) would
// diverge here, so this row is the single most important one in the file.
#[test]
fn row01_denorm_negative_wraps() {
    let inputs = witnesses_with_r_bucket('0');
    assert!(
        !inputs.is_empty(),
        "no negative-bucket witnesses; ERRORS.md row 1 claims they are reachable"
    );
    let n = assert_same_all("ERRORS row 1: cbDenorm negative wrap", inputs.clone());

    // The wrap must be observable, i.e. the C must NOT be returning a clamped 0
    // for all of these; otherwise this test would pass vacuously against a
    // clamping translation.
    let nonzero = inputs.iter().filter(|&&x| call_c(x).r != 0).count();
    assert!(
        nonzero > 0,
        "row 1 is vacuous: every negative-bucket input produced R=0, so wrapping \
         and clamping are indistinguishable here"
    );
    eprintln!("ERRORS row 1: {n} inputs, {nonzero} with a non-zero wrapped R");

    // Pure/near-pure blue is the canonical trigger; check the whole blue ramp
    // with red and green dark.
    let ramp: Vec<Rgb255> = (0..=255u8).map(|b| Rgb255::new(0, 0, b)).collect();
    assert_same_all("ERRORS row 1: blue ramp", ramp);
}

// ---------------------------------------------------------------------------
// ERRORS.md row 2 — float -> unsigned char with a value >= 256 (C UB).
// ---------------------------------------------------------------------------
//
// Same lowering, so values >= 256 also WRAP rather than saturating to 255.
// Max observed `cbDenorm` argument over the whole domain is 269.28 -> 13.
#[test]
fn row02_denorm_overflow_wraps() {
    let inputs = witnesses_with_r_bucket('2');
    assert!(
        !inputs.is_empty(),
        "no overflow-bucket witnesses; ERRORS.md row 2 claims they are reachable"
    );
    let n = assert_same_all("ERRORS row 2: cbDenorm overflow wrap", inputs.clone());

    // Prove the wrap is observable: a saturating translation would emit 255.
    let wrapped = inputs.iter().filter(|&&x| call_c(x).r != 255).count();
    assert!(
        wrapped > 0,
        "row 2 is vacuous: every overflow-bucket input produced R=255, so wrapping \
         and saturating are indistinguishable here"
    );
    eprintln!("ERRORS row 2: {n} inputs, {wrapped} with a wrapped (non-255) R");

    // The trigger region is high red + high green + low blue.
    let mut v = Vec::new();
    for r in 235..=255u8 {
        for g in 235..=255u8 {
            for b in 0..=20u8 {
                v.push(Rgb255::new(r, g, b));
            }
        }
    }
    assert_same_all("ERRORS row 2: high-R/high-G/low-B region", v);
}

// ---------------------------------------------------------------------------
// ERRORS.md row 3 — the `cvttss2si` "integer indefinite" case.
// ---------------------------------------------------------------------------
//
// If the `cbDenorm` argument were NaN or outside i32 range, `cvttss2si` yields
// 0x8000_0000 (low byte 0) rather than saturating. This is UNREACHABLE through
// the public API: measured over all 2^24 inputs the argument range is
// [-419.228302, 269.282959]. `cbDenorm` is `static`, so it cannot be called
// directly through either `.so` to probe it.
//
// What is verifiable at the `.so` boundary is exactly that: no input reaches the
// indefinite case, and both libraries agree everywhere. Asserted by bounding the
// observable output rather than by claiming to have called the helper.
#[test]
fn row03_denorm_indefinite_case_unreachable() {
    // If the indefinite path were ever taken the two libraries would still have
    // to agree; the exhaustive sweep in phase_b covers that. Here we confirm the
    // premise that makes the row moot: every input yields a defined, stable,
    // identical result, and no input trips a trap/abort in either library.
    let mut rng = Rng::new(0x1234_5678_9ABC_DEF0);
    let mut v: Vec<Rgb255> = (0..50_000).map(|_| rng.next_rgb()).collect();
    // include the extremes that maximise |cbDenorm argument|
    v.extend([
        Rgb255::new(0, 0, 255),
        Rgb255::new(255, 255, 0),
        Rgb255::new(0, 255, 255),
        Rgb255::new(255, 0, 255),
        Rgb255::new(241, 254, 0),
    ]);
    assert_same_all("ERRORS row 3: no indefinite-case divergence", v);
}

// ---------------------------------------------------------------------------
// ERRORS.md row 4 — cbRemoveGammaRGB threshold, one step either side.
// ---------------------------------------------------------------------------
//
// `RGB.R > 0.04045` where `RGB.R = byte/255.f`: false at byte 10
// (10/255 = 0.0392…), true at byte 11 (11/255 = 0.0431…). Exercise the flip in
// every channel independently and jointly.
#[test]
fn row04_remove_gamma_threshold_boundary() {
    let bytes = [0u8, 1, 9, 10, 11, 12, 13];
    let mut v = Vec::new();
    for &r in &bytes {
        for &g in &bytes {
            for &b in &bytes {
                v.push(Rgb255::new(r, g, b));
            }
        }
    }
    let n = v.len();
    assert_eq!(
        assert_same_all("ERRORS row 4: removeGamma threshold", v),
        n
    );

    // Also flip the boundary in one channel at a time against a mid background,
    // so a per-channel off-by-one cannot hide behind the others.
    for mid in [0u8, 64, 128, 200, 255] {
        let mut w = Vec::new();
        for &t in &[10u8, 11] {
            w.push(Rgb255::new(t, mid, mid));
            w.push(Rgb255::new(mid, t, mid));
            w.push(Rgb255::new(mid, mid, t));
        }
        assert_same_all(&format!("ERRORS row 4: per-channel flip, bg={mid}"), w);
    }
}

// ---------------------------------------------------------------------------
// ERRORS.md row 5 — cbApplyGammaRGB threshold, incl. negative & zero inputs.
// ---------------------------------------------------------------------------
//
// Threshold `> 0.00313080495356037151702786377709`. Unlike row 4 this operates
// on post-matrix values, which can be NEGATIVE — and negatives take the linear
// arm (`* 12.92`), staying negative and feeding row 1's wrap. Also note the
// exponent literal is the truncated `0.4166666666`, not `1.0/2.4`.
#[test]
fn row05_apply_gamma_threshold_boundary() {
    // Inputs that make post-matrix channels hover around zero / go negative:
    // blue-dominant colours drive the red row negative, and near-black inputs
    // put all three channels just around the threshold.
    let mut v = Vec::new();
    for b in 0..=255u8 {
        v.push(Rgb255::new(0, 0, b)); // red row goes most negative
        v.push(Rgb255::new(1, 0, b));
        v.push(Rgb255::new(0, 1, b));
    }
    for r in 0..=20u8 {
        for g in 0..=20u8 {
            v.push(Rgb255::new(r, g, 0));
            v.push(Rgb255::new(r, g, 255));
        }
    }
    let n = v.len();
    assert_eq!(assert_same_all("ERRORS row 5: applyGamma threshold", v), n);

    // Confirm the negative/linear arm is genuinely exercised: some of these must
    // land in the negative cbDenorm bucket (non-trivial wrapped R).
    let neg_seen = (0..=255u8)
        .map(|b| Rgb255::new(0, 0, b))
        .filter(|&x| {
            let o = call_c(x);
            o.r > 128 // a wrapped negative shows up as a large byte
        })
        .count();
    assert!(
        neg_seen > 0,
        "row 5 did not reach the negative/linear arm of cbApplyGammaRGB"
    );
    eprintln!("ERRORS row 5: {neg_seen} blue-ramp inputs produced a wrapped R");
}

// ---------------------------------------------------------------------------
// ERRORS.md row 6 — NaN / unordered comparison.
// ---------------------------------------------------------------------------
//
// C uses `comisd` + `jbe`, which IS taken when unordered, so NaN selects the
// `else` (linear) arm. Rust's `if c > k { .. } else { .. }` also sends NaN to
// `else`. Unreachable through the public API (a `u8/255.f` is never NaN) and the
// helpers are `static`, so this row is verified by construction plus the
// exhaustive sweep. Documented here as an explicit, non-vacuous statement of
// what was and was not testable.
#[test]
fn row06_nan_unordered_arm_unreachable_but_aligned() {
    // Premise: every input channel normalises into [0,1] and is finite, so no
    // NaN can enter either gamma function. Verified indirectly: outputs are
    // fully determined and identical for the entire domain (phase_b S9). Here we
    // assert the domain really is closed under the API — there is no way to hand
    // `tritanopia` a non-finite value, because its only parameter is three `u8`.
    assert_eq!(std::mem::size_of::<Rgb255>(), 3);
    assert_eq!(std::mem::align_of::<Rgb255>(), 1);
    // And spot-check the domain endpoints where a normalised value is exactly
    // 0.0 or exactly 1.0 (the only two values where `pow` could see an edge).
    assert_same_all(
        "ERRORS row 6: normalised endpoints 0.0 / 1.0",
        [
            Rgb255::new(0, 0, 0),
            Rgb255::new(255, 255, 255),
            Rgb255::new(0, 255, 0),
            Rgb255::new(255, 0, 0),
            Rgb255::new(0, 0, 255),
        ],
    );
}

// ---------------------------------------------------------------------------
// ERRORS.md row 7 — minimum input.
// ---------------------------------------------------------------------------
#[test]
fn row07_extremes() {
    // Absolute minimum, and every "one channel at minimum" combination.
    let mut v = vec![Rgb255::new(0, 0, 0)];
    for &x in &[0u8, 1, 255] {
        v.push(Rgb255::new(0, 0, x));
        v.push(Rgb255::new(0, x, 0));
        v.push(Rgb255::new(x, 0, 0));
    }
    assert_same_all("ERRORS row 7: minimum / near-minimum", v);
}

// ---------------------------------------------------------------------------
// ERRORS.md row 8 — maximum input. `u8` IS the range check: there is no value
// one step past 255 that can be passed in a `unsigned char` field, so the
// "one past the documented range" probe degenerates to 255 itself, plus the
// register-level probe in row 9.
// ---------------------------------------------------------------------------
#[test]
fn row08_all_max() {
    let mut v = vec![Rgb255::new(255, 255, 255)];
    for &x in &[254u8, 255] {
        v.push(Rgb255::new(255, 255, x));
        v.push(Rgb255::new(255, x, 255));
        v.push(Rgb255::new(x, 255, 255));
    }
    assert_same_all("ERRORS row 8: maximum / near-maximum", v);
    // u8 cannot represent 256, so assert the type-level range claim instead.
    assert_eq!(u8::MAX, 255);
}

// ---------------------------------------------------------------------------
// ERRORS.md row 9 — out-of-range bit patterns across the FFI boundary.
// ---------------------------------------------------------------------------
//
// This is the analogue of "an enum value with no valid variant". The 3-byte
// struct is class INTEGER under the x86-64 SysV ABI, so it travels in a single
// general-purpose register (`RDI`); the C prologue does `mov %rdi,-0x38(%rbp)`
// and then three `movzbl` at offsets 0,1,2 — the upper 5 bytes and the 4th byte
// are never read. A caller may legally leave arbitrary garbage there.
//
// Called through a `u64` signature (same register class, so the bits land in
// RDI) to check the Rust export ignores those bits identically. Only the low 3
// bytes of the returned register are ABI-meaningful, so only those are compared.
#[test]
fn row09_garbage_high_register_bits() {
    let l = libs();
    let mut rng = Rng::new(0xDEAD_BEEF_CAFE_1234);

    let garbage_patterns: [u64; 8] = [
        0x0000_0000_0000_0000,
        0xFFFF_FFFF_FF00_0000,
        0xAAAA_AAAA_AA00_0000,
        0x5555_5555_5500_0000,
        0x8000_0000_0000_0000,
        0x0000_0000_FF00_0000,
        0x7FFF_FFFF_FF00_0000,
        0xDEAD_BEEF_0000_0000 & 0xFFFF_FFFF_FF00_0000,
    ];

    let mut checked = 0usize;
    for _ in 0..4000 {
        let x = rng.next_rgb();
        let low = (x.r as u64) | ((x.g as u64) << 8) | ((x.b as u64) << 16);
        for &garb in &garbage_patterns {
            let arg = low | garb;
            let cv = unsafe { (l.c_raw)(arg) } & 0x00FF_FFFF;
            let rv = unsafe { (l.r_raw)(arg) } & 0x00FF_FFFF;
            assert_eq!(
                cv, rv,
                "high-bit garbage changed behaviour: arg={arg:#018x} \
                 (rgb=({},{},{})) C={cv:#08x} Rust={rv:#08x}",
                x.r, x.g, x.b
            );

            // It must also agree with the clean struct call: garbage in the
            // unused bits must be ignored, not folded into the result.
            let clean = call_c(x);
            let clean_bits =
                (clean.r as u64) | ((clean.g as u64) << 8) | ((clean.b as u64) << 16);
            assert_eq!(
                cv, clean_bits,
                "C itself was perturbed by garbage bits at arg={arg:#018x}"
            );
            checked += 1;
        }
    }
    eprintln!("ERRORS row 9: {checked} garbage-register-bit calls agreed");
}

// ---------------------------------------------------------------------------
// ERRORS.md row 10 — struct-return: only the low 3 bytes are ABI-meaningful.
// ---------------------------------------------------------------------------
#[test]
fn row10_return_low_three_bytes() {
    let l = libs();
    let mut rng = Rng::new(0x0BAD_F00D_0000_0001);
    for _ in 0..20_000 {
        let x = rng.next_rgb();
        let arg = (x.r as u64) | ((x.g as u64) << 8) | ((x.b as u64) << 16);
        let cv = unsafe { (l.c_raw)(arg) };
        let rv = unsafe { (l.r_raw)(arg) };
        assert_eq!(
            cv & 0x00FF_FFFF,
            rv & 0x00FF_FFFF,
            "low 3 return bytes differ for ({},{},{})",
            x.r,
            x.g,
            x.b
        );
        // And the struct-typed call must see exactly those bytes.
        let s = call_r(x);
        let sbits = (s.r as u64) | ((s.g as u64) << 8) | ((s.b as u64) << 16);
        assert_eq!(sbits, rv & 0x00FF_FFFF, "struct vs raw view disagree");
    }
}

// ---------------------------------------------------------------------------
// ERRORS.md row 11 — never panics / never aborts.
// ---------------------------------------------------------------------------
//
// Relevant because `[profile.release] panic = "abort"` turns any stray Rust
// panic into a process abort, which would be an observable divergence from the C
// (which has no trap path at all: no division by a non-constant, no allocation,
// no I/O, no recursion). Reaching the end of a broad sweep in-process is the
// assertion — an abort would kill the test binary.
#[test]
fn row11_no_panic_any_input() {
    let mut rng = Rng::new(0xFEED_FACE_0000_0002);
    let mut acc = 0u64;
    for _ in 0..200_000 {
        let x = rng.next_rgb();
        let c = call_c(x);
        let r = call_r(x);
        assert_eq!(c, r);
        acc = acc.wrapping_add(r.r as u64 + r.g as u64 + r.b as u64);
    }
    // Also hammer the two UB regions specifically, since those are where a
    // panicking `as`-cast or an overflow check would fire in a debug build.
    for x in witnesses_with_r_bucket('0')
        .into_iter()
        .chain(witnesses_with_r_bucket('2'))
    {
        let c = call_c(x);
        let r = call_r(x);
        assert_eq!(c, r);
        acc = acc.wrapping_add(r.r as u64);
    }
    eprintln!("ERRORS row 11: survived without panic/abort (checksum {acc})");
}
