//! Phase C — error-path differential tests, one test per row of `ERRORS.md`.
//!
//! Each test builds the exact invalid input the C rejects, calls BOTH shared
//! objects and asserts the SAME sentinel (`-1` vs `0`) *and* the same 28-byte
//! struct image (partial in-place mutations included).

mod common;
use common::*;

/// A valid baseline that each row perturbs in exactly one way.
fn base() -> Fields {
    Fields {
        blocksize: 4096,
        samplerate: 44100,
        channels: 2,
        bitdepth: 16,
        channel_mode: 0,
        max_rice_value: 0,
        min_partition_order: 0,
        max_partition_order: 4,
        partition_order: 0xEE,
        padding: [0xAA, 0xBB, 0xCC],
        cur_blocksize: 0xDEAD_BEEF,
    }
}

#[track_caller]
fn expect_reject_unmodified(f: Fields) {
    let o = diff_validate(f);
    assert_eq!(o.ret, -1, "expected rejection for {f:?}");
    assert_eq!(
        o.out,
        f.to_raw(),
        "check fires before any mutation, so the struct must be untouched: {f:?}"
    );
}

// ---------------------------------------------------------------------------
// Row 1 — blocksize < 16
// ---------------------------------------------------------------------------
#[test]
fn err_01_blocksize_too_small() {
    for bs in 0u32..16 {
        expect_reject_unmodified(Fields { blocksize: bs, ..base() });
    }
    // randomized: every other field random too, blocksize still < 16
    let mut rng = Rng::new(0x0001);
    for _ in 0..2_000 {
        let mut f = rng.valid_fields();
        f.blocksize = rng.range_u32(0, 15);
        expect_reject_unmodified(f);
    }
}

// ---------------------------------------------------------------------------
// Row 2 — blocksize > 65535
// ---------------------------------------------------------------------------
#[test]
fn err_02_blocksize_too_large() {
    for bs in [65536u32, 65537, 70000, 0x0001_0000, 0x8000_0000, u32::MAX] {
        expect_reject_unmodified(Fields { blocksize: bs, ..base() });
    }
    let mut rng = Rng::new(0x0002);
    for _ in 0..2_000 {
        let mut f = rng.valid_fields();
        f.blocksize = rng.range_u32(65536, u32::MAX);
        expect_reject_unmodified(f);
    }
}

// ---------------------------------------------------------------------------
// Row 3 — samplerate == 0
// ---------------------------------------------------------------------------
#[test]
fn err_03_samplerate_zero() {
    expect_reject_unmodified(Fields { samplerate: 0, ..base() });
    let mut rng = Rng::new(0x0003);
    for _ in 0..2_000 {
        let mut f = rng.valid_fields();
        f.samplerate = 0;
        expect_reject_unmodified(f);
    }
}

// ---------------------------------------------------------------------------
// Row 4 — samplerate > 655350
// ---------------------------------------------------------------------------
#[test]
fn err_04_samplerate_too_large() {
    for sr in [655351u32, 655352, 1_000_000, 0x8000_0000, u32::MAX] {
        expect_reject_unmodified(Fields { samplerate: sr, ..base() });
    }
    let mut rng = Rng::new(0x0004);
    for _ in 0..2_000 {
        let mut f = rng.valid_fields();
        f.samplerate = rng.range_u32(655351, u32::MAX);
        expect_reject_unmodified(f);
    }
}

// ---------------------------------------------------------------------------
// Row 5 — channels == 0
// ---------------------------------------------------------------------------
#[test]
fn err_05_channels_zero() {
    expect_reject_unmodified(Fields { channels: 0, ..base() });
    let mut rng = Rng::new(0x0005);
    for _ in 0..2_000 {
        let mut f = rng.valid_fields();
        f.channels = 0;
        expect_reject_unmodified(f);
    }
}

// ---------------------------------------------------------------------------
// Row 6 — channels > 8
// ---------------------------------------------------------------------------
#[test]
fn err_06_channels_too_large() {
    for ch in [9u32, 10, 16, 255, 256, 0x8000_0000, u32::MAX] {
        expect_reject_unmodified(Fields { channels: ch, ..base() });
    }
    let mut rng = Rng::new(0x0006);
    for _ in 0..2_000 {
        let mut f = rng.valid_fields();
        f.channels = rng.range_u32(9, u32::MAX);
        expect_reject_unmodified(f);
    }
}

// ---------------------------------------------------------------------------
// Row 7 — bitdepth == 0
// ---------------------------------------------------------------------------
#[test]
fn err_07_bitdepth_zero() {
    expect_reject_unmodified(Fields { bitdepth: 0, ..base() });
    let mut rng = Rng::new(0x0007);
    for _ in 0..2_000 {
        let mut f = rng.valid_fields();
        f.bitdepth = 0;
        expect_reject_unmodified(f);
    }
}

// ---------------------------------------------------------------------------
// Row 8 — bitdepth > 32
// ---------------------------------------------------------------------------
#[test]
fn err_08_bitdepth_too_large() {
    for bd in [33u32, 34, 64, 255, 256, 0x8000_0000, u32::MAX] {
        expect_reject_unmodified(Fields { bitdepth: bd, ..base() });
    }
    let mut rng = Rng::new(0x0008);
    for _ in 0..2_000 {
        let mut f = rng.valid_fields();
        f.bitdepth = rng.range_u32(33, u32::MAX);
        expect_reject_unmodified(f);
    }
}

// ---------------------------------------------------------------------------
// Row 9 — max_rice_value > 30 (only reachable when != 0)
// ---------------------------------------------------------------------------
#[test]
fn err_09_max_rice_value_too_large() {
    for mrv in 31u8..=255 {
        let o = diff_validate(Fields { max_rice_value: mrv, ..base() });
        assert_eq!(o.ret, -1, "max_rice_value = {mrv} must be rejected");
        // channel_mode is already INDEPENDENT here, so nothing was mutated.
        assert_eq!(o.out, Fields { max_rice_value: mrv, ..base() }.to_raw());
    }
    let mut rng = Rng::new(0x0009);
    for _ in 0..2_000 {
        let mut f = rng.valid_fields();
        f.max_rice_value = rng.range_u8(31, 255);
        let o = diff_validate(f);
        assert_eq!(o.ret, -1);
    }
}

/// Row 9 with the in-place `channel_mode` rewrite already applied before the
/// rejection (the mutation must survive the `-1`).
#[test]
fn err_09b_max_rice_partial_mutation() {
    for mrv in [31u8, 32, 100, 255] {
        for (ch, bd) in [(3u32, 16u32), (1, 8), (2, 32), (8, 24)] {
            let f = Fields {
                channels: ch,
                bitdepth: bd,
                channel_mode: 3,
                max_rice_value: mrv,
                ..base()
            };
            let o = diff_validate(f);
            assert_eq!(o.ret, -1);
            let got = Fields::from_raw(o.out);
            assert_eq!(got.channel_mode, 0, "channel_mode reset happens before the -1");
            assert_eq!(got.max_rice_value, mrv);
            assert_eq!(got.partition_order, f.partition_order, "not reached");
            assert_eq!(got.cur_blocksize, f.cur_blocksize, "not reached");
        }
    }
    // ... and the case where the mode is preserved (stereo, bitdepth < 32).
    for mrv in [31u8, 200] {
        for mode in [1u8, 2, 3, 4, 255] {
            let f = Fields { channel_mode: mode, max_rice_value: mrv, ..base() };
            let o = diff_validate(f);
            assert_eq!(o.ret, -1);
            assert_eq!(Fields::from_raw(o.out).channel_mode, mode);
        }
    }
}

// ---------------------------------------------------------------------------
// Row 10 — max_partition_order > 15
// ---------------------------------------------------------------------------
#[test]
fn err_10_max_partition_order_too_large() {
    for mpo in 16u8..=255 {
        let f = Fields { max_partition_order: mpo, ..base() };
        let o = diff_validate(f);
        assert_eq!(o.ret, -1, "max_partition_order = {mpo} must be rejected");
    }
    let mut rng = Rng::new(0x0010);
    for _ in 0..2_000 {
        let mut f = rng.valid_fields();
        f.max_partition_order = rng.range_u8(16, 255);
        let o = diff_validate(f);
        assert_eq!(o.ret, -1);
    }
}

/// Row 10 with `channel_mode` and `max_rice_value` already rewritten.
#[test]
fn err_10b_partial_mutation() {
    for mpo in [16u8, 17, 100, 255] {
        // channels != 2 -> mode reset; max_rice_value == 0 -> auto-derived
        for (bd, want_rice) in [(8u32, 14u8), (16, 14), (17, 30), (32, 30)] {
            let f = Fields {
                channels: 5,
                bitdepth: bd,
                channel_mode: 2,
                max_rice_value: 0,
                max_partition_order: mpo,
                ..base()
            };
            let o = diff_validate(f);
            assert_eq!(o.ret, -1);
            let got = Fields::from_raw(o.out);
            assert_eq!(got.channel_mode, 0);
            assert_eq!(got.max_rice_value, want_rice, "rice default applied before the -1");
            assert_eq!(got.partition_order, f.partition_order);
            assert_eq!(got.cur_blocksize, f.cur_blocksize);
        }
    }
}

// ---------------------------------------------------------------------------
// Row 11 — min_partition_order > max_partition_order
// ---------------------------------------------------------------------------
#[test]
fn err_11_min_gt_max_partition_order() {
    for max_po in 0u8..=15 {
        for min_po in (max_po + 1)..=255 {
            let f =
                Fields { min_partition_order: min_po, max_partition_order: max_po, ..base() };
            let o = diff_validate(f);
            assert_eq!(o.ret, -1, "min {min_po} > max {max_po} must be rejected");
            let got = Fields::from_raw(o.out);
            assert_eq!(got.partition_order, f.partition_order, "loop not reached");
            assert_eq!(got.cur_blocksize, f.cur_blocksize, "not reached");
        }
    }
    let mut rng = Rng::new(0x0011);
    for _ in 0..2_000 {
        let mut f = rng.valid_fields();
        f.max_partition_order = rng.range_u8(0, 14);
        f.min_partition_order = rng.range_u8(f.max_partition_order + 1, 255);
        assert_eq!(diff_validate(f).ret, -1);
    }
}

/// Row 11 with the earlier in-place mutations visible.
#[test]
fn err_11b_partial_mutation() {
    let f = Fields {
        channels: 4,
        bitdepth: 24,
        channel_mode: 1,
        max_rice_value: 0,
        min_partition_order: 9,
        max_partition_order: 8,
        ..base()
    };
    let o = diff_validate(f);
    assert_eq!(o.ret, -1);
    let got = Fields::from_raw(o.out);
    assert_eq!(got.channel_mode, 0);
    assert_eq!(got.max_rice_value, 30);
    assert_eq!(got.partition_order, f.partition_order);
    assert_eq!(got.cur_blocksize, f.cur_blocksize);
}

// ---------------------------------------------------------------------------
// Row 12 — every field invalid at once: the FIRST check must win
// ---------------------------------------------------------------------------
#[test]
fn err_12_first_check_wins() {
    let f = Fields {
        blocksize: 0,
        samplerate: 0,
        channels: 0,
        bitdepth: 0,
        channel_mode: 7,
        max_rice_value: 200,
        min_partition_order: 200,
        max_partition_order: 100,
        partition_order: 0x11,
        padding: [9, 9, 9],
        cur_blocksize: 0x1234_5678,
    };
    expect_reject_unmodified(f);

    // Peel the invalid fields off one at a time; each successive check fires.
    let mut g = f;
    g.blocksize = 4096;
    expect_reject_unmodified(g); // samplerate == 0
    g.samplerate = 48000;
    expect_reject_unmodified(g); // channels == 0
    g.channels = 2;
    expect_reject_unmodified(g); // bitdepth == 0
    g.bitdepth = 16;
    let o = diff_validate(g); // now max_rice_value == 200 fires, after mode rewrite
    assert_eq!(o.ret, -1);
    g.max_rice_value = 14;
    let o = diff_validate(g); // max_partition_order == 100 fires
    assert_eq!(o.ret, -1);
    g.max_partition_order = 15;
    let o = diff_validate(g); // min_partition_order 200 > 15 fires
    assert_eq!(o.ret, -1);
    g.min_partition_order = 0;
    let o = diff_validate(g); // finally valid
    assert_eq!(o.ret, 0);
}

// ---------------------------------------------------------------------------
// Row 13 — randomized single-field-invalid structs
// ---------------------------------------------------------------------------
#[test]
fn err_13_randomized_invalid_structs() {
    let mut rng = Rng::new(0x0013_0013_0013_0013);
    for _ in 0..2_000 {
        let mut f = rng.valid_fields();
        match rng.next_u64() % 11 {
            0 => f.blocksize = rng.range_u32(0, 15),
            1 => f.blocksize = rng.range_u32(65536, u32::MAX),
            2 => f.samplerate = 0,
            3 => f.samplerate = rng.range_u32(655351, u32::MAX),
            4 => f.channels = 0,
            5 => f.channels = rng.range_u32(9, u32::MAX),
            6 => f.bitdepth = 0,
            7 => f.bitdepth = rng.range_u32(33, u32::MAX),
            8 => f.max_rice_value = rng.range_u8(31, 255),
            9 => f.max_partition_order = rng.range_u8(16, 255),
            _ => {
                f.max_partition_order = rng.range_u8(0, 14);
                f.min_partition_order = rng.range_u8(f.max_partition_order + 1, 255);
            }
        }
        let o = diff_validate(f);
        assert_eq!(o.ret, -1, "expected rejection for {f:?}");
    }
}

// ---------------------------------------------------------------------------
// Row 14 — NULL pointer: the C has no null check, so both must fault the same
// way. Verified in a forked child so the harness survives.
// ---------------------------------------------------------------------------
#[derive(Debug, PartialEq, Eq)]
enum Termination {
    Exited(i32),
    Signalled(i32),
}

fn run_in_child<F: FnOnce()>(body: F) -> Termination {
    unsafe {
        let pid = libc::fork();
        assert!(pid >= 0, "fork failed");
        if pid == 0 {
            body();
            libc::_exit(0);
        }
        let mut status: libc::c_int = 0;
        let r = libc::waitpid(pid, &mut status, 0);
        assert_eq!(r, pid, "waitpid failed");
        if libc::WIFSIGNALED(status) {
            Termination::Signalled(libc::WTERMSIG(status))
        } else {
            Termination::Exited(libc::WEXITSTATUS(status))
        }
    }
}

#[test]
fn err_14_null_pointer_faults_identically() {
    let l = libs();
    let cf = l.c.validate;
    let rf = l.rust.validate;
    let c_term = run_in_child(|| {
        let ret = unsafe { cf(std::ptr::null_mut()) };
        unsafe { libc::_exit(if ret == 0 { 10 } else { 11 }) };
    });
    let r_term = run_in_child(|| {
        let ret = unsafe { rf(std::ptr::null_mut()) };
        unsafe { libc::_exit(if ret == 0 { 10 } else { 11 }) };
    });
    assert_eq!(
        c_term, r_term,
        "NULL handling diverges: C = {c_term:?}, Rust = {r_term:?}"
    );
    assert_eq!(
        c_term,
        Termination::Signalled(libc::SIGSEGV),
        "expected an unchecked NULL dereference (SIGSEGV) in both"
    );
}

// ---------------------------------------------------------------------------
// Row 15 — out-of-range enum values across the FFI boundary
// ---------------------------------------------------------------------------
#[test]
fn err_15_channel_mode_all_256_values() {
    for mode in 0u8..=255 {
        for ch in [1u32, 2, 3, 8] {
            for bd in [1u32, 16, 31, 32] {
                let f = Fields {
                    channels: ch,
                    bitdepth: bd,
                    channel_mode: mode,
                    max_rice_value: 0,
                    min_partition_order: 0,
                    max_partition_order: 15,
                    ..base()
                };
                let o = diff_validate(f);
                assert_eq!(o.ret, 0);
                let want = if mode != 0 && (ch != 2 || bd == 32) { 0 } else { mode };
                assert_eq!(Fields::from_raw(o.out).channel_mode, want);
            }
        }
    }
    // Also on rejecting paths, so the "reset then -1" ordering is compared.
    for mode in 0u8..=255 {
        let f = Fields { channels: 7, channel_mode: mode, max_rice_value: 31, ..base() };
        let o = diff_validate(f);
        assert_eq!(o.ret, -1);
        assert_eq!(Fields::from_raw(o.out).channel_mode, if mode != 0 { 0 } else { 0 });
    }
}

// ---------------------------------------------------------------------------
// Row 16 — one step past every documented boundary, both sides
// ---------------------------------------------------------------------------
#[test]
fn err_16_one_past_every_boundary() {
    // (field setter, values around the boundary)
    for bs in [15u32, 16, 65535, 65536] {
        diff_validate(Fields { blocksize: bs, ..base() });
    }
    for sr in [0u32, 1, 655350, 655351] {
        diff_validate(Fields { samplerate: sr, ..base() });
    }
    for ch in [0u32, 1, 8, 9] {
        diff_validate(Fields { channels: ch, ..base() });
    }
    for bd in [0u32, 1, 32, 33] {
        diff_validate(Fields { bitdepth: bd, ..base() });
    }
    for mrv in [0u8, 1, 30, 31] {
        diff_validate(Fields { max_rice_value: mrv, ..base() });
    }
    for mpo in [0u8, 15, 16] {
        diff_validate(Fields { max_partition_order: mpo, min_partition_order: 0, ..base() });
    }
    for max_po in [0u8, 1, 14, 15] {
        for min_po in [max_po, max_po.wrapping_add(1)] {
            diff_validate(Fields {
                min_partition_order: min_po,
                max_partition_order: max_po,
                ..base()
            });
        }
    }
    // Cross-product of all boundary values at once (4^4 * 4 * 3 combinations).
    for bs in [15u32, 16, 65535, 65536] {
        for sr in [0u32, 1, 655350, 655351] {
            for ch in [0u32, 1, 2, 8, 9] {
                for bd in [0u32, 1, 16, 17, 32, 33] {
                    for mrv in [0u8, 1, 30, 31] {
                        for mpo in [0u8, 15, 16] {
                            diff_validate(Fields {
                                blocksize: bs,
                                samplerate: sr,
                                channels: ch,
                                bitdepth: bd,
                                max_rice_value: mrv,
                                max_partition_order: mpo,
                                min_partition_order: 0,
                                channel_mode: 3,
                                ..base()
                            });
                        }
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 17 — all-zero and all-ones struct images
// ---------------------------------------------------------------------------
#[test]
fn err_17_all_zero_and_all_ones() {
    let z = diff_validate_raw(Raw([0u8; TFLAC_SIZE]));
    assert_eq!(z.ret, -1, "blocksize 0 -> rejected");
    assert_eq!(z.out, Raw([0u8; TFLAC_SIZE]));

    let f = diff_validate_raw(Raw([0xFFu8; TFLAC_SIZE]));
    assert_eq!(f.ret, -1, "blocksize 0xFFFFFFFF -> rejected");
    assert_eq!(f.out, Raw([0xFFu8; TFLAC_SIZE]));

    // Alternating patterns
    for pat in [0x55u8, 0xAA, 0x01, 0x7F, 0x80] {
        diff_validate_raw(Raw([pat; TFLAC_SIZE]));
    }
}

// ---------------------------------------------------------------------------
// Row 18 — padding bytes must be preserved on both error and success paths
// ---------------------------------------------------------------------------
#[test]
fn err_18_padding_bytes_preserved() {
    for pad in [[0xAAu8, 0xAA, 0xAA], [0xFF, 0x00, 0xFF], [1, 2, 3]] {
        // error path
        let f = Fields { blocksize: 4, padding: pad, ..base() };
        let o = diff_validate(f);
        assert_eq!(o.ret, -1);
        assert_eq!(Fields::from_raw(o.out).padding, pad);
        // success path
        let g = Fields { padding: pad, ..base() };
        let o = diff_validate(g);
        assert_eq!(o.ret, 0);
        assert_eq!(Fields::from_raw(o.out).padding, pad);
    }
}

// ---------------------------------------------------------------------------
// Row 19 — tflac_size_memory degenerate / oversized inputs
// ---------------------------------------------------------------------------
#[test]
fn err_19_size_memory_extremes() {
    for bs in [0u32, 1, 15, 16, 0x3FFF_FFFB, 0x3FFF_FFFC, 0x3FFF_FFFD, 0xFFFF_FFFF] {
        diff_size_memory(bs);
    }
}

// ---------------------------------------------------------------------------
// Row 20 — double call observes the first call's mutations
// ---------------------------------------------------------------------------
#[test]
fn err_20_double_call_sees_mutations() {
    let l = libs();
    let mut rng = Rng::new(0x0020_0020_0020_0020);
    for _ in 0..2_000 {
        // mix of valid and invalid starting states
        let f = if rng.next_u64() % 2 == 0 {
            rng.valid_fields()
        } else {
            let mut g = rng.valid_fields();
            g.max_rice_value = rng.range_u8(31, 255);
            g
        };
        let mut cbuf = f.to_raw();
        let mut rbuf = f.to_raw();
        for call in 0..2 {
            let cret = unsafe { (l.c.validate)(cbuf.0.as_mut_ptr()) };
            let rret = unsafe { (l.rust.validate)(rbuf.0.as_mut_ptr()) };
            assert_eq!(cret, rret, "call #{call} for {f:?}");
            assert_eq!(cbuf, rbuf, "call #{call} for {f:?}");
        }
    }
}
