//! Phase B — valid-path differential tests, one test per `CONFIGS.md` row.
//!
//! Every test loads both `.so`s via `libloading` and compares `hdr_compare`
//! return values byte-for-byte. Randomized rows use the fixed seed
//! `Rng::SEED` so failures reproduce exactly.

mod common;

use common::*;

/// Number of randomized vectors per shape-driven row.
const N: usize = 50_000;

// ---------------------------------------------------------------------------
// Rows 1-9: valid `h2` across the CLASS x L x N x S shape cross-product,
// with an `h1` that agrees on every compared bit.
// ---------------------------------------------------------------------------

/// Build a valid `h2` with the requested shape.
/// `class_e` forces `h2[1] ∈ {0xE2, 0xE3}` (which pins `L == 1`).
fn make_h2(want_e: bool, l: u8, n: u8, s: u8, rng: &mut Rng) -> [u8; 3] {
    let b1 = if want_e {
        assert_eq!(l, 1, "E-class can only ever have L == 1");
        if rng.below(2) == 0 { 0xE2 } else { 0xE3 }
    } else {
        // F-class: 1111 xxxx.  L = bits 1..2 of the whole byte.
        let cands: Vec<u8> = (0xF0u16..=0xFF)
            .map(|v| v as u8)
            .filter(|&b| layer(b) == l)
            .collect();
        assert!(!cands.is_empty(), "no F-class byte with L={l}");
        rng.pick(&cands)
    };
    assert!(n != 15 && s != 3);
    let b2 = (n << 4) | (s << 2) | (rng.u8() & 0x03);
    let h2 = [0xff, b1, b2];
    assert!(h2_is_valid(&h2), "constructed h2 {h2:02x?} is not valid");
    h2
}

fn run_shape(p: &Pair, row: &str, want_e: bool, l: u8, n_choices: &[u8], s: u8) {
    let mut rng = Rng::seeded();
    let mut ones = 0usize;
    for _ in 0..N {
        let n = rng.pick(n_choices);
        let h2 = make_h2(want_e, l, n, s, &mut rng);
        let h1 = matching_h1(&h2, &mut rng);
        ones += p.assert_same(&h1, &h2, row) as usize;
    }
    // A matching h1 against a valid h2 must always accept; if this ever fails
    // the row was constructed wrong (not a Rust bug), so it is worth asserting.
    assert_eq!(ones, N, "[{row}] expected every matching pair to return 1");
}

#[test]
fn row01_fclass_l1_n0_s0() {
    run_shape(&Pair::load(), "row01", false, 1, &[0], 0);
}

#[test]
fn row02_fclass_l2_n0_s0() {
    run_shape(&Pair::load(), "row02", false, 2, &[0], 0);
}

#[test]
fn row03_fclass_l3_n0_s0() {
    run_shape(&Pair::load(), "row03", false, 3, &[0], 0);
}

#[test]
fn row04_row05_eclass_n0_s0() {
    let p = Pair::load();
    // Rows 4 and 5: make_h2 picks 0xE2/0xE3 at random, so both are covered;
    // pin each explicitly as well.
    run_shape(&p, "row04_row05", true, 1, &[0], 0);
    let mut rng = Rng::seeded();
    for fixed_b1 in [0xE2u8, 0xE3u8] {
        for _ in 0..N / 2 {
            let b2 = (rng.u8() & 0x03) | 0x00; // N=0, S=0
            let h2 = [0xff, fixed_b1, b2];
            assert!(h2_is_valid(&h2));
            let h1 = matching_h1(&h2, &mut rng);
            assert_eq!(
                p.assert_same(&h1, &h2, "row04_row05_fixed"),
                1,
                "b1={fixed_b1:#04x}"
            );
        }
    }
}

#[test]
fn row06_fclass_nonzero_nibble() {
    let p = Pair::load();
    let ns: Vec<u8> = (1..=14).collect();
    for l in [1u8, 2, 3] {
        run_shape(&p, "row06", false, l, &ns, 0);
    }
}

#[test]
fn row07_boundary_n14_s2_l3() {
    run_shape(&Pair::load(), "row07", false, 3, &[14], 2);
}

#[test]
fn row08_boundary_n1_s1_l1() {
    run_shape(&Pair::load(), "row08", false, 1, &[1], 1);
}

#[test]
fn row09_full_cross_product() {
    let p = Pair::load();
    let mut rng = Rng::seeded();
    let mut combos = 0usize;
    for want_e in [false, true] {
        let ls: &[u8] = if want_e { &[1] } else { &[1, 2, 3] };
        for &l in ls {
            for n in [0u8, 1, 7, 14] {
                for s in [0u8, 1, 2] {
                    combos += 1;
                    for _ in 0..2_000 {
                        let h2 = make_h2(want_e, l, n, s, &mut rng);
                        let h1 = matching_h1(&h2, &mut rng);
                        assert_eq!(
                            p.assert_same(&h1, &h2, "row09"),
                            1,
                            "combo e={want_e} l={l} n={n} s={s}"
                        );
                    }
                }
            }
        }
    }
    assert_eq!(combos, (3 + 1) * 4 * 3, "cross-product size changed");
}

// ---------------------------------------------------------------------------
// Rows 10-13: bits the C masks away must not change the result.
// ---------------------------------------------------------------------------

#[test]
fn row10_ignored_bit0_of_byte1() {
    let p = Pair::load();
    let mut rng = Rng::seeded();
    for _ in 0..N {
        let b1 = rng.pick(&valid_b1_values());
        let b2 = rng.pick(&valid_b2_values());
        let h2 = [0xff, b1, b2];
        let mut h1 = matching_h1(&h2, &mut rng);
        // differ ONLY in bit 0x01 of byte 1
        h1[1] = h2[1] ^ 0x01;
        assert_eq!(
            p.assert_same(&h1, &h2, "row10"),
            1,
            "bit0 of byte1 must be ignored: h1={h1:02x?} h2={h2:02x?}"
        );
    }
}

#[test]
fn row11_ignored_bits_0x03_of_byte2() {
    let p = Pair::load();
    let mut rng = Rng::seeded();
    for _ in 0..N {
        let b1 = rng.pick(&valid_b1_values());
        let b2 = rng.pick(&valid_b2_values());
        let h2 = [0xff, b1, b2];
        let mut h1 = matching_h1(&h2, &mut rng);
        // keep 0xF0-zeroness and 0x0C bits, flip a non-empty subset of 0x03
        let flip = 1 + (rng.below(3) as u8); // 1, 2 or 3
        h1[2] = (h1[2] & 0xFC) | ((h2[2] & 0x03) ^ flip);
        assert_eq!(
            p.assert_same(&h1, &h2, "row11"),
            1,
            "bits 0x03 of byte2 must be ignored: h1={h1:02x?} h2={h2:02x?}"
        );
    }
}

#[test]
fn row12_h1_is_never_validity_checked() {
    let p = Pair::load();
    let mut rng = Rng::seeded();
    let mut checked = 0usize;
    for _ in 0..N {
        let b1 = rng.pick(&valid_b1_values());
        // h2[2] with a NON-ZERO high nibble (1..=14) so h1 may use 0xF
        let n = 1 + (rng.below(14) as u8);
        let s = rng.below(3) as u8;
        let h2 = [0xff, b1, (n << 4) | (s << 2) | (rng.u8() & 0x03)];
        assert!(h2_is_valid(&h2));
        // h1[2] high nibble = 15, a value hdr_valid would REJECT for h2
        let h1 = [
            rng.u8(),
            h2[1] ^ (rng.u8() & 0x01),
            0xF0 | (h2[2] & 0x0C) | (rng.u8() & 0x03),
        ];
        assert_eq!(nibble(h1[2]), 15);
        assert_eq!(
            p.assert_same(&h1, &h2, "row12"),
            1,
            "h1 must not be validity-checked: h1={h1:02x?} h2={h2:02x?}"
        );
        checked += 1;
    }
    assert_eq!(checked, N);
}

#[test]
fn row13_h1_byte0_never_read() {
    let p = Pair::load();
    let mut rng = Rng::seeded();
    for _ in 0..2_000 {
        let b1 = rng.pick(&valid_b1_values());
        let b2 = rng.pick(&valid_b2_values());
        let h2 = [0xff, b1, b2];
        let base = matching_h1(&h2, &mut rng);
        let mut prev: Option<i32> = None;
        for v in 0u16..=255 {
            let h1 = [v as u8, base[1], base[2]];
            let r = p.assert_same(&h1, &h2, "row13");
            if let Some(prev) = prev {
                assert_eq!(prev, r, "h1[0]={v:#04x} changed the result");
            }
            prev = Some(r);
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 14-15: aliasing (h1 == h2, one buffer).
// ---------------------------------------------------------------------------

#[test]
fn row14_row15_aliased_pointers() {
    let p = Pair::load();
    let mut rng = Rng::seeded();
    let mut valid_seen = 0usize;
    let mut invalid_seen = 0usize;
    for _ in 0..200_000 {
        // Bias toward the sync byte so both branches are well covered: an
        // unbiased 3-byte draw is valid only ~1 time in 6700.
        let h: [u8; 3] = [
            if rng.below(2) == 0 { 0xff } else { rng.u8() },
            rng.u8(),
            rng.u8(),
        ];
        let r = p.assert_same_raw(h.as_ptr(), h.as_ptr(), "row14_row15");
        if h2_is_valid(&h) {
            assert_eq!(r, 1, "aliased valid header must accept: {h:02x?}");
            valid_seen += 1;
        } else {
            assert_eq!(r, 0, "aliased invalid header must reject: {h:02x?}");
            invalid_seen += 1;
        }
    }
    // Guard against a degenerate generator silently covering only one branch.
    assert!(valid_seen > 100, "row14 saw too few valid headers");
    assert!(invalid_seen > 100, "row15 saw too few invalid headers");
    // Also sweep every valid h2 exhaustively in the aliased configuration.
    for b1 in valid_b1_values() {
        for b2 in valid_b2_values() {
            let h = [0xffu8, b1, b2];
            assert_eq!(
                p.assert_same_raw(h.as_ptr(), h.as_ptr(), "row14_exhaustive"),
                1
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 16-18: valid `h2`, mismatching `h1` (the three comparison branches).
// ---------------------------------------------------------------------------

#[test]
fn row16_byte1_mismatch_on_mask_fe() {
    let p = Pair::load();
    let mut rng = Rng::seeded();
    for _ in 0..N {
        let b1 = rng.pick(&valid_b1_values());
        let b2 = rng.pick(&valid_b2_values());
        let h2 = [0xff, b1, b2];
        let mut h1 = matching_h1(&h2, &mut rng);
        // flip a random NON-EMPTY subset of mask 0xFE
        let mut flip = rng.u8() & 0xFE;
        if flip == 0 {
            flip = 0x02;
        }
        h1[1] ^= flip;
        assert_eq!(
            p.assert_same(&h1, &h2, "row16"),
            0,
            "byte1 differing on 0xFE must reject: h1={h1:02x?} h2={h2:02x?}"
        );
    }
}

#[test]
fn row17_byte2_mismatch_on_mask_0c() {
    let p = Pair::load();
    let mut rng = Rng::seeded();
    for flip in [0x04u8, 0x08, 0x0C] {
        for _ in 0..N / 3 {
            let b1 = rng.pick(&valid_b1_values());
            let b2 = rng.pick(&valid_b2_values());
            let h2 = [0xff, b1, b2];
            let mut h1 = matching_h1(&h2, &mut rng);
            h1[2] ^= flip;
            // flipping 0x04/0x08 cannot touch the high nibble, so NZ still agrees
            assert_eq!(
                p.assert_same(&h1, &h2, "row17"),
                0,
                "byte2 differing on 0x0C (flip={flip:#04x}) must reject: h1={h1:02x?} h2={h2:02x?}"
            );
        }
    }
}

#[test]
fn row18_high_nibble_zeroness_disagrees() {
    let p = Pair::load();
    let mut rng = Rng::seeded();
    // (a) h2[2] high nibble == 0, h1[2] high nibble != 0
    for _ in 0..N / 2 {
        let b1 = rng.pick(&valid_b1_values());
        let s = rng.below(3) as u8;
        let h2 = [0xff, b1, (s << 2) | (rng.u8() & 0x03)];
        assert_eq!(nibble(h2[2]), 0);
        let h1 = [
            rng.u8(),
            h2[1] ^ (rng.u8() & 0x01),
            (((rng.below(15) as u8) + 1) << 4) | (h2[2] & 0x0C) | (rng.u8() & 0x03),
        ];
        assert_eq!(
            p.assert_same(&h1, &h2, "row18a"),
            0,
            "NZ disagreement (a) must reject: h1={h1:02x?} h2={h2:02x?}"
        );
    }
    // (b) h2[2] high nibble != 0, h1[2] high nibble == 0
    for _ in 0..N / 2 {
        let b1 = rng.pick(&valid_b1_values());
        let n = 1 + (rng.below(14) as u8);
        let s = rng.below(3) as u8;
        let h2 = [0xff, b1, (n << 4) | (s << 2) | (rng.u8() & 0x03)];
        let h1 = [
            rng.u8(),
            h2[1] ^ (rng.u8() & 0x01),
            (h2[2] & 0x0C) | (rng.u8() & 0x03),
        ];
        assert_eq!(nibble(h1[2]), 0);
        assert_eq!(
            p.assert_same(&h1, &h2, "row18b"),
            0,
            "NZ disagreement (b) must reject: h1={h1:02x?} h2={h2:02x?}"
        );
    }
}

// ---------------------------------------------------------------------------
// Row 27: pointer placement / alignment / separate allocations.
// ---------------------------------------------------------------------------

#[test]
fn row27_pointer_placement_and_alignment() {
    let p = Pair::load();
    let mut rng = Rng::seeded();
    for _ in 0..20_000 {
        let h2: [u8; 3] = [
            if rng.below(4) == 0 { rng.u8() } else { 0xff },
            rng.u8(),
            rng.u8(),
        ];
        let h1 = matching_h1(&h2, &mut rng);
        let expect = p.assert_same(&h1, &h2, "row27_baseline");

        // Same logical bytes at every offset 0..8 inside one 32-byte buffer.
        // The two 3-byte windows are kept disjoint (h1 in 0..11, h2 in 16..27)
        // so they never alias each other.
        for off1 in 0..8usize {
            for off2 in 16..24usize {
                let mut buf = [0u8; 32];
                buf[off1..off1 + 3].copy_from_slice(&h1);
                buf[off2..off2 + 3].copy_from_slice(&h2);
                let r = p.assert_same_raw(
                    unsafe { buf.as_ptr().add(off1) },
                    unsafe { buf.as_ptr().add(off2) },
                    "row27_offsets",
                );
                assert_eq!(r, expect, "offset {off1}/{off2} changed the result");
            }
        }

        // Two separate heap allocations.
        let a = h1.to_vec();
        let b = h2.to_vec();
        assert_eq!(
            p.assert_same_raw(a.as_ptr(), b.as_ptr(), "row27_heap"),
            expect
        );
    }
}
