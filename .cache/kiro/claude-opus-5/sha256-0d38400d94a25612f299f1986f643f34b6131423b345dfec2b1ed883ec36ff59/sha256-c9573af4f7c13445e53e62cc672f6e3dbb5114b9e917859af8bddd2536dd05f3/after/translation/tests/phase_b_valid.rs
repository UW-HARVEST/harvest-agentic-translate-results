//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md`. Every row drives BOTH `.so`s through their
//! exported `slice` symbol with many seeded-random inputs and compares the
//! return code plus the exact stdout bytes.

mod common;

use common::*;
use std::ffi::c_int;

const ITERS: usize = 64;
const ITERS_HEAVY: usize = 256;

/// row 1 — no start, no stop, empty string
#[test]
fn cfg01_null_null_empty() {
    assert_same_ret("cfg01", &Call::new(b"", None, None), 0);
}

/// row 2 — no start, no stop, len == 1 (sweep the whole byte range)
#[test]
fn cfg02_null_null_len1() {
    for b in 1u16..=255 {
        let s = [b as u8];
        assert_same_ret("cfg02", &Call::new(&s, None, None), 0);
    }
}

/// row 3 — no start, no stop, random printable ASCII, len 2..64
#[test]
fn cfg03_null_null_ascii() {
    let mut rng = Rng::new(SEED ^ 3);
    for _ in 0..ITERS_HEAVY {
        let len = rng.range_incl(2, 64) as usize;
        let s = Alpha::PrintableAscii.make(&mut rng, len);
        assert_same_ret("cfg03", &Call::new(&s, None, None), 0);
    }
}

/// row 4 — no start, no stop, long string 256..4096
#[test]
fn cfg04_null_null_long() {
    let mut rng = Rng::new(SEED ^ 4);
    for _ in 0..ITERS {
        let len = rng.range_incl(256, 4096) as usize;
        let s = Alpha::AnyNonZero.make(&mut rng, len);
        assert_same_ret("cfg04", &Call::new(&s, None, None), 0);
    }
}

/// row 5 — no start, no stop, non-UTF-8 high bytes
#[test]
fn cfg05_null_null_high_bytes() {
    let mut rng = Rng::new(SEED ^ 5);
    for _ in 0..ITERS_HEAVY {
        let len = rng.range_incl(1, 96) as usize;
        let s = Alpha::HighBytes.make(&mut rng, len);
        assert_same_ret("cfg05", &Call::new(&s, None, None), 0);
    }
}

/// row 6 — format-specifier bytes carried as data
#[test]
fn cfg06_null_null_format_specifiers() {
    let mut rng = Rng::new(SEED ^ 6);
    for fixed in [
        &b"%s"[..],
        b"%n",
        b"%%",
        b"%.*s",
        b"%s%s%s%s%s%s%s%s",
        b"%n%n%n%n",
        b"100%% sure",
        b"%1$s",
    ] {
        assert_same_ret("cfg06", &Call::new(fixed, None, None), 0);
    }
    for _ in 0..ITERS_HEAVY {
        let len = rng.range_incl(1, 80) as usize;
        let s = Alpha::FormatSpecifiers.make(&mut rng, len);
        assert_same_ret("cfg06", &Call::new(&s, None, None), 0);
    }
}

/// row 7 — embedded newlines and control bytes
#[test]
fn cfg07_null_null_control_bytes() {
    let mut rng = Rng::new(SEED ^ 7);
    for fixed in [&b"\n"[..], b"a\nb", b"\n\n\n", b"\r\n", b"\ttab\t", b"\x1b[31m"] {
        assert_same_ret("cfg07", &Call::new(fixed, None, None), 0);
    }
    for _ in 0..ITERS_HEAVY {
        let len = rng.range_incl(1, 64) as usize;
        let s = Alpha::Control.make(&mut rng, len);
        assert_same_ret("cfg07", &Call::new(&s, None, None), 0);
    }
}

/// row 8 — explicit start 0, no stop
#[test]
fn cfg08_start0_null() {
    let mut rng = Rng::new(SEED ^ 8);
    for _ in 0..ITERS_HEAVY {
        let len = rng.range_incl(1, 64) as usize;
        let s = Alpha::PrintableAscii.make(&mut rng, len);
        assert_same_ret("cfg08", &Call::new(&s, Some(0), None), 0);
    }
}

/// row 9 — interior start, no stop
#[test]
fn cfg09_interior_start_null_stop() {
    let mut rng = Rng::new(SEED ^ 9);
    for _ in 0..ITERS_HEAVY {
        let len = rng.range_incl(2, 128) as usize;
        let s = Alpha::AnyNonZero.make(&mut rng, len);
        let st = rng.range_incl(1, (len - 1) as u64) as c_int;
        assert_same_ret("cfg09", &Call::new(&s, Some(st), None), 0);
    }
}

/// row 10 — start == len-1, no stop (last character)
#[test]
fn cfg10_start_last_null_stop() {
    let mut rng = Rng::new(SEED ^ 10);
    for _ in 0..ITERS_HEAVY {
        let len = rng.range_incl(1, 128) as usize;
        let s = Alpha::AnyNonZero.make(&mut rng, len);
        assert_same_ret("cfg10", &Call::new(&s, Some(len as c_int - 1), None), 0);
    }
}

/// row 11 — start == len (accepted boundary), no stop ⇒ zero-width output
#[test]
fn cfg11_start_eq_len_null_stop() {
    let mut rng = Rng::new(SEED ^ 11);
    for _ in 0..ITERS_HEAVY {
        let len = rng.range_incl(0, 128) as usize;
        let s = Alpha::AnyNonZero.make(&mut rng, len);
        assert_same_ret("cfg11", &Call::new(&s, Some(len as c_int), None), 0);
    }
}

/// row 12 — no start, random valid stop
#[test]
fn cfg12_null_start_random_stop() {
    let mut rng = Rng::new(SEED ^ 12);
    for _ in 0..ITERS_HEAVY {
        let len = rng.range_incl(1, 128) as usize;
        let s = Alpha::AnyNonZero.make(&mut rng, len);
        let e = rng.range_incl(1, len as u64) as c_int;
        assert_same_ret("cfg12", &Call::new(&s, None, Some(e)), 0);
    }
}

/// row 13 — no start, stop == len
#[test]
fn cfg13_null_start_stop_eq_len() {
    let mut rng = Rng::new(SEED ^ 13);
    for _ in 0..ITERS_HEAVY {
        let len = rng.range_incl(1, 128) as usize;
        let s = Alpha::AnyNonZero.make(&mut rng, len);
        assert_same_ret("cfg13", &Call::new(&s, None, Some(len as c_int)), 0);
    }
}

/// row 14 — no start, stop == 1 (narrowest slice from the front)
#[test]
fn cfg14_null_start_stop1() {
    let mut rng = Rng::new(SEED ^ 14);
    for _ in 0..ITERS_HEAVY {
        let len = rng.range_incl(1, 128) as usize;
        let s = Alpha::AnyNonZero.make(&mut rng, len);
        assert_same_ret("cfg14", &Call::new(&s, None, Some(1)), 0);
    }
}

/// row 15 — both pointers, random valid 0 <= s < e <= len
#[test]
fn cfg15_both_random_valid() {
    let mut rng = Rng::new(SEED ^ 15);
    for _ in 0..ITERS_HEAVY {
        let len = rng.range_incl(1, 64) as usize;
        let s = Alpha::PrintableAscii.make(&mut rng, len);
        let st = rng.below(len as u64) as c_int;
        let e = rng.range_incl(st as u64 + 1, len as u64) as c_int;
        assert_same_ret("cfg15", &Call::new(&s, Some(st), Some(e)), 0);
    }
}

/// row 16 — both pointers, minimum width e == s + 1
#[test]
fn cfg16_both_min_width() {
    let mut rng = Rng::new(SEED ^ 16);
    for _ in 0..ITERS_HEAVY {
        let len = rng.range_incl(1, 128) as usize;
        let s = Alpha::AnyNonZero.make(&mut rng, len);
        let st = rng.below(len as u64) as c_int;
        assert_same_ret("cfg16", &Call::new(&s, Some(st), Some(st + 1)), 0);
    }
}

/// row 17 — both pointers, whole string
#[test]
fn cfg17_both_whole_string() {
    let mut rng = Rng::new(SEED ^ 17);
    for _ in 0..ITERS_HEAVY {
        let len = rng.range_incl(1, 256) as usize;
        let s = Alpha::AnyNonZero.make(&mut rng, len);
        assert_same_ret("cfg17", &Call::new(&s, Some(0), Some(len as c_int)), 0);
    }
}

/// row 18 — both pointers, last character
#[test]
fn cfg18_both_last_char() {
    let mut rng = Rng::new(SEED ^ 18);
    for _ in 0..ITERS_HEAVY {
        let len = rng.range_incl(1, 256) as usize;
        let s = Alpha::AnyNonZero.make(&mut rng, len);
        assert_same_ret(
            "cfg18",
            &Call::new(&s, Some(len as c_int - 1), Some(len as c_int)),
            0,
        );
    }
}

/// row 19 — degenerate single-char string with both bounds explicit
#[test]
fn cfg19_both_len1() {
    for b in 1u16..=255 {
        let s = [b as u8];
        assert_same_ret("cfg19", &Call::new(&s, Some(0), Some(1)), 0);
    }
}

/// row 20 — both pointers, non-UTF-8 / control content
#[test]
fn cfg20_both_binary_content() {
    let mut rng = Rng::new(SEED ^ 20);
    for alpha in [Alpha::HighBytes, Alpha::Control, Alpha::AnyNonZero] {
        for _ in 0..ITERS_HEAVY {
            let len = rng.range_incl(1, 96) as usize;
            let s = alpha.make(&mut rng, len);
            let st = rng.below(len as u64) as c_int;
            let e = rng.range_incl(st as u64 + 1, len as u64) as c_int;
            assert_same_ret("cfg20", &Call::new(&s, Some(st), Some(e)), 0);
        }
    }
}

/// row 21 — both pointers, long strings
#[test]
fn cfg21_both_long_strings() {
    let mut rng = Rng::new(SEED ^ 21);
    for _ in 0..ITERS {
        let len = rng.range_incl(256, 4096) as usize;
        let s = Alpha::AnyNonZero.make(&mut rng, len);
        let st = rng.below(len as u64) as c_int;
        let e = rng.range_incl(st as u64 + 1, len as u64) as c_int;
        assert_same_ret("cfg21", &Call::new(&s, Some(st), Some(e)), 0);
    }
}

/// row 22 — empty string with explicit start 0 (0 == len boundary)
#[test]
fn cfg22_empty_explicit_start0() {
    assert_same_ret("cfg22", &Call::new(b"", Some(0), None), 0);
}

/// row 23 — exhaustive small-case oracle: every pointer mode × every valid
/// (start, stop) pair for len 0..=8.
#[test]
fn cfg23_exhaustive_small() {
    let mut rng = Rng::new(SEED ^ 23);
    for len in 0usize..=8 {
        for _rep in 0..4 {
            let s = Alpha::AnyNonZero.make(&mut rng, len);
            let l = len as c_int;

            // NULL / NULL
            assert_same_ret("cfg23", &Call::new(&s, None, None), 0);

            // start only: 0..=len are all valid
            for st in 0..=l {
                assert_same_ret("cfg23", &Call::new(&s, Some(st), None), 0);
            }

            // stop only: start defaults to 0, so 1..=len are valid
            for e in 1..=l {
                assert_same_ret("cfg23", &Call::new(&s, None, Some(e)), 0);
            }

            // both: valid iff 0 <= st < e <= len
            for st in 0..=l {
                for e in (st + 1)..=l {
                    assert_same_ret("cfg23", &Call::new(&s, Some(st), Some(e)), 0);
                }
            }
        }
    }
}

/// row 24 — a long mixed sequence of calls under one stdout capture, so libc
/// buffering, message ordering and cross-call state are compared too.
#[test]
fn cfg24_call_sequence_ordering() {
    let mut rng = Rng::new(SEED ^ 24);
    let mut bufs: Vec<Vec<u8>> = Vec::new();
    for _ in 0..200 {
        let len = rng.range_incl(0, 40) as usize;
        bufs.push(Alpha::AnyNonZero.make(&mut rng, len));
    }
    let mut calls: Vec<Call<'_>> = Vec::new();
    for b in &bufs {
        let l = b.len() as i64;
        // Deliberately mix successes and every failure mode so the interleaved
        // puts()/printf() byte stream is compared as a whole.
        let (s, e) = match rng.below(8) {
            0 => (None, None),
            1 => (Some(0), None),
            2 => (Some(l as c_int), None),
            3 => (Some((l + 1) as c_int), None),
            4 => (None, Some(l as c_int)),
            5 => (None, Some(-1)),
            6 => (Some(l as c_int), Some(l as c_int)),
            _ => {
                if l >= 1 {
                    let st = rng.below(l as u64) as c_int;
                    let en = rng.range_incl(st as u64 + 1, l as u64) as c_int;
                    (Some(st), Some(en))
                } else {
                    (Some(0), None)
                }
            }
        };
        calls.push(Call::new(b, s, e));
    }
    assert_same_sequence("cfg24", &calls);
}

/// rows 25 & 26 — the callee must not write through any of its three pointers.
/// (`assert_same` checks this on every call; this test makes the row explicit
/// and covers all pointer modes including the aliased one.)
#[test]
fn cfg25_26_arguments_are_read_only() {
    let mut rng = Rng::new(SEED ^ 25);
    for _ in 0..ITERS_HEAVY {
        let len = rng.range_incl(1, 64) as usize;
        let s = Alpha::AnyNonZero.make(&mut rng, len);
        let l = len as c_int;
        assert_same("cfg25", &Call::new(&s, None, None));
        assert_same("cfg25", &Call::new(&s, Some(0), Some(l)));
        assert_same("cfg25", &Call::new(&s, Some(l), None));
        assert_same("cfg25", &Call::new(&s, None, Some(l)));
        // Out-of-range values must not be "clamped" into the caller's ints.
        assert_same("cfg25", &Call::new(&s, Some(l + 1), None));
        assert_same("cfg25", &Call::new(&s, Some(-7), Some(-9)));
    }
}
