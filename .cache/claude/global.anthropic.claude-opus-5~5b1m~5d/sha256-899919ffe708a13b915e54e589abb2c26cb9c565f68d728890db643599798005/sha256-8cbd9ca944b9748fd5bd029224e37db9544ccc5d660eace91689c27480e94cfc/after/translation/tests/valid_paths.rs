//! Phase B — valid-path differential tests, one test per row of `CONFIGS.md`.
//!
//! Every assertion compares the C `.so` and the Rust `.so` through `dlopen` +
//! exported `hdr_compare`. Randomized rows use a fixed seed (`common::SEED`)
//! so failures are reproducible.

mod common;

use common::{make_byte1, make_byte2, valid_pair, Libs, Rng, SEED};

const ITERS: usize = 20_000;

/// Row 1 — fully matching pair, sync form-F, all valid sub-fields → expect 1.
#[test]
fn row01_matching_pair_form_f() {
    let libs = Libs::load();
    let mut rng = Rng::new(SEED ^ 1);
    for _ in 0..ITERS {
        let layer = rng.range(1, 3) as u8;
        let b1 = make_byte1(true, layer, rng.u8());
        let b2 = make_byte2(rng.range(1, 14) as u8, rng.range(0, 2) as u8, rng.u8());
        let h2 = [0xff, b1, b2, rng.u8()];
        let h1 = [rng.u8(), b1, b2, rng.u8()];
        assert_eq!(libs.diff(&h1, &h2), 1, "h1={h1:02x?} h2={h2:02x?}");
    }
}

/// Row 2 — fully matching pair, sync form-E (`h2[1] & 0xFE == 0xe2`) → expect 1.
#[test]
fn row02_matching_pair_form_e() {
    let libs = Libs::load();
    let mut rng = Rng::new(SEED ^ 2);
    for _ in 0..ITERS {
        let b1 = make_byte1(false, 1, rng.u8());
        assert_eq!(b1 & 0xFE, 0xe2);
        let b2 = make_byte2(rng.range(1, 14) as u8, rng.range(0, 2) as u8, rng.u8());
        let h2 = [0xff, b1, b2, rng.u8()];
        let h1 = [rng.u8(), b1, b2, rng.u8()];
        assert_eq!(libs.diff(&h1, &h2), 1, "h1={h1:02x?} h2={h2:02x?}");
    }
}

/// Rows 3, 4, 5 — each valid layer field value in isolation.
#[test]
fn row03_04_05_each_layer_field() {
    let libs = Libs::load();
    for layer in 1u8..=3 {
        let mut rng = Rng::new(SEED ^ (0x300 + u64::from(layer)));
        for _ in 0..ITERS {
            let b1 = make_byte1(true, layer, rng.u8());
            assert_ne!((b1 >> 1) & 3, 0);
            assert_eq!((b1 >> 1) & 3, layer);
            let b2 = make_byte2(rng.range(1, 14) as u8, rng.range(0, 2) as u8, rng.u8());
            let h2 = [0xff, b1, b2, rng.u8()];
            let h1 = [rng.u8(), b1, b2, rng.u8()];
            assert_eq!(libs.diff(&h1, &h2), 1, "layer={layer} h1={h1:02x?} h2={h2:02x?}");
        }
    }
}

/// Row 6 — axis D don't-care: `h1[1]` differs from `h2[1]` only in bit 0.
#[test]
fn row06_protection_bit_is_dont_care() {
    let libs = Libs::load();
    let mut rng = Rng::new(SEED ^ 6);
    for _ in 0..ITERS {
        let (mut h1, h2) = valid_pair(&mut rng);
        h1[1] = h2[1] ^ 0x01;
        assert_eq!(libs.diff(&h1, &h2), 1, "h1={h1:02x?} h2={h2:02x?}");
    }
}

/// Row 7 — axis G don't-care: `h1[2]`/`h2[2]` differ only in bits 1..0.
#[test]
fn row07_byte2_low_bits_are_dont_care() {
    let libs = Libs::load();
    let mut rng = Rng::new(SEED ^ 7);
    for _ in 0..ITERS {
        let layer = rng.range(1, 3) as u8;
        let b1 = make_byte1(true, layer, rng.u8());
        let bitrate = rng.range(1, 14) as u8; // non-free on both sides
        let srate = rng.range(0, 2) as u8;
        let b2a = make_byte2(bitrate, srate, rng.u8());
        let b2b = make_byte2(bitrate, srate, rng.u8());
        let h2 = [0xff, b1, b2b, rng.u8()];
        let h1 = [rng.u8(), b1, b2a, rng.u8()];
        assert_eq!(libs.diff(&h1, &h2), 1, "h1={h1:02x?} h2={h2:02x?}");
    }
}

/// Row 8 — axis K don't-care: `h1[0]` is never read; sweep all 256 values.
#[test]
fn row08_h1_byte0_is_dont_care() {
    let libs = Libs::load();
    let mut rng = Rng::new(SEED ^ 8);
    for _ in 0..2_000 {
        let (mut h1, h2) = valid_pair(&mut rng);
        let mut results = Vec::with_capacity(256);
        for v in 0u16..256 {
            h1[0] = v as u8;
            results.push(libs.diff(&h1, &h2));
        }
        assert!(
            results.windows(2).all(|w| w[0] == w[1]),
            "result depended on h1[0]: {results:?}"
        );
    }
}

/// Row 9 — axis J both-free branch.
#[test]
fn row09_both_free_bitrate() {
    let libs = Libs::load();
    let mut rng = Rng::new(SEED ^ 9);
    for _ in 0..ITERS {
        let b1 = make_byte1(rng.below(2) == 0, rng.range(1, 3) as u8, rng.u8());
        let srate = rng.range(0, 2) as u8;
        let b2a = make_byte2(0, srate, rng.u8());
        let b2b = make_byte2(0, srate, rng.u8());
        assert_eq!(b2a & 0xF0, 0);
        assert_eq!(b2b & 0xF0, 0);
        let h2 = [0xff, b1, b2b, rng.u8()];
        let h1 = [rng.u8(), b1, b2a, rng.u8()];
        assert_eq!(libs.diff(&h1, &h2), 1, "h1={h1:02x?} h2={h2:02x?}");
    }
}

/// Row 10 — axis J both non-free with DIFFERENT bitrate nibbles.
/// `h1[2] >> 4` may even be 15 — only `h2` is validated.
#[test]
fn row10_both_non_free_different_bitrates() {
    let libs = Libs::load();
    let mut rng = Rng::new(SEED ^ 10);
    for _ in 0..ITERS {
        let b1 = make_byte1(rng.below(2) == 0, rng.range(1, 3) as u8, rng.u8());
        let srate = rng.range(0, 2) as u8;
        let br_h2 = rng.range(1, 14) as u8;
        let mut br_h1 = rng.range(1, 15) as u8; // 1..=15, incl. the h2-invalid 15
        if br_h1 == br_h2 {
            br_h1 = if br_h2 == 1 { 2 } else { br_h2 - 1 };
        }
        let b2b = make_byte2(br_h2, srate, rng.u8());
        let b2a = (br_h1 << 4) | (srate << 2) | (rng.u8() & 3);
        let h2 = [0xff, b1, b2b, rng.u8()];
        let h1 = [rng.u8(), b1, b2a, rng.u8()];
        assert_eq!(libs.diff(&h1, &h2), 1, "h1={h1:02x?} h2={h2:02x?}");
    }
}

/// Row 11 — axis J mismatch: exactly one side is "free bitrate" → expect 0.
#[test]
fn row11_free_bitrate_mismatch() {
    let libs = Libs::load();
    let mut rng = Rng::new(SEED ^ 11);
    for _ in 0..ITERS {
        let b1 = make_byte1(rng.below(2) == 0, rng.range(1, 3) as u8, rng.u8());
        let srate = rng.range(0, 2) as u8;
        let h1_free = rng.below(2) == 0;
        let br_h1 = if h1_free { 0 } else { rng.range(1, 15) as u8 };
        let br_h2 = if h1_free { rng.range(1, 14) as u8 } else { 0 };
        let b2a = (br_h1 << 4) | (srate << 2) | (rng.u8() & 3);
        let b2b = (br_h2 << 4) | (srate << 2) | (rng.u8() & 3);
        let h2 = [0xff, b1, b2b, rng.u8()];
        let h1 = [rng.u8(), b1, b2a, rng.u8()];
        assert_eq!(libs.diff(&h1, &h2), 0, "h1={h1:02x?} h2={h2:02x?}");
    }
}

/// Row 12 — axis F: matching sample-rate index swept over all valid values.
#[test]
fn row12_each_sample_rate_index() {
    let libs = Libs::load();
    for srate in 0u8..=2 {
        let mut rng = Rng::new(SEED ^ (0x1200 + u64::from(srate)));
        for _ in 0..ITERS {
            let b1 = make_byte1(rng.below(2) == 0, rng.range(1, 3) as u8, rng.u8());
            let b2b = make_byte2(rng.range(1, 14) as u8, srate, rng.u8());
            let b2a = make_byte2(rng.range(1, 14) as u8, srate, rng.u8());
            let h2 = [0xff, b1, b2b, rng.u8()];
            let h1 = [rng.u8(), b1, b2a, rng.u8()];
            assert_eq!(libs.diff(&h1, &h2), 1, "srate={srate} h1={h1:02x?} h2={h2:02x?}");
        }
    }
}

/// Row 13 — axis E boundary: max valid bitrate nibble 14 on both sides.
#[test]
fn row13_max_valid_bitrate_nibble() {
    let libs = Libs::load();
    let mut rng = Rng::new(SEED ^ 13);
    for _ in 0..ITERS {
        let b1 = make_byte1(rng.below(2) == 0, rng.range(1, 3) as u8, rng.u8());
        let srate = rng.range(0, 2) as u8;
        let b2 = make_byte2(14, srate, rng.u8());
        assert_eq!(b2 >> 4, 14);
        let h2 = [0xff, b1, b2, rng.u8()];
        let h1 = [rng.u8(), b1, make_byte2(14, srate, rng.u8()), rng.u8()];
        assert_eq!(libs.diff(&h1, &h2), 1, "h1={h1:02x?} h2={h2:02x?}");
    }
}

/// Row 14 — axis L: buffers allocated to EXACTLY 3 bytes (no readable byte 3).
#[test]
fn row14_exact_three_byte_buffers() {
    let libs = Libs::load();
    let mut rng = Rng::new(SEED ^ 14);
    for _ in 0..ITERS {
        // Mix of valid pairs and pure noise.
        let (h1, h2) = if rng.below(2) == 0 {
            valid_pair(&mut rng)
        } else {
            let mut a = [0u8; 4];
            let mut b = [0u8; 4];
            rng.fill(&mut a);
            rng.fill(&mut b);
            if rng.below(2) == 0 {
                b[0] = 0xff;
            }
            (a, b)
        };
        libs.diff_exact(&h1[..3], &h2[..3]);
    }
}

/// Row 15 — axis L: 16-byte buffers with garbage tails must give the same
/// answer as the 3-byte-exact buffers.
#[test]
fn row15_garbage_tail_does_not_change_result() {
    let libs = Libs::load();
    let mut rng = Rng::new(SEED ^ 15);
    for _ in 0..ITERS {
        let mut a = [0u8; 16];
        let mut b = [0u8; 16];
        rng.fill(&mut a);
        rng.fill(&mut b);
        if rng.below(2) == 0 {
            b[0] = 0xff;
        }

        let long = libs.diff(&a, &b);
        let short = libs.diff_exact(&a[..3], &b[..3]);
        assert_eq!(
            long, short,
            "tail bytes changed the result: a={a:02x?} b={b:02x?}"
        );

        // Also: 0xFF-saturated tails.
        let mut a2 = a;
        let mut b2 = b;
        for i in 3..16 {
            a2[i] = 0xff;
            b2[i] = 0xff;
        }
        assert_eq!(libs.diff(&a2, &b2), long);
    }
}

/// Row 16 — unconstrained fuzz over all read bytes (valid and invalid mixed).
#[test]
fn row16_unconstrained_fuzz() {
    let libs = Libs::load();
    let mut rng = Rng::new(SEED ^ 16);
    let mut ones = 0usize;
    for _ in 0..500_000 {
        let mut a = [0u8; 4];
        let mut b = [0u8; 4];
        rng.fill(&mut a);
        rng.fill(&mut b);
        // Bias a third of the cases towards a valid sync byte so the deeper
        // conjuncts are actually reached.
        if rng.below(3) == 0 {
            b[0] = 0xff;
            b[1] |= 0xF0;
        }
        if libs.diff(&a, &b) == 1 {
            ones += 1;
        }
    }
    assert!(ones > 0, "fuzz never produced a match — corpus is degenerate");
}

/// Row 17 — exhaustive over `(h1[1], h2[1])` (65 536 pairs) crossed with a
/// representative set of `(h1[2], h2[2])`.
#[test]
fn row17_exhaustive_byte1_pairs() {
    let libs = Libs::load();
    // Representative byte-2 pairs: equal-valid, srate-mismatch, free-mismatch,
    // bitrate-15, srate-3.
    let byte2_pairs: [(u8, u8); 6] = [
        (0x90, 0x90), // both non-free, srate 0
        (0x94, 0x90), // srate differs
        (0x00, 0x90), // free vs non-free
        (0x00, 0x00), // both free
        (0xF0, 0xF0), // h2 bitrate nibble 15 -> invalid
        (0x9C, 0x9C), // srate index 3 -> invalid
    ];
    for (a2, b2) in byte2_pairs {
        for a1 in 0u16..256 {
            for b1 in 0u16..256 {
                let h1 = [0x5a, a1 as u8, a2, 0xaa];
                let h2 = [0xff, b1 as u8, b2, 0x55];
                libs.diff(&h1, &h2);
            }
        }
    }
}

/// Row 18 — exhaustive over `(h1[2], h2[2])` (65 536 pairs) for every value of
/// `h2[1]` with `h1[1] == h2[1]` (256 × 65 536 calls per implementation).
#[test]
fn row18_exhaustive_byte2_pairs_all_byte1() {
    let libs = Libs::load();
    for b1 in 0u16..256 {
        let b1 = b1 as u8;
        for a2 in 0u16..256 {
            for b2 in 0u16..256 {
                let h1 = [0x00, b1, a2 as u8, 0xff];
                let h2 = [0xff, b1, b2 as u8, 0x00];
                libs.diff(&h1, &h2);
            }
        }
    }
}

/// Row 19 — exhaustive over `h2[0]` (axis A) crossed with a valid remainder.
#[test]
fn row19_exhaustive_h2_byte0() {
    let libs = Libs::load();
    let mut rng = Rng::new(SEED ^ 19);
    for _ in 0..64 {
        let (h1, mut h2) = valid_pair(&mut rng);
        for v in 0u16..256 {
            h2[0] = v as u8;
            let got = libs.diff(&h1, &h2);
            let expect = if v == 0xff { 1 } else { 0 };
            assert_eq!(got, expect, "h2[0]={v:#04x} h1={h1:02x?} h2={h2:02x?}");
        }
    }
}

/// Row 20 — realistic MPEG-audio frame-header corpus, all ordered pairs.
#[test]
fn row20_realistic_header_corpus() {
    let libs = Libs::load();
    let corpus: [[u8; 4]; 14] = [
        [0xFF, 0xFB, 0x90, 0x64], // MPEG1 Layer3, 128 kbps, 44.1 kHz
        [0xFF, 0xFB, 0x90, 0x00],
        [0xFF, 0xFB, 0x94, 0x64], // 48 kHz
        [0xFF, 0xFB, 0x98, 0x64], // 32 kHz
        [0xFF, 0xFA, 0x90, 0x64], // protection bit cleared
        [0xFF, 0xFB, 0x00, 0x64], // free bitrate
        [0xFF, 0xFB, 0xE0, 0x64], // bitrate index 14
        [0xFF, 0xFB, 0xF0, 0x64], // bitrate index 15 (invalid)
        [0xFF, 0xFB, 0x9C, 0x64], // srate index 3 (invalid)
        [0xFF, 0xFD, 0x90, 0x64], // Layer 2
        [0xFF, 0xFF, 0x90, 0x64], // Layer 3 field = 3
        [0xFF, 0xF1, 0x90, 0x64], // layer field 0 (invalid)
        [0xFF, 0xE3, 0x90, 0x64], // form-E
        [0xFF, 0xE2, 0x90, 0x64], // form-E, protection cleared
    ];
    for a in &corpus {
        for b in &corpus {
            libs.diff(a, b);
            libs.diff_exact(&a[..3], &b[..3]);
        }
    }
    // Self-comparison of every valid corpus member must yield 1.
    for a in &corpus {
        let self_cmp = libs.diff(a, a);
        // Only headers that pass hdr_valid can self-match.
        let valid = a[0] == 0xff
            && ((a[1] & 0xF0) == 0xf0 || (a[1] & 0xFE) == 0xe2)
            && ((a[1] >> 1) & 3) != 0
            && (a[2] >> 4) != 15
            && ((a[2] >> 2) & 3) != 3;
        assert_eq!(self_cmp, i32::from(valid), "self-compare {a:02x?}");
    }
}

/// Sanity: both `.so` files really are two distinct files and both export the
/// symbol (guards against accidentally testing one library against itself).
#[test]
fn harness_loads_two_distinct_libraries() {
    let libs = Libs::load();
    assert_ne!(libs.c_path, libs.rs_path);
    assert!(libs.c_path.to_string_lossy().contains("c_src"));
    assert!(libs
        .rs_path
        .file_name()
        .unwrap()
        .to_string_lossy()
        .starts_with("libhdr_compare_lib"));
}

/// Strengthening sweep (backs the factorization argument in `CONFIGS.md`):
/// `hdr_compare` reads exactly 5 bytes, and with `h2[0]` fixed to `0xff` the
/// result factors into `A(h1[1], h2[1]) && B(h1[2], h2[2])`. Rows 17-19 cover
/// each factor exhaustively; this test additionally hammers the *joint*
/// 4-byte space with 20 million randomized draws so the factorization itself
/// is never assumed.
#[test]
fn row_joint_fourbyte_highvolume_sweep() {
    let libs = Libs::load();
    let mut rng = Rng::new(SEED ^ 0xF00D);
    let mut ones = 0usize;
    let mut zeros = 0usize;
    for _ in 0..20_000_000u32 {
        let w = rng.next_u64();
        let h1 = [(w >> 32) as u8, (w >> 8) as u8, (w >> 16) as u8, 0x00];
        let h2 = [0xffu8, (w >> 24) as u8, w as u8, 0x00];
        match libs.diff(&h1, &h2) {
            0 => zeros += 1,
            1 => ones += 1,
            v => panic!("unexpected return {v}"),
        }
    }
    assert!(ones > 1_000 && zeros > 1_000, "degenerate sweep: {ones} ones / {zeros} zeros");
}
