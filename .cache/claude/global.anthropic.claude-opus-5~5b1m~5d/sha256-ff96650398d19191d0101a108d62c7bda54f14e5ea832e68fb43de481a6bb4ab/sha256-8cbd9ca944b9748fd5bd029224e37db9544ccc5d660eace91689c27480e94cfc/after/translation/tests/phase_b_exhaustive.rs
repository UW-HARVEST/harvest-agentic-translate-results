//! Exhaustive per-axis sweeps.
//!
//! `update_frame_header` is a pure function of five inputs, and each of the four
//! `switch` axes contributes to a DISJOINT bit range of `frame_header` via `|=`.
//! That makes an exhaustive sweep of each axis over its entire `u32` domain a
//! genuinely strong check, and it is cheap enough to actually run.
//!
//! Marked `#[ignore]` because a full 3 x 2^32 sweep takes minutes; run with
//! `cargo test --release --offline -- --ignored --nocapture`.

mod common;
use common::*;

/// Sweep one `u32` field over its complete domain, everything else fixed.
fn sweep_axis(name: &str, set: impl Fn(&mut Tflac, u32)) {
    let p = pair();
    // channels = 1 in mode 0 contributes 0, keeping the picture clean.
    let base = Tflac {
        samplerate: 44_100,
        channels: 1,
        bitdepth: 16,
        channel_mode: 0,
        frame_header: 0xDEAD_BEEF,
        cur_blocksize: 4_096,
    };
    let mut diverged = 0u64;
    let mut v: u32 = 0;
    loop {
        let mut t = base;
        set(&mut t, v);
        let raw = RawTflac::from_fields(&t);
        let c = p.c.call_raw(&raw);
        let r = p.rust.call_raw(&raw);
        if c != r {
            if diverged < 10 {
                eprintln!("{name}={v}: C {c:?} vs Rust {r:?}");
            }
            diverged += 1;
        }
        if v == u32::MAX {
            break;
        }
        v += 1;
    }
    assert_eq!(diverged, 0, "{name}: {diverged} divergences over the full u32 domain");
    eprintln!("{name}: exhaustive 2^32 sweep clean");
}

#[test]
#[ignore = "full 2^32 sweep; run with --ignored"]
fn exhaustive_samplerate() {
    sweep_axis("samplerate", |t, v| t.samplerate = v);
}

#[test]
#[ignore = "full 2^32 sweep; run with --ignored"]
fn exhaustive_cur_blocksize() {
    sweep_axis("cur_blocksize", |t, v| t.cur_blocksize = v);
}

#[test]
#[ignore = "full 2^32 sweep; run with --ignored"]
fn exhaustive_bitdepth() {
    sweep_axis("bitdepth", |t, v| t.bitdepth = v);
}

#[test]
#[ignore = "full 2^32 sweep; run with --ignored"]
fn exhaustive_channels() {
    sweep_axis("channels", |t, v| t.channels = v);
}

/// `channel_mode` is a `u8`, so this axis is exhaustible cheaply — and it is
/// crossed with every `channels` boundary, since mode 0 is the only one that
/// reads `channels`.
#[test]
fn exhaustive_channel_mode_x_channels() {
    let p = pair();
    for cm in 0u8..=255 {
        for ch in [0u32, 1, 2, 7, 8, 9, 15, 16, 17, 0x0FFF_FFFF, 0x1000_0000, u32::MAX] {
            let t = Tflac {
                samplerate: 48_000,
                channels: ch,
                bitdepth: 24,
                channel_mode: cm,
                frame_header: 0,
                cur_blocksize: 4_608,
            };
            assert_eq!(p.c.call(&t), p.rust.call(&t), "channel_mode={cm} channels={ch}");
        }
    }
}

/// Exhaustive over both low-order axes jointly: every `cur_blocksize` in
/// `0..=65_536` crossed with every `samplerate` in `0..=65_536` would be 4.3e9
/// pairs, so this takes the full cross-product of the ranges where the code's
/// boundaries actually live (`0..=1024` x `0..=1024`) plus the case values.
#[test]
fn exhaustive_blocksize_x_samplerate_small() {
    let p = pair();
    let mut bs_vals: Vec<u32> = (0u32..=1024).collect();
    bs_vals.extend_from_slice(&BLOCKSIZE_CASES);
    bs_vals.extend_from_slice(&[32_769, u32::MAX]);
    let mut sr_vals: Vec<u32> = (0u32..=1024).collect();
    sr_vals.extend_from_slice(&SAMPLERATE_CASES);
    sr_vals.extend_from_slice(&[65_535, 65_536, 65_537, 256_000, 655_350, 655_360, u32::MAX]);

    for &bs in &bs_vals {
        for &sr in &sr_vals {
            let t = Tflac {
                samplerate: sr,
                channels: 2,
                bitdepth: 16,
                channel_mode: 1,
                frame_header: 0,
                cur_blocksize: bs,
            };
            assert_eq!(p.c.call(&t), p.rust.call(&t), "bs={bs} sr={sr}");
        }
    }
}
