//! Phase B — valid-path differential tests, one test per `CONFIGS.md` row.
//!
//! Every row calls BOTH `.so`s through their exported `driver` symbol and
//! requires byte-identical stdout. Ranged rows use `SAMPLES` seeded-random draws
//! plus both endpoints, so a row is not certified by a single hand-picked value.

mod common;

use common::*;
use std::ffi::{c_char, c_int};

// ---------------------------------------------------------------------------
// Rows 1-19: ctype equivalence classes of the `char` domain, global locale "C".
// ---------------------------------------------------------------------------

#[test]
fn cfg_01_nul() {
    reset_global_locale();
    diff_char(0x00, "row 1: NUL");
}

#[test]
fn cfg_02_control_01_to_08() {
    reset_global_locale();
    diff_random_in_range(0x01, 0x08, 2, "row 2: pure control 0x01..=0x08");
}

#[test]
fn cfg_03_tab() {
    reset_global_locale();
    diff_char(0x09, "row 3: TAB (cntrl|space|blank)");
}

#[test]
fn cfg_04_whitespace_controls_0a_to_0d() {
    reset_global_locale();
    diff_random_in_range(0x0A, 0x0D, 4, "row 4: \\n \\v \\f \\r (cntrl|space)");
}

#[test]
fn cfg_05_control_0e_to_1f() {
    reset_global_locale();
    diff_random_in_range(0x0E, 0x1F, 5, "row 5: pure control 0x0E..=0x1F");
}

#[test]
fn cfg_06_space() {
    reset_global_locale();
    diff_char(0x20, "row 6: SPACE (blank|space|print, not graph)");
}

#[test]
fn cfg_07_punct_21_to_2f() {
    reset_global_locale();
    diff_random_in_range(0x21, 0x2F, 7, "row 7: punctuation 0x21..=0x2F");
}

#[test]
fn cfg_08_digits() {
    reset_global_locale();
    diff_random_in_range(b'0', b'9', 8, "row 8: digits (5 mask bits at once)");
}

#[test]
fn cfg_09_punct_3a_to_40() {
    reset_global_locale();
    diff_random_in_range(0x3A, 0x40, 9, "row 9: punctuation 0x3A..=0x40");
}

#[test]
fn cfg_10_upper_hex_a_to_f() {
    reset_global_locale();
    diff_random_in_range(b'A', b'F', 10, "row 10: A-F (upper|xdigit)");
}

#[test]
fn cfg_11_upper_g_to_z() {
    reset_global_locale();
    diff_random_in_range(b'G', b'Z', 11, "row 11: G-Z (upper, not xdigit)");
}

#[test]
fn cfg_12_punct_5b_to_60() {
    reset_global_locale();
    diff_random_in_range(0x5B, 0x60, 12, "row 12: punctuation 0x5B..=0x60");
}

#[test]
fn cfg_13_lower_hex_a_to_f() {
    reset_global_locale();
    diff_random_in_range(b'a', b'f', 13, "row 13: a-f (lower|xdigit)");
}

#[test]
fn cfg_14_lower_g_to_z() {
    reset_global_locale();
    diff_random_in_range(b'g', b'z', 14, "row 14: g-z (lower, not xdigit)");
}

#[test]
fn cfg_15_punct_7b_to_7e() {
    reset_global_locale();
    diff_random_in_range(0x7B, 0x7E, 15, "row 15: punctuation 0x7B..=0x7E");
}

#[test]
fn cfg_16_del() {
    reset_global_locale();
    diff_char(0x7F, "row 16: DEL (cntrl, max positive char)");
}

#[test]
fn cfg_17_most_negative_char() {
    reset_global_locale();
    diff_char(0x80u8 as c_char, "row 17: 0x80 == -128, lowest table index");
}

#[test]
fn cfg_18_negative_range_81_to_fe() {
    reset_global_locale();
    diff_random_in_range(0x81, 0xFE, 18, "row 18: negative table indices");
}

#[test]
fn cfg_19_eof_slot_ff() {
    reset_global_locale();
    diff_char(0xFFu8 as c_char, "row 19: 0xFF == -1, the EOF table slot");
}

// ---------------------------------------------------------------------------
// Row 22: seeded-random over the whole domain, many more draws.
// ---------------------------------------------------------------------------

#[test]
fn cfg_22_random_full_domain() {
    reset_global_locale();
    let mut rng = Rng::new(SEED ^ 22);
    for i in 0..512 {
        let v = rng.next_u8();
        diff_char(v as c_char, &format!("row 22: random byte [draw {i}]"));
    }
}

// ---------------------------------------------------------------------------
// Rows 23-26: the `int`-typed view of `driver` (ABI-level call shape).
// ---------------------------------------------------------------------------

#[test]
fn cfg_23_int_clean_low_byte() {
    reset_global_locale();
    for v in [0x0000_0041, 0x0000_0000, 0x0000_007F, 0x0000_00FF] {
        diff_int(v, "row 23: int with zero high bits");
    }
}

#[test]
fn cfg_24_int_garbage_high_bits() {
    reset_global_locale();
    // Hand-picked adversarial patterns first.
    for v in [
        0xDEAD_BE41u32 as c_int,
        0x7FFF_FF00u32 as c_int,
        0x0000_0100u32 as c_int,
        0xFFFF_FF41u32 as c_int,
        0x1234_5680u32 as c_int,
    ] {
        diff_int(v, "row 24: int with garbage high bits");
    }
    // Then seeded-random high bits over a random low byte.
    let mut rng = Rng::new(SEED ^ 24);
    for i in 0..128 {
        let v = rng.next_u32() as c_int;
        diff_int(v, &format!("row 24: random 32-bit arg [draw {i}]"));
    }
}

#[test]
fn cfg_25_int_128_and_256() {
    reset_global_locale();
    // Neither is representable in a signed char; both must narrow the same way.
    diff_int(128, "row 25: int 128 (narrows to -128)");
    diff_int(256, "row 25: int 256 (narrows to 0)");
    diff_int(255, "row 25: int 255 (narrows to -1)");
    diff_int(257, "row 25: int 257 (narrows to 1)");
}

#[test]
fn cfg_26_int_extremes() {
    reset_global_locale();
    for v in [c_int::MIN, c_int::MAX, -1, 0, 1, -256, -257, -65280] {
        diff_int(v, "row 26: int extreme");
    }
    let mut rng = Rng::new(SEED ^ 26);
    for i in 0..128 {
        // Full i32 range including negatives.
        let v = rng.next_u32() as i32;
        diff_int(v, &format!("row 26: random i32 [draw {i}]"));
    }
}

// ---------------------------------------------------------------------------
// Rows 32-36: multiplicity, ordering, stdout buffering, threading.
// ---------------------------------------------------------------------------

#[test]
fn cfg_32_repeated_calls_are_idempotent() {
    reset_global_locale();
    let cd = c_driver();
    let rd = rust_driver();
    for &c in &[0i8, b'A' as i8, b'z' as i8, 0x7F, -1, -128] {
        let c_first = capture(|| unsafe { cd(c) });
        let r_first = capture(|| unsafe { rd(c) });
        assert_eq!(c_first, r_first, "row 32: first call, c={c}");
        for n in 1..8 {
            let c_n = capture(|| unsafe { cd(c) });
            let r_n = capture(|| unsafe { rd(c) });
            assert_eq!(c_n, c_first, "row 32: C call {n} differs from call 0, c={c}");
            assert_eq!(
                r_n, r_first,
                "row 32: Rust call {n} differs from call 0, c={c}"
            );
            assert_eq!(c_n, r_n, "row 32: call {n} diverges, c={c}");
        }
    }
}

#[test]
fn cfg_33_interleaved_calls_share_one_stdout_buffer() {
    reset_global_locale();
    let cd = c_driver();
    let rd = rust_driver();
    for &c in &[b'Q' as i8, 0x09, -1, -128, 0] {
        // Baselines, captured separately.
        let c_alone = capture(|| unsafe { cd(c) });
        let r_alone = capture(|| unsafe { rd(c) });
        assert_eq!(c_alone, r_alone, "row 33: baseline diverges, c={c}");

        // C then Rust inside ONE capture, with no flush in between. If the Rust
        // wrapper wrote through its own buffered writer instead of libc's
        // `stdout`, the two halves would not appear in call order.
        let mut want = c_alone.clone();
        want.extend_from_slice(&r_alone);
        let got = capture(|| unsafe {
            cd(c);
            rd(c);
        });
        assert_eq!(
            got,
            want,
            "row 33: C-then-Rust interleaving, c={c}\n  got : {}\n  want: {}",
            render(&got),
            render(&want)
        );

        // And the other order.
        let mut want = r_alone.clone();
        want.extend_from_slice(&c_alone);
        let got = capture(|| unsafe {
            rd(c);
            cd(c);
        });
        assert_eq!(
            got,
            want,
            "row 33: Rust-then-C interleaving, c={c}\n  got : {}\n  want: {}",
            render(&got),
            render(&want)
        );
    }
}

#[test]
fn cfg_34_callee_never_flushes_stdout_itself() {
    reset_global_locale();
    set_stdout_buffering(IOFBF);
    let cd = c_driver();
    let rd = rust_driver();
    for &c in &[b'A' as i8, 0, -1, -128, 0x7F] {
        let (c_before, c_after) = capture_two_stage(|| unsafe { cd(c) });
        let (r_before, r_after) = capture_two_stage(|| unsafe { rd(c) });
        // The point of the row: whatever the C leaves unflushed, the Rust must
        // leave unflushed too (identically), and the post-flush bytes must match.
        assert_eq!(
            c_before,
            r_before,
            "row 34: pre-flush bytes differ, c={c}\n  C   : {}\n  Rust: {}",
            render(&c_before),
            render(&r_before)
        );
        assert_eq!(c_after, r_after, "row 34: post-flush bytes differ, c={c}");
        assert!(!c_after.is_empty(), "row 34: nothing captured at all");
    }
    set_stdout_buffering(IOFBF);
}

#[test]
fn cfg_35_stdout_buffering_modes() {
    reset_global_locale();
    for (name, mode) in [
        ("_IOFBF (fully buffered)", IOFBF),
        ("_IOLBF (line buffered)", IOLBF),
        ("_IONBF (unbuffered)", IONBF),
    ] {
        set_stdout_buffering(mode);
        for &c in &[b'A' as i8, b'0' as i8, 0x09, 0x20, 0, -1, -128, 0x7F] {
            diff_char(c, &format!("row 35: stdout {name}, c={c}"));
        }
    }
    set_stdout_buffering(IOFBF);
}

#[test]
fn cfg_36_called_from_a_non_main_thread() {
    reset_global_locale();
    // `__ctype_b_loc()` is a per-thread accessor, so run a full sweep off the
    // main thread as well.
    let h = std::thread::spawn(|| {
        diff_all_chars("row 36: non-main thread, full sweep");
        let mut rng = Rng::new(SEED ^ 36);
        for i in 0..64 {
            let v = rng.next_u32() as c_int;
            diff_int(v, &format!("row 36: non-main thread, random i32 [{i}]"));
        }
    });
    h.join().expect("worker thread panicked");
}

/// Several threads hammering `driver` at once. Captures are serialised by the
/// harness lock, so this checks that neither implementation keeps mutable global
/// state that the other does not.
#[test]
fn cfg_36b_multiple_threads() {
    reset_global_locale();
    let handles: Vec<_> = (0..4u64)
        .map(|t| {
            std::thread::spawn(move || {
                let mut rng = Rng::new(SEED ^ 0x360 ^ t);
                for i in 0..48 {
                    let v = rng.next_u8();
                    diff_char(v as c_char, &format!("row 36b: thread {t} draw {i}"));
                }
            })
        })
        .collect();
    for h in handles {
        h.join().expect("worker thread panicked");
    }
}
