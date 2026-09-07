//! Phase C — error / rejection-path differential tests.
//!
//! One test per row of `ERRORS.md`. `update_frame_header` returns `void` and has
//! no error codes, so the "same rejection" assertion is: for the exact invalid
//! input/condition, both `.so`s must produce the *same* 24 output bytes, and the
//! rejecting field must contribute *exactly* the bits the C contributes
//! (asserted explicitly, so "both silently did nothing" is verified rather than
//! assumed).

mod common;
use common::*;

const N: usize = 4000;

/// Bits 8..=11 of `frame_header` — the sample-rate nibble.
fn sr_nibble(fh: u32) -> u32 {
    (fh >> 8) & 0x0F
}
/// Bits 12..=15 — the block-size nibble.
fn bs_nibble(fh: u32) -> u32 {
    (fh >> 12) & 0x0F
}
/// Bits 1..=3 — the bit-depth field.
fn bd_field(fh: u32) -> u32 {
    (fh >> 1) & 0x07
}
/// Bits 4..=7 — the channel-assignment nibble.
fn cm_nibble(fh: u32) -> u32 {
    (fh >> 4) & 0x0F
}

/// A "clean" base input: every other axis chosen so it contributes a known,
/// non-interfering value, so the field under test can be isolated.
fn clean() -> Input {
    Input {
        samplerate: 44100,   // -> 0x09
        channels: 2,         // -> 0x01 in the channel nibble
        bitdepth: 16,        // -> 0x04
        channel_mode: 0,     // INDEPENDENT
        frame_header_initial: 0,
        cur_blocksize: 4096, // -> 0x0C
        padding: 0,
    }
}

// --------------------------------------------------------------------------
// Row 1 — block-size `default`, `<= 256` -> ORs 0x06 << 12 (never rejects)
// --------------------------------------------------------------------------
#[test]
fn err_row_01_blocksize_default_le_256() {
    let libs = Libs::load();
    let mut rng = Rng::new(SEED ^ 0x101);
    for bs in 0u32..=256 {
        if is_blocksize_case(bs) {
            continue;
        }
        let mut inp = clean();
        inp.cur_blocksize = bs;
        libs.assert_same(inp, &format!("err1 bs={bs}"));

        let mut c = inp.to_raw();
        libs.call_c(&mut c);
        assert_eq!(bs_nibble(c.frame_header()), 0x06, "err1 bs={bs}: C nibble");
        let mut r = inp.to_raw();
        libs.call_rust(&mut r);
        assert_eq!(bs_nibble(r.frame_header()), 0x06, "err1 bs={bs}: Rust nibble");
    }
    for i in 0..N {
        let mut inp = rand_input(&mut rng);
        inp.cur_blocksize = rand_blocksize_default_le256(&mut rng);
        libs.assert_same(inp, &format!("err1 random {i}"));
    }
}

// --------------------------------------------------------------------------
// Row 2 — block-size `default`, `> 256` -> ORs 0x07 << 12
// --------------------------------------------------------------------------
#[test]
fn err_row_02_blocksize_default_gt_256() {
    let libs = Libs::load();
    let mut rng = Rng::new(SEED ^ 0x102);
    let explicit = [257u32, 258, 1000, 65535, 65536, u32::MAX - 1, u32::MAX];
    for &bs in explicit.iter() {
        let mut inp = clean();
        inp.cur_blocksize = bs;
        libs.assert_same(inp, &format!("err2 bs={bs}"));
        let mut c = inp.to_raw();
        libs.call_c(&mut c);
        assert_eq!(bs_nibble(c.frame_header()), 0x07, "err2 bs={bs}: C nibble");
        let mut r = inp.to_raw();
        libs.call_rust(&mut r);
        assert_eq!(bs_nibble(r.frame_header()), 0x07, "err2 bs={bs}: Rust nibble");
    }
    for i in 0..N {
        let mut inp = rand_input(&mut rng);
        inp.cur_blocksize = rand_blocksize_default_gt256(&mut rng);
        libs.assert_same(inp, &format!("err2 random {i}"));
    }
}

// --------------------------------------------------------------------------
// Row 3 — samplerate %1000==0 && /1000<256 -> ORs 0x0C
// --------------------------------------------------------------------------
#[test]
fn err_row_03_samplerate_mod1000_kilo_ok() {
    let libs = Libs::load();
    let mut rng = Rng::new(SEED ^ 0x103);
    for k in 0u32..256 {
        let sr = k * 1000;
        if is_samplerate_case(sr) {
            continue;
        }
        let mut inp = clean();
        inp.samplerate = sr;
        libs.assert_same(inp, &format!("err3 sr={sr}"));
        let mut c = inp.to_raw();
        libs.call_c(&mut c);
        assert_eq!(sr_nibble(c.frame_header()), 0x0C, "err3 sr={sr}: C nibble");
        let mut r = inp.to_raw();
        libs.call_rust(&mut r);
        assert_eq!(sr_nibble(r.frame_header()), 0x0C, "err3 sr={sr}: Rust nibble");
    }
    for i in 0..N {
        let mut inp = rand_input(&mut rng);
        inp.samplerate = rand_sr_kilo_ok(&mut rng);
        libs.assert_same(inp, &format!("err3 random {i}"));
    }
}

// --------------------------------------------------------------------------
// Row 4 — SILENT REJECTION: %1000==0 && /1000>=256 -> no SR bits
// --------------------------------------------------------------------------
#[test]
fn err_row_04_samplerate_mod1000_kilo_overflow_silent() {
    let libs = Libs::load();
    let mut rng = Rng::new(SEED ^ 0x104);
    let explicit = [256_000u32, 257_000, 1_000_000, 4_294_967_000];
    for &sr in explicit.iter() {
        let mut inp = clean();
        inp.samplerate = sr;
        libs.assert_same(inp, &format!("err4 sr={sr}"));
        let mut c = inp.to_raw();
        libs.call_c(&mut c);
        assert_eq!(
            sr_nibble(c.frame_header()),
            0x00,
            "err4 sr={sr}: C must contribute NO sample-rate bits"
        );
        let mut r = inp.to_raw();
        libs.call_rust(&mut r);
        assert_eq!(
            sr_nibble(r.frame_header()),
            0x00,
            "err4 sr={sr}: Rust must also contribute NO sample-rate bits"
        );
    }
    for i in 0..N {
        let mut inp = rand_input(&mut rng);
        inp.samplerate = rand_sr_kilo_overflow(&mut rng);
        libs.assert_same(inp, &format!("err4 random {i}"));
    }
}

// --------------------------------------------------------------------------
// Row 5 — %1000!=0 && <65536 -> ORs 0x0D
// --------------------------------------------------------------------------
#[test]
fn err_row_05_samplerate_below_65536() {
    let libs = Libs::load();
    let mut rng = Rng::new(SEED ^ 0x105);
    for sr in 0u32..65536 {
        if sr % 1000 == 0 || is_samplerate_case(sr) {
            continue;
        }
        let mut inp = clean();
        inp.samplerate = sr;
        let mut c = inp.to_raw();
        libs.call_c(&mut c);
        let mut r = inp.to_raw();
        libs.call_rust(&mut r);
        assert_eq!(c, r, "err5 sr={sr}: divergence\nC={c:?}\nR={r:?}");
        assert_eq!(sr_nibble(c.frame_header()), 0x0D, "err5 sr={sr}: C nibble");
    }
    for i in 0..N {
        let mut inp = rand_input(&mut rng);
        inp.samplerate = rand_sr_lt_65536(&mut rng);
        libs.assert_same(inp, &format!("err5 random {i}"));
    }
}

// --------------------------------------------------------------------------
// Row 6 — %1000!=0 && >=65536 && %10==0 && /10<65536 -> ORs 0x0E
// --------------------------------------------------------------------------
#[test]
fn err_row_06_samplerate_deca_ok() {
    let libs = Libs::load();
    let mut rng = Rng::new(SEED ^ 0x106);
    let explicit = [65_540u32, 65_550, 100_010, 655_340, 655_350];
    for &sr in explicit.iter() {
        let mut inp = clean();
        inp.samplerate = sr;
        libs.assert_same(inp, &format!("err6 sr={sr}"));
        let mut c = inp.to_raw();
        libs.call_c(&mut c);
        assert_eq!(sr_nibble(c.frame_header()), 0x0E, "err6 sr={sr}: C nibble");
        let mut r = inp.to_raw();
        libs.call_rust(&mut r);
        assert_eq!(sr_nibble(r.frame_header()), 0x0E, "err6 sr={sr}: Rust nibble");
    }
    for i in 0..N {
        let mut inp = rand_input(&mut rng);
        inp.samplerate = rand_sr_deca_ok(&mut rng);
        libs.assert_same(inp, &format!("err6 random {i}"));
    }
}

// --------------------------------------------------------------------------
// Row 7 — SILENT REJECTION: %10==0 but /10 >= 65536 -> no SR bits
// --------------------------------------------------------------------------
#[test]
fn err_row_07_samplerate_deca_overflow_silent() {
    let libs = Libs::load();
    let mut rng = Rng::new(SEED ^ 0x107);
    let explicit = [655_370u32, 655_360, 1_000_010, 4_294_967_290];
    for &sr in explicit.iter() {
        let mut inp = clean();
        inp.samplerate = sr;
        libs.assert_same(inp, &format!("err7 sr={sr}"));
        let mut c = inp.to_raw();
        libs.call_c(&mut c);
        assert_eq!(
            sr_nibble(c.frame_header()),
            0x00,
            "err7 sr={sr}: C must contribute NO sample-rate bits"
        );
        let mut r = inp.to_raw();
        libs.call_rust(&mut r);
        assert_eq!(
            sr_nibble(r.frame_header()),
            0x00,
            "err7 sr={sr}: Rust must also contribute NO sample-rate bits"
        );
    }
    for i in 0..N {
        let mut inp = rand_input(&mut rng);
        inp.samplerate = rand_sr_deca_overflow(&mut rng);
        libs.assert_same(inp, &format!("err7 random {i}"));
    }
}

// --------------------------------------------------------------------------
// Row 8 — SILENT REJECTION: unrepresentable rate -> no SR bits
// --------------------------------------------------------------------------
#[test]
fn err_row_08_samplerate_no_representation_silent() {
    let libs = Libs::load();
    let mut rng = Rng::new(SEED ^ 0x108);
    let explicit = [65_536u32, 65_537, 65_539, u32::MAX - 1, u32::MAX];
    for &sr in explicit.iter() {
        let mut inp = clean();
        inp.samplerate = sr;
        libs.assert_same(inp, &format!("err8 sr={sr}"));
        let mut c = inp.to_raw();
        libs.call_c(&mut c);
        assert_eq!(
            sr_nibble(c.frame_header()),
            0x00,
            "err8 sr={sr}: C must contribute NO sample-rate bits"
        );
        let mut r = inp.to_raw();
        libs.call_rust(&mut r);
        assert_eq!(
            sr_nibble(r.frame_header()),
            0x00,
            "err8 sr={sr}: Rust must also contribute NO sample-rate bits"
        );
    }
    for i in 0..N {
        let mut inp = rand_input(&mut rng);
        inp.samplerate = rand_sr_none(&mut rng);
        libs.assert_same(inp, &format!("err8 random {i}"));
    }
}

// --------------------------------------------------------------------------
// Row 9 — OUT-OF-RANGE ENUM across the FFI boundary.
//
// `channel_mode` is a `tflac_u8`, cast to `enum TFLAC_CHANNEL_MODE` after `%4`.
// Values >= 4 (including `TFLAC_CHANNEL_MODE_COUNT == 4`) are real inputs a C
// caller can pass; they must fold identically in both libs.
// --------------------------------------------------------------------------
#[test]
fn err_row_09_channel_mode_out_of_enum_range() {
    let libs = Libs::load();

    // Exhaustive over the entire byte domain, for every channel count shape.
    let ch_shapes = [0u32, 1, 2, 8, 9, 16, 17, 0xFFFF_FFFF];
    for m in 0u16..=255 {
        for &ch in ch_shapes.iter() {
            let mut inp = clean();
            inp.channel_mode = m as u8;
            inp.channels = ch;
            libs.assert_same(inp, &format!("err9 channel_mode={m} channels={ch}"));
        }
    }

    // And assert the folding is what the C actually does: mode >= 4 behaves
    // exactly like mode % 4.
    for m in 0u16..=255 {
        let folded = (m % 4) as u8;
        let mut a = clean();
        a.channel_mode = m as u8;
        let mut b = clean();
        b.channel_mode = folded;

        let (mut ca, mut cb) = (a.to_raw(), b.to_raw());
        libs.call_c(&mut ca);
        libs.call_c(&mut cb);
        assert_eq!(
            ca.frame_header(),
            cb.frame_header(),
            "err9: C did not fold channel_mode {m} to {folded}"
        );

        let (mut ra, mut rb) = (a.to_raw(), b.to_raw());
        libs.call_rust(&mut ra);
        libs.call_rust(&mut rb);
        assert_eq!(
            ra.frame_header(),
            ca.frame_header(),
            "err9: Rust diverged for channel_mode {m}"
        );
        assert_eq!(
            rb.frame_header(),
            cb.frame_header(),
            "err9: Rust diverged for folded channel_mode {folded}"
        );
    }

    // Value 4 == TFLAC_CHANNEL_MODE_COUNT specifically.
    let mut inp = clean();
    inp.channel_mode = 4;
    libs.assert_same(inp, "err9 channel_mode == TFLAC_CHANNEL_MODE_COUNT");
    let mut c = inp.to_raw();
    libs.call_c(&mut c);
    assert_eq!(
        cm_nibble(c.frame_header()),
        0x01,
        "err9: channel_mode 4 must fold to INDEPENDENT with channels=2 -> nibble 1"
    );
}

// --------------------------------------------------------------------------
// Row 10 — the channel-mode `switch` `default:` label is DEAD CODE.
//
// Proven by showing every byte value 0..=255 lands on one of the four named
// cases: the channel nibble always equals the value the matching case ORs.
// --------------------------------------------------------------------------
#[test]
fn err_row_10_channel_mode_switch_default_dead() {
    let libs = Libs::load();
    for m in 0u16..=255 {
        let mut inp = clean();
        inp.channel_mode = m as u8;
        inp.channels = 2; // INDEPENDENT -> (2-1) << 4 = nibble 0x01
        let expected = match m % 4 {
            0 => 0x01,
            1 => 0x08,
            2 => 0x09,
            _ => 0x0A,
        };
        let mut c = inp.to_raw();
        libs.call_c(&mut c);
        assert_eq!(
            cm_nibble(c.frame_header()),
            expected,
            "err10 channel_mode={m}: C hit an unexpected branch"
        );
        let mut r = inp.to_raw();
        libs.call_rust(&mut r);
        assert_eq!(
            cm_nibble(r.frame_header()),
            expected,
            "err10 channel_mode={m}: Rust hit an unexpected branch"
        );
        assert_eq!(c, r, "err10 channel_mode={m}: byte divergence");
    }
}

// --------------------------------------------------------------------------
// Row 11 — channels == 0: unsigned underflow, NOT rejected.
// --------------------------------------------------------------------------
#[test]
fn err_row_11_channels_zero_underflow() {
    let libs = Libs::load();
    let mut rng = Rng::new(SEED ^ 0x10B);

    let mut inp = clean();
    inp.channels = 0;
    inp.channel_mode = 0;
    libs.assert_same(inp, "err11 channels=0 clean");

    // (0u32 - 1) << 4 == 0xFFFFFFF0
    let expect = (0xFFF8u32 << 16) | (0x0Cu32 << 12) | (0x09u32 << 8) | (0x04u32 << 1);
    let expect = expect | 0xFFFF_FFF0u32;
    let mut c = inp.to_raw();
    libs.call_c(&mut c);
    assert_eq!(
        c.frame_header(),
        expect,
        "err11: C underflow result changed from the derived value"
    );
    let mut r = inp.to_raw();
    libs.call_rust(&mut r);
    assert_eq!(r.frame_header(), c.frame_header(), "err11: Rust diverged");

    for i in 0..N {
        let mut inp = rand_input(&mut rng);
        inp.channels = 0;
        inp.channel_mode = (rng.range_u32(0, 63) * 4) as u8;
        libs.assert_same(inp, &format!("err11 random {i}"));
    }
    // channels==0 with the other three modes: `channels` must be ignored.
    for i in 0..N {
        let mut inp = rand_input(&mut rng);
        inp.channels = 0;
        inp.channel_mode = (rng.range_u32(0, 63) * 4 + rng.range_u32(1, 3)) as u8;
        libs.assert_same(inp, &format!("err11 nonindep random {i}"));
    }
}

// --------------------------------------------------------------------------
// Row 12 — channels - 1 > 0x0F: bits bleed out of the nibble.
// --------------------------------------------------------------------------
#[test]
fn err_row_12_channels_overflow_bleeds() {
    let libs = Libs::load();
    let mut rng = Rng::new(SEED ^ 0x10C);
    for &ch in &[17u32, 18, 33, 256, 4096, 0x000F_FFFF, 0x0FFF_FFFF, 0xEFFF_FFFF] {
        let mut inp = clean();
        inp.channels = ch;
        inp.channel_mode = 0;
        libs.assert_same(inp, &format!("err12 channels={ch}"));

        // Non-vacuity: the contribution genuinely reaches outside the 4-bit
        // channel field (a property of the input shape).
        let contrib = ch.wrapping_sub(1) << 4;
        assert_ne!(
            contrib & !0x0000_00F0u32,
            0,
            "err12 channels={ch}: contribution {contrib:#X} does not leave the channel nibble"
        );

        // And the C's full 32-bit result is exactly base | contribution.
        let base = (0xFFF8u32 << 16) | (0x0Cu32 << 12) | (0x09u32 << 8) | (0x04u32 << 1);
        let mut c = inp.to_raw();
        libs.call_c(&mut c);
        assert_eq!(
            c.frame_header(),
            base | contrib,
            "err12 channels={ch}: C result is not base|((channels-1)<<4)"
        );
        let mut r = inp.to_raw();
        libs.call_rust(&mut r);
        assert_eq!(
            r.frame_header(),
            c.frame_header(),
            "err12 channels={ch}: Rust diverged"
        );
    }
    for i in 0..N * 2 {
        let mut inp = rand_input(&mut rng);
        inp.channels = rng.range_u32(17, 0xEFFF_FFFF);
        inp.channel_mode = (rng.range_u32(0, 63) * 4) as u8;
        libs.assert_same(inp, &format!("err12 random {i}"));
    }
}

// --------------------------------------------------------------------------
// Row 13 — channels - 1 >= 0x1000_0000: `<< 4` truncates mod 2^32.
// --------------------------------------------------------------------------
#[test]
fn err_row_13_channels_shift_truncates() {
    let libs = Libs::load();
    let mut rng = Rng::new(SEED ^ 0x10D);
    for &ch in &[
        0x1000_0001u32,
        0x1000_0000,
        0xF000_0000,
        0xF000_0001,
        0xFFFF_FFFF,
        0xFFFF_FFFE,
    ] {
        let mut inp = clean();
        inp.channels = ch;
        inp.channel_mode = 0;
        libs.assert_same(inp, &format!("err13 channels={ch}"));
    }
    for i in 0..N * 2 {
        let mut inp = rand_input(&mut rng);
        inp.channels = rng.range_u32(0xF000_0000, u32::MAX);
        inp.channel_mode = (rng.range_u32(0, 63) * 4) as u8;
        libs.assert_same(inp, &format!("err13 random {i}"));
    }
}

// --------------------------------------------------------------------------
// Row 14 — SILENT REJECTION: unsupported bit depth -> no bit-depth bits.
// --------------------------------------------------------------------------
#[test]
fn err_row_14_bitdepth_unsupported_silent() {
    let libs = Libs::load();
    let mut rng = Rng::new(SEED ^ 0x10E);
    for bd in 0u32..=64 {
        if is_bitdepth_case(bd) {
            continue;
        }
        let mut inp = clean();
        inp.bitdepth = bd;
        libs.assert_same(inp, &format!("err14 bitdepth={bd}"));
        let mut c = inp.to_raw();
        libs.call_c(&mut c);
        assert_eq!(
            bd_field(c.frame_header()),
            0x00,
            "err14 bitdepth={bd}: C must contribute NO bit-depth bits"
        );
        let mut r = inp.to_raw();
        libs.call_rust(&mut r);
        assert_eq!(
            bd_field(r.frame_header()),
            0x00,
            "err14 bitdepth={bd}: Rust must also contribute NO bit-depth bits"
        );
    }
    for &bd in &[u32::MAX, u32::MAX - 1, 0x8000_0000, 33, 17] {
        let mut inp = clean();
        inp.bitdepth = bd;
        libs.assert_same(inp, &format!("err14 extreme bitdepth={bd}"));
    }
    for i in 0..N {
        let mut inp = rand_input(&mut rng);
        inp.bitdepth = rand_bitdepth_default(&mut rng);
        libs.assert_same(inp, &format!("err14 random {i}"));
    }
}

// --------------------------------------------------------------------------
// Row 15 — NULL pointer. Neither lib checks; both must fault identically.
//
// Run out-of-process (a segfault cannot be caught in-process) and compare the
// termination signal of the C and Rust callers.
// --------------------------------------------------------------------------
#[test]
fn err_row_15_null_pointer() {
    use std::process::Command;

    // Child mode: dispatched by env var, calls the requested lib with NULL.
    if let Ok(which) = std::env::var("NULL_DEREF_TARGET") {
        let libs = Libs::load();
        unsafe {
            match which.as_str() {
                "c" => libs.call_c_raw(std::ptr::null_mut()),
                "rust" => libs.call_rust_raw(std::ptr::null_mut()),
                other => panic!("unknown NULL_DEREF_TARGET {other}"),
            }
        }
        // If we somehow survive, exit distinctly so the parent sees it.
        std::process::exit(77);
    }

    let exe = std::env::current_exe().expect("current_exe");
    let run = |target: &str| -> (Option<i32>, Option<i32>) {
        let out = Command::new(&exe)
            .args(["err_row_15_null_pointer", "--exact", "--nocapture"])
            .env("NULL_DEREF_TARGET", target)
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

    // Neither lib may survive a NULL `tflac*` (77 is the "we survived" marker).
    assert_ne!(c_code, Some(77), "err15: C returned normally from a NULL tflac*");
    assert_ne!(
        r_code,
        Some(77),
        "err15: Rust returned normally from a NULL tflac* — it must fault like the C"
    );
    #[cfg(unix)]
    {
        assert!(
            c_sig.is_some(),
            "err15: expected the C library to be killed by a signal, got code {c_code:?}"
        );
        assert!(
            r_sig.is_some(),
            "err15: expected the Rust library to be killed by a signal, got code {r_code:?}"
        );
        // The C has no null check, so it must actually fault (SIGSEGV = 11).
        assert_eq!(
            c_sig,
            Some(11),
            "err15: expected the C library to SIGSEGV on a NULL tflac*"
        );

        if cfg!(debug_assertions) {
            // The `.so` under test was built by the same cargo profile as this
            // test binary. With `debug_assertions` on, rustc inserts its own
            // null-pointer UB check ahead of every raw dereference, so the Rust
            // library aborts (SIGABRT = 6) instead of reaching the faulting
            // load. That is compiler-inserted sanitization — the moral
            // equivalent of building the C with `-fsanitize=null` — not a
            // behavioural difference in the translation. The shipped artifact is
            // the release `cdylib`, where the assertion below is exact.
            assert!(
                r_sig == Some(11) || r_sig == Some(6),
                "err15 (debug profile): Rust must either fault (SIGSEGV) or trip its \
                 own UB check (SIGABRT); got signal {r_sig:?}"
            );
        } else {
            assert_eq!(
                (c_code, c_sig),
                (r_code, r_sig),
                "err15: NULL deref outcome differs. C=(code {c_code:?}, signal {c_sig:?}) \
                 RUST=(code {r_code:?}, signal {r_sig:?})"
            );
        }
    }
    #[cfg(not(unix))]
    assert_eq!(c_code, r_code, "err15: NULL deref exit code differs");
}

// --------------------------------------------------------------------------
// Row 16 — only `frame_header` is written; all other bytes are preserved.
// --------------------------------------------------------------------------
#[test]
fn err_row_16_only_frame_header_written() {
    let libs = Libs::load();
    let mut rng = Rng::new(SEED ^ 0x110);
    for i in 0..N * 4 {
        let inp = rand_input(&mut rng);
        let before = inp.to_raw();
        let mut c = before;
        let mut r = before;
        libs.call_c(&mut c);
        libs.call_rust(&mut r);
        assert_eq!(c, r, "err16 iter {i}: byte divergence\nC={c:?}\nR={r:?}");
        for off in 0..TFLAC_SIZE {
            if (OFF_FRAME_HEADER..OFF_FRAME_HEADER + 4).contains(&off) {
                continue;
            }
            assert_eq!(
                c.0[off], before.0[off],
                "err16 iter {i}: C wrote byte {off} (ground truth says it should not)"
            );
            assert_eq!(
                r.0[off], before.0[off],
                "err16 iter {i}: Rust wrote byte {off} but C did not"
            );
        }
    }
}

// --------------------------------------------------------------------------
// Generic FFI boundaries beyond the table: misaligned pointer, and a struct
// placed at the very end of a mapped region followed by a guard so an
// out-of-bounds write would be caught.
// --------------------------------------------------------------------------

/// Both libs must write within the 24-byte struct only — verified with poison
/// bytes on both sides of the struct inside a larger allocation.
#[test]
fn err_generic_no_out_of_bounds_write() {
    let libs = Libs::load();
    let mut rng = Rng::new(SEED ^ 0x111);
    const PAD: usize = 64;
    for i in 0..N {
        let words = (PAD * 2 + TFLAC_SIZE) / 4;
        let mut c_words = vec![0xDEAD_BEEFu32; words];
        {
            let buf: &mut [u8] = unsafe {
                std::slice::from_raw_parts_mut(c_words.as_mut_ptr() as *mut u8, words * 4)
            };
            buf[PAD..PAD + TFLAC_SIZE].copy_from_slice(&rand_input(&mut rng).to_raw().0);
        }
        let mut r_words = c_words.clone();
        unsafe {
            libs.call_c_raw((c_words.as_mut_ptr() as *mut u8).add(PAD));
            libs.call_rust_raw((r_words.as_mut_ptr() as *mut u8).add(PAD));
        }
        assert_eq!(c_words, r_words, "err-generic iter {i}: OOB/content divergence");
        // Guard regions untouched in both.
        for w in 0..PAD / 4 {
            assert_eq!(c_words[w], 0xDEAD_BEEF, "C clobbered leading guard word {w}");
            assert_eq!(r_words[w], 0xDEAD_BEEF, "Rust clobbered leading guard word {w}");
        }
        for w in (PAD + TFLAC_SIZE) / 4..words {
            assert_eq!(c_words[w], 0xDEAD_BEEF, "C clobbered trailing guard word {w}");
            assert_eq!(r_words[w], 0xDEAD_BEEF, "Rust clobbered trailing guard word {w}");
        }
    }
}
