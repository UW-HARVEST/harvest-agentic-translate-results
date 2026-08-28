//! Phase C — error/rejection-path differential tests, one test per
//! `ERRORS.md` row.
//!
//! `update_frame_header` has no `return`, no error code and no `assert`, so its
//! rejections are *silent*: `default:` arms and guarded `if`s that contribute no
//! bits, plus one unsigned-underflow path and one UB path (NULL). Each test
//! asserts the two `.so`s agree on the exact resulting record — and pins the
//! specific bit-field value the C produces, so "both wrong the same way" cannot
//! be mistaken for success.

mod common;

use common::*;

// --- bit-field accessors for the FLAC frame header --------------------------

fn bs_nibble(fh: u32) -> u32 {
    (fh >> 12) & 0xF
}
fn sr_nibble(fh: u32) -> u32 {
    (fh >> 8) & 0xF
}
fn ch_nibble(fh: u32) -> u32 {
    (fh >> 4) & 0xF
}
fn bd_field(fh: u32) -> u32 {
    (fh >> 1) & 0x7
}

/// A "clean" base whose other fields never disturb the nibble under test:
/// samplerate 44100 (`0x9`), channels 2 + mode 0 (`0x1`), bitdepth 16 (`0x4`),
/// blocksize 4096 (`0xC`).
fn clean() -> Input {
    Input::new(44100, 2, 16, 0, 4096)
}

// ===========================================================================
// Row 1 — t == NULL
// ===========================================================================

const SIGSEGV: i32 = 11;
const SIGABRT: i32 = 6;
/// Exit code the helper uses if the call returns without faulting at all.
const NO_FAULT: i32 = 42;

/// Run the NULL-deref helper in a child process against one shared object and
/// return `(exit_code, signal)`. `ulimit -c 0` keeps the fault from dumping core.
fn run_null_child(lib: &std::path::Path) -> (Option<i32>, Option<i32>) {
    use std::os::unix::process::ExitStatusExt;
    use std::process::Command;

    let exe = std::env::current_exe().expect("current_exe");
    let out = Command::new("sh")
        .arg("-c")
        .arg(
            "ulimit -c 0 2>/dev/null; exec \"$0\" --exact err_01_null_deref_helper --ignored \
             --nocapture",
        )
        .arg(&exe)
        .env("NULL_LIB", lib)
        .env("RUST_BACKTRACE", "0")
        .output()
        .expect("spawn child");
    (out.status.code(), out.status.signal())
}

/// Both implementations dereference `t` without a NULL check (`lib.c:12`), so
/// both must fault — and the *shipped* Rust artifact must fault with the exact
/// same signal as the C.
#[test]
fn err_01_null_pointer_both_segv() {
    let c = run_null_child(&c_lib_path());
    assert_eq!(
        c,
        (None, Some(SIGSEGV)),
        "expected the C library to die of SIGSEGV on a NULL pointer, got {c:?}"
    );

    // (1) The release cdylib is what an external caller loads. It must match the
    //     C bit-for-bit, signal included.
    let release = rust_lib_release_path().expect(
        "release cdylib not built — run `cargo build --release --offline` before `cargo test`",
    );
    let r_release = run_null_child(&release);
    assert_eq!(
        c, r_release,
        "the release Rust .so diverged from C on a NULL pointer: C = {c:?}, Rust = {r_release:?}"
    );

    // (2) The .so actually under test must fault too, never return normally.
    let under_test = rust_lib_path();
    let r = run_null_child(&under_test);
    assert_ne!(
        r.0,
        Some(NO_FAULT),
        "Rust returned normally from a NULL pointer where C faulted"
    );
    assert!(r.1.is_some(), "Rust did not terminate by signal: {r:?}");
    if r != c {
        // The only tolerated difference: a debug-assertions build, where rustc
        // injects its own "null pointer dereference occurred" check that panics
        // (and aborts, since it cannot unwind out of `extern "C"`) *before* the
        // faulting store. That instrumentation is absent from the release build
        // verified in (1).
        assert!(
            has_rustc_ub_checks(&under_test),
            "Rust .so {} diverged from C on NULL ({r:?} vs {c:?}) and it is NOT a \
             debug-assertions build — this is a real divergence",
            under_test.display()
        );
        assert_eq!(
            r.1,
            Some(SIGABRT),
            "unexpected signal from the debug-assertions build: {r:?}"
        );
    }
}

#[test]
#[ignore = "helper for err_01: intentionally dereferences NULL; run in a child process"]
fn err_01_null_deref_helper() {
    let path = std::path::PathBuf::from(std::env::var("NULL_LIB").expect("NULL_LIB"));
    let f = load_symbol(&path);
    unsafe { f(std::ptr::null_mut()) };
    // Reaching here means no fault occurred — report it distinctly.
    std::process::exit(NO_FAULT);
}

// ===========================================================================
// Rows 2-4 — cur_blocksize rejections
// ===========================================================================

/// Row 2 — blocksize matches no case and is `<= 256` → nibble `0x6`.
#[test]
fn err_02_blocksize_default_le_256() {
    let d = Diff::load();
    let mut checked = 0;
    for bs in 0..=256u32 {
        if BLOCKSIZE_CASES.contains(&bs) {
            continue;
        }
        let fh = d.check(&Input { cur_blocksize: bs, ..clean() });
        assert_eq!(bs_nibble(fh), 0x6, "blocksize {bs} must yield nibble 0x6");
        checked += 1;
    }
    assert_eq!(checked, 257 - 2, "192 and 256 are the only case labels in 0..=256");
    // including the explicitly invalid zero
    assert_eq!(bs_nibble(d.check(&Input { cur_blocksize: 0, ..clean() })), 0x6);
}

/// Row 3 — blocksize matches no case and is `> 256` → nibble `0x7`.
#[test]
fn err_03_blocksize_default_gt_256() {
    let d = Diff::load();
    let mut rng = Rng::new(SEED ^ 0xC003);
    for _ in 0..iters(20_000) {
        let bs = rng.range_u32(257, u32::MAX);
        if BLOCKSIZE_CASES.contains(&bs) {
            continue;
        }
        let fh = d.check(&Input { cur_blocksize: bs, ..clean() });
        assert_eq!(bs_nibble(fh), 0x7, "blocksize {bs} must yield nibble 0x7");
    }
    for bs in [257u32, 258, 1000, 65535, 65536, 0x8000_0000, u32::MAX - 1, u32::MAX] {
        assert_eq!(bs_nibble(d.check(&Input { cur_blocksize: bs, ..clean() })), 0x7, "bs={bs}");
    }
}

/// Row 4 — one step either side of the `<= 256` boundary at `lib.c:55`.
#[test]
fn err_04_blocksize_boundary_255_256_257() {
    let d = Diff::load();
    assert_eq!(bs_nibble(d.check(&Input { cur_blocksize: 255, ..clean() })), 0x6);
    // 256 is a *case* label, so it does NOT reach the ternary.
    assert_eq!(bs_nibble(d.check(&Input { cur_blocksize: 256, ..clean() })), 0x8);
    assert_eq!(bs_nibble(d.check(&Input { cur_blocksize: 257, ..clean() })), 0x7);
    // the other case labels ± 1 all fall to the default arm
    for &c in BLOCKSIZE_CASES.iter() {
        for bs in [c - 1, c + 1] {
            if BLOCKSIZE_CASES.contains(&bs) {
                continue;
            }
            let want = if bs <= 256 { 0x6 } else { 0x7 };
            assert_eq!(bs_nibble(d.check(&Input { cur_blocksize: bs, ..clean() })), want, "bs={bs}");
        }
    }
}

// ===========================================================================
// Rows 5-11 — samplerate rejections
// ===========================================================================

/// Row 5 — `%1000 == 0` but `/1000 >= 256`: the inner `if` fails, no bits.
#[test]
fn err_05_samplerate_khz_out_of_range() {
    let d = Diff::load();
    let mut rng = Rng::new(SEED ^ 0xC005);
    for _ in 0..iters(20_000) {
        let sr = rng.range_u32(256, u32::MAX / 1000) * 1000;
        if SAMPLERATE_CASES.contains(&sr) {
            continue;
        }
        let fh = d.check(&Input { samplerate: sr, ..clean() });
        assert_eq!(sr_nibble(fh), 0x0, "samplerate {sr} must contribute no bits");
    }
}

/// Row 6 — `/1000 == 256` exactly (one step past `< 256`).
#[test]
fn err_06_samplerate_khz_boundary() {
    let d = Diff::load();
    assert_eq!(sr_nibble(d.check(&Input { samplerate: 255_000, ..clean() })), 0xC);
    assert_eq!(sr_nibble(d.check(&Input { samplerate: 256_000, ..clean() })), 0x0);
    assert_eq!(sr_nibble(d.check(&Input { samplerate: 257_000, ..clean() })), 0x0);
}

/// Row 7 — `%10 == 0` but `/10 >= 65536`: the inner `if` fails, no bits.
#[test]
fn err_07_samplerate_dahz_out_of_range() {
    let d = Diff::load();
    let mut rng = Rng::new(SEED ^ 0xC007);
    for _ in 0..iters(20_000) {
        let sr = rng.range_u32(65536, u32::MAX / 10) * 10;
        if sr % 1000 == 0 || SAMPLERATE_CASES.contains(&sr) {
            continue;
        }
        let fh = d.check(&Input { samplerate: sr, ..clean() });
        assert_eq!(sr_nibble(fh), 0x0, "samplerate {sr} must contribute no bits");
    }
    assert_eq!(sr_nibble(d.check(&Input { samplerate: 655_360, ..clean() })), 0x0);
}

/// Row 8 — `/10 == 65536` exactly (one step past `< 65536`).
#[test]
fn err_08_samplerate_dahz_boundary() {
    let d = Diff::load();
    assert_eq!(sr_nibble(d.check(&Input { samplerate: 655_350, ..clean() })), 0xE);
    assert_eq!(sr_nibble(d.check(&Input { samplerate: 655_360, ..clean() })), 0x0);
    assert_eq!(sr_nibble(d.check(&Input { samplerate: 655_370, ..clean() })), 0x0);
}

/// Row 9 — `%1000 != 0`, `>= 65536`, `%10 != 0`: no `else`, no bits.
#[test]
fn err_09_samplerate_unrepresentable() {
    let d = Diff::load();
    let mut rng = Rng::new(SEED ^ 0xC009);
    for _ in 0..iters(20_000) {
        let sr = rng.range_u32(65536, u32::MAX);
        if sr % 10 == 0 || sr % 1000 == 0 || SAMPLERATE_CASES.contains(&sr) {
            continue;
        }
        let fh = d.check(&Input { samplerate: sr, ..clean() });
        assert_eq!(sr_nibble(fh), 0x0, "samplerate {sr} must contribute no bits");
    }
    for sr in [65537u32, 65539, 100_001, u32::MAX, u32::MAX - 1] {
        assert_eq!(sr_nibble(d.check(&Input { samplerate: sr, ..clean() })), 0x0, "sr={sr}");
    }
}

/// Row 10 — the `< 65536` boundary at `lib.c:97`.
#[test]
fn err_10_samplerate_65536_boundary() {
    let d = Diff::load();
    assert_eq!(sr_nibble(d.check(&Input { samplerate: 65_535, ..clean() })), 0xD);
    // 65536 % 1000 == 536, 65536 < 65536 is false, 65536 % 10 == 6 -> no bits.
    assert_eq!(sr_nibble(d.check(&Input { samplerate: 65_536, ..clean() })), 0x0);
    assert_eq!(sr_nibble(d.check(&Input { samplerate: 65_537, ..clean() })), 0x0);
    // 65540 is a multiple of 10 -> 0xE
    assert_eq!(sr_nibble(d.check(&Input { samplerate: 65_540, ..clean() })), 0xE);
}

/// Row 11 — `samplerate == 0` is silently accepted as `0xC`, not rejected.
#[test]
fn err_11_samplerate_zero() {
    let d = Diff::load();
    let fh = d.check(&Input { samplerate: 0, ..clean() });
    assert_eq!(sr_nibble(fh), 0xC, "samplerate 0 takes the %1000==0 && /1000<256 path");
    // with every other field also at an extreme
    let mut rng = Rng::new(SEED ^ 0xC011);
    let pools = Pools::new();
    for _ in 0..iters(5000) {
        let mut inp = random_input(&mut rng, &pools);
        inp.samplerate = 0;
        d.check(&inp);
    }
}

// ===========================================================================
// Rows 12-14 — channels
// ===========================================================================

/// Row 12 — `channels == 0` underflows: `0u32 - 1 == 0xFFFF_FFFF`, `<< 4`.
#[test]
fn err_12_channels_zero_underflow() {
    let d = Diff::load();
    let fh = d.check(&Input { channels: 0, channel_mode: 0, ..clean() });
    // 0xFFF80000 | 0xC000 | 0x900 | 0xFFFFFFF0 | 0x8
    assert_eq!(fh, 0xFFFF_FFF8, "channels==0 must OR in 0xFFFFFFF0; got 0x{fh:08X}");
    assert_eq!(fh & 0xFFFF_FFF0, 0xFFFF_FFF0);

    // Every mode whose %4 == 0 underflows the same way; modes 1..3 never read
    // `channels`, so their result is identical to a well-formed channels value.
    for m in 0..=255u8 {
        let got = d.check(&Input { channels: 0, channel_mode: m, ..clean() });
        if m % 4 == 0 {
            assert_eq!(got & 0xFFFF_FFF0, 0xFFFF_FFF0, "mode {m} should underflow");
        } else {
            let with_two = d.check(&Input { channels: 2, channel_mode: m, ..clean() });
            assert_eq!(got, with_two, "mode {m} must ignore channels, so it cannot underflow");
            assert_eq!(got & 0xFFF8_0000, 0xFFF8_0000, "mode {m}: sync code intact");
            assert_eq!(got >> 20, 0xFFF, "mode {m} must not smear the high bits");
        }
    }
}

/// Row 13 — `channels > 8` is not range-checked and overflows the 4-bit field.
#[test]
fn err_13_channels_out_of_range() {
    let d = Diff::load();
    // channels 17 -> (17-1)<<4 == 0x100, i.e. it corrupts the samplerate nibble.
    let fh = d.check(&Input { channels: 17, channel_mode: 0, ..clean() });
    assert_eq!(fh, 0xFFF8_0000 | 0xC000 | 0x900 | 0x100 | 0x8, "got 0x{fh:08X}");
    // With samplerate 8000 (nibble 0x4, bit 8 clear) the bleed is observable:
    // the samplerate nibble reads back as 0x5 instead of 0x4.
    let fh = d.check(&Input::new(8000, 17, 16, 0, 4096));
    assert_eq!(fh, 0xFFF8_C508, "got 0x{fh:08X}");
    assert_eq!(sr_nibble(fh), 0x5, "channel overflow bleeds into the samplerate nibble");
    assert_eq!(sr_nibble(d.check(&Input::new(8000, 2, 16, 0, 4096))), 0x4, "control: no bleed");

    let mut rng = Rng::new(SEED ^ 0xC013);
    for _ in 0..iters(20_000) {
        let ch = rng.range_u32(9, u32::MAX);
        d.check(&Input { channels: ch, channel_mode: 0, ..clean() });
    }
    for ch in [9u32, 16, 17, 255, 256, 65535, 0x0FFF_FFFF, 0x1000_0000, u32::MAX] {
        d.check(&Input { channels: ch, channel_mode: 0, ..clean() });
    }
}

/// Row 14 — `(channels - 1) << 4` discards the top 4 bits (no panic, no wrap).
#[test]
fn err_14_channels_shift_truncation() {
    let d = Diff::load();
    // channels == 0x1000_0001 -> 0x1000_0000 << 4 == 0 (mod 2^32)
    let fh = d.check(&Input { channels: 0x1000_0001, channel_mode: 0, ..clean() });
    assert_eq!(fh, 0xFFF8_0000 | 0xC000 | 0x900 | 0x8, "got 0x{fh:08X}");
    assert_eq!(ch_nibble(fh), 0x0);
    // channels == u32::MAX -> 0xFFFFFFFE << 4 == 0xFFFFFFE0
    let fh = d.check(&Input { channels: u32::MAX, channel_mode: 0, ..clean() });
    assert_eq!(fh, 0xFFF8_0000 | 0xC000 | 0x900 | 0xFFFF_FFE0 | 0x8, "got 0x{fh:08X}");
    // exhaustive over the truncation neighbourhood
    for ch in 0x0FFF_FFF0u32..=0x1000_0010 {
        d.check(&Input { channels: ch, channel_mode: 0, ..clean() });
    }
}

// ===========================================================================
// Rows 15-17 — out-of-range channel_mode enum values across the FFI boundary
// ===========================================================================

/// Row 15 — `channel_mode == TFLAC_CHANNEL_MODE_COUNT (4)`, an enum value with
/// no meaningful variant, aliases to INDEPENDENT via `% 4`.
#[test]
fn err_15_channel_mode_count_alias() {
    let d = Diff::load();
    let independent = d.check(&Input { channel_mode: 0, ..clean() });
    let count = d.check(&Input { channel_mode: 4, ..clean() });
    assert_eq!(count, independent, "channel_mode 4 (MODE_COUNT) must behave as mode 0");
    assert_eq!(ch_nibble(count), 0x1, "channels=2 -> (2-1)<<4");
}

/// Row 16 — all 256 out-of-range/aliasing `channel_mode` values.
#[test]
fn err_16_channel_mode_all_256_values() {
    let d = Diff::load();
    let base: Vec<u32> = (0..4).map(|m| d.check(&Input { channel_mode: m, ..clean() })).collect();
    for m in 0..=255u8 {
        let got = d.check(&Input { channel_mode: m, ..clean() });
        assert_eq!(
            got,
            base[(m % 4) as usize],
            "channel_mode {m} must alias to {} via % 4",
            m % 4
        );
        assert_eq!(m % 4, m & 3, "the C `% 4` on an unsigned type is a mask");
    }
    // and with randomized companions
    let mut rng = Rng::new(SEED ^ 0xC016);
    let pools = Pools::new();
    for _ in 0..iters(20_000) {
        let mut inp = random_input(&mut rng, &pools);
        inp.channel_mode = rng.next_u8();
        d.check(&inp);
    }
}

/// Row 17 — the `default:` arm of the channel-mode switch is unreachable: for
/// every one of the 256 byte values one of the four real arms is taken.
#[test]
fn err_17_channel_mode_default_unreachable() {
    let d = Diff::load();
    // channels = 2 so that mode 0 yields a *nonzero* nibble (0x1) and can be
    // distinguished from "no arm taken".
    for m in 0..=255u8 {
        let fh = d.check(&Input { channel_mode: m, channels: 2, ..clean() });
        let want = match m % 4 {
            0 => 0x1, // (2 - 1) << 4
            1 => 0x8,
            2 => 0x9,
            _ => 0xA,
        };
        assert_eq!(ch_nibble(fh), want, "channel_mode {m}: default arm must never be taken");
    }
}

// ===========================================================================
// Rows 18-19 — bitdepth rejections
// ===========================================================================

/// Row 18 — `bitdepth` outside the 6 cases → `default: break`, no bits.
#[test]
fn err_18_bitdepth_default() {
    let d = Diff::load();
    for bd in 0..=64u32 {
        if BITDEPTH_CASES.contains(&bd) {
            continue;
        }
        let fh = d.check(&Input { bitdepth: bd, ..clean() });
        assert_eq!(bd_field(fh), 0x0, "bitdepth {bd} must contribute no bits");
    }
    let mut rng = Rng::new(SEED ^ 0xC018);
    for _ in 0..iters(20_000) {
        let bd = rng.range_u32(33, u32::MAX);
        if BITDEPTH_CASES.contains(&bd) {
            continue;
        }
        assert_eq!(bd_field(d.check(&Input { bitdepth: bd, ..clean() })), 0x0, "bd={bd}");
    }
    for bd in [0u32, 1, 33, u32::MAX - 1, u32::MAX] {
        assert_eq!(bd_field(d.check(&Input { bitdepth: bd, ..clean() })), 0x0, "bd={bd}");
    }
}

/// Row 19 — one step past each valid bitdepth.
#[test]
fn err_19_bitdepth_off_by_one() {
    let d = Diff::load();
    let want = |bd: u32| -> u32 {
        match bd {
            8 => 1,
            12 => 2,
            16 => 4,
            20 => 5,
            24 => 6,
            32 => 7,
            _ => 0,
        }
    };
    for &c in BITDEPTH_CASES.iter() {
        for bd in [c - 1, c, c + 1] {
            let fh = d.check(&Input { bitdepth: bd, ..clean() });
            assert_eq!(bd_field(fh), want(bd), "bitdepth {bd}");
        }
    }
}

// ===========================================================================
// Row 20 — all fields simultaneously at an extreme
// ===========================================================================

#[test]
fn err_20_all_fields_extremes() {
    let d = Diff::load();

    // Everything zero: bs 0 -> 0x6, sr 0 -> 0xC, mode 0 + channels 0 ->
    // 0xFFFFFFF0, bd 0 -> none.
    let all_zero = Input {
        samplerate: 0,
        channels: 0,
        bitdepth: 0,
        channel_mode: 0,
        frame_header: 0,
        cur_blocksize: 0,
        padding: [0, 0, 0],
    };
    let fh = d.check(&all_zero);
    assert_eq!(fh, 0xFFFF_FFF0, "all-zero input; got 0x{fh:08X}");

    // Everything at its maximum: bs MAX -> 0x7, sr MAX -> none,
    // channel_mode 255 -> 255 % 4 == 3 -> MID_SIDE 0xA0 (channels ignored),
    // bd MAX -> none.
    let all_max = Input {
        samplerate: u32::MAX,
        channels: u32::MAX,
        bitdepth: u32::MAX,
        channel_mode: u8::MAX,
        frame_header: u32::MAX,
        cur_blocksize: u32::MAX,
        padding: [0xFF, 0xFF, 0xFF],
    };
    let fh = d.check(&all_max);
    assert_eq!(fh, 0xFFF8_70A0, "all-max input; got 0x{fh:08X}");

    // Max everything but with mode forced to INDEPENDENT so channels is read:
    // (u32::MAX - 1) << 4 == 0xFFFF_FFE0.
    let fh = d.check(&Input { channel_mode: 0, ..all_max });
    assert_eq!(fh, 0xFFFF_FFE0, "got 0x{fh:08X}");

    // A sweep of mixed extremes.
    for &sr in [0u32, 1, u32::MAX].iter() {
        for &ch in [0u32, 1, u32::MAX].iter() {
            for &bd in [0u32, 8, u32::MAX].iter() {
                for &bs in [0u32, 256, u32::MAX].iter() {
                    for &m in [0u8, 3, 4, 255].iter() {
                        d.check(&Input {
                            samplerate: sr,
                            channels: ch,
                            bitdepth: bd,
                            channel_mode: m,
                            frame_header: u32::MAX,
                            cur_blocksize: bs,
                            padding: [0xFF, 0x00, 0xFF],
                        });
                    }
                }
            }
        }
    }
}

// ===========================================================================
// Row 21 — nothing outside the documented bits/bytes is ever written
// ===========================================================================

#[test]
fn err_21_no_stray_writes() {
    let d = Diff::load();
    let pools = Pools::new();
    let mut rng = Rng::new(SEED ^ 0xC021);
    for _ in 0..iters(50_000) {
        let inp = random_input(&mut rng, &pools);
        // `check` already asserts the 16 guard bytes on each side are pristine
        // and that the two libraries agree on all 24 record bytes.
        let cb = d.run_c(&inp);
        let rb = d.run_rust(&inp);
        assert_eq!(cb.bytes(), rb.bytes(), "record diverged for {inp:?}");

        // The 3 padding bytes must survive untouched in both.
        assert_eq!(
            &cb.bytes()[GUARD + 13..GUARD + 16],
            &inp.padding[..],
            "C clobbered struct padding for {inp:?}"
        );
        assert_eq!(
            &rb.bytes()[GUARD + 13..GUARD + 16],
            &inp.padding[..],
            "Rust clobbered struct padding for {inp:?}"
        );

        // The input fields must be left alone; only frame_header changes.
        for r in [OFF_SAMPLERATE, OFF_CHANNELS, OFF_BITDEPTH, OFF_CUR_BLOCKSIZE] {
            let pristine = Buf::new(&inp);
            assert_eq!(
                &cb.bytes()[GUARD + r..GUARD + r + 4],
                &pristine.bytes()[GUARD + r..GUARD + r + 4],
                "C modified an input field at offset {r}"
            );
        }

        // Bit 0 of frame_header is never set by any path.
        assert_eq!(cb.frame_header() & 1, 0, "bit 0 set for {inp:?}");
        // 0xFFF8_0000 is ORed in unconditionally at lib.c:12, so those bits are
        // always present in the result, whatever the inputs.
        assert_eq!(cb.frame_header() & 0xFFF8_0000, 0xFFF8_0000, "sync code lost for {inp:?}");
    }
}

// ===========================================================================
// Generic FFI boundary checks (beyond the table)
// ===========================================================================

/// The C `enum TFLAC_CHANNEL_MODE` is file-local, but `channel_mode` is the
/// value that reaches it. Cover the whole `u8` domain crossed with every other
/// axis's boundary values — the classic "out-of-range enum" blind spot.
#[test]
fn err_22_out_of_range_enum_cross_product() {
    let d = Diff::load();
    for m in 0..=255u8 {
        for &ch in [0u32, 1, 2, 8, 9, 17, u32::MAX].iter() {
            for &bs in [0u32, 256, 257, 4096, u32::MAX].iter() {
                for &sr in [0u32, 44100, 65536, 655_360, u32::MAX].iter() {
                    d.check(&Input::new(sr, ch, 16, m, bs));
                }
            }
        }
    }
}

/// A record whose bytes are entirely random (including the padding) — proves no
/// hidden dependence on uninitialised bytes.
#[test]
fn err_23_fully_random_record_bytes() {
    let d = Diff::load();
    let mut rng = Rng::new(SEED ^ 0xC023);
    for _ in 0..iters(200_000) {
        d.check(&Input {
            samplerate: rng.next_u32(),
            channels: rng.next_u32(),
            bitdepth: rng.next_u32(),
            channel_mode: rng.next_u8(),
            frame_header: rng.next_u32(),
            cur_blocksize: rng.next_u32(),
            padding: [rng.next_u8(), rng.next_u8(), rng.next_u8()],
        });
    }
}
