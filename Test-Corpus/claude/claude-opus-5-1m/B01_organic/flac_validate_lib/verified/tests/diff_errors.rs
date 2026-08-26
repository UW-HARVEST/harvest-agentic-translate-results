//! Phase C — error-path differential tests, one test per `ERRORS.md` row.
//!
//! Each test builds the exact invalid input the C code rejects, calls BOTH the
//! C `.so` and the Rust `.so` through their exported symbols, and asserts they
//! return the SAME sentinel (`-1`) **and** leave `struct tflac` in the same
//! byte-for-byte state (the C code mutates the struct before some rejections).

mod common;

use common::*;

const ITERS: usize = 20_000;

/// A valid struct with randomized free fields, used as the base to perturb.
fn base(rng: &mut Rng) -> Fields {
    let max_po = rng.range_u8(0, 15);
    let min_po = rng.range_u8(0, max_po);
    Fields {
        blocksize: [16u32, 17, 4096, 32768, 65535][(rng.next_u64() % 5) as usize],
        samplerate: rng.range_u32(1, 655_350),
        channels: rng.range_u32(1, 8),
        bitdepth: rng.range_u32(1, 32),
        channel_mode: rng.next_u8(),
        max_rice_value: rng.range_u8(0, 30),
        min_partition_order: min_po,
        max_partition_order: max_po,
        partition_order: rng.next_u8(),
        pad: [rng.next_u8(), rng.next_u8(), rng.next_u8()],
        cur_blocksize: rng.next_u32(),
    }
}

/// Asserts C and Rust agree, that C rejects with `-1`, and that the struct came
/// back completely unmodified (true for `ERRORS.md` rows 1..8).
fn expect_reject_unmodified(row: &str, f: Fields) {
    check_validate_ret(row, f, -1);
    let (_, out) = pair().c.validate(f);
    assert_eq!(
        out.to_raw(),
        f.to_raw(),
        "[{row}] C must not modify the struct before this rejection\n  in : {f:?}\n  out: {out:?}"
    );
}

/// Asserts C and Rust agree and that C rejects with `-1` (the struct may have
/// been partially mutated — `check_validate_ret` already compares it).
fn expect_reject(row: &str, f: Fields) {
    check_validate_ret(row, f, -1);
    // The pure-output fields are never written on a rejection.
    let (_, out) = pair().c.validate(f);
    assert_eq!(
        out.partition_order, f.partition_order,
        "[{row}] partition_order written on rejection: {f:?}"
    );
    assert_eq!(
        out.cur_blocksize, f.cur_blocksize,
        "[{row}] cur_blocksize written on rejection: {f:?}"
    );
}

// ---------------------------------------------------------------------------
// Row 1 — blocksize < 16
// ---------------------------------------------------------------------------

#[test]
fn err_row01_blocksize_too_small() {
    let mut rng = Rng::new(0x1001);
    for bs in 0u32..16 {
        for _ in 0..64 {
            let mut f = base(&mut rng);
            f.blocksize = bs;
            expect_reject_unmodified("err#1", f);
        }
    }
}

// ---------------------------------------------------------------------------
// Row 2 — blocksize > 65535
// ---------------------------------------------------------------------------

#[test]
fn err_row02_blocksize_too_large() {
    let mut rng = Rng::new(0x1002);
    for bs in [
        65536u32,
        65537,
        65551,
        0x0001_0000,
        0x7FFF_FFFF,
        0x8000_0000,
        0xFFFF_FFFF,
    ] {
        for _ in 0..64 {
            let mut f = base(&mut rng);
            f.blocksize = bs;
            expect_reject_unmodified("err#2", f);
        }
    }
    for _ in 0..iters(ITERS) {
        let mut f = base(&mut rng);
        f.blocksize = rng.range_u32(65536, u32::MAX);
        expect_reject_unmodified("err#2", f);
    }
}

// ---------------------------------------------------------------------------
// Row 3 — samplerate == 0
// ---------------------------------------------------------------------------

#[test]
fn err_row03_samplerate_zero() {
    let mut rng = Rng::new(0x1003);
    for _ in 0..iters(ITERS) {
        let mut f = base(&mut rng);
        f.samplerate = 0;
        expect_reject_unmodified("err#3", f);
    }
}

// ---------------------------------------------------------------------------
// Row 4 — samplerate > 655350
// ---------------------------------------------------------------------------

#[test]
fn err_row04_samplerate_too_large() {
    let mut rng = Rng::new(0x1004);
    for sr in [655_351u32, 655_352, 1_000_000, 0x7FFF_FFFF, 0xFFFF_FFFF] {
        for _ in 0..64 {
            let mut f = base(&mut rng);
            f.samplerate = sr;
            expect_reject_unmodified("err#4", f);
        }
    }
    for _ in 0..iters(ITERS) {
        let mut f = base(&mut rng);
        f.samplerate = rng.range_u32(655_351, u32::MAX);
        expect_reject_unmodified("err#4", f);
    }
}

// ---------------------------------------------------------------------------
// Row 5 — channels == 0
// ---------------------------------------------------------------------------

#[test]
fn err_row05_channels_zero() {
    let mut rng = Rng::new(0x1005);
    for _ in 0..iters(ITERS) {
        let mut f = base(&mut rng);
        f.channels = 0;
        expect_reject_unmodified("err#5", f);
    }
}

// ---------------------------------------------------------------------------
// Row 6 — channels > 8
// ---------------------------------------------------------------------------

#[test]
fn err_row06_channels_too_large() {
    let mut rng = Rng::new(0x1006);
    for ch in [9u32, 10, 16, 255, 256, 0x7FFF_FFFF, 0xFFFF_FFFF] {
        for _ in 0..64 {
            let mut f = base(&mut rng);
            f.channels = ch;
            expect_reject_unmodified("err#6", f);
        }
    }
    for _ in 0..iters(ITERS) {
        let mut f = base(&mut rng);
        f.channels = rng.range_u32(9, u32::MAX);
        expect_reject_unmodified("err#6", f);
    }
}

// ---------------------------------------------------------------------------
// Row 7 — bitdepth == 0
// ---------------------------------------------------------------------------

#[test]
fn err_row07_bitdepth_zero() {
    let mut rng = Rng::new(0x1007);
    for _ in 0..iters(ITERS) {
        let mut f = base(&mut rng);
        f.bitdepth = 0;
        expect_reject_unmodified("err#7", f);
    }
}

// ---------------------------------------------------------------------------
// Row 8 — bitdepth > 32
// ---------------------------------------------------------------------------

#[test]
fn err_row08_bitdepth_too_large() {
    let mut rng = Rng::new(0x1008);
    for bd in [33u32, 34, 64, 255, 256, 0x7FFF_FFFF, 0xFFFF_FFFF] {
        for _ in 0..64 {
            let mut f = base(&mut rng);
            f.bitdepth = bd;
            expect_reject_unmodified("err#8", f);
        }
    }
    for _ in 0..iters(ITERS) {
        let mut f = base(&mut rng);
        f.bitdepth = rng.range_u32(33, u32::MAX);
        expect_reject_unmodified("err#8", f);
    }
}

// ---------------------------------------------------------------------------
// Row 9 — max_rice_value != 0 && max_rice_value > 30
// ---------------------------------------------------------------------------

#[test]
fn err_row09_max_rice_value_too_large() {
    let mut rng = Rng::new(0x1009);
    // Exhaustive over every rejecting value, with randomized surroundings.
    for rice in 31u8..=255 {
        for _ in 0..16 {
            let mut f = base(&mut rng);
            f.max_rice_value = rice;
            expect_reject("err#9", f);
            // Premise: max_rice_value itself is never rewritten on this path.
            let (_, out) = pair().c.validate(f);
            assert_eq!(out.max_rice_value, rice, "err#9 premise: {f:?}");
        }
    }
    // Explicitly exercise the "channel_mode already zeroed" partial mutation.
    for mode in 1u8..=255 {
        let mut f = base(&mut rng);
        f.channel_mode = mode;
        f.channels = 3; // != 2 => mode forced to 0 before the rice rejection
        f.bitdepth = 16;
        f.max_rice_value = 31;
        expect_reject("err#9-partial", f);
        assert_eq!(
            pair().c.validate(f).1.channel_mode,
            0,
            "err#9 partial-mutation premise: {f:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// Row 10 — max_partition_order > 15
// ---------------------------------------------------------------------------

#[test]
fn err_row10_max_partition_order_too_large() {
    let mut rng = Rng::new(0x100A);
    for max_po in 16u8..=255 {
        for _ in 0..16 {
            let mut f = base(&mut rng);
            f.max_partition_order = max_po;
            f.min_partition_order = rng.next_u8();
            expect_reject("err#10", f);
        }
    }
    // The rice auto-fill happens BEFORE this rejection: verify both buckets.
    for (bd, want_rice) in [(16u32, 14u8), (17, 30), (32, 30)] {
        for _ in 0..256 {
            let mut f = base(&mut rng);
            f.bitdepth = bd;
            f.max_rice_value = 0;
            f.max_partition_order = 16;
            expect_reject("err#10-partial", f);
            assert_eq!(
                pair().c.validate(f).1.max_rice_value,
                want_rice,
                "err#10 rice auto-fill premise: {f:?}"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Row 11 — min_partition_order > max_partition_order (with max <= 15)
// ---------------------------------------------------------------------------

#[test]
fn err_row11_min_gt_max_partition_order() {
    let mut rng = Rng::new(0x100B);
    for max_po in 0u8..=15 {
        for min_po in (max_po as u16 + 1)..=255 {
            let mut f = base(&mut rng);
            f.max_partition_order = max_po;
            f.min_partition_order = min_po as u8;
            expect_reject("err#11", f);
        }
    }
    // Adjacent pairs, plus the rice auto-fill partial mutation.
    for max_po in 0u8..=14 {
        for _ in 0..64 {
            let mut f = base(&mut rng);
            f.max_partition_order = max_po;
            f.min_partition_order = max_po + 1;
            f.max_rice_value = 0;
            f.bitdepth = if rng.next_u64() % 2 == 0 { 16 } else { 24 };
            expect_reject("err#11-adjacent", f);
            let want = if f.bitdepth <= 16 { 14 } else { 30 };
            assert_eq!(
                pair().c.validate(f).1.max_rice_value,
                want,
                "err#11 rice auto-fill premise: {f:?}"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Rejection precedence — several triggers at once
// ---------------------------------------------------------------------------

#[test]
fn err_rejection_precedence_multi_trigger() {
    let mut rng = Rng::new(0x100C);
    // Hand-built combinations where more than one check would fire.
    let bad_bs = [0u32, 15, 65536, u32::MAX];
    let bad_sr = [0u32, 655_351, u32::MAX];
    let bad_ch = [0u32, 9, u32::MAX];
    let bad_bd = [0u32, 33, u32::MAX];
    let bad_rice = [31u8, 255];
    let bad_maxpo = [16u8, 255];
    for &bs in &bad_bs {
        for &sr in &bad_sr {
            for &ch in &bad_ch {
                for &bd in &bad_bd {
                    for &rice in &bad_rice {
                        for &mpo in &bad_maxpo {
                            let mut f = base(&mut rng);
                            f.blocksize = bs;
                            f.samplerate = sr;
                            f.channels = ch;
                            f.bitdepth = bd;
                            f.max_rice_value = rice;
                            f.max_partition_order = mpo;
                            f.min_partition_order = 255;
                            // Earliest failing check wins -> blocksize.
                            expect_reject_unmodified("err#precedence", f);
                        }
                    }
                }
            }
        }
    }
    // Randomized: every field independently drawn from {valid, invalid}.
    for _ in 0..iters(500_000) {
        let mut f = base(&mut rng);
        if rng.next_u64() % 2 == 0 {
            f.blocksize = rng.pick(&bad_bs);
        }
        if rng.next_u64() % 2 == 0 {
            f.samplerate = rng.pick(&bad_sr);
        }
        if rng.next_u64() % 2 == 0 {
            f.channels = rng.pick(&bad_ch);
        }
        if rng.next_u64() % 2 == 0 {
            f.bitdepth = rng.pick(&bad_bd);
        }
        if rng.next_u64() % 2 == 0 {
            f.max_rice_value = rng.pick(&bad_rice);
        }
        if rng.next_u64() % 2 == 0 {
            f.max_partition_order = rng.pick(&bad_maxpo);
        }
        if rng.next_u64() % 2 == 0 {
            f.min_partition_order = rng.next_u8();
        }
        check_validate("err#precedence-rand", f);
    }
}

// ---------------------------------------------------------------------------
// G1 — NULL pointer: the C code has no null check and dereferences `t`
// ---------------------------------------------------------------------------

/// Child-process half of the NULL test. Loads the `.so` named by
/// `HARVEST_NULL_SO` and calls `flac_validate(NULL)`, which must fault.
#[test]
#[ignore = "child process half of err_g1_null_pointer_segv_parity"]
fn err_g1_null_child() {
    let path = std::env::var("HARVEST_NULL_SO").expect("HARVEST_NULL_SO must be set");
    let lib = unsafe { libloading::Library::new(&path) }.expect("dlopen");
    let f: libloading::Symbol<unsafe extern "C" fn(*mut u8) -> std::ffi::c_int> =
        unsafe { lib.get(b"flac_validate\0") }.expect("flac_validate");
    let ret = unsafe { f(std::ptr::null_mut()) };
    // Must never get here: the call dereferences NULL.
    println!("UNEXPECTEDLY RETURNED {ret}");
    std::process::exit(0);
}

#[test]
fn err_g1_null_pointer_segv_parity() {
    use std::os::unix::process::ExitStatusExt;

    let run = |so: &std::path::Path| -> (Option<i32>, Option<i32>) {
        let out = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "err_g1_null_child",
                "--ignored",
                "--nocapture",
                "--test-threads=1",
            ])
            .env("HARVEST_NULL_SO", so)
            .env("RUST_BACKTRACE", "0")
            .output()
            .expect("spawn child");
        (out.status.code(), out.status.signal())
    };

    let c = run(&c_so_path());
    let r = run(&rust_release_so_path());

    const SIGSEGV: i32 = 11;
    assert_eq!(
        c,
        (None, Some(SIGSEGV)),
        "C flac_validate(NULL) should fault with SIGSEGV, got {c:?}"
    );
    assert_eq!(
        r, c,
        "Rust flac_validate(NULL) must fault exactly like C: rust={r:?} c={c:?}"
    );
}

// ---------------------------------------------------------------------------
// G2 — out-of-range enum value for channel_mode
// ---------------------------------------------------------------------------

#[test]
fn err_g2_out_of_range_channel_mode_enum() {
    let mut rng = Rng::new(0x100D);
    // Every u8 value, including TFLAC_CHANNEL_MODE_COUNT (4) and beyond, for
    // both branch outcomes of the `channels != 2 || bitdepth == 32` test.
    for mode in 0u8..=255 {
        // survives (channels == 2 && bitdepth != 32)
        let mut a = base(&mut rng);
        a.channel_mode = mode;
        a.channels = 2;
        a.bitdepth = rng.range_u32(1, 31);
        check_validate_ret("err#G2a", a, 0);
        assert_eq!(pair().c.validate(a).1.channel_mode, mode, "G2a: {a:?}");

        // forced to independent (channels != 2)
        let mut b = base(&mut rng);
        b.channel_mode = mode;
        b.channels = if rng.next_u64() % 2 == 0 { 1 } else { 8 };
        b.bitdepth = rng.range_u32(1, 31);
        check_validate_ret("err#G2b", b, 0);
        assert_eq!(pair().c.validate(b).1.channel_mode, 0, "G2b: {b:?}");

        // forced to independent (bitdepth == 32)
        let mut c = base(&mut rng);
        c.channel_mode = mode;
        c.channels = 2;
        c.bitdepth = 32;
        check_validate_ret("err#G2c", c, 0);
        assert_eq!(pair().c.validate(c).1.channel_mode, 0, "G2c: {c:?}");

        // rejected later, after the mode was possibly rewritten
        let mut d = base(&mut rng);
        d.channel_mode = mode;
        d.channels = 3;
        d.max_partition_order = 16;
        check_validate_ret("err#G2d", d, -1);
    }
}

// ---------------------------------------------------------------------------
// G3..G8 — one step past every documented range boundary
// ---------------------------------------------------------------------------

#[test]
fn err_g3_max_rice_value_one_past() {
    let mut rng = Rng::new(0x100E);
    for _ in 0..iters(ITERS) {
        let mut ok = base(&mut rng);
        ok.max_rice_value = 30;
        check_validate_ret("err#G3-ok", ok, 0);

        let mut bad = ok;
        bad.max_rice_value = 31;
        check_validate_ret("err#G3-bad", bad, -1);
    }
}

#[test]
fn err_g4_max_partition_order_one_past() {
    let mut rng = Rng::new(0x100F);
    for _ in 0..iters(ITERS) {
        let mut ok = base(&mut rng);
        ok.max_partition_order = 15;
        ok.min_partition_order = rng.range_u8(0, 15);
        check_validate_ret("err#G4-ok", ok, 0);

        let mut bad = ok;
        bad.max_partition_order = 16;
        check_validate_ret("err#G4-bad", bad, -1);
    }
}

#[test]
fn err_g5_blocksize_one_past() {
    let mut rng = Rng::new(0x1010);
    for _ in 0..iters(ITERS) {
        let mut f = base(&mut rng);
        for (bs, want) in [(15u32, -1), (16, 0), (65535, 0), (65536, -1)] {
            f.blocksize = bs;
            check_validate_ret("err#G5", f, want);
        }
    }
}

#[test]
fn err_g6_samplerate_one_past() {
    let mut rng = Rng::new(0x1011);
    for _ in 0..iters(ITERS) {
        let mut f = base(&mut rng);
        for (sr, want) in [(0u32, -1), (1, 0), (655_350, 0), (655_351, -1)] {
            f.samplerate = sr;
            check_validate_ret("err#G6", f, want);
        }
    }
}

#[test]
fn err_g7_channels_one_past() {
    let mut rng = Rng::new(0x1012);
    for _ in 0..iters(ITERS) {
        let mut f = base(&mut rng);
        for (ch, want) in [(0u32, -1), (1, 0), (8, 0), (9, -1)] {
            f.channels = ch;
            check_validate_ret("err#G7", f, want);
        }
    }
}

#[test]
fn err_g8_bitdepth_one_past() {
    let mut rng = Rng::new(0x1013);
    for _ in 0..iters(ITERS) {
        let mut f = base(&mut rng);
        for (bd, want) in [(0u32, -1), (1, 0), (32, 0), (33, -1)] {
            f.bitdepth = bd;
            check_validate_ret("err#G8", f, want);
        }
    }
}

// ---------------------------------------------------------------------------
// G9 — partition_order / cur_blocksize are pure outputs
// ---------------------------------------------------------------------------

#[test]
fn err_g9_output_fields_are_pure_outputs() {
    let mut rng = Rng::new(0x1014);
    for _ in 0..iters(ITERS) {
        // On success they must be overwritten regardless of the seeded garbage.
        let mut ok = base(&mut rng);
        let garbage_po = rng.next_u8();
        let garbage_cb = rng.next_u32();
        ok.partition_order = garbage_po;
        ok.cur_blocksize = garbage_cb;
        check_validate_ret("err#G9-ok", ok, 0);
        let (_, out) = pair().c.validate(ok);
        assert_eq!(out.cur_blocksize, ok.blocksize, "G9: {ok:?}");

        // On rejection they must keep the garbage.
        let mut bad = ok;
        bad.blocksize = 0;
        check_validate_ret("err#G9-bad", bad, -1);
        let (_, bout) = pair().c.validate(bad);
        assert_eq!(bout.partition_order, garbage_po, "G9 bad: {bad:?}");
        assert_eq!(bout.cur_blocksize, garbage_cb, "G9 bad: {bad:?}");
    }
}

// ---------------------------------------------------------------------------
// G10 — tflac_size_memory has no error path: zero / oversized / wrapping
// ---------------------------------------------------------------------------

#[test]
fn err_g10_size_memory_extremes() {
    for bs in [
        0u32,
        1,
        2,
        3,
        4,
        15,
        16,
        0x3FFF_FFFF,
        0x4000_0000,
        0x4000_0001,
        0x7FFF_FFFF,
        0x8000_0000,
        0xFFFF_FFF0,
        0xFFFF_FFFC,
        0xFFFF_FFFD,
        0xFFFF_FFFE,
        0xFFFF_FFFF,
    ] {
        check_size_memory("err#G10", bs);
    }
}

// ---------------------------------------------------------------------------
// Exhaustive error-path sweeps over the byte-sized fields.
// ---------------------------------------------------------------------------

/// ERRORS rows 9, 10, 11 + G3, G4, EXHAUSTIVELY: every `(max_rice_value,
/// min_partition_order, max_partition_order)` triple is 256^3 = 16.7M, which is
/// pruned to the two 2-D planes the rejections actually depend on.
#[test]
fn err_exhaustive_u8_fields() {
    let p = pair();
    let mut f = Fields::valid_base();
    f.blocksize = 4096;
    f.samplerate = 44100;
    f.channels = 2;
    f.bitdepth = 24;
    f.channel_mode = 2;

    let cmp = |f: Fields| {
        let (cret, cout) = p.c.validate(f);
        for r in &p.rust {
            let (rret, rout) = r.validate(f);
            assert_eq!(
                (cret, cout.to_raw()),
                (rret, rout.to_raw()),
                "exhaustive u8 mismatch ({} vs {}) for {f:?}",
                p.c.name,
                r.name
            );
        }
        cret
    };

    // Plane 1: all 65 536 (min_partition_order, max_partition_order) pairs,
    // including every out-of-range value, with rice auto-fill active.
    f.max_rice_value = 0;
    for min_po in 0u8..=255 {
        f.min_partition_order = min_po;
        for max_po in 0u8..=255 {
            f.max_partition_order = max_po;
            let want = if max_po > 15 || min_po > max_po { -1 } else { 0 };
            assert_eq!(cmp(f), want, "unexpected C verdict for {f:?}");
        }
    }

    // Plane 2: all 256 max_rice_value x all 256 max_partition_order values.
    f.min_partition_order = 0;
    for rice in 0u8..=255 {
        f.max_rice_value = rice;
        for max_po in 0u8..=255 {
            f.max_partition_order = max_po;
            let want = if rice > 30 {
                -1
            } else if max_po > 15 {
                -1
            } else {
                0
            };
            assert_eq!(cmp(f), want, "unexpected C verdict for {f:?}");
        }
    }

    // Plane 3: all 256 channel_mode x all 256 max_rice_value, so the
    // "partially mutated then rejected" states are enumerated too.
    f.max_partition_order = 15;
    for mode in 0u8..=255 {
        f.channel_mode = mode;
        for rice in 0u8..=255 {
            f.max_rice_value = rice;
            let want = if rice > 30 { -1 } else { 0 };
            assert_eq!(cmp(f), want, "unexpected C verdict for {f:?}");
        }
    }
}

/// ERRORS rows 1..8 + G5..G8, EXHAUSTIVELY over each u32 field's decision
/// boundary neighbourhood (the checks are pure comparisons, so a dense sweep
/// around every threshold plus the extremes is exhaustive in effect).
#[test]
fn err_exhaustive_u32_field_boundaries() {
    let p = pair();
    let base_ok = {
        let mut f = Fields::valid_base();
        f.blocksize = 4096;
        f.samplerate = 44100;
        f.channels = 2;
        f.bitdepth = 16;
        f.channel_mode = 3;
        f.max_rice_value = 0;
        f.min_partition_order = 1;
        f.max_partition_order = 12;
        f
    };
    let cmp = |f: Fields, want: i32| {
        let (cret, cout) = p.c.validate(f);
        assert_eq!(cret, want, "unexpected C verdict for {f:?}");
        for r in &p.rust {
            let (rret, rout) = r.validate(f);
            assert_eq!(
                (cret, cout.to_raw()),
                (rret, rout.to_raw()),
                "u32 boundary mismatch ({} vs {}) for {f:?}",
                p.c.name,
                r.name
            );
        }
    };

    // blocksize: whole valid range plus a dense invalid neighbourhood.
    for bs in 0u32..=70_000 {
        let mut f = base_ok;
        f.blocksize = bs;
        cmp(f, if (16..=65535).contains(&bs) { 0 } else { -1 });
    }
    for bs in [u32::MAX, u32::MAX - 1, 1 << 31, 1 << 16, (1 << 16) + 1] {
        let mut f = base_ok;
        f.blocksize = bs;
        cmp(f, -1);
    }

    // samplerate: dense sweep across the whole valid range and past the bound.
    for sr in 0u32..=660_000 {
        let mut f = base_ok;
        f.samplerate = sr;
        cmp(f, if (1..=655_350).contains(&sr) { 0 } else { -1 });
    }
    for sr in [u32::MAX, u32::MAX - 1, 1 << 31, 1_000_000] {
        let mut f = base_ok;
        f.samplerate = sr;
        cmp(f, -1);
    }

    // channels: exhaustive over a wide neighbourhood plus extremes.
    for ch in 0u32..=1024 {
        let mut f = base_ok;
        f.channels = ch;
        cmp(f, if (1..=8).contains(&ch) { 0 } else { -1 });
    }
    for ch in [u32::MAX, u32::MAX - 1, 1 << 31, 1 << 16] {
        let mut f = base_ok;
        f.channels = ch;
        cmp(f, -1);
    }

    // bitdepth: exhaustive over a wide neighbourhood plus extremes.
    for bd in 0u32..=1024 {
        let mut f = base_ok;
        f.bitdepth = bd;
        cmp(f, if (1..=32).contains(&bd) { 0 } else { -1 });
    }
    for bd in [u32::MAX, u32::MAX - 1, 1 << 31, 1 << 16] {
        let mut f = base_ok;
        f.bitdepth = bd;
        cmp(f, -1);
    }
}
