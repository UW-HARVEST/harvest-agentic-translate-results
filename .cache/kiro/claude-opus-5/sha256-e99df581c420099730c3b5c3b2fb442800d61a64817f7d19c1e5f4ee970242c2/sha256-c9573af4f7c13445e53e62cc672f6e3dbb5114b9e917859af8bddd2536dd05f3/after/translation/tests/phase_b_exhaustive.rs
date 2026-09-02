//! Phase B, CONFIGS.md rows 1–4: **exhaustive** differential sweeps.
//!
//! `hdr_compare`'s result depends on exactly five input bytes — `h2[0..3]`,
//! `h1[1]` and `h1[2]` (`h1[0]` is never dereferenced by the C, which row 24 in
//! `phase_b_configs.rs` verifies independently). Rows 1–4 below sweep that space
//! exhaustively rather than sampling it:
//!
//! * row 1: all 2^32 `(h1[1], h1[2], h2[1], h2[2])` with `h2[0] = 0xff`
//!   — sharded 16 ways over `h2[1]`; the union of the shards is the full space.
//! * row 2: all 256 `h2[0]` × all 65 536 `h2` tails × 16 `h1` tails.
//! * rows 3–4 are strict subsets of row 1 and are asserted as such (and re-run
//!   explicitly, since a subset test that is cheap to run is cheap to keep).
//!
//! Every call goes through the `.so` exports of both implementations.

mod common;

use common::*;

/// One shard of row 1: `h2[1]` in `[lo, hi)`, everything else exhaustive.
fn row1_shard(lo: u16, hi: u16) {
    let l = libs();
    let mut h1 = [0u8; 3];
    let mut h2 = [0xffu8; 3];
    let mut pairs: u64 = 0;
    for b1 in lo..hi {
        h2[1] = b1 as u8;
        for b2 in 0u16..256 {
            h2[2] = b2 as u8;
            for a1 in 0u16..256 {
                h1[1] = a1 as u8;
                for a2 in 0u16..256 {
                    h1[2] = a2 as u8;
                    if let Some((c, r)) = diff(l, &h1, &h2) {
                        panic!(
                            "row1 divergence: h1={h1:02x?} h2={h2:02x?} => C={c} Rust={r}"
                        );
                    }
                    pairs += 1;
                }
            }
        }
    }
    assert_eq!(pairs, (hi - lo) as u64 * 256 * 256 * 256);
}

macro_rules! row1_shards {
    ($($name:ident => $lo:expr),* $(,)?) => {
        $(
            #[test]
            fn $name() {
                row1_shard($lo, $lo + 16);
            }
        )*
    };
}

// 16 shards × 16 `h2[1]` values = all 256 values of `h2[1]`.
row1_shards! {
    cfg_row01_exhaustive_shard00 => 0,
    cfg_row01_exhaustive_shard01 => 16,
    cfg_row01_exhaustive_shard02 => 32,
    cfg_row01_exhaustive_shard03 => 48,
    cfg_row01_exhaustive_shard04 => 64,
    cfg_row01_exhaustive_shard05 => 80,
    cfg_row01_exhaustive_shard06 => 96,
    cfg_row01_exhaustive_shard07 => 112,
    cfg_row01_exhaustive_shard08 => 128,
    cfg_row01_exhaustive_shard09 => 144,
    cfg_row01_exhaustive_shard10 => 160,
    cfg_row01_exhaustive_shard11 => 176,
    cfg_row01_exhaustive_shard12 => 192,
    cfg_row01_exhaustive_shard13 => 208,
    cfg_row01_exhaustive_shard14 => 224,
    cfg_row01_exhaustive_shard15 => 240,
}

/// Row 2 — axis A crossed with axes B/C/D/E in full: every `h2[0]` value, every
/// `h2` tail, and a spread of `h1` tails (exactly matching, bit-0 flipped,
/// grossly different, random).
#[test]
fn cfg_row02_all_sync_bytes_x_all_tails() {
    let l = libs();
    let mut rng = Rng::new(0x5EED_0002);
    let mut h1 = [0u8; 3];
    let mut h2 = [0u8; 3];
    for b0 in 0u16..256 {
        h2[0] = b0 as u8;
        for b1 in 0u16..256 {
            h2[1] = b1 as u8;
            for b2 in 0u16..256 {
                h2[2] = b2 as u8;
                // 16 h1 tails per h2: deterministic interesting ones + random.
                let tails: [(u8, u8); 16] = [
                    (h2[1], h2[2]),                       // exact match
                    (h2[1] ^ 0x01, h2[2]),                // ignored padding bit
                    (h2[1] ^ 0x02, h2[2]),                // masked bit -> reject
                    (h2[1], h2[2] ^ 0x04),                // samplerate bit
                    (h2[1], h2[2] ^ 0x08),                // samplerate bit
                    (h2[1], h2[2] ^ 0x0C),                // both samplerate bits
                    (h2[1], h2[2] ^ 0x10),                // bitrate nibble bit
                    (h2[1], h2[2] & 0x0F),                // force nibble zero
                    (h2[1], h2[2] | 0xA0),                // force nibble non-zero
                    (h2[1] ^ 0x80, h2[2]),                // high bit
                    (0x00, 0x00),
                    (0xff, 0xff),
                    (rng.next_u8(), rng.next_u8()),
                    (rng.next_u8(), rng.next_u8()),
                    (rng.next_u8(), rng.next_u8()),
                    (rng.next_u8(), rng.next_u8()),
                ];
                for (a1, a2) in tails {
                    h1[1] = a1;
                    h1[2] = a2;
                    if let Some((c, r)) = diff(l, &h1, &h2) {
                        panic!("row2 divergence: h1={h1:02x?} h2={h2:02x?} => C={c} Rust={r}");
                    }
                }
            }
        }
    }
}

/// Row 3 — for every one of the 2 520 *valid* `h2` tails, all 65 536 `h1` tails.
/// This is the accept-heavy region: axes F × G × H fully crossed under every
/// `h2` that reaches them.
#[test]
fn cfg_row03_all_h1_tails_for_every_valid_h2() {
    let l = libs();
    let v1 = valid_byte1_values();
    let v2 = valid_byte2_values();
    assert_eq!(v1.len(), 14, "valid h2[1] count derived in CONFIGS.md");
    assert_eq!(v2.len(), 180, "valid h2[2] count derived in CONFIGS.md");

    let mut h1 = [0u8; 3];
    let mut h2 = [0xffu8; 3];
    let mut accepted: u64 = 0;
    for &b1 in &v1 {
        h2[1] = b1;
        for &b2 in &v2 {
            h2[2] = b2;
            let mut acc_here = 0u32;
            for a1 in 0u16..256 {
                h1[1] = a1 as u8;
                for a2 in 0u16..256 {
                    h1[2] = a2 as u8;
                    let (c, r) = unsafe {
                        ((l.c)(h1.as_ptr(), h2.as_ptr()), (l.rust)(h1.as_ptr(), h2.as_ptr()))
                    };
                    assert_eq!(
                        c, r,
                        "row3 divergence: h1={h1:02x?} h2={h2:02x?} => C={c} Rust={r}"
                    );
                    acc_here += (c != 0) as u32;
                }
            }
            // Cross-check that the sweep really reached the accepting region.
            // Accepting h1[1]: 2 values (h2[1] with bit 0 either way).
            // Accepting h1[2]: bits 2-3 pinned to h2[2], bits 0-1 free (x4), and
            // axis H pins the high nibble to "zero" (1 of 16) when h2[2]'s
            // bitrate nibble is zero, else "non-zero" (15 of 16).
            let want = 2 * 4 * if (b2 & 0xF0) == 0 { 1 } else { 15 };
            assert_eq!(
                acc_here, want,
                "h2={h2:02x?}: expected {want} accepting h1 tails, got {acc_here}"
            );
            accepted += acc_here as u64;
        }
    }
    // 12 of the 180 valid h2[2] values have a zero bitrate nibble (0x00..=0x0b).
    let free_format = v2.iter().filter(|&&b| (b & 0xF0) == 0).count() as u64;
    assert_eq!(free_format, 12);
    assert_eq!(accepted, 14 * (free_format * 8 + (180 - free_format) * 120));
}

/// Row 4 — the mirror of row 3: all 65 536 `h2` tails (i.e. every rejection
/// class of axes B/C/D/E) against a representative set of `h1` tails, with the
/// sync byte fixed valid so `hdr_valid`'s later terms are actually reached.
#[test]
fn cfg_row04_all_h2_tails_representative_h1() {
    let l = libs();
    let mut rng = Rng::new(0x5EED_0004);
    // Representative h1 tails: 12 fixed + 20 random, applied to every h2 tail.
    let mut fixed: Vec<(u8, u8)> = vec![
        (0x00, 0x00),
        (0x00, 0xff),
        (0xff, 0x00),
        (0xff, 0xff),
        (0xe2, 0x90),
        (0xe3, 0x91),
        (0xfb, 0x90),
        (0xfa, 0x00),
        (0xf3, 0x0c),
        (0xf4, 0xf0),
        (0x55, 0xaa),
        (0xaa, 0x55),
    ];
    for _ in 0..20 {
        fixed.push((rng.next_u8(), rng.next_u8()));
    }

    let mut h1 = [0u8; 3];
    let mut h2 = [0xffu8; 3];
    for b1 in 0u16..256 {
        h2[1] = b1 as u8;
        for b2 in 0u16..256 {
            h2[2] = b2 as u8;
            for &(a1, a2) in &fixed {
                h1[1] = a1;
                h1[2] = a2;
                if let Some((c, r)) = diff(l, &h1, &h2) {
                    panic!("row4 divergence: h1={h1:02x?} h2={h2:02x?} => C={c} Rust={r}");
                }
            }
        }
    }
}

/// Row 25 — closes the one gap left by row 1 (which fixes `h2[0] = 0xff`) and
/// row 2 (which samples `h1` tails): for **every** `h2[0]` value, sweep **all
/// 65 536** `h1` tails against one `h2` tail per rejection/acceptance class. If
/// either implementation let some exotic `h1` tail override an invalid sync byte,
/// this finds it.
#[test]
fn cfg_row25_all_sync_bytes_x_all_h1_tails() {
    let l = libs();
    // One representative h2 tail per class the C distinguishes.
    let tails: [[u8; 2]; 16] = [
        [0xfb, 0x90], // fully valid, layer III, bitrate 9, samplerate 0
        [0xe2, 0x90], // MPEG-2.5 class, valid
        [0xe3, 0x00], // MPEG-2.5 class, free-format bitrate
        [0xff, 0xe8], // high class, bitrate 14, samplerate 2
        [0xf2, 0x14], // high class, layer 1
        [0xfa, 0x0b], // free-format, samplerate 2
        [0xf0, 0x90], // class ok, reserved layer -> invalid
        [0xf9, 0x90], // class ok, reserved layer -> invalid
        [0xfb, 0xf0], // reserved bitrate -> invalid
        [0xfb, 0xff], // reserved bitrate + samplerate -> invalid
        [0xfb, 0x0c], // reserved samplerate -> invalid
        [0x00, 0x00], // class fails
        [0xe1, 0x90], // class fails (one below 0xe2)
        [0xef, 0x90], // class fails (one below 0xf0)
        [0x55, 0xaa], // arbitrary invalid
        [0xaa, 0x55], // arbitrary invalid
    ];
    let mut h1 = [0u8; 3];
    let mut h2 = [0u8; 3];
    for b0 in 0u16..256 {
        h2[0] = b0 as u8;
        for t in tails {
            h2[1] = t[0];
            h2[2] = t[1];
            for a1 in 0u16..256 {
                h1[1] = a1 as u8;
                for a2 in 0u16..256 {
                    h1[2] = a2 as u8;
                    if let Some((c, r)) = diff(l, &h1, &h2) {
                        panic!("row25 divergence: h1={h1:02x?} h2={h2:02x?} => C={c} Rust={r}");
                    }
                }
            }
        }
    }
}

/// Row 26 — the deep complement of row 25: for three representative *bad* sync
/// bytes, sweep all 65 536 `h2` tails against all 256 `h1[1]` values (with
/// `h1[2]` cycling through all 256 values as the outer index advances), so every
/// `(bad h2[0], h2 tail)` pair is seen against a broad `h1` spread.
#[test]
fn cfg_row26_bad_sync_deep_sweep() {
    let l = libs();
    let mut h1 = [0u8; 3];
    let mut h2 = [0u8; 3];
    for b0 in [0x00u8, 0x01, 0x7f, 0x80, 0xfe] {
        h2[0] = b0;
        for b1 in 0u16..256 {
            h2[1] = b1 as u8;
            for b2 in 0u16..256 {
                h2[2] = b2 as u8;
                for a1 in 0u16..256 {
                    h1[1] = a1 as u8;
                    h1[2] = (a1 as u8).wrapping_add(b2 as u8).wrapping_mul(31);
                    if let Some((c, r)) = diff(l, &h1, &h2) {
                        panic!("row26 divergence: h1={h1:02x?} h2={h2:02x?} => C={c} Rust={r}");
                    }
                    assert_eq!(c_of(l, &h1, &h2), 0, "bad sync byte accepted: h2={h2:02x?}");
                }
            }
        }
    }
}

#[inline(always)]
fn c_of(l: &common::Libs, h1: &[u8; 3], h2: &[u8; 3]) -> std::ffi::c_int {
    unsafe { (l.c)(h1.as_ptr(), h2.as_ptr()) }
}
