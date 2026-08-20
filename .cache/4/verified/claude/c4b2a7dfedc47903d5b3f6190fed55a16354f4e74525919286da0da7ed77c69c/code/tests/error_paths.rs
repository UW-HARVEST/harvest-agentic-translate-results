//! Phase C — error-path differential tests.
//!
//! One test per row of `ERRORS.md` (rows 1..10 and 16; rows 11..15 are the
//! read-extent contract and live in `read_extent.rs`, which needs guard pages
//! and a child process).
//!
//! The library has no error codes: its only rejection signal is the `int 0`
//! returned by `hdr_compare`. Each test therefore asserts *both* that C and
//! Rust agree **and** that the shared answer is exactly `0` (the rejection
//! sentinel) — never `1`, and never some other integer.

mod common;

use common::*;

/// Row 1: `h2[0] != 0xff` (all 255 non-`0xff` values), rest of `h2` valid.
#[test]
fn row01_h2_sync_byte0_invalid() {
    let im = load();
    let mut rng = Rng::new(SEED ^ 101);
    let mut rejected = 0;
    for b0 in 0u16..=255 {
        for _ in 0..32 {
            let sync = [Sync::Mpeg1, Sync::Mpeg2, Sync::Mpeg25][rng.below(3) as usize];
            let layer = if sync == Sync::Mpeg25 { 1 } else { 1 + rng.u8() % 3 };
            let mut h2 = make_hdr(
                sync,
                layer,
                1 + rng.u8() % 14,
                rng.u8() % 3,
                rng.u8() & 1 == 1,
                rng.u8() & 1 == 1,
                rng.u8() & 1 == 1,
                rng.u8(),
            );
            h2[0] = b0 as u8;
            let h1 = h2; // identical, so only hdr_valid can reject
            let got = im.assert_eq_slices(&h1, &h2, "err01/h2[0]");
            if b0 == 0xFF {
                assert_eq!(got, 1, "err01: 0xff must be accepted, h2={h2:02x?}");
            } else {
                assert_eq!(got, 0, "err01: b0={b0:#04x} must be rejected, h2={h2:02x?}");
                rejected += 1;
            }
        }
    }
    assert_eq!(rejected, 255 * 32);
}

/// Row 2: `h2[1]` passes neither `(x & 0xF0) == 0xF0` nor `(x & 0xFE) == 0xE2`.
#[test]
fn row02_h2_sync_bits_invalid() {
    let im = load();
    let mut rng = Rng::new(SEED ^ 102);
    let mut rejected = 0usize;
    for b1 in 0u16..=255 {
        let b1 = b1 as u8;
        for _ in 0..32 {
            // h2[2] always valid so that h2[1] is the sole reason to reject.
            let h2 = [
                0xFF,
                b1,
                ((1 + rng.u8() % 14) << 4) | ((rng.u8() % 3) << 2) | (rng.u8() & 3),
                rng.u8(),
            ];
            let h1 = h2;
            let got = im.assert_eq_slices(&h1, &h2, "err02/h2[1]-sync");
            if sync_ok(b1) {
                // layer code 0 is a *different* row; skip its expectation here
                if ((b1 >> 1) & 3) != 0 {
                    assert_eq!(got, 1, "err02: {b1:#04x} should pass, h2={h2:02x?}");
                }
            } else {
                assert_eq!(got, 0, "err02: {b1:#04x} must be rejected, h2={h2:02x?}");
                rejected += 1;
            }
        }
    }
    // 256 - 18 sync-passing values = 238 rejected byte values
    assert_eq!(rejected, 238 * 32, "expected 238 rejecting h2[1] values");
}

/// Row 3: reserved layer code 0 — the only sync-passing values with layer 0
/// are 0xF0, 0xF1, 0xF8, 0xF9.
#[test]
fn row03_h2_layer_reserved() {
    let im = load();
    let mut rng = Rng::new(SEED ^ 103);
    let layer0: Vec<u8> = sync_passing_b1()
        .into_iter()
        .filter(|&b| ((b >> 1) & 3) == 0)
        .collect();
    assert_eq!(layer0, vec![0xF0, 0xF1, 0xF8, 0xF9], "layer-0 set");
    for &b1 in &layer0 {
        for br in 0u8..=14 {
            for srate in 0u8..=2 {
                for lo in 0u8..4 {
                    let h2 = [0xFF, b1, (br << 4) | (srate << 2) | lo, rng.u8()];
                    let h1 = h2;
                    let got = im.assert_eq_slices(&h1, &h2, "err03/layer0");
                    assert_eq!(got, 0, "err03: h2={h2:02x?} must be rejected");
                }
            }
        }
    }
}

/// Row 4: reserved bitrate index 15 (`(h2[2] >> 4) == 15`).
#[test]
fn row04_h2_bitrate_index_15() {
    let im = load();
    let mut rng = Rng::new(SEED ^ 104);
    for &b1 in &sync_passing_b1() {
        if ((b1 >> 1) & 3) == 0 {
            continue; // layer-0 is row 3's business
        }
        for lo in 0u8..16 {
            let h2 = [0xFF, b1, 0xF0 | lo, rng.u8()];
            assert_eq!(h2[2] >> 4, 15);
            let h1 = h2;
            let got = im.assert_eq_slices(&h1, &h2, "err04/bitrate15");
            assert_eq!(got, 0, "err04: h2={h2:02x?} must be rejected");
        }
    }
}

/// Row 5: reserved sample-rate index 3 (`((h2[2] >> 2) & 3) == 3`).
#[test]
fn row05_h2_samplerate_index_3() {
    let im = load();
    let mut rng = Rng::new(SEED ^ 105);
    for &b1 in &sync_passing_b1() {
        if ((b1 >> 1) & 3) == 0 {
            continue;
        }
        for br in 0u8..=14 {
            for lo in 0u8..4 {
                let h2 = [0xFF, b1, (br << 4) | 0x0C | lo, rng.u8()];
                assert_eq!((h2[2] >> 2) & 3, 3);
                let h1 = h2;
                let got = im.assert_eq_slices(&h1, &h2, "err05/srate3");
                assert_eq!(got, 0, "err05: h2={h2:02x?} must be rejected");
            }
        }
    }
}

/// Row 6: `((h1[1] ^ h2[1]) & 0xFE) != 0` — every one of the 254 deltas.
#[test]
fn row06_h1_version_layer_mismatch() {
    let im = load();
    let mut rng = Rng::new(SEED ^ 106);
    for m in 0u16..=255 {
        let m = m as u8;
        for _ in 0..8 {
            let sync = [Sync::Mpeg1, Sync::Mpeg2, Sync::Mpeg25][rng.below(3) as usize];
            let layer = if sync == Sync::Mpeg25 { 1 } else { 1 + rng.u8() % 3 };
            let srate = rng.u8() % 3;
            let br = 1 + rng.u8() % 14;
            let h2 = make_hdr(sync, layer, br, srate, false, false, rng.u8() & 1 == 1, 0);
            assert!(model_valid(&h2));
            // h1 agrees on everything except h1[1] ^= m
            let h1 = [rng.u8(), h2[1] ^ m, (br << 4) | (srate << 2) | (rng.u8() & 3), rng.u8()];
            let got = im.assert_eq_slices(&h1, &h2, "err06/b1-mismatch");
            if m & 0xFE == 0 {
                assert_eq!(got, 1, "err06: m={m:#04x} is masked out, h1={h1:02x?}");
            } else {
                assert_eq!(got, 0, "err06: m={m:#04x} must be rejected, h1={h1:02x?} h2={h2:02x?}");
            }
        }
    }
}

/// Row 7: `((h1[2] ^ h2[2]) & 0x0C) != 0` — all three non-zero deltas.
#[test]
fn row07_h1_samplerate_mismatch() {
    let im = load();
    let mut rng = Rng::new(SEED ^ 107);
    for delta in [0x04u8, 0x08, 0x0C] {
        for _ in 0..4_000 {
            let sync = [Sync::Mpeg1, Sync::Mpeg2, Sync::Mpeg25][rng.below(3) as usize];
            let layer = if sync == Sync::Mpeg25 { 1 } else { 1 + rng.u8() % 3 };
            let srate = rng.u8() % 3;
            let br = 1 + rng.u8() % 14;
            let h2 = make_hdr(sync, layer, br, srate, false, false, true, 0);
            assert!(model_valid(&h2));
            // keep bitrate non-zero on both sides so only the srate bits differ
            let h1 = [rng.u8(), (h2[1] & 0xFE) | (rng.u8() & 1), h2[2] ^ delta, rng.u8()];
            assert_ne!((h1[2] ^ h2[2]) & 0x0C, 0);
            assert_ne!(h1[2] & 0xF0, 0);
            let got = im.assert_eq_slices(&h1, &h2, "err07/srate-mismatch");
            assert_eq!(got, 0, "err07: delta={delta:#04x} h1={h1:02x?} h2={h2:02x?}");
        }
    }
}

/// Row 8: free-format mismatch, `h2` non-free and `h1` free.
#[test]
fn row08_free_format_mismatch_a() {
    let im = load();
    let mut rng = Rng::new(SEED ^ 108);
    for br2 in 1u8..=14 {
        for lo in 0u8..4 {
            for _ in 0..16 {
                let sync = [Sync::Mpeg1, Sync::Mpeg2, Sync::Mpeg25][rng.below(3) as usize];
                let layer = if sync == Sync::Mpeg25 { 1 } else { 1 + rng.u8() % 3 };
                let srate = rng.u8() % 3;
                let h2 = make_hdr(sync, layer, br2, srate, false, false, true, 0);
                assert!(model_valid(&h2));
                let h1 = [rng.u8(), (h2[1] & 0xFE) | (rng.u8() & 1), (srate << 2) | lo, rng.u8()];
                assert_eq!(h1[2] & 0xF0, 0);
                assert_ne!(h2[2] & 0xF0, 0);
                let got = im.assert_eq_slices(&h1, &h2, "err08/free-mismatch-a");
                assert_eq!(got, 0, "err08: h1={h1:02x?} h2={h2:02x?}");
            }
        }
    }
}

/// Row 9: free-format mismatch, `h2` free and `h1` non-free.
#[test]
fn row09_free_format_mismatch_b() {
    let im = load();
    let mut rng = Rng::new(SEED ^ 109);
    for br1 in 1u8..=15 {
        for lo in 0u8..4 {
            for _ in 0..16 {
                let sync = [Sync::Mpeg1, Sync::Mpeg2, Sync::Mpeg25][rng.below(3) as usize];
                let layer = if sync == Sync::Mpeg25 { 1 } else { 1 + rng.u8() % 3 };
                let srate = rng.u8() % 3;
                let h2 = make_hdr(sync, layer, 0, srate, false, false, true, 0);
                assert!(model_valid(&h2));
                assert_eq!(h2[2] & 0xF0, 0);
                let h1 = [
                    rng.u8(),
                    (h2[1] & 0xFE) | (rng.u8() & 1),
                    (br1 << 4) | (srate << 2) | lo,
                    rng.u8(),
                ];
                let got = im.assert_eq_slices(&h1, &h2, "err09/free-mismatch-b");
                assert_eq!(got, 0, "err09: h1={h1:02x?} h2={h2:02x?}");
            }
        }
    }
}

/// Row 10: each invalid-`h2` family combined with a simultaneously mismatching
/// `h1` — the first failing gate must short-circuit and the result must still
/// be exactly `0`.
#[test]
fn row10_combined_failures() {
    let im = load();
    let mut rng = Rng::new(SEED ^ 110);
    // family selectors: 0=byte0, 1=sync bits, 2=layer0, 3=bitrate15, 4=srate3
    for family in 0u8..5 {
        for _ in 0..8_000 {
            let mut h2 = valid_h2();
            match family {
                0 => h2[0] = loop {
                    let v = rng.u8();
                    if v != 0xFF {
                        break v;
                    }
                },
                1 => {
                    h2[1] = loop {
                        let v = rng.u8();
                        if !sync_ok(v) {
                            break v;
                        }
                    }
                }
                2 => h2[1] = [0xF0u8, 0xF1, 0xF8, 0xF9][rng.below(4) as usize],
                3 => h2[2] = 0xF0 | (rng.u8() & 0x0F),
                _ => h2[2] = (rng.u8() & 0xF0) | 0x0C | (rng.u8() & 3),
            }
            if family == 3 {
                // make sure it is the bitrate, not the srate, that is reserved
                if (h2[2] >> 2) & 3 == 3 {
                    h2[2] &= !0x08;
                }
            }
            if family == 4 && (h2[2] >> 4) == 15 {
                h2[2] &= 0x7F;
            }
            assert!(!model_valid(&h2), "row10 family={family} h2={h2:02x?} should be invalid");
            // h1 mismatches too, in every way at once
            let mut h1 = [0u8; 4];
            rng.fill(&mut h1);
            h1[1] = h2[1] ^ 0x02; // masked-in delta
            h1[2] = (h2[2] ^ 0x04) & 0x0F; // srate delta + free-format flip
            let got = im.assert_eq_slices(&h1, &h2, "err10/combined");
            assert_eq!(got, 0, "err10 family={family}: h1={h1:02x?} h2={h2:02x?}");
        }
    }
}

/// Row 16: out-of-range / reserved field encodings across the FFI boundary.
///
/// C `enum`-like bit fields accept any value, so every reserved code (layer 0,
/// bitrate 15, sample-rate 3) and every byte value must be handled identically.
/// Each byte position is swept over its complete `0..=255` domain.
#[test]
fn row16_all_reserved_encodings_exhaustive() {
    let im = load();
    let base_h2 = valid_h2();

    // sweep h2[0]
    for v in 0u16..=255 {
        let mut h2 = base_h2;
        h2[0] = v as u8;
        for h1 in [h2, base_h2, [0, 0, 0, 0], [0xFF, 0xFF, 0xFF, 0xFF]] {
            im.assert_eq_slices(&h1, &h2, "err16/h2[0]");
        }
    }
    // sweep h2[1]
    for v in 0u16..=255 {
        let mut h2 = base_h2;
        h2[1] = v as u8;
        for h1 in [h2, base_h2, [0, 0, 0, 0], [0xFF, 0xFF, 0xFF, 0xFF]] {
            im.assert_eq_slices(&h1, &h2, "err16/h2[1]");
        }
    }
    // sweep h2[2]
    for v in 0u16..=255 {
        let mut h2 = base_h2;
        h2[2] = v as u8;
        for h1 in [h2, base_h2, [0, 0, 0, 0], [0xFF, 0xFF, 0xFF, 0xFF]] {
            im.assert_eq_slices(&h1, &h2, "err16/h2[2]");
        }
    }
    // sweep h1[0], h1[1], h1[2], h1[3] against several h2
    let h2_set = [
        base_h2,
        [0xFF, 0xE2, 0x10, 0x00], // MPEG2.5
        [0xFF, 0xF3, 0x00, 0x00], // free format
        [0xFF, 0xF0, 0x90, 0x00], // reserved layer -> invalid
        [0xFF, 0xFB, 0xF0, 0x00], // reserved bitrate -> invalid
        [0xFF, 0xFB, 0x9C, 0x00], // reserved srate -> invalid
        [0x00, 0xFB, 0x90, 0x00], // bad sync byte -> invalid
    ];
    for h2 in h2_set {
        for pos in 0usize..4 {
            for v in 0u16..=255 {
                let mut h1 = base_h2;
                h1[pos] = v as u8;
                im.assert_eq_slices(&h1, &h2, "err16/h1-sweep");
            }
        }
    }
}

/// Generic boundary: values one step past every documented valid range.
#[test]
fn boundary_one_past_valid_ranges() {
    let im = load();
    // bitrate index: valid 0..=14, one past = 15
    // sample-rate index: valid 0..=2, one past = 3
    // layer code: valid 1..=3, one below = 0
    for (label, h2) in [
        ("bitrate 14 (last valid)", [0xFFu8, 0xFB, 0xE0, 0x00]),
        ("bitrate 15 (one past)", [0xFF, 0xFB, 0xF0, 0x00]),
        ("srate 2 (last valid)", [0xFF, 0xFB, 0x98, 0x00]),
        ("srate 3 (one past)", [0xFF, 0xFB, 0x9C, 0x00]),
        ("layer 1 (lowest valid)", [0xFF, 0xF3, 0x90, 0x00]),
        ("layer 0 (one below)", [0xFF, 0xF1, 0x90, 0x00]),
        ("sync 0xF0 boundary", [0xFF, 0xF2, 0x90, 0x00]),
        ("sync 0xEF (one below 0xF0)", [0xFF, 0xEF, 0x90, 0x00]),
        ("sync 0xE1 (one below 0xE2)", [0xFF, 0xE1, 0x90, 0x00]),
        ("sync 0xE3 (top of 0xE2 pair)", [0xFF, 0xE3, 0x90, 0x00]),
        ("sync 0xE4 (one past 0xE3)", [0xFF, 0xE4, 0x90, 0x00]),
        ("sync byte 0xFE (one below 0xFF)", [0xFE, 0xFB, 0x90, 0x00]),
    ] {
        let got = im.assert_eq_slices(&h2, &h2, label);
        assert_eq!(
            got,
            model_valid(&h2) as i32,
            "boundary {label}: h2={h2:02x?} got={got}"
        );
    }
}
