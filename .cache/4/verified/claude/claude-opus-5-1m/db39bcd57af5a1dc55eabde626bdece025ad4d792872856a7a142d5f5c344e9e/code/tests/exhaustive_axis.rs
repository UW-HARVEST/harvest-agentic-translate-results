//! Optional deep sweeps: exhaustively enumerate an ENTIRE 32-bit argument axis.
//!
//! Rationale. Expanding the C expression shows the three terms share the
//! `blocksize` factor:
//!
//! ```text
//! T1 + T2 + T3 = blocksize * M   (mod 2^32)
//! M = bitdepth*channels*(channels!=2)
//!   + bitdepth*(channels==2)
//!   + (bitdepth + (bitdepth!=32))*(channels==2)
//! f(bs, ch, bd) = 18 + ch + ((bs * M + 7) / 8)      (all mod 2^32)
//! ```
//!
//! So the whole function is determined by `channels` and the single derived
//! multiplier `M`. Sweeping one axis exhaustively while the others are pinned to
//! representative values therefore covers a very large, structurally meaningful
//! slice of the 2^96 input space.
//!
//! These tests are `#[ignore]`d because they take minutes; run them with:
//!
//! ```sh
//! cargo test --release --offline --test exhaustive_axis -- --ignored --nocapture
//! ```

mod common;

use common::*;
use std::time::Instant;

/// Sweep every one of the 2^32 `blocksize` values for a handful of
/// (channels, bitdepth) pairs that cover both stereo predicates, the
/// `bitdepth == 32` predicate, and the degenerate cases.
#[test]
#[ignore = "exhaustive 2^32 sweep; run explicitly with --release --ignored"]
fn exhaustive_all_blocksizes() {
    let l = libs();
    let pairs: &[(u32, u32)] = &[
        (2, 32),        // stereo, bitdepth == 32 (T3 keeps bitdepth)
        (2, 16),        // stereo, bitdepth != 32 (T3 uses bitdepth + 1)
        (2, 0),         // stereo, bitdepth == 0  (T3 == blocksize)
        (2, u32::MAX),  // stereo, inner +1 wraps to 0
        (1, 16),        // mono
        (3, 24),        // non-stereo multi-channel
        (0, 16),        // channels == 0, everything vanishes
        (u32::MAX, 1),  // outer 18 + channels wraps
    ];

    for &(ch, bd) in pairs {
        let t0 = Instant::now();
        let mut bs: u32 = 0;
        loop {
            let c = l.c(bs, ch, bd);
            let r = l.rust(bs, ch, bd);
            if c != r {
                panic!(
                    "DIVERGENCE at max_size_frame({bs}, {ch}, {bd}): C={c} Rust={r}"
                );
            }
            if bs == u32::MAX {
                break;
            }
            bs += 1;
        }
        println!(
            "channels={ch}, bitdepth={bd}: all 2^32 blocksizes match ({:?})",
            t0.elapsed()
        );
    }
}

/// Sweep every one of the 2^32 `bitdepth` values for representative
/// (blocksize, channels) pairs.
#[test]
#[ignore = "exhaustive 2^32 sweep; run explicitly with --release --ignored"]
fn exhaustive_all_bitdepths() {
    let l = libs();
    let pairs: &[(u32, u32)] = &[
        (4096, 2),      // stereo, realistic blocksize
        (4096, 1),      // mono
        (4096, 3),      // non-stereo
        (1, 2),         // smallest non-zero blocksize, stereo
        (u32::MAX, 2),  // blocksize wraps the products, stereo
        (0, 2),         // blocksize == 0
    ];

    for &(bs, ch) in pairs {
        let t0 = Instant::now();
        let mut bd: u32 = 0;
        loop {
            let c = l.c(bs, ch, bd);
            let r = l.rust(bs, ch, bd);
            if c != r {
                panic!(
                    "DIVERGENCE at max_size_frame({bs}, {ch}, {bd}): C={c} Rust={r}"
                );
            }
            if bd == u32::MAX {
                break;
            }
            bd += 1;
        }
        println!(
            "blocksize={bs}, channels={ch}: all 2^32 bitdepths match ({:?})",
            t0.elapsed()
        );
    }
}

/// Sweep every one of the 2^32 `channels` values for representative
/// (blocksize, bitdepth) pairs. This is the axis that feeds BOTH the `T1`
/// multiplier and the outer `18U + channels`, so it exercises the two wrap
/// sites simultaneously.
#[test]
#[ignore = "exhaustive 2^32 sweep; run explicitly with --release --ignored"]
fn exhaustive_all_channels() {
    let l = libs();
    let pairs: &[(u32, u32)] = &[
        (4096, 16),
        (4096, 32),
        (1, 1),
        (u32::MAX, u32::MAX),
        (0, 16),
    ];

    for &(bs, bd) in pairs {
        let t0 = Instant::now();
        let mut ch: u32 = 0;
        loop {
            let c = l.c(bs, ch, bd);
            let r = l.rust(bs, ch, bd);
            if c != r {
                panic!(
                    "DIVERGENCE at max_size_frame({bs}, {ch}, {bd}): C={c} Rust={r}"
                );
            }
            if ch == u32::MAX {
                break;
            }
            ch += 1;
        }
        println!(
            "blocksize={bs}, bitdepth={bd}: all 2^32 channels match ({:?})",
            t0.elapsed()
        );
    }
}

/// Exhaustively cover the derived multiplier `M` for the stereo path by sweeping
/// all 2^32 `bitdepth` values at `blocksize == 1`, where the quotient is
/// `(M + 7) / 8` and every distinct `M` is therefore directly observable.
#[test]
#[ignore = "exhaustive 2^32 sweep; run explicitly with --release --ignored"]
fn exhaustive_multiplier_stereo() {
    let l = libs();
    let t0 = Instant::now();
    let mut bd: u32 = 0;
    loop {
        let c = l.c(1, 2, bd);
        let r = l.rust(1, 2, bd);
        assert_eq!(c, r, "DIVERGENCE at max_size_frame(1, 2, {bd})");
        if bd == u32::MAX {
            break;
        }
        bd += 1;
    }
    println!("stereo multiplier M: all 2^32 bitdepths match ({:?})", t0.elapsed());
}
