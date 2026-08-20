//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md` (rows 1..14 and 21). Every row is driven
//! with many inputs from a fixed-seed PRNG and/or a full enumeration of the
//! field(s) the row is about. Both implementations are invoked through their
//! `.so` exports; results must be byte-identical.

mod common;

use common::*;

/// Rows 1–3 share this shape: `h1 == h2`, `h2` valid, sync form varies.
fn identical_valid(sync: Sync, tag: &str) {
    let im = load();
    let mut rng = Rng::new(SEED ^ tag.len() as u64);
    let mut n = 0usize;
    for layer in 1u8..=3 {
        // For MPEG2.5 the layer bits are fixed by the 0xE2 encoding.
        if sync == Sync::Mpeg25 && layer != 1 {
            continue;
        }
        for bitrate in 1u8..=14 {
            for srate in 0u8..=2 {
                for _ in 0..24 {
                    let h = make_hdr(
                        sync,
                        layer,
                        bitrate,
                        srate,
                        rng.u8() & 1 == 1,
                        rng.u8() & 1 == 1,
                        rng.u8() & 1 == 1,
                        rng.u8(),
                    );
                    assert!(model_valid(&h), "{tag}: built an invalid h2 {h:02x?}");
                    let got = im.assert_eq_slices(&h, &h, tag);
                    assert_eq!(got, 1, "{tag}: expected match for identical valid {h:02x?}");
                    n += 1;
                }
            }
        }
    }
    assert!(n >= 500, "{tag}: only {n} cases");
}

#[test]
fn row01_identical_valid_mpeg1() {
    identical_valid(Sync::Mpeg1, "row01/mpeg1");
}

#[test]
fn row02_identical_valid_mpeg2() {
    identical_valid(Sync::Mpeg2, "row02/mpeg2");
}

#[test]
fn row03_identical_valid_mpeg25() {
    identical_valid(Sync::Mpeg25, "row03/mpeg2.5");
}

/// Row 4: free-format bitrate index 0 on both sides.
#[test]
fn row04_free_format_both_sides() {
    let im = load();
    let mut rng = Rng::new(SEED ^ 4);
    for sync in [Sync::Mpeg1, Sync::Mpeg2, Sync::Mpeg25] {
        for layer in 1u8..=3 {
            if sync == Sync::Mpeg25 && layer != 1 {
                continue;
            }
            for srate in 0u8..=2 {
                for _ in 0..64 {
                    let h2 = make_hdr(
                        sync,
                        layer,
                        0,
                        srate,
                        rng.u8() & 1 == 1,
                        rng.u8() & 1 == 1,
                        rng.u8() & 1 == 1,
                        rng.u8(),
                    );
                    assert!(model_valid(&h2));
                    // h1: also free format, same version/layer/srate, other bits random.
                    let mut h1 = h2;
                    h1[0] = rng.u8();
                    h1[1] = (h2[1] & 0xFE) | (rng.u8() & 1);
                    h1[2] = (h2[2] & 0x0C) | (rng.u8() & 0x03); // bitrate index stays 0
                    h1[3] = rng.u8();
                    let got = im.assert_eq_slices(&h1, &h2, "row04/free-format");
                    assert_eq!(got, 1, "row04: h1={h1:02x?} h2={h2:02x?}");
                }
            }
        }
    }
}

/// Row 5: only the CRC/protection bit (`h[1] & 1`) differs — masked out.
#[test]
fn row05_crc_bit_irrelevant() {
    let im = load();
    let mut rng = Rng::new(SEED ^ 5);
    for _ in 0..4096 {
        let sync = [Sync::Mpeg1, Sync::Mpeg2, Sync::Mpeg25][rng.below(3) as usize];
        let layer = if sync == Sync::Mpeg25 { 1 } else { 1 + rng.u8() % 3 };
        let h2 = make_hdr(sync, layer, 1 + rng.u8() % 14, rng.u8() % 3, false, false, false, 0);
        assert!(model_valid(&h2));
        let mut h1 = h2;
        h1[1] ^= 1; // flip only bit 0
        let got = im.assert_eq_slices(&h1, &h2, "row05/crc-bit");
        assert_eq!(got, 1, "row05: h1={h1:02x?} h2={h2:02x?}");
        // and the symmetric direction
        let got = im.assert_eq_slices(&h2, &h1, "row05/crc-bit-swapped");
        assert_eq!(got, 1);
    }
}

/// Row 6: only padding/private bits (`h[2] & 0x03`) differ — masked out.
#[test]
fn row06_padding_private_irrelevant() {
    let im = load();
    let mut rng = Rng::new(SEED ^ 6);
    for _ in 0..4096 {
        let sync = [Sync::Mpeg1, Sync::Mpeg2, Sync::Mpeg25][rng.below(3) as usize];
        let layer = if sync == Sync::Mpeg25 { 1 } else { 1 + rng.u8() % 3 };
        let h2 = make_hdr(sync, layer, 1 + rng.u8() % 14, rng.u8() % 3, false, false, true, 0);
        assert!(model_valid(&h2));
        for delta in 1u8..=3 {
            let mut h1 = h2;
            h1[2] ^= delta;
            let got = im.assert_eq_slices(&h1, &h2, "row06/pad-priv");
            assert_eq!(got, 1, "row06: delta={delta:#04x} h1={h1:02x?} h2={h2:02x?}");
        }
    }
}

/// Row 7: differing but both non-zero bitrate indices (including the reserved
/// index 15 in `h1`, which the C never validates).
#[test]
fn row07_differing_nonzero_bitrate() {
    let im = load();
    let mut rng = Rng::new(SEED ^ 7);
    for b2 in 1u8..=14 {
        for b1 in 1u8..=15 {
            for _ in 0..8 {
                let sync = [Sync::Mpeg1, Sync::Mpeg2, Sync::Mpeg25][rng.below(3) as usize];
                let layer = if sync == Sync::Mpeg25 { 1 } else { 1 + rng.u8() % 3 };
                let srate = rng.u8() % 3;
                let h2 = make_hdr(sync, layer, b2, srate, false, false, true, 0);
                assert!(model_valid(&h2));
                let mut h1 = h2;
                h1[2] = (b1 << 4) | (srate << 2) | (rng.u8() & 3);
                let got = im.assert_eq_slices(&h1, &h2, "row07/bitrate");
                assert_eq!(got, 1, "row07: h1={h1:02x?} h2={h2:02x?}");
            }
        }
    }
}

/// Row 8: `h1[0]` is never read; `h[3]` and beyond are never read.
#[test]
fn row08_unread_bytes_irrelevant() {
    let im = load();
    let mut rng = Rng::new(SEED ^ 8);
    let base = valid_h2();
    for b0 in 0u16..=255 {
        for _ in 0..8 {
            let h2 = [base[0], base[1], base[2], rng.u8()];
            let h1 = [b0 as u8, base[1] ^ (rng.u8() & 1), base[2] ^ (rng.u8() & 3), rng.u8()];
            let got = im.assert_eq_slices(&h1, &h2, "row08/unread");
            assert_eq!(got, 1, "row08: h1={h1:02x?} h2={h2:02x?}");
            // Trailing bytes beyond index 2 must be irrelevant for longer buffers too.
            let mut long1 = [0u8; 16];
            let mut long2 = [0u8; 16];
            rng.fill(&mut long1);
            rng.fill(&mut long2);
            long1[..3].copy_from_slice(&h1[..3]);
            long2[..3].copy_from_slice(&h2[..3]);
            let got2 = im.assert_eq_slices(&long1, &long2, "row08/long");
            assert_eq!(got, got2, "row08: trailing bytes changed the result");
        }
    }
}

/// Row 9: `h1` is an otherwise completely invalid header but agrees with a
/// valid `h2` under the compared masks.
#[test]
fn row09_h1_never_validated() {
    let im = load();
    let mut rng = Rng::new(SEED ^ 9);
    for _ in 0..8192 {
        let sync = [Sync::Mpeg1, Sync::Mpeg2, Sync::Mpeg25][rng.below(3) as usize];
        let layer = if sync == Sync::Mpeg25 { 1 } else { 1 + rng.u8() % 3 };
        let srate = rng.u8() % 3;
        let h2 = make_hdr(sync, layer, 1 + rng.u8() % 14, srate, false, false, true, 0);
        assert!(model_valid(&h2));
        // h1: bogus sync byte, reserved bitrate index 15, random pad/private.
        let h1 = [
            rng.u8() & 0xFE,           // never 0xFF (and never read anyway)
            (h2[1] & 0xFE) | (rng.u8() & 1),
            0xF0 | (h2[2] & 0x0C) | (rng.u8() & 0x03),
            rng.u8(),
        ];
        assert!(!model_valid(&h1), "row09: h1 should be invalid: {h1:02x?}");
        let got = im.assert_eq_slices(&h1, &h2, "row09/h1-invalid");
        assert_eq!(got, 1, "row09: h1={h1:02x?} h2={h2:02x?}");
        // The reverse (invalid header as h2) must be rejected.
        let got = im.assert_eq_slices(&h2, &h1, "row09/reverse");
        assert_eq!(got, 0, "row09 reverse: h1={h2:02x?} h2={h1:02x?}");
    }
}

/// Row 10: aliased pointers (`h1 == h2`).
#[test]
fn row10_aliased_pointers() {
    let im = load();
    let mut rng = Rng::new(SEED ^ 10);
    for _ in 0..20_000 {
        let mut h = [0u8; 4];
        rng.fill(&mut h);
        if rng.u8() & 1 == 1 {
            h[0] = 0xFF; // bias towards valid
        }
        let p = h.as_ptr();
        let (c, r) = im.both(p, p);
        assert_eq!(c, r, "row10 DIVERGENCE aliased h={h:02x?} -> C={c} Rust={r}");
        // With a single buffer the masked comparisons are trivially equal, so
        // the result is exactly hdr_valid(h).
        assert_eq!(c, model_valid(&h) as i32, "row10 model mismatch h={h:02x?}");
    }
}

/// Row 11: overlapping buffers taken from one random byte pool.
#[test]
fn row11_overlapping_buffers() {
    let im = load();
    let mut rng = Rng::new(SEED ^ 11);
    for _ in 0..2_000 {
        let mut pool = [0u8; 32];
        rng.fill(&mut pool);
        // sprinkle sync bytes so some windows are valid headers
        for i in 0..pool.len() {
            if rng.u8() & 3 == 0 {
                pool[i] = 0xFF;
            }
        }
        for i in 0..(pool.len() - 3) {
            for j in 0..(pool.len() - 3) {
                let (c, r) = im.both(unsafe { pool.as_ptr().add(i) }, unsafe {
                    pool.as_ptr().add(j)
                });
                assert_eq!(
                    c, r,
                    "row11 DIVERGENCE i={i} j={j} pool={pool:02x?} -> C={c} Rust={r}"
                );
            }
        }
    }
}

/// Row 12: the same logical headers at various (mis)alignments.
#[test]
fn row12_misaligned_pointers() {
    let im = load();
    let mut rng = Rng::new(SEED ^ 12);
    let mut buf1 = [0u8; 64];
    let mut buf2 = [0u8; 64];
    for _ in 0..2_000 {
        let mut h1 = [0u8; 4];
        let mut h2 = [0u8; 4];
        rng.fill(&mut h1);
        rng.fill(&mut h2);
        if rng.u8() & 1 == 1 {
            h2 = valid_h2();
            h2[2] = (h2[2] & 0xF0) | (rng.u8() & 0x0F);
            if (h2[2] >> 2) & 3 == 3 {
                h2[2] &= !0x0C;
            }
        }
        let mut baseline: Option<i32> = None;
        for &off in &[0usize, 1, 2, 3, 5, 7, 8, 13] {
            rng.fill(&mut buf1);
            rng.fill(&mut buf2);
            buf1[off..off + 4].copy_from_slice(&h1);
            buf2[off..off + 4].copy_from_slice(&h2);
            let (c, r) = im.both(unsafe { buf1.as_ptr().add(off) }, unsafe {
                buf2.as_ptr().add(off)
            });
            assert_eq!(c, r, "row12 DIVERGENCE off={off} h1={h1:02x?} h2={h2:02x?}");
            match baseline {
                None => baseline = Some(c),
                Some(b) => assert_eq!(b, c, "row12: alignment {off} changed the result"),
            }
        }
    }
}

/// Row 13: every invalid-`h2` family, with a matching `h1`.
#[test]
fn row13_invalid_h2_families() {
    let im = load();
    let mut rng = Rng::new(SEED ^ 13);

    // (a) h2[0] != 0xFF, everything else valid.
    for b0 in 0u16..=255 {
        let mut h2 = valid_h2();
        h2[0] = b0 as u8;
        let h1 = h2;
        let got = im.assert_eq_slices(&h1, &h2, "row13a/sync-byte0");
        assert_eq!(got, (b0 == 0xFF) as i32, "row13a b0={b0:#04x}");
    }
    // (b) h2[1] fails the sync gate.
    for b1 in 0u16..=255 {
        let mut h2 = valid_h2();
        h2[1] = b1 as u8;
        let h1 = h2;
        let got = im.assert_eq_slices(&h1, &h2, "row13b/sync-bits");
        assert_eq!(got, model_valid(&h2) as i32, "row13b b1={b1:#04x}");
    }
    // (c) layer code 0 (only reachable with sync-passing h2[1]).
    for b1 in [0xF0u8, 0xF1, 0xF8, 0xF9] {
        for _ in 0..64 {
            let mut h2 = valid_h2();
            h2[1] = b1;
            h2[2] = ((1 + rng.u8() % 14) << 4) | ((rng.u8() % 3) << 2) | (rng.u8() & 3);
            let h1 = h2;
            let got = im.assert_eq_slices(&h1, &h2, "row13c/layer0");
            assert_eq!(got, 0, "row13c b1={b1:#04x} h2={h2:02x?}");
        }
    }
    // (d) bitrate index 15.
    for low in 0u8..16 {
        let mut h2 = valid_h2();
        h2[2] = 0xF0 | low;
        let h1 = h2;
        let got = im.assert_eq_slices(&h1, &h2, "row13d/bitrate15");
        assert_eq!(got, 0, "row13d h2={h2:02x?}");
    }
    // (e) sample-rate index 3.
    for br in 0u8..=14 {
        for lo in 0u8..4 {
            let mut h2 = valid_h2();
            h2[2] = (br << 4) | 0x0C | lo;
            let h1 = h2;
            let got = im.assert_eq_slices(&h1, &h2, "row13e/srate3");
            assert_eq!(got, 0, "row13e h2={h2:02x?}");
        }
    }
}

/// Row 14: full cross-product of the three `h1`-vs-`h2` mismatch flags.
#[test]
fn row14_mismatch_flag_crossproduct() {
    let im = load();
    let mut rng = Rng::new(SEED ^ 14);
    for mism in 0u8..8 {
        let b1_mism = mism & 1 != 0;
        let srate_mism = mism & 2 != 0;
        let free_mism = mism & 4 != 0;
        for _ in 0..2_000 {
            let sync = [Sync::Mpeg1, Sync::Mpeg2, Sync::Mpeg25][rng.below(3) as usize];
            let layer = if sync == Sync::Mpeg25 { 1 } else { 1 + rng.u8() % 3 };
            let srate = rng.u8() % 3;
            // h2 bitrate: free when we need the "h2 free" side of a free mismatch
            let h2_free = free_mism && rng.u8() & 1 == 1;
            let br2 = if h2_free { 0 } else { 1 + rng.u8() % 14 };
            let h2 = make_hdr(sync, layer, br2, srate, false, false, true, 0);
            assert!(model_valid(&h2));

            let mut b1 = (h2[1] & 0xFE) | (rng.u8() & 1);
            if b1_mism {
                // flip a bit inside the 0xFE mask
                b1 ^= 2u8 << (rng.u8() % 7);
                if (b1 ^ h2[1]) & 0xFE == 0 {
                    b1 ^= 0x02;
                }
            }
            let mut srate1 = srate;
            if srate_mism {
                srate1 = (srate + 1 + rng.u8() % 3) & 3;
                if srate1 == srate {
                    srate1 = (srate + 1) & 3;
                }
            }
            let br1 = if free_mism {
                if h2_free { 1 + rng.u8() % 15 } else { 0 }
            } else if br2 == 0 {
                0
            } else {
                1 + rng.u8() % 15
            };
            let h1 = [rng.u8(), b1, (br1 << 4) | (srate1 << 2) | (rng.u8() & 3), rng.u8()];

            let expected = if mism == 0 { 1 } else { 0 };
            let got = im.assert_eq_slices(&h1, &h2, "row14/mismatch-flags");
            assert_eq!(
                got, expected,
                "row14 mism={mism:03b} h1={h1:02x?} h2={h2:02x?}"
            );
        }
    }
}

/// Row 21: realistic MP3 frame-header corpus, cross-multiplied.
#[test]
fn row21_frame_header_corpus() {
    let im = load();
    let mut corpus: Vec<[u8; 4]> = Vec::new();
    for sync in [Sync::Mpeg1, Sync::Mpeg2, Sync::Mpeg25] {
        for layer in 0u8..4 {
            if sync == Sync::Mpeg25 && layer != 1 {
                continue;
            }
            for bitrate in 0u8..16 {
                for srate in 0u8..4 {
                    for pad in [false, true] {
                        for priv_ in [false, true] {
                            for crc in [false, true] {
                                corpus.push(make_hdr(
                                    sync, layer, bitrate, srate, pad, priv_, crc, 0x00,
                                ));
                            }
                        }
                    }
                }
            }
        }
    }
    // also a few non-0xFF sync bytes
    let mut extra = Vec::new();
    for h in corpus.iter().take(64) {
        let mut x = *h;
        x[0] = 0x00;
        extra.push(x);
        let mut y = *h;
        y[0] = 0xFE;
        extra.push(y);
    }
    corpus.extend(extra);
    assert!(corpus.len() > 1000, "corpus too small: {}", corpus.len());

    let mut rng = Rng::new(SEED ^ 21);
    let n = corpus.len();
    // exhaustive over h2, sampled over h1 (full n^2 would be ~2.6M pairs; take
    // a deterministic stride plus random picks to keep it fast but broad)
    for (i, h2) in corpus.iter().enumerate() {
        for k in 0..24 {
            let j = (i * 7 + k * 53 + rng.below(n as u32) as usize) % n;
            let h1 = corpus[j];
            im.assert_eq_slices(&h1, h2, "row21/corpus");
        }
    }
}
