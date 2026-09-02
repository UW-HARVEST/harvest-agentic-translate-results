//! Phase C — error-path / rejection differential tests.
//!
//! One `#[test]` per row of `ERRORS.md`. The C has no error return at all
//! (`void` function, zero `return`/`assert`/null-check), so each row asserts
//! the *exact* observable the C produces for that invalid/degenerate input —
//! the resulting bit pattern, or, for row 20, the exact process death — rather
//! than merely "both did something".

mod common;

use common::*;
use std::process::Command;

/// Extract the four bit fields the C writes, so assertions name the actual
/// nibble rather than a magic constant.
fn nibbles(h: u32) -> (u32, u32, u32, u32) {
    (
        (h >> 12) & 0xF, // block size
        (h >> 8) & 0xF,  // sample rate
        (h >> 4) & 0xF,  // channel assignment
        (h >> 1) & 0x7,  // bit depth
    )
}

/// A struct whose non-tested axes are all "quiet": they contribute no bits, so
/// the field under test is observable in isolation.
fn quiet() -> Tflac {
    Tflac {
        samplerate: 65537,  // B-d6: no sample-rate bits
        channels: 1,        // independent, (1-1)<<4 == 0
        bitdepth: 7,        // default: no bit-depth bits
        channel_mode: 0,    // independent
        frame_header: 0xDEAD_BEEF, // must be overwritten
        cur_blocksize: 0,   // default <= 256 -> 0x6
    }
}

// --- Row 1: cur_blocksize default && <= 256 ---------------------------------

#[test]
fn err_01_blocksize_default_le_256() {
    let p = Pair::load();
    let mut rng = Rng::new(0x1001);
    let mut n = 0;
    for bs in 0u32..=256 {
        if BLOCKSIZES.contains(&bs) {
            continue;
        }
        let mut t = quiet();
        t.cur_blocksize = bs;
        p.check(t);
        assert_eq!(nibbles(p.c_header(t)).0, 0x06, "blocksize {bs}");
        n += 1;
        // and with randomized other axes
        let mut t2 = rand_other_axes(&mut rng);
        t2.cur_blocksize = bs;
        p.check(t2);
    }
    assert!(n > 240, "only {n} values probed");
}

// --- Row 2: cur_blocksize default && > 256 ----------------------------------

#[test]
fn err_02_blocksize_default_gt_256() {
    let p = Pair::load();
    let mut rng = Rng::new(0x1002);
    for _ in 0..5000 {
        let bs = loop {
            let c = rng.range_u32(257, u32::MAX);
            if !BLOCKSIZES.contains(&c) {
                break c;
            }
        };
        let mut t = quiet();
        t.cur_blocksize = bs;
        p.check(t);
        assert_eq!(nibbles(p.c_header(t)).0, 0x07, "blocksize {bs}");
    }
    for bs in [257u32, 258, 32769, 100_000, u32::MAX - 1] {
        let mut t = quiet();
        t.cur_blocksize = bs;
        p.check(t);
        assert_eq!(nibbles(p.c_header(t)).0, 0x07, "blocksize {bs}");
    }
}

// --- Row 3: cur_blocksize == 0 ----------------------------------------------

#[test]
fn err_03_blocksize_zero() {
    let p = Pair::load();
    let mut t = quiet();
    t.cur_blocksize = 0;
    p.check(t);
    assert_eq!(nibbles(p.c_header(t)).0, 0x06);
    assert_eq!(p.c_header(t), BASE | (0x06 << 12));
}

// --- Row 4: cur_blocksize == u32::MAX ---------------------------------------

#[test]
fn err_04_blocksize_u32max() {
    let p = Pair::load();
    let mut t = quiet();
    t.cur_blocksize = u32::MAX;
    p.check(t);
    assert_eq!(nibbles(p.c_header(t)).0, 0x07);
    assert_eq!(p.c_header(t), BASE | (0x07 << 12));
}

// --- Row 5: samplerate %1000==0 && /1000 >= 256 -> no bits ------------------

#[test]
fn err_05_samplerate_khz_overflow() {
    let p = Pair::load();
    let mut rng = Rng::new(0x1005);
    let mut probes: Vec<u32> = vec![256_000, 257_000, 1_000_000, 4_294_967_000];
    for _ in 0..3000 {
        let k = rng.range_u32(256, 4_294_967);
        probes.push(k * 1000);
    }
    for &sr in &probes {
        assert_eq!(sr % 1000, 0);
        assert!(sr / 1000 >= 256);
        assert!(!SAMPLERATES.contains(&sr), "{sr} is enumerated");
        let mut t = quiet();
        t.samplerate = sr;
        p.check(t);
        assert_eq!(nibbles(p.c_header(t)).1, 0x00, "samplerate {sr}");
    }
}

// --- Row 6: samplerate B-d5 -> no bits --------------------------------------

#[test]
fn err_06_samplerate_dahz_overflow() {
    let p = Pair::load();
    let mut rng = Rng::new(0x1006);
    let mut probes: Vec<u32> = vec![655_370, 655_380, 4_294_967_290];
    for _ in 0..3000 {
        let k = rng.range_u32(65536, 429_496_729);
        let sr = k * 10;
        if sr % 1000 != 0 {
            probes.push(sr);
        }
    }
    for &sr in &probes {
        assert_ne!(sr % 1000, 0);
        assert!(sr >= 65536 && sr % 10 == 0 && sr / 10 >= 65536);
        let mut t = quiet();
        t.samplerate = sr;
        p.check(t);
        assert_eq!(nibbles(p.c_header(t)).1, 0x00, "samplerate {sr}");
    }
}

// --- Row 7: samplerate B-d6 -> no branch taken ------------------------------

#[test]
fn err_07_samplerate_unrepresentable() {
    let p = Pair::load();
    let mut rng = Rng::new(0x1007);
    let mut probes: Vec<u32> = vec![65537, 65539, u32::MAX];
    for _ in 0..3000 {
        let sr = rng.range_u32(65536, u32::MAX);
        if sr % 1000 != 0 && sr % 10 != 0 {
            probes.push(sr);
        }
    }
    for &sr in &probes {
        assert!(sr >= 65536 && sr % 1000 != 0 && sr % 10 != 0);
        let mut t = quiet();
        t.samplerate = sr;
        p.check(t);
        assert_eq!(nibbles(p.c_header(t)).1, 0x00, "samplerate {sr}");
    }
}

// --- Row 8: samplerate == 0 -> 0x0C (NOT zero) ------------------------------

#[test]
fn err_08_samplerate_zero() {
    let p = Pair::load();
    let mut t = quiet();
    t.samplerate = 0;
    p.check(t);
    // 0 % 1000 == 0 and 0 / 1000 == 0 < 256, so the C DOES set 0x0C here.
    assert_eq!(nibbles(p.c_header(t)).1, 0x0C, "samplerate 0");
}

// --- Row 9: samplerate == u32::MAX ------------------------------------------

#[test]
fn err_09_samplerate_u32max() {
    let p = Pair::load();
    for sr in [u32::MAX, u32::MAX - 2] {
        let mut t = quiet();
        t.samplerate = sr;
        p.check(t);
        assert_eq!(nibbles(p.c_header(t)).1, 0x00, "samplerate {sr}");
    }
}

// --- Row 10: samplerate 65535 / 65536 boundary ------------------------------

#[test]
fn err_10_samplerate_65536_boundary() {
    let p = Pair::load();
    let expect: &[(u32, u32)] = &[
        (65534, 0x0D), // %1000!=0, <65536
        (65535, 0x0D),
        (65536, 0x00), // %1000==536!=0, >=65536, %10==6!=0
        (65537, 0x00),
    ];
    for &(sr, want) in expect {
        let mut t = quiet();
        t.samplerate = sr;
        p.check(t);
        assert_eq!(nibbles(p.c_header(t)).1, want, "samplerate {sr}");
    }
}

// --- Row 11: samplerate 255000 / 256000 boundary ----------------------------

#[test]
fn err_11_samplerate_256khz_boundary() {
    let p = Pair::load();
    let expect: &[(u32, u32)] = &[
        (254_000, 0x0C),
        (255_000, 0x0C), // /1000 == 255 < 256
        (256_000, 0x00), // /1000 == 256, not < 256
        (257_000, 0x00),
    ];
    for &(sr, want) in expect {
        let mut t = quiet();
        t.samplerate = sr;
        p.check(t);
        assert_eq!(nibbles(p.c_header(t)).1, want, "samplerate {sr}");
    }
}

// --- Row 12: samplerate 655350 / 655360 boundary ----------------------------

#[test]
fn err_12_samplerate_655360_boundary() {
    let p = Pair::load();
    let expect: &[(u32, u32)] = &[
        (655_340, 0x0E), // /10 == 65534 < 65536
        (655_350, 0x0E), // /10 == 65535 < 65536
        (655_360, 0x00), // /10 == 65536, not < 65536
        (655_370, 0x00),
    ];
    for &(sr, want) in expect {
        assert_ne!(sr % 1000, 0);
        let mut t = quiet();
        t.samplerate = sr;
        p.check(t);
        assert_eq!(nibbles(p.c_header(t)).1, want, "samplerate {sr}");
    }
}

// --- Row 13: out-of-range enum value for channel_mode -----------------------

#[test]
fn err_13_channel_mode_out_of_range_enum() {
    let p = Pair::load();
    let mut rng = Rng::new(0x1013);

    // TFLAC_CHANNEL_MODE_COUNT (4) and every other byte with no valid variant.
    for m in 4u16..=255 {
        let m = m as u8;
        let mut t = quiet();
        t.channel_mode = m;
        t.channels = 1;
        p.check(t);

        // It must behave EXACTLY like the aliased in-range mode, because the C
        // reduces with `% 4` before the switch (so `default:` is unreachable).
        let mut aliased = t;
        aliased.channel_mode = m % 4;
        assert_eq!(
            p.c_header(t),
            p.c_header(aliased),
            "channel_mode {m} must alias to {}",
            m % 4
        );

        let want = match m % 4 {
            0 => 0x00, // (channels 1 - 1) << 4
            1 => 0x08,
            2 => 0x09,
            _ => 0x0A,
        };
        assert_eq!(nibbles(p.c_header(t)).2, want, "channel_mode {m}");
    }

    // Same, with the other axes randomized.
    for _ in 0..5000 {
        let mut t = rand_other_axes(&mut rng);
        t.channel_mode = rng.range_u32(4, 255) as u8;
        p.check(t);
        let mut aliased = t;
        aliased.channel_mode = t.channel_mode % 4;
        assert_eq!(p.c_header(t), p.c_header(aliased));
    }
}

// --- Row 14: channels == 0 unsigned underflow -------------------------------

#[test]
fn err_14_channels_zero_underflow() {
    let p = Pair::load();
    let mut rng = Rng::new(0x1014);

    let mut t = quiet();
    t.channels = 0;
    t.channel_mode = 0;
    p.check(t);
    let h = p.c_header(t);
    // (0u32 - 1) << 4 == 0xFFFFFFF0, OR-ed over BASE|0x6000 -> 0xFFFFFFF0 | low bits
    assert_eq!(h, (BASE | (0x06 << 12)) | 0xFFFF_FFF0, "got 0x{h:08X}");
    assert_eq!(h & 0xFFFF_FFF0, 0xFFFF_FFF0);

    for _ in 0..5000 {
        let mut t = rand_other_axes(&mut rng);
        t.channels = 0;
        t.channel_mode = rng.range_u32(0, 63) as u8 * 4; // %4 == 0
        p.check(t);
        assert_eq!(p.c_header(t) & 0xFFFF_FFF0, 0xFFFF_FFF0);
    }

    // channels == 0 in a NON-independent mode must be ignored entirely.
    for residue in 1u8..4 {
        let mut a = quiet();
        a.channels = 0;
        a.channel_mode = residue;
        let mut b = quiet();
        b.channels = 12345;
        b.channel_mode = residue;
        p.check(a);
        p.check(b);
        assert_eq!(
            p.c_header(a),
            p.c_header(b),
            "channels must be ignored for mode {residue}"
        );
    }
}

// --- Row 15: channels out of the valid 1..=8 range --------------------------

#[test]
fn err_15_channels_out_of_range() {
    let p = Pair::load();
    let mut rng = Rng::new(0x1015);
    for ch in 9u32..=1000 {
        let mut t = quiet();
        t.channels = ch;
        t.channel_mode = 0;
        p.check(t);
        assert_eq!(
            p.c_header(t),
            (BASE | (0x06 << 12)) | ch.wrapping_sub(1).wrapping_shl(4),
            "channels {ch}"
        );
    }
    for _ in 0..5000 {
        let mut t = rand_other_axes(&mut rng);
        t.channel_mode = 0;
        t.channels = rng.range_u32(9, u32::MAX);
        p.check(t);
    }
}

// --- Row 16: channels == u32::MAX (shift off the top) -----------------------

#[test]
fn err_16_channels_u32max() {
    let p = Pair::load();
    for ch in [u32::MAX, u32::MAX - 1, 0x1000_0000, 0x2000_0000] {
        let mut t = quiet();
        t.channels = ch;
        t.channel_mode = 0;
        p.check(t);
        assert_eq!(
            p.c_header(t),
            (BASE | (0x06 << 12)) | ch.wrapping_sub(1).wrapping_shl(4),
            "channels 0x{ch:08X}"
        );
    }
    // Explicit documented value for u32::MAX: (0xFFFFFFFE << 4) == 0xFFFFFFE0.
    let mut t = quiet();
    t.channels = u32::MAX;
    t.channel_mode = 0;
    assert_eq!(p.c_header(t) & 0xFFFF_FFE0, 0xFFFF_FFE0);
}

// --- Row 17: bitdepth default -> no bits ------------------------------------

#[test]
fn err_17_bitdepth_default() {
    let p = Pair::load();
    let mut rng = Rng::new(0x1017);
    for bd in 0u32..=200 {
        if BITDEPTHS.contains(&bd) {
            continue;
        }
        let mut t = quiet();
        t.bitdepth = bd;
        p.check(t);
        assert_eq!(nibbles(p.c_header(t)).3, 0, "bitdepth {bd}");
        assert_eq!(p.c_header(t) & 0xF, 0, "bitdepth {bd} low bits");
    }
    for _ in 0..5000 {
        let bd = loop {
            let c = rng.next_u32();
            if !BITDEPTHS.contains(&c) {
                break c;
            }
        };
        let mut t = quiet();
        t.bitdepth = bd;
        p.check(t);
        assert_eq!(nibbles(p.c_header(t)).3, 0, "bitdepth {bd}");
    }
}

// --- Row 18: bitdepth == 0 --------------------------------------------------

#[test]
fn err_18_bitdepth_zero() {
    let p = Pair::load();
    let mut t = quiet();
    t.bitdepth = 0;
    p.check(t);
    assert_eq!(nibbles(p.c_header(t)).3, 0);
}

// --- Row 19: bitdepth one step past the range -------------------------------

#[test]
fn err_19_bitdepth_past_range() {
    let p = Pair::load();
    for bd in [7u32, 9, 11, 13, 15, 17, 19, 21, 23, 25, 31, 33, u32::MAX] {
        let mut t = quiet();
        t.bitdepth = bd;
        p.check(t);
        assert_eq!(nibbles(p.c_header(t)).3, 0, "bitdepth {bd}");
    }
}

// --- Row 20: NULL pointer ---------------------------------------------------

/// The C dereferences `t` with no null check, so `update_frame_header(NULL)` is
/// UB that manifests as a fatal signal. Run each side in a fresh child process
/// and assert they die the SAME way (same signal), i.e. the Rust does not
/// silently return, unwind, or produce a different termination.
#[test]
fn err_20_null_pointer() {
    let exe = std::env::current_exe().expect("current_exe");

    let run = |which: &str| -> (Option<i32>, Option<i32>) {
        let out = Command::new(&exe)
            .arg("--exact")
            .arg("err_20_null_pointer_child")
            .arg("--nocapture")
            .arg("--ignored")
            .env("NULL_DEREF_TARGET", which)
            .output()
            .expect("spawn child");
        #[cfg(unix)]
        {
            use std::os::unix::process::ExitStatusExt;
            (out.status.code(), out.status.signal())
        }
        #[cfg(not(unix))]
        {
            (out.status.code(), None)
        }
    };

    let (c_code, c_sig) = run("c");
    let (r_code, r_sig) = run("rust");

    assert_eq!(
        (c_code, c_sig),
        (r_code, r_sig),
        "null-pointer termination differs: C exit={c_code:?} signal={c_sig:?} vs \
         Rust exit={r_code:?} signal={r_sig:?}"
    );
    // Both must actually die on a signal (SIGSEGV = 11), not return normally.
    assert_eq!(c_sig, Some(11), "expected SIGSEGV from the C, got {c_sig:?}");
    assert_eq!(
        r_sig,
        Some(11),
        "expected SIGSEGV from the Rust, got {r_sig:?}"
    );
}

/// Child half of `err_20_null_pointer`: dereferences NULL in exactly one of the
/// two libraries and is expected to be killed by a signal.
#[test]
#[ignore]
fn err_20_null_pointer_child() {
    let which = std::env::var("NULL_DEREF_TARGET").unwrap_or_default();
    let p = Pair::load();
    let f = match which.as_str() {
        "c" => p.c,
        "rust" => p.rust,
        other => panic!("NULL_DEREF_TARGET must be c|rust, got {other:?}"),
    };
    unsafe { f(std::ptr::null_mut()) };
    // Unreachable if the callee really dereferences its argument.
    eprintln!("returned from NULL call into {which} — no fault");
    std::process::exit(0);
}
