//! Phase C — error-path differential tests. One test per row of `ERRORS.md`.
//!
//! Each test asserts the SAME return code (`-1`) *and* the same resulting 28
//! struct bytes, so partial mutation before an error return is verified too.

mod harness;

use harness::{Rng, Tflac, load_pair};

const SEED: u64 = 0xBADC0DE_9876_5432u64;

/// Builds an otherwise-valid struct whose bytes (incl. output fields and tail
/// padding) are random garbage, so any spurious write shows up in the compare.
fn base(rng: &mut Rng) -> Tflac {
    let mut t = Tflac::zeroed();
    rng.fill(&mut t.0);
    t.set_blocksize(rng.range_u32(16, 65535))
        .set_samplerate(rng.range_u32(1, 655350))
        .set_channels(rng.range_u32(1, 8))
        .set_bitdepth(rng.range_u32(1, 32))
        .set_max_rice_value(rng.range_u8(0, 30));
    let maxpo = rng.range_u8(0, 15);
    let minpo = rng.range_u8(0, maxpo);
    t.set_max_partition_order(maxpo)
        .set_min_partition_order(minpo);
    t
}

/// Assert both implementations agree AND that the rejection really is `-1`.
fn expect_reject(p: &harness::Pair, t: &Tflac) {
    p.cmp_validate(t);
    let (rc, _) = p.c.validate(t);
    assert_eq!(rc, -1, "expected C to reject, got rc={rc} for {t:?}");
}

// ---------------------------------------------------------------------------
// ERRORS row 1 — blocksize < 16
// ---------------------------------------------------------------------------
#[test]
fn err_01_blocksize_too_small() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 1);
    for bs in 0u32..16 {
        let mut t = base(&mut rng);
        t.set_blocksize(bs);
        expect_reject(&p, &t);
    }
    for _ in 0..20_000 {
        let mut t = base(&mut rng);
        t.set_blocksize(rng.range_u32(0, 15));
        expect_reject(&p, &t);
    }
    // One step past the lower bound in the valid direction must NOT reject.
    let mut t = base(&mut rng);
    t.set_blocksize(16);
    p.cmp_validate(&t);
    assert_eq!(p.c.validate(&t).0, 0);
}

// ---------------------------------------------------------------------------
// ERRORS row 2 — blocksize > 65535
// ---------------------------------------------------------------------------
#[test]
fn err_02_blocksize_too_large() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 2);
    for bs in [65536u32, 65537, 65540, 0x1_0000, 0x7FFF_FFFF, 0x8000_0000, u32::MAX] {
        let mut t = base(&mut rng);
        t.set_blocksize(bs);
        expect_reject(&p, &t);
    }
    for _ in 0..20_000 {
        let mut t = base(&mut rng);
        t.set_blocksize(rng.range_u32(65536, u32::MAX));
        expect_reject(&p, &t);
    }
    let mut t = base(&mut rng);
    t.set_blocksize(65535);
    p.cmp_validate(&t);
    assert_eq!(p.c.validate(&t).0, 0);
}

// ---------------------------------------------------------------------------
// ERRORS row 3 — samplerate == 0
// ---------------------------------------------------------------------------
#[test]
fn err_03_samplerate_zero() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 3);
    for _ in 0..20_000 {
        let mut t = base(&mut rng);
        t.set_samplerate(0);
        expect_reject(&p, &t);
    }
    let mut t = base(&mut rng);
    t.set_samplerate(1);
    p.cmp_validate(&t);
    assert_eq!(p.c.validate(&t).0, 0);
}

// ---------------------------------------------------------------------------
// ERRORS row 4 — samplerate > 655350
// ---------------------------------------------------------------------------
#[test]
fn err_04_samplerate_too_large() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 4);
    for sr in [655351u32, 655352, 700000, 0x7FFF_FFFF, 0x8000_0000, u32::MAX] {
        let mut t = base(&mut rng);
        t.set_samplerate(sr);
        expect_reject(&p, &t);
    }
    for _ in 0..20_000 {
        let mut t = base(&mut rng);
        t.set_samplerate(rng.range_u32(655351, u32::MAX));
        expect_reject(&p, &t);
    }
    let mut t = base(&mut rng);
    t.set_samplerate(655350);
    p.cmp_validate(&t);
    assert_eq!(p.c.validate(&t).0, 0);
}

// ---------------------------------------------------------------------------
// ERRORS row 5 — channels == 0
// ---------------------------------------------------------------------------
#[test]
fn err_05_channels_zero() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 5);
    for _ in 0..20_000 {
        let mut t = base(&mut rng);
        t.set_channels(0);
        expect_reject(&p, &t);
    }
    let mut t = base(&mut rng);
    t.set_channels(1);
    p.cmp_validate(&t);
    assert_eq!(p.c.validate(&t).0, 0);
}

// ---------------------------------------------------------------------------
// ERRORS row 6 — channels > 8
// ---------------------------------------------------------------------------
#[test]
fn err_06_channels_too_large() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 6);
    for ch in [9u32, 10, 16, 255, 256, 0x8000_0000, u32::MAX] {
        let mut t = base(&mut rng);
        t.set_channels(ch);
        expect_reject(&p, &t);
    }
    for _ in 0..20_000 {
        let mut t = base(&mut rng);
        t.set_channels(rng.range_u32(9, u32::MAX));
        expect_reject(&p, &t);
    }
    let mut t = base(&mut rng);
    t.set_channels(8);
    p.cmp_validate(&t);
    assert_eq!(p.c.validate(&t).0, 0);
}

// ---------------------------------------------------------------------------
// ERRORS row 7 — bitdepth == 0
// ---------------------------------------------------------------------------
#[test]
fn err_07_bitdepth_zero() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 7);
    for _ in 0..20_000 {
        let mut t = base(&mut rng);
        t.set_bitdepth(0);
        expect_reject(&p, &t);
    }
    let mut t = base(&mut rng);
    t.set_bitdepth(1);
    p.cmp_validate(&t);
    assert_eq!(p.c.validate(&t).0, 0);
}

// ---------------------------------------------------------------------------
// ERRORS row 8 — bitdepth > 32
// ---------------------------------------------------------------------------
#[test]
fn err_08_bitdepth_too_large() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 8);
    for bd in [33u32, 34, 64, 255, 256, 0x8000_0000, u32::MAX] {
        let mut t = base(&mut rng);
        t.set_bitdepth(bd);
        expect_reject(&p, &t);
    }
    for _ in 0..20_000 {
        let mut t = base(&mut rng);
        t.set_bitdepth(rng.range_u32(33, u32::MAX));
        expect_reject(&p, &t);
    }
    let mut t = base(&mut rng);
    t.set_bitdepth(32);
    p.cmp_validate(&t);
    assert_eq!(p.c.validate(&t).0, 0);
}

// ---------------------------------------------------------------------------
// ERRORS row 9 — max_rice_value in 31..=255
// (with the channel_mode-normalising prefix active, so partial mutation shows)
// ---------------------------------------------------------------------------
#[test]
fn err_09_max_rice_value_too_large() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 9);
    for mrv in 31..=255u8 {
        // channels != 2 => channel_mode is rewritten to 0 before the reject.
        let mut t = base(&mut rng);
        t.set_max_rice_value(mrv)
            .set_channel_mode(rng.range_u8(1, 255))
            .set_channels(5);
        expect_reject(&p, &t);
        // channels == 2, bitdepth < 32 => channel_mode survives the reject.
        let mut t = base(&mut rng);
        t.set_max_rice_value(mrv)
            .set_channel_mode(rng.range_u8(1, 255))
            .set_channels(2)
            .set_bitdepth(16);
        expect_reject(&p, &t);
        // channels == 2, bitdepth == 32 => rewritten to 0 before the reject.
        let mut t = base(&mut rng);
        t.set_max_rice_value(mrv)
            .set_channel_mode(rng.range_u8(1, 255))
            .set_channels(2)
            .set_bitdepth(32);
        expect_reject(&p, &t);
    }
    for _ in 0..20_000 {
        let mut t = base(&mut rng);
        t.set_max_rice_value(rng.range_u8(31, 255))
            .set_channel_mode(rng.next_u8());
        expect_reject(&p, &t);
    }
    // 30 is still valid.
    let mut t = base(&mut rng);
    t.set_max_rice_value(30);
    p.cmp_validate(&t);
    assert_eq!(p.c.validate(&t).0, 0);
}

// ---------------------------------------------------------------------------
// ERRORS row 10 — max_partition_order > 15
// (channel_mode AND max_rice_value already mutated at this point)
// ---------------------------------------------------------------------------
#[test]
fn err_10_max_partition_order_too_large() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 10);
    for maxpo in 16..=255u8 {
        for (ch, bd) in [(5u32, 8u32), (2, 16), (2, 32), (1, 24)] {
            let mut t = base(&mut rng);
            t.set_max_partition_order(maxpo)
                .set_min_partition_order(rng.next_u8())
                .set_channel_mode(rng.range_u8(1, 255))
                .set_max_rice_value(0) // forces the defaulting mutation
                .set_channels(ch)
                .set_bitdepth(bd);
            expect_reject(&p, &t);
        }
    }
    for _ in 0..20_000 {
        let mut t = base(&mut rng);
        t.set_max_partition_order(rng.range_u8(16, 255))
            .set_min_partition_order(rng.next_u8())
            .set_channel_mode(rng.next_u8())
            .set_max_rice_value(0);
        expect_reject(&p, &t);
    }
    // 15 is still valid.
    let mut t = base(&mut rng);
    t.set_max_partition_order(15).set_min_partition_order(0);
    p.cmp_validate(&t);
    assert_eq!(p.c.validate(&t).0, 0);
}

// ---------------------------------------------------------------------------
// ERRORS row 11 — min_partition_order > max_partition_order
// ---------------------------------------------------------------------------
#[test]
fn err_11_min_gt_max_partition_order() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 11);
    for maxpo in 0..=15u8 {
        for minpo in (maxpo as u16 + 1)..=255 {
            let mut t = base(&mut rng);
            t.set_max_partition_order(maxpo)
                .set_min_partition_order(minpo as u8)
                .set_channel_mode(rng.range_u8(1, 255))
                .set_max_rice_value(0)
                .set_channels(rng.range_u32(1, 8))
                .set_bitdepth(rng.range_u32(1, 32));
            expect_reject(&p, &t);
        }
    }
    for _ in 0..20_000 {
        let mut t = base(&mut rng);
        let maxpo = rng.range_u8(0, 14);
        t.set_max_partition_order(maxpo)
            .set_min_partition_order(rng.range_u8(maxpo + 1, 255))
            .set_channel_mode(rng.next_u8())
            .set_max_rice_value(0);
        expect_reject(&p, &t);
    }
    // min == max is valid.
    for po in 0..=15u8 {
        let mut t = base(&mut rng);
        t.set_min_partition_order(po).set_max_partition_order(po);
        p.cmp_validate(&t);
        assert_eq!(p.c.validate(&t).0, 0);
    }
}

// ---------------------------------------------------------------------------
// ERRORS row 12 — null pointer. The C dereferences `t` unconditionally, so both
// implementations must die the same way. Run each in a child process.
// ---------------------------------------------------------------------------

const NULL_CHILD_ENV: &str = "TFLAC_NULL_DEREF_CHILD";

fn null_deref_child(which: &str) -> ! {
    let p = load_pair();
    let imp = if which == "c" { &p.c } else { &p.rs };
    // SAFETY: intentionally reproducing the C's UB to observe the fault.
    let rc = unsafe { imp.validate_raw(std::ptr::null_mut()) };
    // If we somehow survive, report the return code via exit status.
    eprintln!("{} survived null deref with rc={rc}", imp.name);
    std::process::exit(if rc == -1 { 70 } else { 71 });
}

#[test]
#[ignore = "child process helper for err_12"]
fn null_child_c() {
    if std::env::var(NULL_CHILD_ENV).is_ok() {
        null_deref_child("c");
    }
}

#[test]
#[ignore = "child process helper for err_12"]
fn null_child_rust() {
    if std::env::var(NULL_CHILD_ENV).is_ok() {
        null_deref_child("rust");
    }
}

#[test]
fn err_12_null_pointer_same_signal() {
    use std::os::unix::process::ExitStatusExt;

    let exe = std::env::current_exe().expect("current_exe");
    let run = |test_name: &str| -> (Option<i32>, Option<i32>) {
        let out = std::process::Command::new(&exe)
            .args([test_name, "--exact", "--ignored", "--test-threads=1"])
            .env(NULL_CHILD_ENV, "1")
            .env("RUST_BACKTRACE", "0")
            .output()
            .expect("spawn child");
        (out.status.code(), out.status.signal())
    };

    let (code_c, sig_c) = run("null_child_c");
    let (code_rs, sig_rs) = run("null_child_rust");

    assert_eq!(
        (code_c, sig_c),
        (code_rs, sig_rs),
        "null-pointer behaviour differs: C exit={code_c:?} signal={sig_c:?}, \
         Rust exit={code_rs:?} signal={sig_rs:?}"
    );
    // Both must actually fault (SIGSEGV == 11), not silently succeed.
    assert_eq!(
        sig_c,
        Some(11),
        "expected SIGSEGV from the C null deref, got exit={code_c:?} signal={sig_c:?}"
    );
}

// ---------------------------------------------------------------------------
// Extra generic FFI boundary sweep: out-of-range enum values for channel_mode
// across every field combination that reaches the enum comparison.
// ---------------------------------------------------------------------------
#[test]
fn enum_channel_mode_exhaustive() {
    let p = load_pair();
    for mode in 0..=255u8 {
        for ch in 1..=8u32 {
            for bd in [1u32, 16, 17, 31, 32] {
                for mrv in [0u8, 1, 30, 31, 255] {
                    for (minpo, maxpo) in [(0u8, 0u8), (0, 15), (15, 15), (1, 0), (0, 16)] {
                        let mut t = Tflac::valid();
                        t.set_channel_mode(mode)
                            .set_channels(ch)
                            .set_bitdepth(bd)
                            .set_max_rice_value(mrv)
                            .set_min_partition_order(minpo)
                            .set_max_partition_order(maxpo);
                        p.cmp_validate(&t);
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Extra: every u8 field driven across its full 0..=255 domain.
// ---------------------------------------------------------------------------
#[test]
fn u8_fields_full_domain() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 0xFF);
    for v in 0..=255u8 {
        for field in 0..5u8 {
            let mut t = base(&mut rng);
            match field {
                0 => t.set_channel_mode(v),
                1 => t.set_max_rice_value(v),
                2 => t.set_min_partition_order(v),
                3 => t.set_max_partition_order(v),
                _ => t.set_partition_order(v),
            };
            p.cmp_validate(&t);
        }
        // and all of them at once
        let mut t = base(&mut rng);
        t.set_channel_mode(v)
            .set_max_rice_value(v)
            .set_min_partition_order(v)
            .set_max_partition_order(v)
            .set_partition_order(v);
        p.cmp_validate(&t);
    }
}
