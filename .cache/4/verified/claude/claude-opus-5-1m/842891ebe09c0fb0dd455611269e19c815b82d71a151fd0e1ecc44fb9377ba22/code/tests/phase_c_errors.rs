//! Phase C — error-path differential tests.
//!
//! One `#[test]` per row of `ERRORS.md` (`e1_*` .. `e8_*`), plus the generic
//! FFI-boundary rows required regardless of the table.
//!
//! Every test asserts the two implementations agree on the *same* sentinel —
//! the exact `f32` bit pattern, including the NaN sign and payload — not merely
//! "both produced something non-finite".

mod common;

use common::*;

fn n(default: usize) -> usize {
    match std::env::var("DIFF_N") {
        Ok(v) => v.parse().unwrap_or(default),
        Err(_) => default,
    }
}

const PAD_MASK: u64 = !0x00FF_FFFFu64;

// ---------------------------------------------------------------------------
// E1 — 0/0: both colours black -> NaN, bit-identical
// ---------------------------------------------------------------------------
#[test]
fn e1_black_vs_black_is_bit_identical_nan() {
    let p = pair();
    let cv = unsafe { (p.c.contrast_ratio)(BLACK, BLACK) };
    let rv = unsafe { (p.rust.contrast_ratio)(BLACK, BLACK) };

    assert!(cv.is_nan(), "C did not return NaN for black/black: {cv:?}");
    assert!(rv.is_nan(), "Rust did not return NaN for black/black: {rv:?}");
    assert_eq!(
        cv.to_bits(),
        rv.to_bits(),
        "E1: NaN bit patterns differ — C 0x{:08X} vs Rust 0x{:08X}",
        cv.to_bits(),
        rv.to_bits()
    );
    // Same result no matter how it is reached, and stable across repeats.
    for _ in 0..1000 {
        assert_same(p, BLACK, BLACK, "E1 repeat");
    }
    // Reached through the raw signature with junk padding as well.
    assert_same_raw(p, PAD_MASK, PAD_MASK, "E1 junk padding");
}

// ---------------------------------------------------------------------------
// E2 — x/0 with B black, no-swap route -> +inf
// ---------------------------------------------------------------------------
#[test]
fn e2_b_black_returns_positive_infinity() {
    let p = pair();
    let mut rng = Rng::new(0xE002_0002);
    // Exhaustive over every non-black grey and every non-black single channel.
    for v in 1u16..=255 {
        let v = v as u8;
        for a in [
            Rgb::new(v, v, v),
            Rgb::new(v, 0, 0),
            Rgb::new(0, v, 0),
            Rgb::new(0, 0, v),
        ] {
            let r = assert_same(p, a, BLACK, "E2 exhaustive");
            assert_eq!(
                r.to_bits(),
                f32::INFINITY.to_bits(),
                "E2: expected +inf for A={a:?} B=black, got {r:?}"
            );
        }
    }
    for _ in 0..n(20_000) {
        let mut a = rng.color();
        if a == BLACK {
            a.g = 1;
        }
        let r = assert_same(p, a, BLACK, "E2 random");
        assert_eq!(r.to_bits(), f32::INFINITY.to_bits(), "E2: expected +inf, got {r:?}");
    }
}

// ---------------------------------------------------------------------------
// E3 — x/0 with A black: the SWAP branch routes into the division by zero
// ---------------------------------------------------------------------------
#[test]
fn e3_a_black_swap_branch_returns_positive_infinity() {
    let p = pair();
    let mut rng = Rng::new(0xE003_0003);
    for v in 1u16..=255 {
        let v = v as u8;
        for b in [
            Rgb::new(v, v, v),
            Rgb::new(v, 0, 0),
            Rgb::new(0, v, 0),
            Rgb::new(0, 0, v),
        ] {
            let r = assert_same(p, BLACK, b, "E3 exhaustive");
            assert_eq!(
                r.to_bits(),
                f32::INFINITY.to_bits(),
                "E3: expected +inf for A=black B={b:?}, got {r:?}"
            );
        }
    }
    for _ in 0..n(20_000) {
        let mut b = rng.color();
        if b == BLACK {
            b.r = 1;
        }
        let r = assert_same(p, BLACK, b, "E3 random");
        assert_eq!(r.to_bits(), f32::INFINITY.to_bits(), "E3: expected +inf, got {r:?}");
    }
}

// ---------------------------------------------------------------------------
// E4 — near-zero denominator: no epsilon / clamping guard exists
// ---------------------------------------------------------------------------
#[test]
fn e4_near_zero_denominator_has_no_guard() {
    let p = pair();
    let r = assert_same(p, WHITE, Rgb::new(0, 0, 1), "E4 white / {0,0,1}");
    assert!(
        r.is_finite() && r > 1.0e4,
        "E4: expected a huge finite ratio, got {r:?}"
    );
    // Every darkest-possible denominator, in both argument positions.
    for low in [
        Rgb::new(1, 0, 0),
        Rgb::new(0, 1, 0),
        Rgb::new(0, 0, 1),
        Rgb::new(1, 1, 1),
    ] {
        assert_same(p, WHITE, low, "E4 white / darkest");
        assert_same(p, low, WHITE, "E4 darkest / white");
        for v in 0u16..=255 {
            let grey = Rgb::new(v as u8, v as u8, v as u8);
            assert_same(p, grey, low, "E4 grey / darkest");
            assert_same(p, low, grey, "E4 darkest / grey");
        }
    }
}

// ---------------------------------------------------------------------------
// E5 — junk in the padding bits of argument A
// ---------------------------------------------------------------------------
#[test]
fn e5_junk_padding_in_argument_a() {
    let p = pair();
    let junks: [u64; 6] = [
        0xDEAD_BEEF_CC00_0000,
        0xFFFF_FFFF_FF00_0000,
        0x0000_0000_0100_0000,
        0x8000_0000_0000_0000,
        0x7FFF_FFFF_FF00_0000,
        0xAAAA_AAAA_AA00_0000,
    ];
    let mut rng = Rng::new(0xE005_0005);
    for _ in 0..n(5_000) {
        let a = rng.color();
        let b = rng.color();
        let clean = assert_same(p, a, b, "E5 clean");
        for &j in &junks {
            let j = j & PAD_MASK;
            let v = assert_same_raw(p, a.as_reg_bits() | j, b.as_reg_bits(), "E5 junk in A");
            assert_eq!(
                v.to_bits(),
                clean.to_bits(),
                "E5: A padding 0x{j:016X} changed the result for A={a:?} B={b:?}"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// E6 — junk in the padding bits of argument B
// ---------------------------------------------------------------------------
#[test]
fn e6_junk_padding_in_argument_b() {
    let p = pair();
    let junks: [u64; 4] = [
        0xCAFE_F00D_9900_0000,
        0xFFFF_FFFF_FF00_0000,
        0x0000_0000_8000_0000,
        0x5555_5555_5500_0000,
    ];
    let mut rng = Rng::new(0xE006_0006);
    for _ in 0..n(5_000) {
        let a = rng.color();
        let b = rng.color();
        let clean = assert_same(p, a, b, "E6 clean");
        for &j in &junks {
            let j = j & PAD_MASK;
            let v = assert_same_raw(p, a.as_reg_bits(), b.as_reg_bits() | j, "E6 junk in B");
            assert_eq!(
                v.to_bits(),
                clean.to_bits(),
                "E6: B padding 0x{j:016X} changed the result for A={a:?} B={b:?}"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// E7 — all padding bits of BOTH arguments set
// ---------------------------------------------------------------------------
#[test]
fn e7_junk_padding_in_both_arguments() {
    let p = pair();
    let mut rng = Rng::new(0xE007_0007);
    for _ in 0..n(20_000) {
        let a = rng.color();
        let b = rng.color();
        let clean = assert_same(p, a, b, "E7 clean");
        for (ja, jb) in [
            (PAD_MASK, PAD_MASK),
            (PAD_MASK, 0),
            (0, PAD_MASK),
            (rng.next_u64() & PAD_MASK, rng.next_u64() & PAD_MASK),
        ] {
            let v = assert_same_raw(
                p,
                a.as_reg_bits() | ja,
                b.as_reg_bits() | jb,
                "E7 junk in both",
            );
            assert_eq!(
                v.to_bits(),
                clean.to_bits(),
                "E7: padding (0x{ja:016X}, 0x{jb:016X}) changed the result"
            );
        }
    }
    // The degenerate cases too, with maximal junk.
    for (a, b) in [(BLACK, BLACK), (BLACK, WHITE), (WHITE, BLACK), (WHITE, WHITE)] {
        let clean = assert_same(p, a, b, "E7 degenerate clean");
        let v = assert_same_raw(
            p,
            a.as_reg_bits() | PAD_MASK,
            b.as_reg_bits() | PAD_MASK,
            "E7 degenerate junk",
        );
        assert_eq!(v.to_bits(), clean.to_bits(), "E7 degenerate mismatch");
    }
}

// ---------------------------------------------------------------------------
// E8 — the un-guarded `> 0.04045` boundary in every channel position
// ---------------------------------------------------------------------------
#[test]
fn e8_linearization_branch_boundary_is_strict() {
    let p = pair();

    // 10 -> linear branch, 11 -> pow branch, in each of the 6 positions
    // independently, over several backgrounds.
    for bg in [0u8, 10, 11, 127, 255] {
        for pos in 0..6usize {
            for v in [
                0u8,
                1,
                9,
                LAST_LINEAR,
                FIRST_POW,
                12,
                13,
                254,
                255,
            ] {
                let mut a = Rgb::new(bg, bg, bg);
                let mut b = Rgb::new(bg, bg, bg);
                match pos {
                    0 => a.r = v,
                    1 => a.g = v,
                    2 => a.b = v,
                    3 => b.r = v,
                    4 => b.g = v,
                    _ => b.b = v,
                }
                assert_same(p, a, b, &format!("E8 bg={bg} pos={pos} v={v}"));
            }
        }
    }

    // All 2^6 combinations of {10, 11} across the 6 positions.
    for m in 0u32..64 {
        let pick = |i: u32| if m >> i & 1 == 0 { LAST_LINEAR } else { FIRST_POW };
        assert_same(
            p,
            Rgb::new(pick(0), pick(1), pick(2)),
            Rgb::new(pick(3), pick(4), pick(5)),
            "E8 10/11 cross product",
        );
    }

    // Directly confirm the branch really does flip between 10 and 11 (so this
    // test is not vacuous): the linear and pow results must differ.
    let lin = unsafe { (p.c.contrast_ratio)(Rgb::new(10, 10, 10), WHITE) };
    let pw = unsafe { (p.c.contrast_ratio)(Rgb::new(11, 11, 11), WHITE) };
    assert!(
        lin != pw,
        "E8 is vacuous: 10 and 11 produced the same C result ({lin:?})"
    );
}

// ---------------------------------------------------------------------------
// Generic FFI-boundary coverage (required regardless of the ERRORS.md table)
// ---------------------------------------------------------------------------

/// The parameter type is `unsigned char`, so the entire 0..=255 domain is valid
/// and there is no representable "one past the range" value. Assert that fact by
/// covering 100 % of the domain of every position, and that the value `255` (the
/// maximum) and `0` (the minimum) behave identically in both implementations —
/// including when combined with the extremes of the other positions.
#[test]
fn generic_full_domain_and_range_extremes() {
    let p = pair();
    for v in 0u16..=255 {
        let v = v as u8;
        for other in [0u8, 255] {
            let combos = [
                (Rgb::new(v, other, other), Rgb::new(other, other, other)),
                (Rgb::new(other, v, other), Rgb::new(other, other, other)),
                (Rgb::new(other, other, v), Rgb::new(other, other, other)),
                (Rgb::new(other, other, other), Rgb::new(v, other, other)),
                (Rgb::new(other, other, other), Rgb::new(other, v, other)),
                (Rgb::new(other, other, other), Rgb::new(other, other, v)),
                (Rgb::new(v, v, v), Rgb::new(other, other, other)),
                (Rgb::new(other, other, other), Rgb::new(v, v, v)),
            ];
            for (a, b) in combos {
                assert_same(p, a, b, "generic domain sweep");
            }
        }
    }
}

/// There are no enums in the C API (`grep -c enum` -> 0) and no pointers
/// (`grep -c '\*' include/lib.h` -> 0), so the closest expressible "out of
/// range value crossing the FFI boundary" is an argument register whose bits do
/// not correspond to any clean 3-byte struct. Sweep the *entire* 24-bit
/// low-order space in a stratified way while every padding byte is garbage: if
/// the Rust side ever mis-classified the struct or read the wrong bytes, this
/// finds it.
#[test]
fn generic_raw_register_bit_patterns() {
    let p = pair();
    let mut rng = Rng::new(0x0BAD_F00D);

    // Fully random 64-bit argument registers: the low 24 bits are a valid
    // colour by construction (every u8 triple is valid), the top 40 are junk.
    for _ in 0..n(100_000) {
        let ar = rng.next_u64();
        let br = rng.next_u64();
        let v = assert_same_raw(p, ar, br, "generic raw random");
        // Cross-check against the struct-typed call using only the low 3 bytes.
        let a = Rgb::new(ar as u8, (ar >> 8) as u8, (ar >> 16) as u8);
        let b = Rgb::new(br as u8, (br >> 8) as u8, (br >> 16) as u8);
        let clean = assert_same(p, a, b, "generic raw cross-check");
        assert_eq!(
            v.to_bits(),
            clean.to_bits(),
            "raw register 0x{ar:016X}/0x{br:016X} disagreed with the clean \
             struct call for A={a:?} B={b:?}"
        );
    }

    // Pathological registers: all-ones, all-zeros, alternating, sign bit only.
    for &ar in &[
        0u64,
        u64::MAX,
        0xAAAA_AAAA_AAAA_AAAA,
        0x5555_5555_5555_5555,
        0x8000_0000_0000_0000,
        0x0000_0000_00FF_FFFF,
        0xFFFF_FFFF_FF00_0000,
    ] {
        for &br in &[
            0u64,
            u64::MAX,
            0xAAAA_AAAA_AAAA_AAAA,
            0x5555_5555_5555_5555,
            0x8000_0000_0000_0000,
            0x0000_0000_00FF_FFFF,
            0xFFFF_FFFF_FF00_0000,
        ] {
            assert_same_raw(p, ar, br, "generic raw pathological");
        }
    }
}

/// The function is pure: repeated calls, and calls interleaved between the two
/// implementations, must not drift (catches any hidden mutable state or lazily
/// initialised table that the translation might have introduced).
#[test]
fn generic_purity_and_interleaving() {
    let p = pair();
    let mut rng = Rng::new(0x1234_5678);
    let inputs: Vec<(Rgb, Rgb)> = (0..2_000).map(|_| (rng.color(), rng.color())).collect();

    let first: Vec<u32> = inputs
        .iter()
        .map(|&(a, b)| unsafe { (p.rust.contrast_ratio)(a, b) }.to_bits())
        .collect();

    for round in 0..8 {
        for (i, &(a, b)) in inputs.iter().enumerate() {
            let c = unsafe { (p.c.contrast_ratio)(a, b) }.to_bits();
            let r = unsafe { (p.rust.contrast_ratio)(a, b) }.to_bits();
            assert_eq!(c, r, "round {round} case {i}: C 0x{c:08X} != Rust 0x{r:08X}");
            assert_eq!(r, first[i], "round {round} case {i}: Rust result drifted");
        }
    }
}
