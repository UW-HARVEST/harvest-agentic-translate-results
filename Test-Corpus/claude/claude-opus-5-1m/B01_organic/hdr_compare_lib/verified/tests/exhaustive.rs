//! Phase B — exhaustive / large randomized sweeps (`CONFIGS.md` rows 15..19).
//!
//! `hdr_compare`'s result depends on exactly five bytes: `h1[1]`, `h1[2]`,
//! `h2[0]`, `h2[1]`, `h2[2]`. These tests enumerate that space directly:
//! complete 1-byte, 2-byte and 3-byte cross-products, plus a multi-million
//! sample randomized sweep over all five bytes at once.

mod common;

use common::*;

/// Row 15: complete sweep of `h2[0]` (256 values) × a range of valid tails.
#[test]
fn row15_exhaustive_h2_byte0() {
    let im = load();
    for b0 in 0u16..=255 {
        for &b1 in &sync_passing_b1() {
            for br in [0u8, 1, 7, 14, 15] {
                for srate in 0u8..4 {
                    let h2 = [b0 as u8, b1, (br << 4) | (srate << 2), 0x00];
                    let got = im.assert_eq_slices(&h2, &h2, "row15");
                    assert_eq!(got, model_valid(&h2) as i32, "row15 h2={h2:02x?}");
                }
            }
        }
    }
}

/// Row 16: complete 2-byte cross-product `h2[1] × h2[2]` with `h1 == h2`,
/// and again with several fixed `h1` values.
#[test]
fn row16_exhaustive_h2_bytes12() {
    let im = load();
    let h1_fixed: [[u8; 4]; 6] = [
        [0x00, 0x00, 0x00, 0x00],
        [0xFF, 0xFF, 0xFF, 0xFF],
        valid_h2(),
        [0xFF, 0xE2, 0x00, 0x00],
        [0x37, 0xFB, 0x9F, 0x11],
        [0xFF, 0xF3, 0xF3, 0x00],
    ];
    for b1 in 0u16..=255 {
        for b2 in 0u16..=255 {
            let h2 = [0xFFu8, b1 as u8, b2 as u8, 0x5A];
            // h1 == h2 (result is exactly hdr_valid)
            let got = im.assert_eq_slices(&h2, &h2, "row16/identical");
            assert_eq!(got, model_valid(&h2) as i32, "row16 h2={h2:02x?}");
            for h1 in &h1_fixed {
                im.assert_eq_slices(h1, &h2, "row16/fixed-h1");
            }
        }
    }
}

/// Row 17: complete 2-byte cross-product `h1[1] × h1[2]` for 12 fixed `h2`.
#[test]
fn row17_exhaustive_h1_bytes12() {
    let im = load();
    let h2_set: [[u8; 4]; 12] = [
        valid_h2(),                // FF FB 90 00 MPEG1 L3 br9 sr0
        [0xFF, 0xFA, 0x90, 0x00],  // CRC bit clear
        [0xFF, 0xF3, 0x00, 0x00],  // MPEG2 L3, free format
        [0xFF, 0xF2, 0xE4, 0x03],  // MPEG2 L3, bitrate 14, srate 1
        [0xFF, 0xE2, 0x18, 0x00],  // MPEG2.5, bitrate 1, srate 2
        [0xFF, 0xE3, 0x08, 0x02],  // MPEG2.5, free format, srate 2
        [0xFF, 0xFD, 0x14, 0x00],  // MPEG1 L2
        [0xFF, 0xFF, 0x94, 0x00],  // MPEG1 L1
        [0xFF, 0xF0, 0x90, 0x00],  // reserved layer -> invalid
        [0xFF, 0xFB, 0xF4, 0x00],  // reserved bitrate -> invalid
        [0xFF, 0xFB, 0x9C, 0x00],  // reserved srate -> invalid
        [0x00, 0xFB, 0x90, 0x00],  // bad sync byte -> invalid
    ];
    for h2 in &h2_set {
        for b1 in 0u16..=255 {
            for b2 in 0u16..=255 {
                let h1 = [0xA5u8, b1 as u8, b2 as u8, 0x3C];
                im.assert_eq_slices(&h1, h2, "row17");
            }
        }
    }
}

/// Row 18: complete 3-byte cross-products.
///
/// `h2[1] × h2[2] × h1[2]` for several fixed `h1[1]`, and
/// `h2[1] × h2[2] × h1[1]` for several fixed `h1[2]`  (2 × 3 × 16.7M calls).
#[test]
fn row18_exhaustive_three_bytes() {
    let im = load();

    for h1b1 in [0x00u8, 0xFB, 0xE2] {
        for b1 in 0u16..=255 {
            for b2 in 0u16..=255 {
                let h2 = [0xFFu8, b1 as u8, b2 as u8, 0x00];
                for h1b2 in 0u16..=255 {
                    let h1 = [0x00u8, h1b1, h1b2 as u8, 0x00];
                    let (c, r) = im.both(h1.as_ptr(), h2.as_ptr());
                    if c != r {
                        panic!("row18a DIVERGENCE h1={h1:02x?} h2={h2:02x?} C={c} Rust={r}");
                    }
                }
            }
        }
    }

    for h1b2 in [0x00u8, 0x90, 0xF4] {
        for b1 in 0u16..=255 {
            for b2 in 0u16..=255 {
                let h2 = [0xFFu8, b1 as u8, b2 as u8, 0x00];
                for h1b1 in 0u16..=255 {
                    let h1 = [0x00u8, h1b1 as u8, h1b2, 0x00];
                    let (c, r) = im.both(h1.as_ptr(), h2.as_ptr());
                    if c != r {
                        panic!("row18b DIVERGENCE h1={h1:02x?} h2={h2:02x?} C={c} Rust={r}");
                    }
                }
            }
        }
    }
}

/// Row 19: randomized sweep of every read-relevant byte at once.
///
/// ~50 % of the samples are forced to be structurally valid `h2` headers so
/// that the post-`hdr_valid` gates are reached often; the rest are fully
/// random (which also covers the `h2[0] != 0xff` fast-reject path).
#[test]
fn row19_randomized_full_sweep() {
    let im = load();
    let mut rng = Rng::new(SEED);
    const N: usize = 2_000_000;
    let mut valid_h2_seen = 0usize;
    let mut ones = 0usize;
    for i in 0..N {
        let mut h1 = [0u8; 8];
        let mut h2 = [0u8; 8];
        rng.fill(&mut h1);
        rng.fill(&mut h2);
        match i % 4 {
            0 => { /* fully random */ }
            1 => {
                // structurally valid h2
                h2[0] = 0xFF;
                h2[1] = *[0xFBu8, 0xFA, 0xF3, 0xF2, 0xFD, 0xFF, 0xE2, 0xE3]
                    .get(rng.below(8) as usize)
                    .unwrap();
                h2[2] = ((rng.u8() % 15) << 4) | ((rng.u8() % 3) << 2) | (rng.u8() & 3);
            }
            2 => {
                // valid h2 plus an h1 that agrees under the masks most of the time
                h2[0] = 0xFF;
                h2[1] = 0xF0 | (rng.u8() & 0x0F);
                h2[2] = rng.u8();
                h1[1] = h2[1] ^ (rng.u8() & 0x03);
                h1[2] = h2[2] ^ (rng.u8() & 0x0F);
            }
            _ => {
                // near-miss sync bytes
                h2[0] = if rng.u8() & 1 == 1 { 0xFF } else { 0xFE };
                h2[1] = 0xE0 | (rng.u8() & 0x1F);
                h2[2] = rng.u8();
                h1[1] = h2[1];
                h1[2] = h2[2] ^ (rng.u8() & 0x1F);
            }
        }
        if model_valid(&h2) {
            valid_h2_seen += 1;
        }
        let (c, r) = im.both(h1.as_ptr(), h2.as_ptr());
        if c != r {
            panic!("row19 DIVERGENCE #{i} h1={h1:02x?} h2={h2:02x?} C={c} Rust={r}");
        }
        if c == 1 {
            ones += 1;
        }
    }
    // Sanity: the sweep must actually reach both outcomes in bulk.
    assert!(valid_h2_seen > N / 10, "only {valid_h2_seen} valid h2 out of {N}");
    assert!(ones > N / 100, "only {ones} accepted results out of {N}");
    assert!(ones < N - N / 100, "only {} rejected results out of {N}", N - ones);
}

/// Row 22 — **complete** enumeration of the entire input space that can affect
/// the result.
///
/// `hdr_compare` reads at most `h1[1]`, `h1[2]`, `h2[0]`, `h2[1]`, `h2[2]`
/// (proved separately by `tests/read_extent.rs`). This test enumerates all
/// 2^32 combinations of `h1[1] × h1[2] × h2[1] × h2[2]` with `h2[0] == 0xff`
/// (the only value that gets past the first gate), sharded across threads, and
/// then all 2^24 combinations of `h2[1] × h2[2] × h1[1]` for several
/// `h2[0] != 0xff` values. Together with `row15` (complete `h2[0]` sweep) this
/// is an exhaustive equivalence proof over the reachable input space.
#[test]
fn row22_complete_input_space() {
    const THREADS: u16 = 8;
    let counters: Vec<usize> = std::thread::scope(|s| {
        let mut handles = Vec::new();
        for t in 0..THREADS {
            handles.push(s.spawn(move || {
                let im = load();
                let mut n = 0usize;
                let mut h1 = [0u8, 0, 0, 0];
                let mut h2 = [0xFFu8, 0, 0, 0];
                let mut b1 = t;
                while b1 < 256 {
                    h2[1] = b1 as u8;
                    for b2 in 0u16..=255 {
                        h2[2] = b2 as u8;
                        for a1 in 0u16..=255 {
                            h1[1] = a1 as u8;
                            for a2 in 0u16..=255 {
                                h1[2] = a2 as u8;
                                let (c, r) = im.both(h1.as_ptr(), h2.as_ptr());
                                if c != r {
                                    panic!(
                                        "row22 DIVERGENCE h1={h1:02x?} h2={h2:02x?} C={c} Rust={r}"
                                    );
                                }
                                n += 1;
                            }
                        }
                    }
                    b1 += THREADS;
                }
                n
            }));
        }
        handles.into_iter().map(|h| h.join().unwrap()).collect()
    });
    let total: usize = counters.iter().sum();
    assert_eq!(total, 1usize << 32, "row22 did not cover the whole space");

    // h2[0] != 0xff : the first gate must reject regardless of everything else.
    let im = load();
    for bad0 in [0x00u8, 0x01, 0x7F, 0xFE, 0xF0, 0xAA] {
        for b1 in 0u16..=255 {
            for b2 in 0u16..=255 {
                let h2 = [bad0, b1 as u8, b2 as u8, 0x00];
                for a1 in 0u16..=255 {
                    let h1 = [0xFFu8, a1 as u8, b2 as u8, 0x00];
                    let (c, r) = im.both(h1.as_ptr(), h2.as_ptr());
                    if c != r || c != 0 {
                        panic!("row22b DIVERGENCE h1={h1:02x?} h2={h2:02x?} C={c} Rust={r}");
                    }
                }
            }
        }
    }
}
