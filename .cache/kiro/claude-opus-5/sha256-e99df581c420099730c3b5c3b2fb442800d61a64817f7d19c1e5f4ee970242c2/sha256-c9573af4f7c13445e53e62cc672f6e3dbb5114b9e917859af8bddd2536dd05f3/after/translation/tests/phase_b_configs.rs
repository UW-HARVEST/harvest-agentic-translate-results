//! Phase B, CONFIGS.md rows 5–24: one test per configuration row, each driving
//! both `.so` exports and asserting byte-identical `int` results. Rows use many
//! randomized inputs (SplitMix64, fixed seeds) on top of the row's boundary
//! values.
//!
//! Rows 1–4 (the exhaustive sweeps) live in `phase_b_exhaustive.rs`.

mod common;

use common::*;

/// Row 5 — uniformly random over all five relevant bytes. Dominated by the
/// rejecting mass, which is exactly the region a hand-picked happy-path value
/// never visits.
#[test]
fn cfg_row05_uniform_random_20m() {
    let l = libs();
    let mut rng = Rng::new(0x5EED_0005);
    let mut accepted = 0u64;
    for _ in 0..20_000_000u32 {
        let h1 = [rng.next_u8(), rng.next_u8(), rng.next_u8()];
        let h2 = [rng.next_u8(), rng.next_u8(), rng.next_u8()];
        if let Some((c, r)) = diff(l, &h1, &h2) {
            panic!("row5 divergence: h1={h1:02x?} h2={h2:02x?} => C={c} Rust={r}");
        }
        // Cheap re-call to count accepts without a second predicate model.
        accepted += (unsafe { (l.c)(h1.as_ptr(), h2.as_ptr()) } != 0) as u64;
    }
    eprintln!("row5: 20M uniform pairs, {accepted} accepted by C (and by Rust)");
}

/// Row 6 — randomized but biased into the *accepting* region so the deep terms
/// (axes F/G/H) are reached on most draws instead of being short-circuited away.
#[test]
fn cfg_row06_accept_biased_random_20m() {
    let l = libs();
    let v1 = valid_byte1_values();
    let v2 = valid_byte2_values();
    let mut rng = Rng::new(0x5EED_0006);
    let mut accepted = 0u64;
    for _ in 0..20_000_000u32 {
        let b1 = rng.pick(&v1);
        let b2 = rng.pick(&v2);
        let h2 = [0xffu8, b1, b2];
        // h1 tail: near-matching, with small perturbations drawn at random.
        let a1 = b1 ^ (1u8 << rng.below(8)) & if rng.next_u64() & 1 == 0 { 0xff } else { 0x01 };
        let a2 = b2 ^ (1u8 << rng.below(8)) & if rng.next_u64() & 1 == 0 { 0xff } else { 0xf3 };
        let h1 = [rng.next_u8(), a1, a2];
        if let Some((c, r)) = diff(l, &h1, &h2) {
            panic!("row6 divergence: h1={h1:02x?} h2={h2:02x?} => C={c} Rust={r}");
        }
        accepted += (unsafe { (l.c)(h1.as_ptr(), h2.as_ptr()) } != 0) as u64;
    }
    assert!(accepted > 1_000_000, "accept-biased row only accepted {accepted}/20M");
    eprintln!("row6: 20M accept-biased pairs, {accepted} accepted");
}

/// Row 7 — axis A positive: valid sync byte plus an otherwise perfectly matching
/// valid header must be accepted, for every valid `(h2[1], h2[2])`, and with the
/// never-read `h1[0]` randomized.
#[test]
fn cfg_row07_sync_ok_matching_header_accepts() {
    let l = libs();
    let mut rng = Rng::new(0x5EED_0007);
    for &b1 in &valid_byte1_values() {
        for &b2 in &valid_byte2_values() {
            let h2 = [0xffu8, b1, b2];
            let h1 = [rng.next_u8(), b1, b2];
            let got = assert_same(&h1, &h2);
            assert_eq!(got, 1, "C rejected a self-matching valid header {h2:02x?}");
            let _ = l;
        }
    }
}

/// Row 8 — axis A negative: all 255 non-`0xff` sync bytes, each with an
/// otherwise perfectly matching valid header, must be rejected by both.
#[test]
fn cfg_row08_all_bad_sync_bytes_reject() {
    let v1 = valid_byte1_values();
    let v2 = valid_byte2_values();
    let mut rng = Rng::new(0x5EED_0008);
    for b0 in 0u16..256 {
        if b0 as u8 == 0xff {
            continue;
        }
        for &b1 in &v1 {
            for &b2 in &v2 {
                let h2 = [b0 as u8, b1, b2];
                let h1 = [rng.next_u8(), b1, b2];
                assert_eq!(assert_same(&h1, &h2), 0, "bad sync byte accepted: {h2:02x?}");
            }
        }
    }
}

/// Row 9 — axis B class 1: `h2[1]` in `[0xf0, 0xff]` (the `& 0xF0 == 0xf0`
/// branch), each of the 16 values crossed with all 256 `h2[2]` values and
/// matching / perturbed `h1` tails.
#[test]
fn cfg_row09_byte1_high_nibble_class() {
    let mut rng = Rng::new(0x5EED_0009);
    for b1 in 0xf0u16..=0xff {
        let b1 = b1 as u8;
        assert!(byte1_passes_class(b1));
        for b2 in 0u16..256 {
            let b2 = b2 as u8;
            let h2 = [0xffu8, b1, b2];
            for a1 in [b1, b1 ^ 0x01, b1 ^ 0x02, rng.next_u8()] {
                for a2 in [b2, b2 ^ 0x04, b2 ^ 0x10, b2 & 0x0f, rng.next_u8()] {
                    assert_same(&[rng.next_u8(), a1, a2], &h2);
                }
            }
        }
    }
}

/// Row 10 — axis B class 2: `h2[1] ∈ {0xe2, 0xe3}`, the MPEG-2.5 branch that is
/// only reachable through the `(h2[1] & 0xFE) == 0xe2` alternative. Missing this
/// branch would leave a whole family of accepted headers untested.
#[test]
fn cfg_row10_byte1_mpeg25_class() {
    let mut rng = Rng::new(0x5EED_000A);
    for b1 in [0xe2u8, 0xe3] {
        assert!(byte1_passes_class(b1) && (b1 & 0xF0) != 0xf0);
        for b2 in 0u16..256 {
            let b2 = b2 as u8;
            let h2 = [0xffu8, b1, b2];
            for a1 in [b1, b1 ^ 0x01, b1 ^ 0x02, b1 ^ 0x10, rng.next_u8()] {
                for a2 in [b2, b2 ^ 0x04, b2 ^ 0x08, b2 | 0x30, b2 & 0x0f, rng.next_u8()] {
                    assert_same(&[rng.next_u8(), a1, a2], &h2);
                }
            }
        }
    }
}

/// Row 11 — axis B class 3: all 238 `h2[1]` values in neither accepted class.
#[test]
fn cfg_row11_byte1_neither_class() {
    let v2 = valid_byte2_values();
    let mut rng = Rng::new(0x5EED_000B);
    let mut n = 0;
    for b1 in 0u16..256 {
        let b1 = b1 as u8;
        if byte1_passes_class(b1) {
            continue;
        }
        n += 1;
        for &b2 in &v2 {
            let h2 = [0xffu8, b1, b2];
            assert_eq!(assert_same(&[rng.next_u8(), b1, b2], &h2), 0);
            assert_eq!(assert_same(&[rng.next_u8(), rng.next_u8(), rng.next_u8()], &h2), 0);
        }
    }
    assert_eq!(n, 238, "expected 238 h2[1] values outside both classes");
}

/// Row 12 — axis C, the accepted layer-field values 1, 2, 3.
#[test]
fn cfg_row12_byte1_layer_field_nonzero() {
    let v2 = valid_byte2_values();
    let mut rng = Rng::new(0x5EED_000C);
    for layer in 1u8..=3 {
        let mut seen = 0;
        for b1 in 0u16..256 {
            let b1 = b1 as u8;
            if !byte1_passes_class(b1) || ((b1 >> 1) & 3) != layer {
                continue;
            }
            seen += 1;
            for &b2 in &v2 {
                let h2 = [0xffu8, b1, b2];
                assert_eq!(
                    assert_same(&[rng.next_u8(), b1, b2], &h2),
                    1,
                    "layer {layer} header {h2:02x?} should be accepted"
                );
                assert_same(&[rng.next_u8(), b1 ^ 0x01, b2], &h2);
                assert_same(&[rng.next_u8(), rng.next_u8(), rng.next_u8()], &h2);
            }
        }
        assert!(seen > 0, "no h2[1] found with layer field {layer}");
    }
}

/// Row 13 — axis C boundary: `h2[1] ∈ {0xf0, 0xf1, 0xf8, 0xf9}` passes the class
/// test but has the reserved layer field 0, so it must be rejected.
#[test]
fn cfg_row13_byte1_reserved_layer_boundary() {
    let mut rng = Rng::new(0x5EED_000D);
    let expected = [0xf0u8, 0xf1, 0xf8, 0xf9];
    let derived: Vec<u8> = (0u16..256)
        .map(|v| v as u8)
        .filter(|&b| byte1_passes_class(b) && ((b >> 1) & 3) == 0)
        .collect();
    assert_eq!(derived, expected);
    for b1 in expected {
        for b2 in 0u16..256 {
            let b2 = b2 as u8;
            let h2 = [0xffu8, b1, b2];
            assert_eq!(assert_same(&[rng.next_u8(), b1, b2], &h2), 0);
            for _ in 0..8 {
                assert_eq!(
                    assert_same(&[rng.next_u8(), rng.next_u8(), rng.next_u8()], &h2),
                    0
                );
            }
        }
    }
}

/// Row 14 — axis D: free-format bitrate nibble (`h2[2] >> 4 == 0`) on both
/// headers, all 16 low-nibble values each (256 combinations), for every valid
/// `h2[1]`.
#[test]
fn cfg_row14_free_format_both() {
    let mut rng = Rng::new(0x5EED_000E);
    for &b1 in &valid_byte1_values() {
        for b2 in 0u8..16 {
            for a2 in 0u8..16 {
                let h2 = [0xffu8, b1, b2];
                assert_same(&[rng.next_u8(), b1, a2], &h2);
                assert_same(&[rng.next_u8(), b1 ^ 0x01, a2], &h2);
            }
        }
    }
}

/// Row 15 — axis D: bitrate nibble 1…14, all 16 low nibbles, matching and
/// perturbed `h1[2]`.
#[test]
fn cfg_row15_bitrate_nibble_1_to_14() {
    let mut rng = Rng::new(0x5EED_000F);
    for &b1 in &valid_byte1_values() {
        for hi in 1u8..=14 {
            for lo in 0u8..16 {
                let b2 = (hi << 4) | lo;
                let h2 = [0xffu8, b1, b2];
                for a2 in [b2, b2 ^ 0x01, b2 ^ 0x04, b2 ^ 0x08, b2 & 0x0f, b2 ^ 0xf0, rng.next_u8()]
                {
                    assert_same(&[rng.next_u8(), b1, a2], &h2);
                }
            }
        }
    }
}

/// Row 16 — axis D boundary: bitrate nibble 15 (`0xf0..=0xff`) is reserved and
/// must be rejected, whatever `h1` is.
#[test]
fn cfg_row16_bitrate_nibble_15_boundary() {
    let mut rng = Rng::new(0x5EED_0010);
    for &b1 in &valid_byte1_values() {
        for b2 in 0xf0u16..=0xff {
            let b2 = b2 as u8;
            let h2 = [0xffu8, b1, b2];
            assert_eq!(assert_same(&[rng.next_u8(), b1, b2], &h2), 0);
            for _ in 0..16 {
                assert_eq!(
                    assert_same(&[rng.next_u8(), rng.next_u8(), rng.next_u8()], &h2),
                    0
                );
            }
        }
    }
}

/// Row 17 — axis E: samplerate field 0, 1, 2 with `h1[2]` matching and with each
/// samplerate bit flipped.
#[test]
fn cfg_row17_samplerate_field_0_1_2() {
    let mut rng = Rng::new(0x5EED_0011);
    for &b1 in &valid_byte1_values() {
        for sr in 0u8..3 {
            for hi in 0u8..15 {
                for lowbits in 0u8..4 {
                    let b2 = (hi << 4) | (sr << 2) | lowbits;
                    assert_eq!((b2 >> 2) & 3, sr);
                    let h2 = [0xffu8, b1, b2];
                    for a2 in [b2, b2 ^ 0x04, b2 ^ 0x08, b2 ^ 0x0c, b2 ^ 0x03, b2 ^ 0x50] {
                        assert_same(&[rng.next_u8(), b1, a2], &h2);
                    }
                }
            }
        }
    }
}

/// Row 18 — axis E boundary: samplerate field 3 (all 64 such `h2[2]`, including
/// the 4 where axis D also fires) must be rejected.
#[test]
fn cfg_row18_samplerate_field_3_boundary() {
    let mut rng = Rng::new(0x5EED_0012);
    let mut n = 0;
    for &b1 in &valid_byte1_values() {
        for b2 in 0u16..256 {
            let b2 = b2 as u8;
            if ((b2 >> 2) & 3) != 3 {
                continue;
            }
            n += 1;
            let h2 = [0xffu8, b1, b2];
            assert_eq!(assert_same(&[rng.next_u8(), b1, b2], &h2), 0);
            for _ in 0..8 {
                assert_eq!(
                    assert_same(&[rng.next_u8(), rng.next_u8(), rng.next_u8()], &h2),
                    0
                );
            }
        }
    }
    assert_eq!(n, 64 * 14);
}

/// Row 19 — axis F accepting side: `h1[1] == h2[1]` and `h1[1] == h2[1] ^ 0x01`
/// (bit 0 is masked out by `& 0xFE`, so flipping it must not change the result).
#[test]
fn cfg_row19_byte1_bit0_is_ignored() {
    let mut rng = Rng::new(0x5EED_0013);
    for &b1 in &valid_byte1_values() {
        for &b2 in &valid_byte2_values() {
            let h2 = [0xffu8, b1, b2];
            let with = assert_same(&[rng.next_u8(), b1, b2], &h2);
            let flipped = assert_same(&[rng.next_u8(), b1 ^ 0x01, b2], &h2);
            assert_eq!(with, flipped, "bit 0 of h1[1] changed the result for {h2:02x?}");
            assert_eq!(with, 1);
            // Also flip bit 0 of h2[1]: 0xe2/0xe3 and 0xfa/0xfb etc. are a pair.
            let h2b = [0xffu8, b1 ^ 0x01, b2];
            if byte1_valid(b1 ^ 0x01) {
                assert_eq!(assert_same(&[rng.next_u8(), b1, b2], &h2b), 1);
            }
        }
    }
}

/// Row 20 — axis F rejecting side: flip each masked bit (1…7) of `h1[1]`
/// individually; every one must flip the verdict to reject.
#[test]
fn cfg_row20_byte1_masked_bit_flips_reject() {
    let mut rng = Rng::new(0x5EED_0014);
    for &b1 in &valid_byte1_values() {
        for &b2 in &valid_byte2_values() {
            let h2 = [0xffu8, b1, b2];
            for k in 1..8 {
                let a1 = b1 ^ (1u8 << k);
                assert_eq!(
                    assert_same(&[rng.next_u8(), a1, b2], &h2),
                    0,
                    "flipping bit {k} of h1[1] should reject ({h2:02x?})"
                );
            }
        }
    }
}

/// Row 21 — axis G: the samplerate bits of `h1[2]` differing from `h2[2]`.
#[test]
fn cfg_row21_byte2_samplerate_bit_mismatch() {
    let mut rng = Rng::new(0x5EED_0015);
    for &b1 in &valid_byte1_values() {
        for &b2 in &valid_byte2_values() {
            let h2 = [0xffu8, b1, b2];
            for mask in [0x04u8, 0x08, 0x0C] {
                assert_eq!(
                    assert_same(&[rng.next_u8(), b1, b2 ^ mask], &h2),
                    0,
                    "samplerate mismatch mask {mask:#04x} should reject ({h2:02x?})"
                );
            }
            // Bits 0-1 are *not* masked and must be irrelevant.
            for mask in [0x01u8, 0x02, 0x03] {
                assert_eq!(assert_same(&[rng.next_u8(), b1, b2 ^ mask], &h2), 1);
            }
        }
    }
}

/// Row 22 — axis H: all four (h1 nibble zero?, h2 nibble zero?) combinations
/// with axes F and G held passing, so only the free-format agreement term
/// decides. The two mixed cases must reject; the two agreeing cases accept.
#[test]
fn cfg_row22_free_format_nibble_agreement() {
    let mut rng = Rng::new(0x5EED_0016);
    for &b1 in &valid_byte1_values() {
        for &b2 in &valid_byte2_values() {
            let h2 = [0xffu8, b1, b2];
            let h2_zero = (b2 & 0xF0) == 0;
            for hi in 0u8..16 {
                // Keep bits 2-3 equal to h2[2] (axis G passing), vary the nibble.
                let a2 = (hi << 4) | (b2 & 0x0F);
                let a2 = a2 ^ (rng.next_u8() & 0x03); // bits 0-1 are irrelevant
                let a2 = (a2 & 0xF3) | (b2 & 0x0C);
                let h1_zero = (a2 & 0xF0) == 0;
                let got = assert_same(&[rng.next_u8(), b1, a2], &h2);
                let want = if h1_zero == h2_zero { 1 } else { 0 };
                assert_eq!(
                    got, want,
                    "nibble agreement (h1_zero={h1_zero}, h2_zero={h2_zero}) \
                     h1[2]={a2:#04x} h2={h2:02x?}"
                );
            }
        }
    }
}

/// Row 23 — axis I: aliased pointers (`h1 == h2`), for every one of the 65 536
/// tails and every one of the 256 sync bytes. A self-comparison must reduce to
/// plain validity.
#[test]
fn cfg_row23_aliased_pointers() {
    let l = libs();
    for b0 in 0u16..256 {
        for b1 in 0u16..256 {
            for b2 in 0u16..256 {
                let h = [b0 as u8, b1 as u8, b2 as u8];
                let p = h.as_ptr();
                let (c, r) = unsafe { ((l.c)(p, p), (l.rust)(p, p)) };
                assert_eq!(c, r, "aliased divergence for {h:02x?}: C={c} Rust={r}");
            }
        }
    }
}

/// Row 24 — axis I: `h1[0]` is never dereferenced by the C, so sweeping it over
/// all 256 values must not change any verdict. Checked across randomized
/// `h1`/`h2` tails as well as the valid region. (The "unreadable `h1`" half of
/// this row is a rejection condition and lives in `phase_c_errors.rs`.)
#[test]
fn cfg_row24_h1_byte0_is_never_read() {
    let mut rng = Rng::new(0x5EED_0017);
    // Valid region.
    for &b1 in &valid_byte1_values() {
        for &b2 in &valid_byte2_values() {
            let h2 = [0xffu8, b1, b2];
            let base = assert_same(&[0x00, b1, b2], &h2);
            for x in 0u16..256 {
                assert_eq!(
                    assert_same(&[x as u8, b1, b2], &h2),
                    base,
                    "h1[0]={x:#04x} changed the verdict for {h2:02x?}"
                );
            }
        }
    }
    // Random region.
    for _ in 0..20_000 {
        let (a1, a2) = (rng.next_u8(), rng.next_u8());
        let h2 = [rng.next_u8(), rng.next_u8(), rng.next_u8()];
        let base = assert_same(&[0x00, a1, a2], &h2);
        for x in 0u16..256 {
            assert_eq!(assert_same(&[x as u8, a1, a2], &h2), base);
        }
    }
}
