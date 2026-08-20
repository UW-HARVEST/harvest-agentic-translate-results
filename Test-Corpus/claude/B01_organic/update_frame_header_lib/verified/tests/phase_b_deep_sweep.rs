//! Phase B (extended) — allocation-free exhaustive sweeps.
//!
//! `phase_b_configs.rs` covers every `CONFIGS.md` row with randomized inputs.
//! This file removes any doubt about value-dependent branches by sweeping each
//! numeric axis over *every* value in a wide contiguous range, plus the top of
//! the u32 range, plus a strided sweep of the full 32-bit space. No `format!`
//! per case, so the loops run at FFI speed.

mod common;

use common::*;

struct Fast {
    c: UpdateFrameHeaderFn,
    rust: UpdateFrameHeaderFn,
    cases: u64,
    _keep: Diff,
}

impl Fast {
    fn new() -> Self {
        let d = Diff::load();
        Self { c: d.c, rust: d.rust, cases: 0, _keep: d }
    }

    #[inline]
    fn check(&mut self, t: Tflac) {
        let mut a = t;
        let mut b = t;
        unsafe {
            (self.c)(&mut a as *mut Tflac);
            (self.rust)(&mut b as *mut Tflac);
        }
        if a.as_bytes() != b.as_bytes() {
            panic!(
                "MISMATCH\n  input: {t:?}\n  C    : {a:?}\n  Rust : {b:?}\n  \
                 frame_header C=0x{:08X} Rust=0x{:08X}",
                a.frame_header, b.frame_header
            );
        }
        self.cases += 1;
    }

    fn report(&self, what: &str) {
        assert!(self.cases > 0);
        eprintln!("{what}: {} cases, 0 mismatches", self.cases);
    }
}

/// Rotating "other field" configurations so the swept axis is combined with all
/// of the other axes' equivalence classes.
#[inline]
fn rotate(i: usize) -> (u32, u32, u32, u8, u32) {
    let (mode, ch) = CH_CLASS_REPS[i % CH_CLASS_REPS.len()];
    (
        SR_CLASS_REPS[i % SR_CLASS_REPS.len()],
        ch,
        BD_CLASS_REPS[i % BD_CLASS_REPS.len()],
        mode.wrapping_add((i % 251) as u8),
        BS_CLASS_REPS[i % BS_CLASS_REPS.len()],
    )
}

/// Every `samplerate` in 0..=4_000_000 — covers all of `%1000`, `/1000 < 256`,
/// `< 65536`, `%10`, `/10 < 65536` transitions — plus the top of the range.
#[test]
fn sweep_samplerate_full_low_range() {
    let mut f = Fast::new();
    for sr in 0u32..=4_000_000 {
        let (_, ch, bd, mode, bs) = rotate(sr as usize);
        f.check(Tflac::new(sr, ch, bd, mode, bs));
    }
    for sr in (0xFFFF_0000u32..=0xFFFF_FFFF).chain(0x8000_0000u32..=0x8001_0000) {
        let (_, ch, bd, mode, bs) = rotate(sr as usize);
        f.check(Tflac::new(sr, ch, bd, mode, bs));
    }
    f.report("sweep samplerate 0..=4_000_000 + top range");
}

/// Every `cur_blocksize` in 0..=1_000_000 (all 13 literals, both `default:`
/// arms and the `<= 256` boundary) plus the top of the range.
#[test]
fn sweep_blocksize_full_low_range() {
    let mut f = Fast::new();
    for bs in 0u32..=1_000_000 {
        let (sr, ch, bd, mode, _) = rotate(bs as usize);
        f.check(Tflac::new(sr, ch, bd, mode, bs));
    }
    for bs in 0xFFFF_0000u32..=0xFFFF_FFFF {
        let (sr, ch, bd, mode, _) = rotate(bs as usize);
        f.check(Tflac::new(sr, ch, bd, mode, bs));
    }
    f.report("sweep cur_blocksize 0..=1_000_000 + top range");
}

/// Every `bitdepth` in 0..=1_000_000 plus the top of the range (proves the
/// `switch` is on the full u32 and never truncates).
#[test]
fn sweep_bitdepth_full_low_range() {
    let mut f = Fast::new();
    for bd in 0u32..=1_000_000 {
        let (sr, ch, _, mode, bs) = rotate(bd as usize);
        f.check(Tflac::new(sr, ch, bd, mode, bs));
    }
    for bd in 0xFFFF_0000u32..=0xFFFF_FFFF {
        let (sr, ch, _, mode, bs) = rotate(bd as usize);
        f.check(Tflac::new(sr, ch, bd, mode, bs));
    }
    f.report("sweep bitdepth 0..=1_000_000 + top range");
}

/// Every `channels` in 0..=1_000_000 with INDEPENDENT mode (`(channels-1) << 4`
/// bleeding into every higher field), plus the wrap-around at the top.
#[test]
fn sweep_channels_full_low_range() {
    let mut f = Fast::new();
    for ch in 0u32..=1_000_000 {
        let (sr, _, bd, _, bs) = rotate(ch as usize);
        // mode%4 == 0 so `channels` is actually used
        f.check(Tflac::new(sr, ch, bd, ((ch % 64) * 4) as u8, bs));
    }
    for ch in (0xFFFF_0000u32..=0xFFFF_FFFF).chain(0x0FFF_F000u32..=0x1000_1000) {
        let (sr, _, bd, _, bs) = rotate(ch as usize);
        f.check(Tflac::new(sr, ch, bd, ((ch % 64) * 4) as u8, bs));
    }
    f.report("sweep channels 0..=1_000_000 + wrap range");
}

/// Strided sweep of the complete 32-bit space of each axis (prime stride, so the
/// residues mod 10 / mod 1000 / mod 4 all cycle), with the other axes rotating.
#[test]
fn sweep_full_u32_strided() {
    let mut f = Fast::new();
    const STRIDE: u32 = 9_973; // prime, coprime with 10, 1000 and 4

    let mut v: u32 = 0;
    let mut i: usize = 0;
    loop {
        let (sr, ch, bd, mode, bs) = rotate(i);
        f.check(Tflac::new(v, ch, bd, mode, bs)); // samplerate axis
        f.check(Tflac::new(sr, v, bd, (i % 4) as u8 * 0, bs)); // channels axis, mode 0
        f.check(Tflac::new(sr, ch, v, mode, bs)); // bitdepth axis
        f.check(Tflac::new(sr, ch, bd, mode, v)); // cur_blocksize axis
        f.check(Tflac::new(v, v, v, (v & 0xFF) as u8, v)); // all axes together
        i += 1;
        match v.checked_add(STRIDE) {
            Some(next) => v = next,
            None => break,
        }
    }
    f.report("strided full-u32 sweep of all axes");
}

// ---------------------------------------------------------------------------
// Fully exhaustive 32-bit sweeps, sharded across threads.
//
// Each of the four u32 axes is swept over ALL 2^32 values. `update_frame_header`
// is a pure function of the struct's fields, so the work shards perfectly; each
// thread owns its own pair of `Tflac`s. Only `frame_header` is compared here
// (that no other byte is written is proven exhaustively by
// `err_e14_only_frame_header_written`, `cfg_padding_preserved` and every
// `Diff::check` case, all of which compare all 24 bytes).
// ---------------------------------------------------------------------------

#[derive(Copy, Clone)]
enum Axis {
    Samplerate,
    Channels,
    Bitdepth,
    CurBlocksize,
}

impl Axis {
    fn name(self) -> &'static str {
        match self {
            Axis::Samplerate => "samplerate",
            Axis::Channels => "channels",
            Axis::Bitdepth => "bitdepth",
            Axis::CurBlocksize => "cur_blocksize",
        }
    }
    #[inline]
    fn set(self, t: &mut Tflac, v: u32) {
        match self {
            Axis::Samplerate => t.samplerate = v,
            Axis::Channels => t.channels = v,
            Axis::Bitdepth => t.bitdepth = v,
            Axis::CurBlocksize => t.cur_blocksize = v,
        }
    }
}

/// Sweeps `axis` over every value in `lo..=hi`, holding the other fields at
/// `base`. Returns the number of values checked.
fn sweep_range(
    c: UpdateFrameHeaderFn,
    rust: UpdateFrameHeaderFn,
    axis: Axis,
    base: Tflac,
    lo: u32,
    hi: u32,
) -> u64 {
    let mut a = base;
    let mut b = base;
    let mut n = 0u64;
    let mut v = lo;
    loop {
        axis.set(&mut a, v);
        axis.set(&mut b, v);
        unsafe {
            c(&mut a as *mut Tflac);
            rust(&mut b as *mut Tflac);
        }
        if a.frame_header != b.frame_header {
            panic!(
                "MISMATCH {}={} (0x{v:08X}) base={base:?}: C=0x{:08X} Rust=0x{:08X}",
                axis.name(),
                v,
                a.frame_header,
                b.frame_header
            );
        }
        n += 1;
        if v == hi {
            break;
        }
        v += 1;
    }
    n
}

/// Exhaustively sweeps one axis over all 2^32 values, sharded across `shards`
/// threads, for each of `bases` "other fields" configurations.
fn exhaustive_axis(axis: Axis, base: Tflac, shards: u32) -> u64 {
    let d = Diff::load();
    let (c, rust) = (d.c, d.rust);
    let per = (u32::MAX / shards) + 1;

    let total: u64 = std::thread::scope(|scope| {
        let mut handles = Vec::new();
        for k in 0..shards {
            let lo = k.saturating_mul(per);
            let hi = if k == shards - 1 {
                u32::MAX
            } else {
                lo.saturating_add(per - 1)
            };
            handles.push(scope.spawn(move || sweep_range(c, rust, axis, base, lo, hi)));
        }
        handles.into_iter().map(|h| h.join().unwrap()).sum()
    });

    eprintln!(
        "exhaustive {}: {total} values (all 2^32), base={base:?}, 0 mismatches",
        axis.name()
    );
    assert_eq!(total, 4_294_967_296, "must cover all 2^32 values exactly once");
    total
}

#[test]
#[ignore = "exhaustive: all 2^32 samplerate values (~40s on 16 cores)"]
fn exhaustive_samplerate_u32() {
    exhaustive_axis(Axis::Samplerate, Tflac::new(0, 2, 16, 0, 4096), 16);
}

#[test]
#[ignore = "exhaustive: all 2^32 cur_blocksize values (~40s on 16 cores)"]
fn exhaustive_blocksize_u32() {
    exhaustive_axis(Axis::CurBlocksize, Tflac::new(44100, 2, 16, 0, 0), 16);
}

#[test]
#[ignore = "exhaustive: all 2^32 bitdepth values (~40s on 16 cores)"]
fn exhaustive_bitdepth_u32() {
    exhaustive_axis(Axis::Bitdepth, Tflac::new(44100, 2, 0, 0, 4096), 16);
}

#[test]
#[ignore = "exhaustive: all 2^32 channels values (~40s on 16 cores)"]
fn exhaustive_channels_u32() {
    // channel_mode % 4 == 0 so `channels` actually reaches line 109.
    exhaustive_axis(Axis::Channels, Tflac::new(44100, 0, 16, 0, 4096), 16);
}

/// All 2^32 x 4 modes for the channel_mode-sensitive axis: sweeps `channels`
/// exhaustively once per `channel_mode % 4` value.
#[test]
#[ignore = "exhaustive: all 2^32 channels values x 4 channel modes"]
fn exhaustive_channels_x_modes_u32() {
    for mode in 0u8..4 {
        exhaustive_axis(Axis::Channels, Tflac::new(44100, 0, 16, mode, 4096), 16);
    }
}

/// Non-ignored calibration run so the suite always exercises `sweep_range`.
#[test]
fn exhaustive_sweep_range_smoke() {
    let d = Diff::load();
    let base = Tflac::new(44100, 2, 16, 0, 4096);
    let n = sweep_range(d.c, d.rust, Axis::Samplerate, base, 0, 2_000_000);
    assert_eq!(n, 2_000_001);
    let n = sweep_range(d.c, d.rust, Axis::Channels, base, 0xFFFF_0000, 0xFFFF_FFFF);
    assert_eq!(n, 65536);
    eprintln!("sweep_range smoke: ok");
}
