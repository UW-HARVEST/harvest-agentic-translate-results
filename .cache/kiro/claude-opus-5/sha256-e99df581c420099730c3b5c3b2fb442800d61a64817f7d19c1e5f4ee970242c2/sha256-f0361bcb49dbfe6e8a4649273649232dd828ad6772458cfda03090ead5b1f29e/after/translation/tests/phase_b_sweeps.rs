//! Phase B — exhaustive / high-volume sweeps (`CONFIGS.md` rows 24, 25, 26).
//!
//! These are the rows that do not depend on my reading of the C being right:
//! they enumerate the input space mechanically and compare both `.so`s on
//! every point.

mod common;

use common::*;

/// Row 24 — exhaustive sweep of ALL 2^24 `h2` byte triples.
///
/// For every triple, both implementations are called with several fixed `h1`
/// values plus one `h1` derived from `h2`, so both the accept and reject sides
/// of all three comparison branches are hit at every `h2`.
#[test]
fn row24_exhaustive_h2_2pow24() {
    let p = Pair::load();
    let fixed_h1: [[u8; 3]; 4] = [
        [0x00, 0x00, 0x00],
        [0xff, 0xfb, 0x90],
        [0x5a, 0xe3, 0x0f],
        [0xff, 0xff, 0xef],
    ];
    let mut rng = Rng::seeded();
    let mut accepts = 0u64;
    let mut total = 0u64;
    for b0 in 0u16..=255 {
        for b1 in 0u16..=255 {
            for b2 in 0u16..=255 {
                let h2 = [b0 as u8, b1 as u8, b2 as u8];
                for h1 in &fixed_h1 {
                    accepts += p.assert_same(h1, &h2, "row24") as u64;
                    total += 1;
                }
                // plus a matching h1, exercising the accept path at this h2
                let m = matching_h1(&h2, &mut rng);
                let r = p.assert_same(&m, &h2, "row24_matching");
                accepts += r as u64;
                total += 1;
                if h2_is_valid(&h2) {
                    assert_eq!(r, 1, "valid h2={h2:02x?} rejected its matching h1");
                } else {
                    assert_eq!(r, 0, "invalid h2={h2:02x?} accepted");
                }
            }
        }
    }
    assert_eq!(total, (1 << 24) * 5);
    assert!(accepts > 0, "sweep never observed an accept — generator bug");
    eprintln!("row24: {total} pairs compared, {accepts} accepts");
}

/// Row 25 — exhaustive sweep of ALL 2^16 `(h1[1], h1[2])` pairs for a set of
/// `h2` values covering every validity class.
#[test]
fn row25_exhaustive_h1_2pow16() {
    let p = Pair::load();
    let h2_set: [[u8; 3]; 16] = [
        // valid: F-class, each layer, zero and non-zero nibble, each S
        [0xff, 0xf2, 0x00],
        [0xff, 0xf4, 0x04],
        [0xff, 0xf6, 0x08],
        [0xff, 0xfb, 0x90],
        [0xff, 0xfd, 0xe8],
        [0xff, 0xff, 0x14],
        // valid: E-class
        [0xff, 0xe2, 0x00],
        [0xff, 0xe3, 0x73],
        // invalid: bad sync
        [0xfe, 0xfb, 0x90],
        [0x00, 0xfb, 0x90],
        // invalid: byte1 in neither class
        [0xff, 0x00, 0x90],
        [0xff, 0xe4, 0x90],
        // invalid: layer 0
        [0xff, 0xf0, 0x90],
        [0xff, 0xf9, 0x90],
        // invalid: nibble 15
        [0xff, 0xfb, 0xf0],
        // invalid: S == 3
        [0xff, 0xfb, 0x9c],
    ];
    let mut total = 0u64;
    for h2 in &h2_set {
        for b1 in 0u16..=255 {
            for b2 in 0u16..=255 {
                // h1[0] fixed here; row13 proves it is never read.
                let h1 = [0xa5u8, b1 as u8, b2 as u8];
                let r = p.assert_same(&h1, h2, "row25");
                total += 1;
                if !h2_is_valid(h2) {
                    assert_eq!(r, 0, "invalid h2={h2:02x?} accepted h1={h1:02x?}");
                }
            }
        }
    }
    assert_eq!(total, 16 * (1 << 16));
    eprintln!("row25: {total} pairs compared");
}

/// Row 26 — unbiased random fuzz over all five read-relevant bytes.
#[test]
fn row26_random_fuzz() {
    let p = Pair::load();
    let mut rng = Rng::seeded();
    let mut accepts = 0u64;
    const ITERS: usize = 2_000_000;
    for _ in 0..ITERS {
        // Bias a third of the vectors toward the interesting sub-space
        // (sync byte correct) so accepts are not vanishingly rare, and leave
        // the rest fully unbiased.
        let mode = rng.below(3);
        let h2 = match mode {
            0 => [rng.u8(), rng.u8(), rng.u8()],
            1 => [0xff, rng.u8(), rng.u8()],
            _ => [
                0xff,
                rng.pick(&valid_b1_values()),
                rng.pick(&valid_b2_values()),
            ],
        };
        let h1 = if rng.below(2) == 0 {
            [rng.u8(), rng.u8(), rng.u8()]
        } else {
            matching_h1(&h2, &mut rng)
        };
        accepts += p.assert_same(&h1, &h2, "row26") as u64;
    }
    assert!(accepts > 0 && (accepts as usize) < ITERS, "fuzz too degenerate");
    eprintln!("row26: {ITERS} pairs compared, {accepts} accepts");
}

/// Row 31 — **complete** decision surface for the accept side.
///
/// Rows 24 and 25 are each exhaustive in one argument only, so their
/// interaction is not fully covered. The result of `hdr_compare` depends on
/// exactly five bytes: `h2[0..3]`, `h1[1]`, `h1[2]` (`h1[0]` is never read —
/// row 13). Restricted to `h2` values that pass `hdr_valid`, that space is
/// small enough to enumerate in full: every valid `h2` (2520 of them) crossed
/// with all 2^16 `(h1[1], h1[2])` pairs. So every input that can possibly
/// return `1` is compared between the two implementations.
#[test]
fn row31_complete_accept_surface() {
    let p = Pair::load();
    let b1s = valid_b1_values();
    let b2s = valid_b2_values();
    assert_eq!(b1s.len(), 14, "valid h2[1] count changed");
    assert_eq!(b2s.len(), 180, "valid h2[2] count changed");

    let mut total = 0u64;
    let mut accepts = 0u64;
    for &vb1 in &b1s {
        for &vb2 in &b2s {
            let h2 = [0xffu8, vb1, vb2];
            debug_assert!(h2_is_valid(&h2));
            for b1 in 0u16..=255 {
                for b2 in 0u16..=255 {
                    let h1 = [0x00u8, b1 as u8, b2 as u8];
                    accepts += p.assert_same(&h1, &h2, "row31") as u64;
                    total += 1;
                }
            }
        }
    }
    assert_eq!(total, 14 * 180 * (1 << 16));
    // Sanity on the count: for each valid h2, h1[1] is free in bit 0 (2 ways)
    // and h1[2] is free in bits 0x03 (4 ways) times the allowed high nibbles
    // (1 if h2's nibble is zero, else 15).
    let expected_accepts: u64 = b1s.len() as u64
        * b2s
            .iter()
            .map(|&vb2| if vb2 & 0xF0 == 0 { 2 * 4 } else { 2 * 4 * 15 })
            .sum::<u64>();
    assert_eq!(
        accepts, expected_accepts,
        "accept count differs from the closed-form derived from the C source"
    );
    eprintln!("row31: {total} pairs compared, {accepts} accepts (complete accept surface)");
}

/// Row 32 — reject side: every **invalid** `h2` (of the 65 536 byte-1/byte-2
/// combinations under a correct sync byte) against a seeded random sample of
/// `h1` values. Invalid `h2` must reject regardless of `h1`.
#[test]
fn row32_invalid_h2_against_sampled_h1() {
    let p = Pair::load();
    let mut rng = Rng::seeded();
    let mut total = 0u64;
    for b1 in 0u16..=255 {
        for b2 in 0u16..=255 {
            let h2 = [0xffu8, b1 as u8, b2 as u8];
            if h2_is_valid(&h2) {
                continue;
            }
            for _ in 0..48 {
                let h1 = [rng.u8(), rng.u8(), rng.u8()];
                assert_eq!(
                    p.assert_same(&h1, &h2, "row32"),
                    0,
                    "invalid h2={h2:02x?} accepted h1={h1:02x?}"
                );
                total += 1;
            }
            // and the aliased / exactly-equal h1
            assert_eq!(p.assert_same(&h2, &h2, "row32_alias"), 0);
            total += 1;
        }
    }
    eprintln!("row32: {total} pairs compared, all rejected");
}
