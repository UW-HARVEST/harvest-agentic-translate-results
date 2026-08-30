//! Phase B — valid-path differential tests, one test per `CONFIGS.md` row.
//!
//! Every test drives BOTH shared objects through their exported C ABI and
//! requires byte-identical stdout and identical termination status. Randomised
//! rows use a fixed seed so failures reproduce exactly.
//!
//! Tests are ordered lowest-level entry point first (`printLine`,
//! `printIntLine`), then the mid-level ones (`bad`, `good`), then the composed
//! `driver` wrapper — bugs in the composition are invisible to the per-function
//! rows, so both levels are exercised.

mod common;

use common::{assert_same, assert_same_clean, assert_same_isolated, Op, Rng, BAD_SELF_FRAME_MAX};

const SEED: u64 = 0x0BAD_C0DE_1234_5678;

// ===========================================================================
// printLine — the lowest-level entry point (rows 1..8)
// ===========================================================================

/// Row 1 — non-NULL, every possible single non-NUL byte (0x01..=0xFF).
#[test]
fn cfg_01_print_line_single_bytes() {
    let ops: Vec<Op> = (1u8..=255).map(|b| Op::PrintLine(vec![b])).collect();
    assert_same_clean("cfg_01 printLine single bytes 0x01..=0xFF", &ops);
}

/// Row 2 — non-NULL, empty string. `printf("%s\n", "")` must emit just "\n".
#[test]
fn cfg_02_print_line_empty() {
    let ops = [Op::PrintLine(vec![]), Op::PrintLine(vec![]), Op::print_line("x")];
    assert_same_clean("cfg_02 printLine empty string", &ops);
}

/// Row 3 — random printable ASCII, length 1..=200.
#[test]
fn cfg_03_print_line_random_ascii() {
    let mut rng = Rng::new(SEED ^ 3);
    let ops: Vec<Op> = (0..200)
        .map(|_| {
            let len = rng.usize_range(1, 200);
            Op::PrintLine(rng.ascii(len))
        })
        .collect();
    assert_same_clean("cfg_03 printLine random ASCII", &ops);
}

/// Row 4 — strings full of `printf` conversion specifiers. `line` is an
/// *argument* to `printf("%s\n", line)` in C, so these must be printed
/// literally; a translation that passed `line` as the format string would
/// diverge (or crash) here. `%n` in particular would be a write primitive.
#[test]
fn cfg_04_print_line_format_specifiers() {
    let mut rng = Rng::new(SEED ^ 4);
    let atoms: [&str; 12] = [
        "%s", "%d", "%n", "%%", "%p", "%x", "%99999d", "%.*s", "%hhn", "%lu", "%c", "%zu",
    ];
    let mut ops = vec![
        Op::print_line("%s"),
        Op::print_line("%n"),
        Op::print_line("%%"),
        Op::print_line("%s%s%s%s%s%s%s%s"),
        Op::print_line("%n%n%n%n"),
        Op::print_line("100%"),
        Op::print_line("%"),
    ];
    for _ in 0..100 {
        let n = rng.usize_range(1, 10);
        let mut s = String::new();
        for _ in 0..n {
            s.push_str(rng.pick(&atoms));
            if rng.next_u64() % 2 == 0 {
                s.push_str("lit");
            }
        }
        ops.push(Op::print_line(&s));
    }
    assert_same_clean("cfg_04 printLine printf specifiers", &ops);
}

/// Row 5 — embedded `\n`, `\r`, `\t`. The C appends exactly one `\n`
/// regardless of what the payload already contains.
#[test]
fn cfg_05_print_line_embedded_newlines() {
    let mut rng = Rng::new(SEED ^ 5);
    let mut ops = vec![
        Op::print_line("\n"),
        Op::print_line("\n\n\n"),
        Op::print_line("a\nb"),
        Op::print_line("\r\n"),
        Op::print_line("\ttab\t"),
        Op::print_line("trailing\n"),
    ];
    for _ in 0..100 {
        let len = rng.usize_range(1, 40);
        let bytes: Vec<u8> = (0..len)
            .map(|_| *rng.pick(&[b'\n', b'\r', b'\t', b'a', b'Z', b' ', b'%']))
            .collect();
        ops.push(Op::PrintLine(bytes));
    }
    assert_same_clean("cfg_05 printLine embedded newlines", &ops);
}

/// Row 6 — arbitrary non-NUL bytes, i.e. deliberately invalid UTF-8. The C is
/// byte-oriented; a translation that round-tripped through `str` would either
/// panic or mangle these.
#[test]
fn cfg_06_print_line_non_utf8_bytes() {
    let mut rng = Rng::new(SEED ^ 6);
    let mut ops = vec![
        Op::PrintLine(vec![0xFF]),
        Op::PrintLine(vec![0x80, 0x80, 0x80]),
        Op::PrintLine(vec![0xC3]),          // truncated 2-byte sequence
        Op::PrintLine(vec![0xE2, 0x82]),    // truncated 3-byte sequence
        Op::PrintLine(vec![0xF0, 0x9F, 0x92]), // truncated 4-byte sequence
        Op::PrintLine(vec![0xED, 0xA0, 0x80]), // UTF-16 surrogate encoding
        Op::PrintLine((1u8..=255).collect()),  // every non-NUL byte at once
    ];
    for _ in 0..200 {
        let len = rng.usize_range(1, 120);
        ops.push(Op::PrintLine(rng.c_bytes(len)));
    }
    assert_same_clean("cfg_06 printLine arbitrary non-UTF-8 bytes", &ops);
}

/// Row 7 — very long strings, 4 KiB .. 64 KiB, i.e. past any plausible
/// internal buffer and past the stdio buffer, so flushing behaviour matters.
#[test]
fn cfg_07_print_line_very_long() {
    let mut rng = Rng::new(SEED ^ 7);
    let mut ops = Vec::new();
    for _ in 0..8 {
        let len = rng.usize_range(4 * 1024, 64 * 1024);
        ops.push(Op::PrintLine(rng.c_bytes(len)));
    }
    // Exact power-of-two boundaries around the usual 4096-byte stdio buffer.
    for len in [4095usize, 4096, 4097, 8191, 8192, 8193, 65535, 65536] {
        ops.push(Op::PrintLine(rng.ascii(len)));
    }
    assert_same_clean("cfg_07 printLine 4KiB..64KiB", &ops);
}

/// Row 8 — NULL (the guard at driver.c:31). Interleaved with non-NULL calls to
/// prove the NULL case produces *no* output at all rather than a blank line.
#[test]
fn cfg_08_print_line_null_interleaved() {
    let ops = [
        Op::print_line("before"),
        Op::PrintLineNull,
        Op::PrintLineNull,
        Op::print_line("after"),
        Op::PrintLineNull,
    ];
    assert_same_clean("cfg_08 printLine NULL interleaved", &ops);
}

// ===========================================================================
// printIntLine (rows 9..12)
// ===========================================================================

/// Row 9 — zero.
#[test]
fn cfg_09_print_int_line_zero() {
    assert_same_clean("cfg_09 printIntLine(0)", &[Op::PrintIntLine(0)]);
}

/// Row 10 — 500 random positive ints.
#[test]
fn cfg_10_print_int_line_random_positive() {
    let mut rng = Rng::new(SEED ^ 10);
    let ops: Vec<Op> = (0..500)
        .map(|_| Op::PrintIntLine(rng.range(1, i32::MAX)))
        .collect();
    assert_same_clean("cfg_10 printIntLine random positive", &ops);
}

/// Row 11 — 500 random negative ints.
#[test]
fn cfg_11_print_int_line_random_negative() {
    let mut rng = Rng::new(SEED ^ 11);
    let ops: Vec<Op> = (0..500)
        .map(|_| Op::PrintIntLine(rng.range(i32::MIN, -1)))
        .collect();
    assert_same_clean("cfg_11 printIntLine random negative", &ops);
}

/// Row 12 — the exact `int` boundaries, plus a full random sweep of the whole
/// 32-bit domain (catches any sign/width mistake in the `%d` formatting).
#[test]
fn cfg_12_print_int_line_boundaries() {
    let mut rng = Rng::new(SEED ^ 12);
    let mut ops: Vec<Op> = [
        i32::MIN,
        i32::MIN + 1,
        -1_000_000_000,
        -100,
        -10,
        -2,
        -1,
        0,
        1,
        2,
        9,
        10,
        99,
        100,
        1_000_000_000,
        i32::MAX - 1,
        i32::MAX,
    ]
    .iter()
    .map(|&n| Op::PrintIntLine(n))
    .collect();
    for _ in 0..500 {
        ops.push(Op::PrintIntLine(rng.i32_any()));
    }
    assert_same_clean("cfg_12 printIntLine boundaries + full-domain sweep", &ops);
}

// ===========================================================================
// bad — the unguarded sink (rows 13..19)
// ===========================================================================

/// Row 13 — write to the first element.
#[test]
fn cfg_13_bad_first_element() {
    assert_same_clean("cfg_13 bad(0)", &[Op::Bad(0)]);
}

/// Row 14 — every interior in-bounds index, each isolated so the 10-line dump
/// is attributed to exactly one call.
#[test]
fn cfg_14_bad_interior_indices() {
    let ops: Vec<Op> = (1..=8).map(Op::Bad).collect();
    assert_same_isolated("cfg_14 bad(1..=8) interior", &ops);
}

/// Row 15 — write to the last in-bounds element.
#[test]
fn cfg_15_bad_last_element() {
    assert_same_clean("cfg_15 bad(9)", &[Op::Bad(9)]);
}

/// Row 16 — 300 random in-range indices, batched in one process so repeated
/// calls also prove `buffer` is re-zeroed on every entry.
#[test]
fn cfg_16_bad_random_in_range() {
    let mut rng = Rng::new(SEED ^ 16);
    let ops: Vec<Op> = (0..300).map(|_| Op::Bad(rng.range(0, 9))).collect();
    assert_same_clean("cfg_16 bad random 0..=9", &ops);
}

/// Row 17 — the negative branch: 300 random negatives plus `INT_MIN`.
#[test]
fn cfg_17_bad_random_negative() {
    let mut rng = Rng::new(SEED ^ 17);
    let mut ops = vec![Op::Bad(i32::MIN), Op::Bad(i32::MIN + 1), Op::Bad(-1)];
    for _ in 0..300 {
        ops.push(Op::Bad(rng.range(i32::MIN, -1)));
    }
    assert_same_clean("cfg_17 bad random negative", &ops);
}

/// Row 18 — `bad(10)`: one past the end. The write lands in frame padding, so
/// the reference build is benign and the 10 printed values are all zero.
#[test]
fn cfg_18_bad_one_past_end() {
    assert_same_clean("cfg_18 bad(10) one past end", &[Op::Bad(10)]);
    let (c, _r) = common::run_both(&[Op::Bad(10)]);
    assert_eq!(c.lines(), vec![b"0" as &[u8]; 10], "bad(10) dump must be all zeros");
}

/// Row 19 — `bad(11)`: the last overflow index that still lands inside `bad`'s
/// own frame. It aliases the loop counter `i` (`-0x4(%rbp)`), which the `for`
/// statement immediately re-initialises, so the effect is invisible — ten zeros.
///
/// From `bad(12)` on, the write lands on the caller's saved `rbp` / return
/// address; that is not reproducible by any translation and is covered by
/// `tests/phase_b_ub.rs` instead (see `UB.md`).
#[test]
fn cfg_19_bad_overflow_within_own_frame() {
    assert_same_clean("cfg_19 bad(10..=11) overflow inside own frame", &[Op::Bad(10), Op::Bad(11)]);
    assert_same_isolated("cfg_19 bad(10), bad(11) isolated", &[Op::Bad(10), Op::Bad(11)]);
    let (c, _) = common::run_both(&[Op::Bad(11)]);
    assert_eq!(
        c.lines(),
        vec![b"0" as &[u8]; 10],
        "bad(11) must print ten zeros: the write hits `i`, not `buffer`"
    );
}

// ===========================================================================
// good — goodG2B() then goodB2G(data) (rows 20..26)
// ===========================================================================

/// Row 20 — `good(0)`.
#[test]
fn cfg_20_good_zero() {
    assert_same_clean("cfg_20 good(0)", &[Op::Good(0)]);
}

/// Row 21 — `good(7)`: `goodB2G`'s index coincides with `goodG2B`'s hard-coded
/// 7, so both halves emit the identical dump. A translation that shared one
/// static buffer between the two helpers would still pass this row but fail
/// row 20/22 — hence all three are present.
#[test]
fn cfg_21_good_seven() {
    assert_same_clean("cfg_21 good(7)", &[Op::Good(7)]);
    let (c, _) = common::run_both(&[Op::Good(7)]);
    assert_eq!(c.lines().len(), 20, "good() must emit two 10-line dumps");
}

/// Row 22 — `good(9)`, the last in-bounds index.
#[test]
fn cfg_22_good_nine() {
    assert_same_clean("cfg_22 good(9)", &[Op::Good(9)]);
}

/// Row 23 — 300 random in-range indices. Each accepted call must emit exactly
/// 20 lines: `goodG2B`'s dump (always `data = 7`) followed by `goodB2G`'s.
#[test]
fn cfg_23_good_random_in_range() {
    let mut rng = Rng::new(SEED ^ 23);
    let ops: Vec<Op> = (0..300).map(|_| Op::Good(rng.range(0, 9))).collect();
    assert_same_clean("cfg_23 good random 0..=9", &ops);
}

/// Row 24 — `good(10)`: rejected by the upper guard `data < 10`.
#[test]
fn cfg_24_good_at_upper_bound() {
    assert_same_clean("cfg_24 good(10)", &[Op::Good(10)]);
}

/// Row 25 — 300 random indices `>= 10`, including `INT_MAX`. `goodB2G` is fully
/// guarded, so none of these may write out of bounds or crash.
#[test]
fn cfg_25_good_random_above_range() {
    let mut rng = Rng::new(SEED ^ 25);
    let mut ops = vec![Op::Good(10), Op::Good(11), Op::Good(i32::MAX)];
    for _ in 0..300 {
        ops.push(Op::Good(rng.range(10, i32::MAX)));
    }
    assert_same_clean("cfg_25 good random >= 10", &ops);
}

/// Row 26 — 300 random negative indices, including `INT_MIN`.
#[test]
fn cfg_26_good_random_negative() {
    let mut rng = Rng::new(SEED ^ 26);
    let mut ops = vec![Op::Good(-1), Op::Good(i32::MIN)];
    for _ in 0..300 {
        ops.push(Op::Good(rng.range(i32::MIN, -1)));
    }
    assert_same_clean("cfg_26 good random negative", &ops);
}

// ===========================================================================
// driver — the composed pipeline (rows 27..33)
// ===========================================================================

/// Row 27 — both arguments in range: the complete 10x10 grid plus 200 random
/// pairs. Exercises the whole 6-print pipeline end to end.
#[test]
fn cfg_27_driver_both_in_range() {
    let mut ops: Vec<Op> = Vec::new();
    for g in 0..10 {
        for b in 0..10 {
            ops.push(Op::Driver(g, b));
        }
    }
    let mut rng = Rng::new(SEED ^ 27);
    for _ in 0..200 {
        ops.push(Op::Driver(rng.range(0, 9), rng.range(0, 9)));
    }
    assert_same_clean("cfg_27 driver both in range (grid + random)", &ops);
}

/// Row 28 — `badData` negative: the bad half takes its error branch while the
/// good half still emits both dumps.
#[test]
fn cfg_28_driver_bad_negative() {
    let mut rng = Rng::new(SEED ^ 28);
    let ops: Vec<Op> = (0..100)
        .map(|_| Op::Driver(rng.range(0, 9), rng.range(i32::MIN, -1)))
        .collect();
    assert_same_clean("cfg_28 driver badData negative", &ops);
}

/// Row 29 — `badData` past the end, restricted to the two overflow slots that
/// stay inside `bad`'s own frame (10, 11). The overflow here happens *inside a
/// composed call chain* (`driver` -> `bad`), which is where a pipeline-level bug
/// would show up and where a per-function test cannot look.
#[test]
fn cfg_29_driver_bad_past_end() {
    let mut rng = Rng::new(SEED ^ 29);
    let mut ops = vec![Op::Driver(0, 10), Op::Driver(9, 11), Op::Driver(7, 10)];
    for _ in 0..100 {
        ops.push(Op::Driver(rng.range(0, 9), rng.range(10, BAD_SELF_FRAME_MAX)));
    }
    for op in &ops {
        assert!(matches!(op, Op::Driver(_, b) if common::bad_index_is_comparable(*b)));
    }
    assert_same_isolated("cfg_29 driver badData past end", &ops);
}

/// Row 30 — `goodData` negative, `badData` in range.
#[test]
fn cfg_30_driver_good_negative() {
    let mut rng = Rng::new(SEED ^ 30);
    let ops: Vec<Op> = (0..100)
        .map(|_| Op::Driver(rng.range(i32::MIN, -1), rng.range(0, 9)))
        .collect();
    assert_same_clean("cfg_30 driver goodData negative", &ops);
}

/// Row 31 — `goodData >= 10`, `badData` in range.
#[test]
fn cfg_31_driver_good_above_range() {
    let mut rng = Rng::new(SEED ^ 31);
    let ops: Vec<Op> = (0..100)
        .map(|_| Op::Driver(rng.range(10, i32::MAX), rng.range(0, 9)))
        .collect();
    assert_same_clean("cfg_31 driver goodData >= 10", &ops);
}

/// Row 32 — both arguments out of range, covering all four sign/magnitude
/// quadrants of (`goodData`, `badData`).
#[test]
fn cfg_32_driver_both_out_of_range() {
    let mut rng = Rng::new(SEED ^ 32);
    let mut ops = Vec::new();
    for _ in 0..100 {
        // goodData rejected either way; badData restricted to its comparable
        // negative branch (positive out-of-range badData is row 29).
        let g = if rng.next_u64() % 2 == 0 {
            rng.range(i32::MIN, -1)
        } else {
            rng.range(10, i32::MAX)
        };
        ops.push(Op::Driver(g, rng.range(i32::MIN, -1)));
    }
    for _ in 0..100 {
        let g = if rng.next_u64() % 2 == 0 {
            rng.range(i32::MIN, -1)
        } else {
            rng.range(10, i32::MAX)
        };
        ops.push(Op::Driver(g, rng.range(10, BAD_SELF_FRAME_MAX)));
    }
    assert_same_isolated("cfg_32 driver both out of range", &ops);
}

/// Row 33 — the boundary grid `{-1,0,9,10} x {-1,0,9,10}`.
#[test]
fn cfg_33_driver_boundary_grid() {
    let vals = [-1i32, 0, 9, 10];
    let mut ops = Vec::new();
    for &g in &vals {
        for &b in &vals {
            ops.push(Op::Driver(g, b));
        }
    }
    assert_same_clean("cfg_33 driver boundary grid", &ops);
}

// ===========================================================================
// Row 34 — statelessness under interleaving
// ===========================================================================

/// Row 34 — 400 randomly interleaved calls across ALL five exported entry
/// points in a single process. The C library keeps no global state, so the
/// output must be the concatenation of the individual results; any hidden
/// static buffer or leaked state in the Rust translation would surface here and
/// nowhere else.
#[test]
fn cfg_34_interleaved_all_entry_points() {
    let mut rng = Rng::new(SEED ^ 34);
    let mut ops = Vec::with_capacity(400);
    for _ in 0..400 {
        let op = match rng.next_u64() % 6 {
            0 => {
                let len = rng.usize_range(0, 30);
                Op::PrintLine(rng.c_bytes(len))
            }
            1 => Op::PrintLineNull,
            2 => Op::PrintIntLine(rng.i32_any()),
            3 => {
                // stay in the comparable domain
                let d = rng.range(-40, 40);
                Op::Bad(if common::bad_index_is_comparable(d) { d } else { 0 })
            }
            4 => Op::Good(rng.range(-20, 20)),
            _ => {
                let b = rng.range(-20, 11);
                Op::Driver(
                    rng.range(-20, 20),
                    if common::bad_index_is_comparable(b) { b } else { 0 },
                )
            }
        };
        ops.push(op);
    }
    assert_same_clean("cfg_34 interleaved all entry points", &ops);
}

/// Extra: the same interleaved stream replayed op-by-op in fresh processes must
/// concatenate to exactly the batched result — for both libraries. This pins
/// down that neither implementation carries state across calls.
#[test]
fn cfg_34b_batched_equals_isolated() {
    let mut rng = Rng::new(SEED ^ 0x34B);
    let ops: Vec<Op> = (0..40)
        .map(|_| match rng.next_u64() % 4 {
            0 => {
                let len = rng.usize_range(0, 12);
                Op::PrintLine(rng.ascii(len))
            }
            1 => Op::PrintIntLine(rng.i32_any()),
            2 => Op::Bad(rng.range(0, 11)),
            _ => Op::Good(rng.range(-5, 15)),
        })
        .collect();

    for (name, run) in [
        ("C", &common::run_c as &dyn Fn(&str) -> common::Outcome),
        ("Rust", &common::run_rust as &dyn Fn(&str) -> common::Outcome),
    ] {
        let batched = run(&common::script(&ops)).stdout;
        let mut concatenated = Vec::new();
        for op in &ops {
            concatenated.extend_from_slice(&run(&common::script(&[op.clone()])).stdout);
        }
        assert_eq!(
            batched, concatenated,
            "{name}: batched output differs from per-call output => hidden state"
        );
    }

    assert_same("cfg_34b batched stream", &ops);
}
