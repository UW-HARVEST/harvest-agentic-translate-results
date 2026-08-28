//! Deep / exhaustive differential sweeps.
//!
//! `configs.rs` covers every `CONFIGS.md` row; this file pushes the same
//! comparison much further over the input domain. The cheap sweeps run by
//! default; the full 2^32 single-argument sweeps are `#[ignore]`d so the normal
//! suite stays fast, and are run explicitly with `--ignored`.
//!
//! Structure of the C expression, which is what these sweeps stress:
//!   result = 18 + channels + ((blocksize * A + 7) mod 2^32) / 8
//! where A depends only on (channels, bitdepth):
//!   channels == 0 -> A = 0
//!   channels == 2 -> A = bitdepth + (bitdepth + (bitdepth != 32))
//!   otherwise     -> A = bitdepth * channels
//! So the two things worth exhausting are the (channels, bitdepth) -> A map and
//! the blocksize -> quotient map.

mod common;

use common::{impls, Rng, SEED};

const MAX: u32 = u32::MAX;

/// Exhaust the (channels, bitdepth) -> A map for all small values, at several
/// blocksizes (including ones that force wraparound).
#[test]
fn deep_exhaustive_channels_bitdepth_map() {
    let f = impls();
    let blocksizes = [0u32, 1, 3, 8, 4096, 65535, 0x8000_0000, MAX];
    for ch in 0..=1023u32 {
        for bd in 0..=1023u32 {
            for &bs in &blocksizes {
                f.assert_eq(bs, ch, bd);
            }
        }
    }
}

/// Exhaust the full 16-bit `bitdepth` domain on every distinct channel class.
#[test]
fn deep_exhaustive_bitdepth_16bit() {
    let f = impls();
    let mut r = Rng::new(SEED ^ 0xBD16);
    for bd in 0..=65535u32 {
        for ch in [0u32, 1, 2, 3, 8] {
            f.assert_eq(4096, ch, bd);
            f.assert_eq(r.next_u32(), ch, bd);
        }
    }
}

/// Exhaust the full 16-bit `channels` domain (crossing the `== 2` predicate).
#[test]
fn deep_exhaustive_channels_16bit() {
    let f = impls();
    let mut r = Rng::new(SEED ^ 0xC416);
    for ch in 0..=65535u32 {
        f.assert_eq(4096, ch, 16);
        f.assert_eq(4096, ch, 32);
        f.assert_eq(r.next_u32(), ch, r.next_u32());
    }
}

/// Exhaust the low 2^22 of `blocksize` (all quotient/residue behaviour near 0)
/// plus the top 2^16 (all quotient behaviour at the wrap boundary), on both
/// branches of the `channels == 2` predicate.
#[test]
fn deep_exhaustive_blocksize_low_and_high() {
    let f = impls();
    for bs in 0..(1u32 << 22) {
        f.assert_eq(bs, 2, 16);
        f.assert_eq(bs, 1, 16);
    }
    for bs in (MAX - 65535)..=MAX {
        f.assert_eq(bs, 2, 16);
        f.assert_eq(bs, 1, 16);
        f.assert_eq(bs, 2, 32);
        f.assert_eq(bs, 0, 32);
    }
}

/// Large random fuzz over the full 3-argument domain.
#[test]
fn deep_random_fuzz_100m() {
    let f = impls();
    let mut r = Rng::new(SEED ^ 0xF0FF);
    for _ in 0..100_000_000u64 {
        f.assert_eq(r.next_u32(), r.next_u32(), r.next_u32());
    }
}

/// Random fuzz biased toward boundary-ish values, so predicates and
/// wraparounds are hit far more often than uniform sampling would manage.
#[test]
fn deep_random_fuzz_biased_boundaries() {
    let f = impls();
    let mut r = Rng::new(SEED ^ 0xB1A5);
    let interesting: Vec<u32> = {
        let mut v = vec![0u32, 1, 2, 3, 4, 7, 8, 9, 16, 17, 18, 31, 32, 33, 65535, 65536, MAX, MAX - 1, MAX - 17];
        for k in 0..32 {
            v.push(1u32 << k);
            v.push((1u32 << k).wrapping_sub(1));
            v.push((1u32 << k).wrapping_add(1));
        }
        v
    };
    for _ in 0..5_000_000u64 {
        // each argument: 50% an "interesting" value, 50% uniform random
        let pick = |r: &mut Rng| {
            if r.next_u32() & 1 == 0 {
                r.pick(&interesting)
            } else {
                r.next_u32()
            }
        };
        let bs = pick(&mut r);
        let ch = pick(&mut r);
        let bd = pick(&mut r);
        f.assert_eq(bs, ch, bd);
    }
}

// ---------------------------------------------------------------------------
// Full 2^32 single-argument sweeps (slow: run with `--ignored`)
// ---------------------------------------------------------------------------

fn full_u32_blocksize_sweep(ch: u32, bd: u32) {
    let f = impls();
    let mut bs: u32 = 0;
    loop {
        let c = f.c(bs, ch, bd);
        let r = f.rust(bs, ch, bd);
        assert_eq!(
            c, r,
            "DIVERGENCE at blocksize={bs} channels={ch} bitdepth={bd}: C={c} Rust={r}"
        );
        if bs == MAX {
            break;
        }
        bs += 1;
    }
}

/// All 2^32 blocksizes on the stereo branch (both `t2` and `t3` live).
#[test]
#[ignore = "slow: full 2^32 sweep, run with --ignored"]
fn deep_full_blocksize_sweep_stereo_16() {
    full_u32_blocksize_sweep(2, 16);
}

/// All 2^32 blocksizes on the stereo branch at the `bitdepth == 32` boundary
/// (the `+1` is dropped here).
#[test]
#[ignore = "slow: full 2^32 sweep, run with --ignored"]
fn deep_full_blocksize_sweep_stereo_32() {
    full_u32_blocksize_sweep(2, 32);
}

/// All 2^32 blocksizes on the non-stereo branch.
#[test]
#[ignore = "slow: full 2^32 sweep, run with --ignored"]
fn deep_full_blocksize_sweep_mono_16() {
    full_u32_blocksize_sweep(1, 16);
}

/// All 2^32 channels values (crosses the `== 2` predicate and the
/// `18 + channels` overflow).
#[test]
#[ignore = "slow: full 2^32 sweep, run with --ignored"]
fn deep_full_channels_sweep() {
    let f = impls();
    let mut ch: u32 = 0;
    loop {
        let c = f.c(4096, ch, 16);
        let r = f.rust(4096, ch, 16);
        assert_eq!(c, r, "DIVERGENCE at channels={ch}: C={c} Rust={r}");
        if ch == MAX {
            break;
        }
        ch += 1;
    }
}

/// All 2^32 bitdepth values (crosses the `!= 32` predicate and the
/// `bitdepth + 1` overflow), on the stereo branch where it is observable.
#[test]
#[ignore = "slow: full 2^32 sweep, run with --ignored"]
fn deep_full_bitdepth_sweep_stereo() {
    let f = impls();
    let mut bd: u32 = 0;
    loop {
        let c = f.c(4096, 2, bd);
        let r = f.rust(4096, 2, bd);
        assert_eq!(c, r, "DIVERGENCE at bitdepth={bd}: C={c} Rust={r}");
        if bd == MAX {
            break;
        }
        bd += 1;
    }
}
