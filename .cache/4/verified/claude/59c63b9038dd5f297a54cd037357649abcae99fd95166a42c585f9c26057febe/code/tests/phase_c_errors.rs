//! Phase C — error/rejection-path differential tests, one test per `ERRORS.md`
//! row (E1..E16). Every case calls BOTH the C `.so` and the Rust `.so` through
//! `libloading`.
//!
//! `update_frame_header` returns `void` and has no error code, no sentinel and
//! no `assert`, so "the same error/rejection" means: the same *value written to
//! `frame_header`* for the rejecting input (in FLAC a `0x0` nibble is the
//! "invalid / read it from STREAMINFO" encoding), and for the null pointer, the
//! same fatal signal.

mod common;

use common::*;

/// Base value written by line 12 before any `|=`: `0xFFF8U << 16`.
const BASE: u32 = 0xFFF8_0000;

// ---------------------------------------------------------------------------
// E1 — t == NULL: no null check in the C, so both must die with SIGSEGV.
// ---------------------------------------------------------------------------

/// Child-process probe: dereferences a null `tflac*` through one of the two
/// `.so`s. Run only when re-executed by `err_e1_null_pointer_both_segv`.
#[test]
#[ignore = "probe: re-executed as a subprocess by err_e1_null_pointer_both_segv"]
fn null_deref_probe() {
    let which = std::env::var("PROBE_TARGET").expect("PROBE_TARGET must be set");
    let d = Diff::load();
    let f = match which.as_str() {
        "c" => d.c,
        "rust" => d.rust,
        other => panic!("bad PROBE_TARGET {other}"),
    };
    eprintln!("probe {which}: calling update_frame_header(NULL)");
    // SAFETY: deliberately unsound -- this is the input under test. Both
    // implementations dereference it unchecked.
    unsafe { f(std::ptr::null_mut()) };
    eprintln!("probe {which}: SURVIVED (no fault)");
}

#[test]
fn err_e1_null_pointer_both_segv() {
    use std::os::unix::process::ExitStatusExt;

    let run = |target: &str| -> (Option<i32>, Option<i32>) {
        let out = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "null_deref_probe", "--ignored", "--nocapture", "--test-threads=1"])
            .env("PROBE_TARGET", target)
            .output()
            .expect("spawn probe");
        (out.status.code(), out.status.signal())
    };

    let (c_code, c_sig) = run("c");
    let (r_code, r_sig) = run("rust");

    eprintln!("E1 C   : code={c_code:?} signal={c_sig:?}");
    eprintln!("E1 Rust: code={r_code:?} signal={r_sig:?}");

    assert_eq!(
        c_sig, Some(11),
        "expected the C .so to die with SIGSEGV on a null tflac*, got code={c_code:?} signal={c_sig:?}"
    );
    assert_eq!(
        (r_code, r_sig),
        (c_code, c_sig),
        "Rust must reject a null tflac* exactly like the C does (same signal/exit code)"
    );
}

// ---------------------------------------------------------------------------
// E2 / E3 — `cur_blocksize` switch `default:` arms
// ---------------------------------------------------------------------------

#[test]
fn err_e2_blocksize_default_le256() {
    let mut d = Diff::load();
    let mut rng = Rng::new(0xE002_0001);
    // Everything else neutral so `frame_header` is exactly BASE | blocksize.
    for bs in 0u32..=256 {
        if BS_LITERALS.contains(&bs) {
            continue;
        }
        let t = Tflac::new(0, 0, 0, 1, bs); // mode 1 -> fixed 0x80, no channels underflow
        let c = d.check_and_get(&format!("E2 bs={bs}"), t);
        assert_eq!(
            c.frame_header & 0xF000,
            0x6000,
            "C: blocksize {bs} must take the `default:` `<=256` arm (0x6)"
        );
        // and with random surroundings
        for _ in 0..50 {
            let mut t = rng.tflac();
            t.cur_blocksize = bs;
            d.check(&format!("E2 rand bs={bs}"), t);
        }
    }
    d.finish("E2 blocksize default <=256");
}

#[test]
fn err_e3_blocksize_default_gt256() {
    let mut d = Diff::load();
    let mut rng = Rng::new(0xE003_0001);
    let fixed: [u32; 12] = [
        257, 258, 300, 575, 577, 1151, 4609, 32769, 65535, 0x7FFF_FFFF, 0x8000_0000, 0xFFFF_FFFF,
    ];
    for &bs in fixed.iter() {
        let t = Tflac::new(0, 0, 0, 1, bs);
        let c = d.check_and_get(&format!("E3 bs={bs}"), t);
        assert_eq!(
            c.frame_header & 0xF000,
            0x7000,
            "C: blocksize {bs} must take the `default:` `>256` arm (0x7)"
        );
    }
    for _ in 0..50_000 {
        let bs = 257 + rng.below(0xFFFF_FFFE - 257);
        if BS_LITERALS.contains(&bs) {
            continue;
        }
        let mut t = rng.tflac();
        t.cur_blocksize = bs;
        d.check("E3 rand bs>256", t);
    }
    d.finish("E3 blocksize default >256");
}

// ---------------------------------------------------------------------------
// E4 — samplerate: `%1000 == 0` but `/1000 >= 256` -> range check fails, no bits
// ---------------------------------------------------------------------------

#[test]
fn err_e4_samplerate_khz_out_of_range() {
    let mut d = Diff::load();
    let mut rng = Rng::new(0xE004_0001);

    let fixed: [u32; 8] = [
        256_000, 257_000, 300_000, 1_000_000, 2_000_000, 4_294_000_000, 4_294_967_000, 999_000,
    ];
    for &sr in fixed.iter() {
        assert_eq!(sr % 1000, 0);
        assert!(sr / 1000 >= 256 || SR_LITERALS.contains(&sr));
        let t = Tflac::new(sr, 0, 0, 1, 192);
        let c = d.check_and_get(&format!("E4 sr={sr}"), t);
        assert_eq!(
            c.frame_header & 0x0F00,
            0x0000,
            "C: samplerate {sr} fails the `/1000 < 256` check, so NO samplerate bits are set"
        );
    }
    for _ in 0..50_000 {
        let k = 256 + rng.below(4_294_966 - 256);
        let mut t = rng.tflac();
        t.samplerate = k * 1000;
        d.check("E4 rand k*1000, k>=256", t);
    }
    d.finish("E4 samplerate kHz out of range");
}

// ---------------------------------------------------------------------------
// E5 — samplerate: `%10 == 0` but `/10 >= 65536` -> range check fails, no bits
// ---------------------------------------------------------------------------

#[test]
fn err_e5_samplerate_dahz_out_of_range() {
    let mut d = Diff::load();
    let mut rng = Rng::new(0xE005_0001);

    let fixed: [u32; 6] = [655_360, 655_370, 700_000 + 3, 1_234_567_890, 4_294_967_290, 999_999_990];
    for &sr in fixed.iter() {
        if sr % 10 != 0 {
            continue;
        }
        let t = Tflac::new(sr, 0, 0, 1, 192);
        let c = d.check_and_get(&format!("E5 sr={sr}"), t);
        if sr % 1000 != 0 && sr >= 65536 && sr / 10 >= 65536 {
            assert_eq!(
                c.frame_header & 0x0F00,
                0x0000,
                "C: samplerate {sr} fails the `/10 < 65536` check, so NO samplerate bits are set"
            );
        }
    }
    for _ in 0..50_000 {
        let sr = (65536 + rng.below(429_496_729 - 65536)) * 10;
        let mut t = rng.tflac();
        t.samplerate = sr;
        d.check("E5 rand k*10, k>=65536", t);
    }
    d.finish("E5 samplerate daHz out of range");
}

// ---------------------------------------------------------------------------
// E6 — samplerate: no `if` branch taken at all (missing final `else`)
// ---------------------------------------------------------------------------

#[test]
fn err_e6_samplerate_no_branch_taken() {
    let mut d = Diff::load();
    let mut rng = Rng::new(0xE006_0001);

    let fixed: [u32; 8] = [
        65537, 65539, 96_001, 100_001, 123_457, 0x7FFF_FFFF, 0xFFFF_FFFE - 1, 0xFFFF_FFFF,
    ];
    for &sr in fixed.iter() {
        assert!(sr % 1000 != 0 && sr >= 65536 && sr % 10 != 0, "sr={sr} must hit E6");
        let t = Tflac::new(sr, 0, 0, 1, 192);
        let c = d.check_and_get(&format!("E6 sr={sr}"), t);
        assert_eq!(
            c.frame_header & 0x0F00,
            0x0000,
            "C: samplerate {sr} matches no branch, so NO samplerate bits are set"
        );
    }
    for _ in 0..50_000 {
        let mut sr = 65536 + rng.below(0xFFFF_FFFF - 65536);
        if sr % 10 == 0 {
            sr += 1;
        }
        let mut t = rng.tflac();
        t.samplerate = sr;
        d.check("E6 rand >=65536, %10!=0", t);
    }
    d.finish("E6 samplerate no branch taken");
}

// ---------------------------------------------------------------------------
// E7 — out-of-range enum value for `channel_mode` across the FFI boundary
// ---------------------------------------------------------------------------

#[test]
fn err_e7_channel_mode_out_of_range_enum() {
    let mut d = Diff::load();
    let mut rng = Rng::new(0xE007_0001);

    // Every raw u8 -- including 4 (TFLAC_CHANNEL_MODE_COUNT, the sentinel that
    // is *not* a real mode) and everything up to 255, none of which has a
    // matching enumerator -- must behave exactly like `mode % 4`.
    for raw in 0u16..=255 {
        let raw = raw as u8;
        for &ch in [1u32, 2, 0, 8, 0xFFFF_FFFF].iter() {
            for &bs in [192u32, 4096, 0].iter() {
                let t = Tflac::new(44100, ch, 16, raw, bs);
                let c = d.check_and_get(&format!("E7 raw={raw} ch={ch}"), t);

                // Ground truth: the folded mode must give the same result as
                // the raw one -- the switch `default:` is dead code.
                let folded = Tflac::new(44100, ch, 16, raw % 4, bs);
                let folded_c = d.check_and_get(&format!("E7 folded={}", raw % 4), folded);
                assert_eq!(
                    c.frame_header, folded_c.frame_header,
                    "C: channel_mode {raw} must be equivalent to {} (line 106 `% 4`)",
                    raw % 4
                );
            }
        }
        for _ in 0..100 {
            let mut t = rng.tflac();
            t.channel_mode = raw;
            d.check(&format!("E7 rand raw={raw}"), t);
        }
    }

    // Explicit check of the named sentinel TFLAC_CHANNEL_MODE_COUNT == 4.
    let t = Tflac::new(44100, 2, 16, 4, 4096);
    let c = d.check_and_get("E7 TFLAC_CHANNEL_MODE_COUNT", t);
    let indep = d.check_and_get("E7 INDEPENDENT", Tflac::new(44100, 2, 16, 0, 4096));
    assert_eq!(
        c.frame_header, indep.frame_header,
        "mode 4 (MODE_COUNT) folds to 0 (INDEPENDENT)"
    );

    d.finish("E7 out-of-range channel_mode enum");
}

// ---------------------------------------------------------------------------
// E8 — channels == 0: unchecked unsigned underflow
// ---------------------------------------------------------------------------

#[test]
fn err_e8_channels_zero_underflow() {
    let mut d = Diff::load();
    let mut rng = Rng::new(0xE008_0001);

    // With `channels == 0` and INDEPENDENT, `(0u32-1) << 4 == 0xFFFFFFF0` is
    // OR-ed in, so every bit except the low 4 is forced to 1.
    let t = Tflac::new(44100, 0, 16, 0, 4096);
    let c = d.check_and_get("E8 channels=0", t);
    assert_eq!(
        c.frame_header, 0xFFFF_FFF0 | BASE | 0xC000 | 0x0900 | 0x0008,
        "C: channels==0 underflows to 0xFFFFFFF0"
    );
    assert_eq!(c.frame_header, 0xFFFF_FFF8, "sanity: 0xFFFFFFF0 | 0x8 (bitdepth 16 -> 4<<1)");

    for mode_mul in 0u32..64 {
        let raw = (mode_mul * 4) as u8; // all raw modes with `% 4 == 0`
        for _ in 0..200 {
            let mut t = rng.tflac();
            t.channels = 0;
            t.channel_mode = raw;
            d.check(&format!("E8 rand raw_mode={raw}"), t);
        }
    }
    d.finish("E8 channels==0 underflow");
}

// ---------------------------------------------------------------------------
// E9 — channels overflowing the 4-bit channel-assignment field
// ---------------------------------------------------------------------------

#[test]
fn err_e9_channels_overflow_nibble() {
    let mut d = Diff::load();
    let mut rng = Rng::new(0xE009_0001);

    let big: [u32; 14] = [
        9, 10, 15, 16, 17, 32, 255, 256, 65535, 0x00FF_FFFF, 0x0FFF_FFFF, 0x1000_0000,
        0x1000_0001, 0xFFFF_FFFF,
    ];
    for &ch in big.iter() {
        let t = Tflac::new(44100, ch, 16, 0, 4096);
        let c = d.check_and_get(&format!("E9 channels={ch}"), t);
        let expected = BASE | 0xC000 | 0x0900 | (ch.wrapping_sub(1) << 4) | 0x0008;
        assert_eq!(
            c.frame_header, expected,
            "C: channels={ch} -> (channels-1)<<4 mod 2^32 OR-ed in unchecked"
        );
        for _ in 0..500 {
            let mut t = rng.tflac();
            t.channels = ch;
            t.channel_mode = rng.next_u8() & 0xFC;
            d.check(&format!("E9 rand channels={ch}"), t);
        }
    }
    for _ in 0..50_000 {
        let mut t = rng.tflac();
        t.channels = 9 + rng.below(0xFFFF_FFF0);
        t.channel_mode = rng.next_u8() & 0xFC;
        d.check("E9 rand large channels", t);
    }
    d.finish("E9 channels overflow nibble");
}

// ---------------------------------------------------------------------------
// E10 / E11 — `bitdepth` switch `default:` and oversized values
// ---------------------------------------------------------------------------

#[test]
fn err_e10_bitdepth_default() {
    let mut d = Diff::load();
    let mut rng = Rng::new(0xE010_0001);
    for bd in 0u32..=256 {
        if BD_LITERALS.contains(&bd) {
            continue;
        }
        let t = Tflac::new(44100, 0, bd, 1, 192);
        let c = d.check_and_get(&format!("E10 bd={bd}"), t);
        assert_eq!(
            c.frame_header & 0x0000_000E,
            0,
            "C: bitdepth {bd} hits `default:`, so NO sample-size bits are set"
        );
        for _ in 0..40 {
            let mut t = rng.tflac();
            t.bitdepth = bd;
            d.check(&format!("E10 rand bd={bd}"), t);
        }
    }
    d.finish("E10 bitdepth default");
}

#[test]
fn err_e11_bitdepth_oversized_no_truncation() {
    let mut d = Diff::load();
    let mut rng = Rng::new(0xE011_0001);

    // The `switch` is on `tflac_u32`, so 0x108 must NOT alias 8, etc.
    for &(alias, base) in [
        (0x108u32, 8u32),
        (0x10Cu32, 12u32),
        (0x110u32, 16u32),
        (0x114u32, 20u32),
        (0x118u32, 24u32),
        (0x120u32, 32u32),
        (0x1_0008u32 & 0xFFFF_FFFF, 8u32),
    ]
    .iter()
    {
        let a = d.check_and_get(&format!("E11 alias={alias}"), Tflac::new(44100, 0, alias, 1, 192));
        let b = d.check_and_get(&format!("E11 base={base}"), Tflac::new(44100, 0, base, 1, 192));
        assert_eq!(a.frame_header & 0x0E, 0, "C: {alias} must hit `default:`");
        assert_ne!(
            a.frame_header, b.frame_header,
            "C: bitdepth {alias} must NOT be truncated to {base}"
        );
    }

    for &bd in [33u32, 63, 64, 1000, 0x7FFF_FFFF, 0x8000_0000, 0xFFFF_FFFF].iter() {
        let t = Tflac::new(44100, 0, bd, 1, 192);
        let c = d.check_and_get(&format!("E11 bd={bd}"), t);
        assert_eq!(c.frame_header & 0x0E, 0);
        for _ in 0..1000 {
            let mut t = rng.tflac();
            t.bitdepth = bd;
            d.check(&format!("E11 rand bd={bd}"), t);
        }
    }
    d.finish("E11 bitdepth oversized, no truncation");
}

// ---------------------------------------------------------------------------
// E12 — pre-existing `frame_header` garbage is overwritten, not OR-ed
// ---------------------------------------------------------------------------

#[test]
fn err_e12_frame_header_garbage_overwritten() {
    let mut d = Diff::load();
    let mut rng = Rng::new(0xE012_0001);

    for _ in 0..20_000 {
        let base = rng.tflac();
        let mut zeroed = base;
        zeroed.frame_header = 0;
        let ref_c = d.check_and_get("E12 zeroed", zeroed);

        for &garbage in [0xFFFF_FFFFu32, 0xDEAD_BEEF, 0x0000_0001, 0x8000_0000].iter() {
            let mut t = base;
            t.frame_header = garbage;
            let c = d.check_and_get(&format!("E12 garbage=0x{garbage:08X}"), t);
            assert_eq!(
                c.frame_header, ref_c.frame_header,
                "C line 12 assigns (`=`), so prior garbage 0x{garbage:08X} must be discarded"
            );
        }
    }
    d.finish("E12 frame_header garbage overwritten");
}

// ---------------------------------------------------------------------------
// E13 — all fields at their extremes
// ---------------------------------------------------------------------------

#[test]
fn err_e13_all_min_all_max() {
    let mut d = Diff::load();

    // All zero: blocksize 0 -> default & <=256 -> 0x6; samplerate 0 -> %1000==0,
    // 0/1000 < 256 -> 0xC; mode 0 & channels 0 -> underflow; bitdepth 0 -> none.
    let zero = Tflac::default();
    let c = d.check_and_get("E13 all-zero", zero);
    assert_eq!(
        c.frame_header,
        BASE | 0x6000 | 0x0C00 | 0xFFFF_FFF0,
        "C all-zero struct"
    );

    // All 0xFF bytes.
    let max = Tflac {
        samplerate: 0xFFFF_FFFF,
        channels: 0xFFFF_FFFF,
        bitdepth: 0xFFFF_FFFF,
        channel_mode: 0xFF,
        pad: [0xFF; 3],
        frame_header: 0xFFFF_FFFF,
        cur_blocksize: 0xFFFF_FFFF,
    };
    let c = d.check_and_get("E13 all-0xFF", max);
    // blocksize default >256 -> 0x7; samplerate 0xFFFFFFFF: %1000 = 295 != 0,
    // >= 65536, %10 = 5 != 0 -> none; mode 0xFF % 4 = 3 -> MID_SIDE 0xA0;
    // bitdepth 0xFFFFFFFF -> none.
    assert_eq!(c.frame_header, BASE | 0x7000 | 0x00A0, "C all-0xFF struct");
    assert_eq!(c.pad, [0xFF; 3]);

    // Per-field min/max sweep.
    let ext: [u32; 6] = [0, 1, 2, 0x7FFF_FFFF, 0x8000_0000, 0xFFFF_FFFF];
    for &sr in ext.iter() {
        for &ch in ext.iter() {
            for &bd in ext.iter() {
                for &bs in ext.iter() {
                    for &m in [0u8, 1, 2, 3, 4, 0x7F, 0x80, 0xFF].iter() {
                        d.check("E13 extremes", Tflac::new(sr, ch, bd, m, bs));
                    }
                }
            }
        }
    }
    d.finish("E13 all-min / all-max");
}

// ---------------------------------------------------------------------------
// E14 — only `frame_header` may be written
// ---------------------------------------------------------------------------

#[test]
fn err_e14_only_frame_header_written() {
    let mut d = Diff::load();
    let mut rng = Rng::new(0xE014_0001);
    for _ in 0..100_000 {
        let input = rng.tflac();
        let c = d.check_and_get("E14 field preservation", input);
        assert_eq!(c.samplerate, input.samplerate, "C must not write samplerate");
        assert_eq!(c.channels, input.channels, "C must not write channels");
        assert_eq!(c.bitdepth, input.bitdepth, "C must not write bitdepth");
        assert_eq!(c.channel_mode, input.channel_mode, "C must not write channel_mode");
        assert_eq!(c.cur_blocksize, input.cur_blocksize, "C must not write cur_blocksize");
        assert_eq!(c.pad, input.pad, "C must not write the padding bytes");

        // and the Rust `.so` must preserve exactly the same bytes
        let mut r = input;
        unsafe { (d.rust)(&mut r as *mut Tflac) };
        let mut ib = input.as_bytes();
        let mut rb = r.as_bytes();
        ib[16..20].fill(0);
        rb[16..20].fill(0);
        assert_eq!(ib, rb, "Rust must not write anything but frame_header");
    }
    d.finish("E14 only frame_header written");
}

// ---------------------------------------------------------------------------
// E15 — one step past every documented boundary
// ---------------------------------------------------------------------------

#[test]
fn err_e15_off_by_one_boundaries() {
    let mut d = Diff::load();

    // (samplerate, expected samplerate nibble << 8)
    let sr_cases: [(u32, u32); 12] = [
        (255_000, 0x0C00),  // /1000 == 255 -> in range
        (256_000, 0x0000),  // /1000 == 256 -> rejected
        (655_350, 0x0E00),  // /10 == 65535 -> in range
        (655_360, 0x0000),  // /10 == 65536 -> rejected
        (65_535, 0x0D00),   // < 65536
        (65_536, 0x0000),   // not < 65536, %10 != 0, %1000 != 0
        (65_530, 0x0D00),   // < 65536 and %10 == 0 -> still the `< 65536` arm
        (65_540, 0x0E00),   // >= 65536, %10 == 0, /10 < 65536
        (0, 0x0C00),        // %1000 == 0, 0/1000 == 0 < 256
        (1, 0x0D00),        // %1000 != 0, < 65536
        (999, 0x0D00),
        (1_000, 0x0C00),
    ];
    for &(sr, want) in sr_cases.iter() {
        let t = Tflac::new(sr, 0, 0, 1, 192);
        let c = d.check_and_get(&format!("E15 sr={sr}"), t);
        assert_eq!(
            c.frame_header & 0x0F00, want,
            "C: samplerate {sr} boundary -> expected nibble 0x{:X}", want >> 8
        );
    }

    // (cur_blocksize, expected blocksize nibble << 12)
    let bs_cases: [(u32, u32); 12] = [
        (191, 0x6000),
        (192, 0x1000),
        (193, 0x6000),
        (255, 0x6000),
        (256, 0x8000),
        (257, 0x7000),
        (32767, 0x7000),
        (32768, 0xF000),
        (32769, 0x7000),
        (0, 0x6000),
        (1, 0x6000),
        (0xFFFF_FFFF, 0x7000),
    ];
    for &(bs, want) in bs_cases.iter() {
        let t = Tflac::new(0, 0, 0, 1, bs);
        let c = d.check_and_get(&format!("E15 bs={bs}"), t);
        assert_eq!(
            c.frame_header & 0xF000, want,
            "C: cur_blocksize {bs} boundary -> expected nibble 0x{:X}", want >> 12
        );
    }

    // (bitdepth, expected sample-size bits) -- one below/above each literal
    let bd_cases: [(u32, u32); 14] = [
        (7, 0), (8, 1 << 1), (9, 0),
        (11, 0), (12, 2 << 1), (13, 0),
        (15, 0), (16, 4 << 1), (17, 0),
        (19, 0), (20, 5 << 1),
        (23, 0), (24, 6 << 1),
        (32, 7 << 1),
    ];
    for &(bd, want) in bd_cases.iter() {
        let t = Tflac::new(0, 0, bd, 1, 192);
        let c = d.check_and_get(&format!("E15 bd={bd}"), t);
        assert_eq!(c.frame_header & 0x0E, want, "C: bitdepth {bd} boundary");
    }

    // (channels, mode 0) around the 4-bit field boundary
    for ch in 0u32..=17 {
        let t = Tflac::new(44100, ch, 16, 0, 4096);
        let c = d.check_and_get(&format!("E15 ch={ch}"), t);
        assert_eq!(
            c.frame_header,
            BASE | 0xC000 | 0x0900 | (ch.wrapping_sub(1) << 4) | 0x0008,
            "C: channels={ch}"
        );
    }

    // channel_mode around the `% 4` boundary
    for m in 0u8..=8 {
        d.check(&format!("E15 mode={m}"), Tflac::new(44100, 2, 16, m, 4096));
    }

    d.finish("E15 off-by-one boundaries");
}

// ---------------------------------------------------------------------------
// E16 — unaligned struct pointer
// ---------------------------------------------------------------------------

#[test]
fn err_e16_unaligned_pointer() {
    let d = Diff::load();
    let mut rng = Rng::new(0xE016_0001);

    // 24-byte struct placed at an odd address inside a larger buffer.
    #[repr(C, align(8))]
    struct Buf([u8; 32]);

    let mut cases = 0usize;
    for _ in 0..20_000 {
        let input = rng.tflac();
        let ib = input.as_bytes();

        for off in [1usize, 2, 3] {
            let mut cbuf = Buf([0xCD; 32]);
            let mut rbuf = Buf([0xCD; 32]);
            cbuf.0[off..off + 24].copy_from_slice(&ib);
            rbuf.0[off..off + 24].copy_from_slice(&ib);

            // SAFETY: deliberately misaligned (UB in both C and Rust, but a real
            // input an FFI caller can construct; x86-64 permits the access).
            unsafe {
                (d.c)(cbuf.0.as_mut_ptr().add(off) as *mut Tflac);
                (d.rust)(rbuf.0.as_mut_ptr().add(off) as *mut Tflac);
            }
            assert_eq!(
                cbuf.0, rbuf.0,
                "unaligned (+{off}) result differs for {input:?}"
            );
            // Nothing outside the struct may be touched.
            assert!(cbuf.0[..off].iter().all(|&b| b == 0xCD));
            assert!(cbuf.0[off + 24..].iter().all(|&b| b == 0xCD));
            assert!(rbuf.0[..off].iter().all(|&b| b == 0xCD));
            assert!(rbuf.0[off + 24..].iter().all(|&b| b == 0xCD));
            cases += 1;
        }
    }
    eprintln!("E16 unaligned pointer: {cases} cases, 0 mismatches");
}
