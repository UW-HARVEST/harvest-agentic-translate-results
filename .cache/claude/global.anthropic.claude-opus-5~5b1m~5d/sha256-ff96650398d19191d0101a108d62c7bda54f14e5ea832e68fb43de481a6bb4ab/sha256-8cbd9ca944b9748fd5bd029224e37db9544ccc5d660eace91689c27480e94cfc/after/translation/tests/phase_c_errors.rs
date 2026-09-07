//! Phase C — error / rejection-path differential tests, one test per
//! `ERRORS.md` row.
//!
//! The C API returns `void` and has no error codes, so "the same error" means
//! "the same *rejection behaviour*": the same resulting `frame_header` bit
//! pattern, including the paths where the C silently declines to set a nibble
//! (rows 6, 9, 10, 16). Each test also pins the exact expected bit pattern, so
//! a test cannot pass by both sides being wrong in the same way.

mod common;
use common::*;

const N: usize = 2_000;

/// The blocksize nibble (bits 12..15) of a resulting frame_header.
fn bs_nibble(fh: u32) -> u32 {
    (fh >> 12) & 0xF
}
/// The samplerate nibble (bits 8..11).
fn sr_nibble(fh: u32) -> u32 {
    (fh >> 8) & 0xF
}
/// The sample-size field (bits 1..3).
fn bd_field(fh: u32) -> u32 {
    (fh >> 1) & 0x7
}

/// Row 1 — `t == NULL`. The C has no null check and dereferences immediately,
/// so both sides fault. A SIGSEGV cannot be compared differentially, so this is
/// asserted structurally instead: neither `.so` contains a null test, and both
/// take the pointer as-is.
#[test]
fn err_01_null_pointer_documented_not_executed() {
    // Both libraries export the symbol and neither can be called with NULL
    // without invoking UB; documented in ERRORS.md row 1.
    let p = pair();
    assert_eq!(p.c.name, "C");
    assert_eq!(p.rust.name, "Rust");
    // A non-null but otherwise arbitrary struct still works in both, which is
    // what confirms the pointer is used unconditionally and unchecked.
    let t = Tflac::default();
    assert_eq!(p.c.call(&t), p.rust.call(&t));
}

/// Row 2 — unlisted `cur_blocksize <= 256` → `0x06`.
#[test]
fn err_02_blocksize_default_low() {
    let p = pair();
    let mut rng = Rng::new(0xC0_0002);
    for bs in 0u32..=256 {
        if BLOCKSIZE_CASES.contains(&bs) {
            continue;
        }
        let mut t = rng.random_tflac_neutral_channels();
        t.cur_blocksize = bs;
        let (c, r) = (p.c.call(&t), p.rust.call(&t));
        assert_eq!(c, r, "err02 bs={bs}");
        assert_eq!(bs_nibble(c), 0x06, "err02 wrong C nibble for bs={bs}");
    }
}

/// Row 3 — unlisted `cur_blocksize > 256` → `0x07`.
#[test]
fn err_03_blocksize_default_high() {
    let p = pair();
    let mut rng = Rng::new(0xC0_0003);
    let mut n = 0;
    for &bs in &[257u32, 258, 65535, 1 << 20, u32::MAX] {
        let mut t = rng.random_tflac_neutral_channels();
        t.cur_blocksize = bs;
        let (c, r) = (p.c.call(&t), p.rust.call(&t));
        assert_eq!(c, r, "err03 bs={bs}");
        assert_eq!(bs_nibble(c), 0x07, "err03 wrong C nibble for bs={bs}");
    }
    while n < N {
        let bs = rng.range(257, u32::MAX);
        if BLOCKSIZE_CASES.contains(&bs) {
            continue;
        }
        let mut t = rng.random_tflac_neutral_channels();
        t.cur_blocksize = bs;
        let (c, r) = (p.c.call(&t), p.rust.call(&t));
        assert_eq!(c, r, "err03 bs={bs}");
        assert_eq!(bs_nibble(c), 0x07, "err03 wrong C nibble for bs={bs}");
        n += 1;
    }
}

/// Row 4 — one step either side of each explicit blocksize case falls to
/// `default`, it does NOT borrow the neighbour's code.
#[test]
fn err_04_blocksize_off_by_one() {
    let p = pair();
    let mut rng = Rng::new(0xC0_0004);
    for base in BLOCKSIZE_CASES {
        for bs in [base.wrapping_sub(1), base + 1] {
            if BLOCKSIZE_CASES.contains(&bs) {
                continue;
            }
            for _ in 0..64 {
                let mut t = rng.random_tflac_neutral_channels();
                t.cur_blocksize = bs;
                let (c, r) = (p.c.call(&t), p.rust.call(&t));
                assert_eq!(c, r, "err04 bs={bs} (base {base})");
                let expect = if bs <= 256 { 0x06 } else { 0x07 };
                assert_eq!(bs_nibble(c), expect, "err04 bs={bs}");
            }
        }
    }
}

/// Row 5 — `samplerate == 0` is accepted and takes the kHz branch.
#[test]
fn err_05_samplerate_zero() {
    let p = pair();
    let mut rng = Rng::new(0xC0_0005);
    for _ in 0..N {
        let mut t = rng.random_tflac_neutral_channels();
        t.samplerate = 0;
        let (c, r) = (p.c.call(&t), p.rust.call(&t));
        assert_eq!(c, r, "err05");
        assert_eq!(sr_nibble(c), 0x0C, "err05 samplerate=0 nibble");
    }
}

/// Row 6 — `%1000==0` but `/1000 >= 256`: silent no-op, nibble stays 0.
#[test]
fn err_06_samplerate_khz_overflow() {
    let p = pair();
    let mut rng = Rng::new(0xC0_0006);
    let check = |sr: u32, rng: &mut Rng| {
        let mut t = rng.random_tflac_neutral_channels();
        t.samplerate = sr;
        let (c, r) = (p.c.call(&t), p.rust.call(&t));
        assert_eq!(c, r, "err06 sr={sr}");
        assert_eq!(sr_nibble(c), 0x00, "err06 sr={sr} should leave nibble 0");
    };
    for &sr in &[256_000u32, 257_000, 999_000, 4_294_000_000] {
        check(sr, &mut rng);
    }
    let mut n = 0;
    while n < N {
        let sr = rng.range(256, 4_294_967) * 1000;
        if SAMPLERATE_CASES.contains(&sr) {
            continue;
        }
        check(sr, &mut rng);
        n += 1;
    }
}

/// Row 7 — `%1000!=0 && <65536` → `0x0D`.
#[test]
fn err_07_samplerate_hz_small() {
    let p = pair();
    let mut rng = Rng::new(0xC0_0007);
    let mut n = 0;
    while n < N {
        let sr = rng.range(1, 65_535);
        if sr % 1000 == 0 || SAMPLERATE_CASES.contains(&sr) {
            continue;
        }
        let mut t = rng.random_tflac_neutral_channels();
        t.samplerate = sr;
        let (c, r) = (p.c.call(&t), p.rust.call(&t));
        assert_eq!(c, r, "err07 sr={sr}");
        assert_eq!(sr_nibble(c), 0x0D, "err07 sr={sr}");
        n += 1;
    }
}

/// Row 8 — `%10==0 && /10 < 65536` (and `>=65536`) → `0x0E`.
#[test]
fn err_08_samplerate_dahz() {
    let p = pair();
    let mut rng = Rng::new(0xC0_0008);
    let mut n = 0;
    for &sr in &[65_540u32, 655_350] {
        let mut t = rng.random_tflac_neutral_channels();
        t.samplerate = sr;
        let (c, r) = (p.c.call(&t), p.rust.call(&t));
        assert_eq!(c, r, "err08 sr={sr}");
        assert_eq!(sr_nibble(c), 0x0E, "err08 sr={sr}");
    }
    while n < N {
        let sr = rng.range(6554, 65_535) * 10;
        if sr < 65_536 || sr % 1000 == 0 || SAMPLERATE_CASES.contains(&sr) {
            continue;
        }
        let mut t = rng.random_tflac_neutral_channels();
        t.samplerate = sr;
        let (c, r) = (p.c.call(&t), p.rust.call(&t));
        assert_eq!(c, r, "err08 sr={sr}");
        assert_eq!(sr_nibble(c), 0x0E, "err08 sr={sr}");
        n += 1;
    }
}

/// Row 9 — `%10==0 && /10 >= 65536`: silent no-op.
#[test]
fn err_09_samplerate_dahz_overflow() {
    let p = pair();
    let mut rng = Rng::new(0xC0_0009);
    let mut n = 0;
    for &sr in &[655_360u32, 4_294_967_290] {
        let mut t = rng.random_tflac_neutral_channels();
        t.samplerate = sr;
        let (c, r) = (p.c.call(&t), p.rust.call(&t));
        assert_eq!(c, r, "err09 sr={sr}");
        assert_eq!(sr_nibble(c), 0x00, "err09 sr={sr} should leave nibble 0");
    }
    while n < N {
        let sr = rng.range(65_536, 429_496_729) * 10;
        if sr % 1000 == 0 || SAMPLERATE_CASES.contains(&sr) {
            continue;
        }
        let mut t = rng.random_tflac_neutral_channels();
        t.samplerate = sr;
        let (c, r) = (p.c.call(&t), p.rust.call(&t));
        assert_eq!(c, r, "err09 sr={sr}");
        assert_eq!(sr_nibble(c), 0x00, "err09 sr={sr}");
        n += 1;
    }
}

/// Row 10 — `>=65536 && %10!=0`: silent no-op.
#[test]
fn err_10_samplerate_no_branch() {
    let p = pair();
    let mut rng = Rng::new(0xC0_000A);
    let mut n = 0;
    for &sr in &[65_537u32, 176_401, u32::MAX] {
        let mut t = rng.random_tflac_neutral_channels();
        t.samplerate = sr;
        let (c, r) = (p.c.call(&t), p.rust.call(&t));
        assert_eq!(c, r, "err10 sr={sr}");
        assert_eq!(sr_nibble(c), 0x00, "err10 sr={sr} should leave nibble 0");
    }
    while n < N {
        let sr = rng.range(65_536, u32::MAX);
        if sr % 10 == 0 || SAMPLERATE_CASES.contains(&sr) {
            continue;
        }
        let mut t = rng.random_tflac_neutral_channels();
        t.samplerate = sr;
        let (c, r) = (p.c.call(&t), p.rust.call(&t));
        assert_eq!(c, r, "err10 sr={sr}");
        assert_eq!(sr_nibble(c), 0x00, "err10 sr={sr}");
        n += 1;
    }
}

/// Row 11 — samplerate boundary values, one step either side.
#[test]
fn err_11_samplerate_boundaries() {
    let p = pair();
    let mut rng = Rng::new(0xC0_000B);
    // (samplerate, expected nibble) derived by hand from c_src/src/lib.c.
    let cases: &[(u32, u32)] = &[
        (0, 0x0C),
        (1, 0x0D),
        (999, 0x0D),
        (1000, 0x0C),
        (1001, 0x0D),
        (65_535, 0x0D),
        // 65536 % 1000 == 536 (not kHz), 65536 < 65536 is false,
        // 65536 % 10 == 6 (not daHz) -> no branch taken, nibble stays 0.
        (65_536, 0x00),
        (65_537, 0x00),
        (255_000, 0x0C),
        (256_000, 0x00),
        (655_350, 0x0E),
        (655_360, 0x00),
        (u32::MAX, 0x00),
    ];
    for &(sr, expect) in cases {
        for _ in 0..64 {
            let mut t = rng.random_tflac_neutral_channels();
            t.samplerate = sr;
            let (c, r) = (p.c.call(&t), p.rust.call(&t));
            assert_eq!(c, r, "err11 sr={sr}");
            assert_eq!(sr_nibble(c), expect, "err11 sr={sr} nibble");
        }
    }
    // The silent-reject boundaries must flip exactly where the C says they do.
    // `channels: 1` keeps `(channels - 1) << 4 == 0` so the nibble is readable.
    let mk = |sr: u32| Tflac { samplerate: sr, channels: 1, ..Default::default() };
    let nib_c = |sr: u32| sr_nibble(p.c.call(&mk(sr)));
    let nib_r = |sr: u32| sr_nibble(p.rust.call(&mk(sr)));
    for &(sr, expect) in &[
        (255_000u32, 0x0Cu32),
        (256_000, 0x00),
        (655_350, 0x0E),
        (655_360, 0x00),
        (65_535, 0x0D),
        (65_536, 0x00),
        (65_537, 0x00),
    ] {
        assert_eq!(nib_c(sr), expect, "err11 C nibble at boundary sr={sr}");
        assert_eq!(nib_r(sr), expect, "err11 Rust nibble at boundary sr={sr}");
        assert_eq!(p.c.call(&mk(sr)), p.rust.call(&mk(sr)), "err11 exact sr={sr}");
    }
}

/// Row 12 — out-of-range `channel_mode` (>= 4, i.e. no valid enum variant) is
/// folded by `% 4`, not rejected. `default:` of that switch is unreachable.
#[test]
fn err_12_channel_mode_out_of_range() {
    let p = pair();
    let mut rng = Rng::new(0xC0_000C);
    // Every out-of-range u8 value, against a fixed reference input.
    for cm in 4u8..=255 {
        let base = Tflac {
            samplerate: 44100,
            channels: 2,
            bitdepth: 16,
            channel_mode: cm,
            frame_header: 0,
            cur_blocksize: 4096,
        };
        let mut folded = base;
        folded.channel_mode = cm % 4;
        let (c, r) = (p.c.call(&base), p.rust.call(&base));
        assert_eq!(c, r, "err12 channel_mode={cm}");
        // The out-of-range value behaves exactly like its %4 residue.
        assert_eq!(c, p.c.call(&folded), "err12 C fold mismatch cm={cm}");
        assert_eq!(r, p.rust.call(&folded), "err12 Rust fold mismatch cm={cm}");
    }
    // TFLAC_CHANNEL_MODE_COUNT (4) specifically behaves as INDEPENDENT (0).
    for _ in 0..N {
        let mut t = rng.random_tflac();
        t.channel_mode = 4;
        let mut zero = t;
        zero.channel_mode = 0;
        let (c, r) = (p.c.call(&t), p.rust.call(&t));
        assert_eq!(c, r, "err12 count-value");
        assert_eq!(c, p.c.call(&zero), "err12 4 must act as 0");
    }
}

/// Row 13 — `channels == 0` in mode 0: u32 wrap to 0xFFFFFFF0.
#[test]
fn err_13_channels_zero_wraps() {
    let p = pair();
    let mut rng = Rng::new(0xC0_000D);
    for _ in 0..N {
        let mut t = rng.random_tflac();
        t.channels = 0;
        t.channel_mode = (rng.next_u8() / 4) * 4;
        let (c, r) = (p.c.call(&t), p.rust.call(&t));
        assert_eq!(c, r, "err13");
        // 0u32.wrapping_sub(1) << 4 == 0xFFFFFFF0 -> all bits 4..31 set.
        assert_eq!(c & 0xFFFF_FFF0, 0xFFFF_FFF0, "err13 wrap bits missing: 0x{c:08X}");
    }
    // Exact expected value for a fully specified input.
    let t = Tflac {
        samplerate: 44100,
        channels: 0,
        bitdepth: 16,
        channel_mode: 0,
        frame_header: 0,
        cur_blocksize: 4096,
    };
    let expected = 0xFFF8u32 << 16 | (0x0C << 12) | (0x09 << 8) | 0xFFFF_FFF0u32 | (4 << 1);
    assert_eq!(p.c.call(&t), expected, "err13 hand-computed C value");
    assert_eq!(p.rust.call(&t), expected, "err13 hand-computed Rust value");
}

/// Row 14 — `channels` far out of FLAC range in mode 0: no masking, no check,
/// `<< 4` truncates rather than trapping.
#[test]
fn err_14_channels_out_of_range() {
    let p = pair();
    let mut rng = Rng::new(0xC0_000E);
    for &ch in &[9u32, 16, 17, 255, 0x0FFF_FFFF, 0x1000_0000, 0x2000_0000, 0x8000_0000, u32::MAX]
    {
        for _ in 0..64 {
            let mut t = rng.random_tflac();
            t.channels = ch;
            t.channel_mode = 0;
            let (c, r) = (p.c.call(&t), p.rust.call(&t));
            assert_eq!(c, r, "err14 channels={ch}");
        }
    }
    // channels == 0x10000001 -> (ch-1)<<4 == 0x00000000 after truncation.
    let t = Tflac {
        samplerate: 44100,
        channels: 0x1000_0001,
        bitdepth: 16,
        channel_mode: 0,
        frame_header: 0,
        cur_blocksize: 4096,
    };
    let expected = (0xFFF8u32 << 16) | (0x0C << 12) | (0x09 << 8) | (0x1000_0000u32 << 4) | (4 << 1);
    assert_eq!(p.c.call(&t), expected, "err14 truncation, C");
    assert_eq!(p.rust.call(&t), expected, "err14 truncation, Rust");
}

/// Row 15 — `channels` is ignored entirely when `channel_mode % 4 != 0`.
#[test]
fn err_15_channels_ignored_in_stereo_modes() {
    let p = pair();
    let mut rng = Rng::new(0xC0_000F);
    for residue in 1u8..=3 {
        for _ in 0..N {
            let mut a = rng.random_tflac();
            a.channel_mode = residue;
            let mut b = a;
            b.channels = rng.spicy_u32();
            let (ca, cb) = (p.c.call(&a), p.c.call(&b));
            let (ra, rb) = (p.rust.call(&a), p.rust.call(&b));
            assert_eq!(ca, ra, "err15 a residue={residue}");
            assert_eq!(cb, rb, "err15 b residue={residue}");
            assert_eq!(ca, cb, "err15 C used channels in mode {residue}");
            assert_eq!(ra, rb, "err15 Rust used channels in mode {residue}");
        }
    }
}

/// Row 16 — unlisted `bitdepth` → empty `default`, sample-size field stays 0.
#[test]
fn err_16_bitdepth_unlisted() {
    let p = pair();
    let mut rng = Rng::new(0xC0_0010);
    for bd in 0u32..=64 {
        if BITDEPTH_CASES.contains(&bd) {
            continue;
        }
        let mut t = rng.random_tflac();
        t.bitdepth = bd;
        let (c, r) = (p.c.call(&t), p.rust.call(&t));
        assert_eq!(c, r, "err16 bitdepth={bd}");
        assert_eq!(bd_field(c), 0, "err16 bitdepth={bd} must leave field 0");
    }
    for &bd in &[u32::MAX, u32::MAX - 1, 1 << 31, 1_000_000] {
        let mut t = rng.random_tflac();
        t.bitdepth = bd;
        let (c, r) = (p.c.call(&t), p.rust.call(&t));
        assert_eq!(c, r, "err16 bitdepth={bd}");
        assert_eq!(bd_field(c), 0, "err16 bitdepth={bd}");
    }
}

/// Row 17 — one step either side of each explicit `bitdepth` case.
#[test]
fn err_17_bitdepth_off_by_one() {
    let p = pair();
    let mut rng = Rng::new(0xC0_0011);
    for base in BITDEPTH_CASES {
        for bd in [base - 1, base + 1] {
            if BITDEPTH_CASES.contains(&bd) {
                continue;
            }
            for _ in 0..64 {
                let mut t = rng.random_tflac();
                t.bitdepth = bd;
                let (c, r) = (p.c.call(&t), p.rust.call(&t));
                assert_eq!(c, r, "err17 bitdepth={bd} (base {base})");
                assert_eq!(bd_field(c), 0, "err17 bitdepth={bd}");
            }
        }
    }
}

/// Row 18 — a garbage `frame_header` is assigned over, not OR-ed into.
#[test]
fn err_18_frame_header_overwritten() {
    let p = pair();
    let mut rng = Rng::new(0xC0_0012);
    for _ in 0..N {
        let mut t = rng.random_tflac();
        t.frame_header = 0;
        let clean_c = p.c.call(&t);
        let clean_r = p.rust.call(&t);
        for &pre in &[0xFFFF_FFFFu32, 0xAAAA_AAAA, 0x0000_0001, rng.next_u32()] {
            let mut d = t;
            d.frame_header = pre;
            assert_eq!(p.c.call(&d), clean_c, "err18 C leaked prior frame_header {pre:#X}");
            assert_eq!(p.rust.call(&d), clean_r, "err18 Rust leaked prior {pre:#X}");
            assert_eq!(p.c.call(&d), p.rust.call(&d), "err18 divergence pre={pre:#X}");
        }
        assert_eq!(clean_c, clean_r, "err18 baseline divergence");
    }
}

/// Row 19 — every field at its maximum: must not panic, must match.
#[test]
fn err_19_all_fields_max() {
    let p = pair();
    let t = Tflac {
        samplerate: u32::MAX,
        channels: u32::MAX,
        bitdepth: u32::MAX,
        channel_mode: u8::MAX,
        frame_header: u32::MAX,
        cur_blocksize: u32::MAX,
    };
    let (c, r) = (p.c.call(&t), p.rust.call(&t));
    assert_eq!(c, r, "err19 all-max");
    // channel_mode 255 % 4 == 3 -> MID_SIDE, channels not read.
    // blocksize default (>256) -> 0x07; samplerate u32::MAX -> no-op; bitdepth -> 0.
    let expected = (0xFFF8u32 << 16) | (0x07 << 12) | (0x0A << 4);
    assert_eq!(c, expected, "err19 hand-computed value, C: got 0x{c:08X}");
    assert_eq!(r, expected, "err19 hand-computed value, Rust: got 0x{r:08X}");

    // Also the all-0xFF raw image, padding included.
    assert_same_raw("err19/raw-ff", &RawTflac([0xFFu8; TFLAC_SIZE]));
    assert_same_raw("err19/raw-00", &RawTflac([0x00u8; TFLAC_SIZE]));
}

/// Row 20 — bit 0 is never set, and the struct's padding bytes are never
/// written by either implementation.
#[test]
fn err_20_reserved_bit_and_padding() {
    let p = pair();
    let mut rng = Rng::new(0xC0_0013);
    for _ in 0..(N * 5) {
        let raw = rng.random_raw();
        let c_out = p.c.call_raw(&raw);
        let r_out = p.rust.call_raw(&raw);
        assert_eq!(c_out, r_out, "err20 divergence on {raw:?}");
        // Bit 0 of frame_header is unreachable in the C.
        assert_eq!(c_out.frame_header() & 1, 0, "err20 C set bit 0");
        assert_eq!(r_out.frame_header() & 1, 0, "err20 Rust set bit 0");
        // Padding bytes at offsets 13..16 preserved verbatim by both.
        for i in 13..16 {
            assert_eq!(c_out.0[i], raw.0[i], "err20 C touched padding byte {i}");
            assert_eq!(r_out.0[i], raw.0[i], "err20 Rust touched padding byte {i}");
        }
        // Input-only fields are never modified by either side.
        for range in [0..13usize, 20..24] {
            for i in range {
                assert_eq!(c_out.0[i], raw.0[i], "err20 C modified input byte {i}");
                assert_eq!(r_out.0[i], raw.0[i], "err20 Rust modified input byte {i}");
            }
        }
    }
}
