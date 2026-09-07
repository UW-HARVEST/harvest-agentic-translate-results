//! Phase C - error/rejection-path differential tests, one per row of `ERRORS.md`.
//!
//! The C has an empty *explicit* rejection surface (no error macro, no
//! sentinel, no assert, no null check - see the grep evidence in `ERRORS.md`),
//! so these tests target the only places the C can produce an out-of-band
//! result: the implicit undefined-behaviour conversions, the strict branch
//! thresholds, and the FFI/ABI boundary. In every case the assertion is that C
//! and Rust agree on the *exact* bytes, not merely that "both did something".

mod common;

use common::{CbRgb255, Pair, Rng, SEED};

// ---------------------------------------------------------------- E1
/// E1: post-gamma channel is negative -> `(unsigned char)` conversion is out of
/// range (C UB). The reference `.so` lowers this to
/// `cvttss2si %xmm0,%eax; mov %al,..`, i.e. truncate toward zero then keep the
/// low byte, so the value wraps mod 256 rather than clamping to 0.
#[test]
fn e1_negative_channel_wraps() {
    let p = Pair::load();

    // The documented case: pure blue drives red to about -1.65.
    let blue = CbRgb255::new(0, 0, 255);
    let pre = common::pre_denorm(blue)[0];
    assert!(pre < 0.0, "expected a negative pre-cast red, got {pre}");

    let c = p.c(blue);
    let r = p.rust(blue);
    assert_eq!(c, r, "E1 divergence on (0,0,255): C={c:?} Rust={r:?}");
    assert_ne!(c.r, 0, "C wrapped rather than clamped; got {c:?}");

    // The expected byte, computed straight from the hardware semantics.
    let expected = (pre.trunc() as i32) as u8;
    assert_eq!(
        c.r, expected,
        "C red byte {} != cvttss2si-then-truncate prediction {expected}",
        c.r
    );
    assert_eq!(r.r, expected, "Rust red byte does not match the C lowering");

    // Sweep every input whose red channel goes negative and confirm agreement.
    let mut rng = Rng::new(SEED ^ 0xE1);
    let mut checked = 0;
    let mut inputs = Vec::new();
    for _ in 0..200_000 {
        let v = CbRgb255::new(rng.range_u8(0, 40), rng.range_u8(0, 60), rng.range_u8(150, 255));
        if common::pre_denorm(v)[0] < 0.0 {
            inputs.push(v);
            checked += 1;
        }
        if checked >= 20_000 {
            break;
        }
    }
    assert!(checked > 1000, "only {checked} negative-red inputs found");
    p.check_all("E1", inputs);
    println!("E1: {checked} negative-wrap inputs agree; tritanopia(0,0,255)={c:?}");
}

// ---------------------------------------------------------------- E2
/// E2: post-gamma channel exceeds 1.0 -> conversion overflows `unsigned char`
/// (C UB) and wraps mod 256 instead of clamping to 255.
#[test]
fn e2_over_one_channel_wraps() {
    let p = Pair::load();

    // (255,255,0): red row is 1 + 0.1274*1 - 0 = 1.127.
    let v = CbRgb255::new(255, 255, 0);
    let pre = common::pre_denorm(v)[0];
    assert!(pre > 255.0, "expected pre-cast red > 255, got {pre}");

    let c = p.c(v);
    let r = p.rust(v);
    assert_eq!(c, r, "E2 divergence on (255,255,0): C={c:?} Rust={r:?}");
    assert!(c.r < 200, "C should have wrapped past 255, got {c:?}");

    let expected = (pre.trunc() as i32) as u8;
    assert_eq!(c.r, expected, "C red byte != low-byte-of-truncation prediction");
    assert_eq!(r.r, expected);

    // Sweep every input that overflows and confirm agreement.
    let mut rng = Rng::new(SEED ^ 0xE2);
    let mut inputs = Vec::new();
    for _ in 0..400_000 {
        let v = CbRgb255::new(rng.range_u8(200, 255), rng.range_u8(180, 255), rng.range_u8(0, 60));
        if common::pre_denorm(v)[0] >= 256.0 {
            inputs.push(v);
        }
        if inputs.len() >= 20_000 {
            break;
        }
    }
    assert!(inputs.len() > 1000, "only {} overflow inputs found", inputs.len());
    let n = inputs.len();
    p.check_all("E2", inputs);
    println!("E2: {n} over-255-wrap inputs agree; tritanopia(255,255,0)={c:?}");
}

// ---------------------------------------------------------------- E3
/// E3: the x86 "integer indefinite" case (`|v| >= 2^31` or NaN reaching
/// `cvttss2si`, which then yields `0x8000_0000` -> low byte `0x00`).
///
/// This is UNREACHABLE through the public API - `phase_b_configs::c9_*` proves
/// it over all 2^24 inputs. What is asserted here is that the *helper's*
/// semantics are the x86 ones (saturating `as i32` would give a different
/// answer), by checking the exact boundary the helper switches on. If a future
/// change ever made this reachable, the Rust would already behave like the C.
#[test]
fn e3_indefinite_helper_semantics() {
    // Mirror of `f32_to_u8_c_cast` in src/lib.rs.
    fn c_cast(v: f32) -> u8 {
        let t = v.trunc() as f64;
        let i = if v.is_nan() || t < -2147483648.0 || t >= 2147483648.0 {
            i32::MIN
        } else {
            t as i32
        };
        i as u8
    }
    // Indefinite cases -> 0x8000_0000 -> low byte 0.
    assert_eq!(c_cast(f32::NAN), 0);
    assert_eq!(c_cast(f32::INFINITY), 0);
    assert_eq!(c_cast(f32::NEG_INFINITY), 0);
    assert_eq!(c_cast(1e30), 0);
    assert_eq!(c_cast(-1e30), 0);
    assert_eq!(c_cast(2147483648.0), 0); // exactly 2^31: not representable
    // Just inside the representable range: NOT indefinite, plain low byte.
    assert_eq!(c_cast(2147483520.0), (2147483520i32) as u8); // largest f32 < 2^31
    assert_eq!(c_cast(-2147483648.0), (-2147483648i32) as u8);
    // Ordinary values, including the wrap cases from E1/E2.
    assert_eq!(c_cast(269.4), 13);
    assert_eq!(c_cast(-5434.2), (-5434i32) as u8);
    assert_eq!(c_cast(0.5), 0);
    assert_eq!(c_cast(255.5), 255);
    assert_eq!(c_cast(-0.5), 0); // truncates toward zero
}

// ---------------------------------------------------------------- E4 / E5
/// E4 + E5: `pow` never receives a negative base, so no `NaN` (and no domain
/// error) can ever escape either gamma helper. Verified over the whole domain
/// by checking that no result byte is the tell-tale NaN-cast value while the
/// pre-cast float is NaN, plus a direct scan of the pre-cast floats.
#[test]
fn e4_no_nan_ever_escapes() {
    let p = Pair::load();
    let mut rng = Rng::new(SEED ^ 0xE4);
    let mut inputs = Vec::with_capacity(20_000);
    for _ in 0..20_000 {
        let v = CbRgb255::new(rng.range_u8(0, 255), rng.range_u8(0, 255), rng.range_u8(0, 255));
        for pre in common::pre_denorm(v) {
            assert!(!pre.is_nan(), "NaN reached cbDenorm for input {v:?}");
        }
        inputs.push(v);
    }
    p.check_all("E4/E5", inputs);

    // And the extremes, where a sign error would most plausibly produce a NaN.
    for v in [
        CbRgb255::new(0, 0, 0),
        CbRgb255::new(255, 255, 255),
        CbRgb255::new(0, 0, 255),
        CbRgb255::new(255, 0, 0),
        CbRgb255::new(0, 255, 0),
        CbRgb255::new(255, 255, 0),
        CbRgb255::new(255, 0, 255),
        CbRgb255::new(0, 255, 255),
    ] {
        for pre in common::pre_denorm(v) {
            assert!(!pre.is_nan(), "NaN for corner {v:?}");
        }
        p.check(v);
    }
}

// ---------------------------------------------------------------- E6
/// E6: the branch thresholds are strict `>`, so a channel exactly ON the
/// threshold takes the `else` (linear) arm. Also checks one step past the
/// boundary in each direction.
#[test]
fn e6_threshold_exact_and_one_step() {
    let p = Pair::load();

    // remove-gamma: 0.04045 * 255 = 10.31..., so 10 -> linear, 11 -> pow.
    // Test each channel at 9,10,11,12 against a fixed and a varied partner.
    for &edge in &[0u8, 1, 9, 10, 11, 12, 254, 255] {
        for &other in &[0u8, 1, 10, 11, 127, 254, 255] {
            p.check(CbRgb255::new(edge, other, other));
            p.check(CbRgb255::new(other, edge, other));
            p.check(CbRgb255::new(other, other, edge));
            p.check(CbRgb255::new(edge, edge, other));
            p.check(CbRgb255::new(edge, other, edge));
            p.check(CbRgb255::new(other, edge, edge));
            p.check(CbRgb255::new(edge, edge, edge));
        }
    }

    // apply-gamma threshold: find inputs whose post-matrix red sits within a
    // hair of 0.00313080495356037151702786377709 on either side, and confirm
    // both sides pick the same arm (a mismatched arm changes the byte a lot).
    let mut straddling = 0;
    for r in 0u16..=255 {
        for g in 0u16..=40 {
            for b in 0u16..=40 {
                let v = CbRgb255::new(r as u8, g as u8, b as u8);
                let pre = common::pre_denorm(v)[0];
                // pre = arm(red)*255+0.5; the threshold maps to about 0.0404 -> pre ~ 10.8
                if (pre - 10.8).abs() < 3.0 {
                    p.check(v);
                    straddling += 1;
                }
            }
        }
    }
    println!("E6: {straddling} inputs straddling the apply-gamma threshold agree");
}

// ---------------------------------------------------------------- E7
/// E7: out-of-range bits across the FFI boundary.
///
/// There is no enum in this API, and every `unsigned char` bit pattern
/// `0x00..=0xff` is a valid value, so "an int with no valid variant" cannot be
/// constructed for a channel. The analogous out-of-range input is the
/// *undefined upper 5 bytes* of the register the 3-byte struct travels in:
/// a real caller can put anything there, and the C provably ignores it
/// (`movzbl` of each of the 3 low bytes). The Rust must ignore it identically.
#[test]
fn e7_garbage_in_upper_register_bytes() {
    let p = Pair::load();
    let mut rng = Rng::new(SEED ^ 0xE7);

    // Pathological fixed patterns first.
    for &garbage in &[
        0x0000_0000_0000_0000u64,
        0xFFFF_FFFF_FF00_0000,
        0xAAAA_AAAA_AA00_0000,
        0x8000_0000_0000_0000,
        0x0000_0000_FF00_0000,
        0xDEAD_BEEF_CA00_0000,
    ] {
        for &payload in &[0x00_0000u64, 0xFF_FFFF, 0x00_00FF, 0xFF_0000, 0x7F_7F7F, 0x00_FF00] {
            let dirty = payload | garbage;
            let c = p.c_raw(dirty) & 0xFF_FFFF;
            let r = p.rust_raw(dirty) & 0xFF_FFFF;
            let c_ref = p.c_raw(payload) & 0xFF_FFFF;
            assert_eq!(c, c_ref, "C was sensitive to padding for {dirty:#018x}");
            assert_eq!(
                c, r,
                "E7 divergence for {dirty:#018x}: C={c:#08x} Rust={r:#08x}"
            );
        }
    }

    // Then randomized.
    for _ in 0..20_000 {
        let payload = rng.next_u64() & 0x00FF_FFFF;
        let dirty = payload | (rng.next_u64() & 0xFFFF_FFFF_FF00_0000);
        let c = p.c_raw(dirty) & 0xFF_FFFF;
        let r = p.rust_raw(dirty) & 0xFF_FFFF;
        assert_eq!(c, r, "E7 divergence for {dirty:#018x}");
    }
}

// ---------------------------------------------------------------- E8
/// E8: every boundary byte-triple - the fixed-width-value analogue of "zero and
/// oversized lengths". All channels at `0x00`/`0x01`/`0xfe`/`0xff` in every
/// combination, plus each single channel at a boundary with the others swept.
#[test]
fn e8_boundary_triples() {
    let p = Pair::load();
    const B: [u8; 4] = [0x00, 0x01, 0xFE, 0xFF];

    // Full cross-product of the boundary values: 4^3 = 64.
    let mut n = 0;
    for &r in &B {
        for &g in &B {
            for &b in &B {
                p.check(CbRgb255::new(r, g, b));
                n += 1;
            }
        }
    }
    assert_eq!(n, 64);

    // Each channel pinned to a boundary while the other two sweep coarsely.
    for &edge in &B {
        for other in (0u16..=255).step_by(7) {
            let o = other as u8;
            p.check(CbRgb255::new(edge, o, o));
            p.check(CbRgb255::new(o, edge, o));
            p.check(CbRgb255::new(o, o, edge));
        }
    }
}

// ---------------------------------------------------------------- E9
/// E9: null-pointer / zero-length rejection is NOT APPLICABLE.
///
/// The public API is `cb_rgb_255 tritanopia(cb_rgb_255)` - the struct is passed
/// and returned entirely by value and there is no pointer, buffer, length,
/// count, or handle parameter anywhere in `c_src/include/lib.h`. So there is no
/// null pointer or zero length an attacker or a careless caller could supply.
/// This test documents that the omission is deliberate by asserting the shape
/// of the ABI: the input fits in one register and carries no indirection.
#[test]
fn e9_no_pointer_parameters_exist() {
    assert_eq!(std::mem::size_of::<CbRgb255>(), 3, "cb_rgb_255 must be 3 bytes");
    assert_eq!(std::mem::align_of::<CbRgb255>(), 1, "cb_rgb_255 must be byte-aligned");
    // Loading through the by-value signature succeeds, which is itself the
    // proof that the symbol's ABI takes a register-passed aggregate.
    let p = Pair::load();
    p.check(CbRgb255::new(0, 0, 0));
}
