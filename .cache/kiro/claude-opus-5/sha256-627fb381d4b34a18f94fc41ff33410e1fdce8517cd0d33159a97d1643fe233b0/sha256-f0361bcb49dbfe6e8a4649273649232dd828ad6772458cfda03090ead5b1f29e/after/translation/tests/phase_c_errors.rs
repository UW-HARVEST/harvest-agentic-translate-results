// Phase C — error-path differential tests, one test per ERRORS.md row.
//
// The library's whole error surface is `parse_val` returning `false`, which makes
// `driver` print exactly `An error occurred\n` and skip both `run` calls. Each
// test therefore asserts three things against BOTH `.so`s:
//   1. the rejection output is byte-identical,
//   2. it is the exact sentinel (`An error occurred\n`) — not merely "both
//      printed something",
//   3. `the_house` was not mutated (checked with a follow-up `run(0)`).

mod common;

use common::{Op, Rng, SEED, diff_rejection_leaves_state_intact, diff_sequence};

/// Worker entry point for the NULL-pointer crash-parity row (ERRORS.md #9).
/// Behaves as a no-op unless `DRIVER_NULL_WORKER_SO` is set, in which case it
/// dlopens that library, calls `driver(NULL)` and dies exactly as the C does.
fn zz_null_pointer_worker() {
    let _ = common::maybe_act_as_null_worker();
}

fn guard() {
    assert!(
        !common::maybe_act_as_null_worker(),
        "only zz_null_pointer_worker may act as the null worker"
    );
}

// ------------------------------------------------------------------ row 1

fn errors_row01_empty_string() {
    guard();
    diff_rejection_leaves_state_intact("row01", &[""]);
    // ...and interleaved with successful calls, so the error path is also
    // exercised from a non-pristine state.
    diff_sequence(
        "row01/stateful",
        &[
            Op::Run(3),
            Op::driver(""),
            Op::Run(3),
            Op::driver(""),
            Op::driver("7"),
            Op::driver(""),
        ],
    );
}

// ------------------------------------------------------------------ row 2

fn errors_row02_non_numeric() {
    guard();
    let inputs = [
        "abc", "!", "++1", "--1", "e5", ".5", "NaN", "inf", "INF", "nan", "x", "/", ":", "\u{7f}",
        "é", "-+3", "+-3", "\\", "'", "\"", "#5", "five",
    ];
    diff_rejection_leaves_state_intact("row02", &inputs);

    // Randomized non-numeric strings: every byte drawn from the non-digit,
    // non-sign, non-whitespace set so the C can never consume a digit.
    let mut rng = Rng::new(SEED ^ 102);
    let alphabet: Vec<u8> = (1u8..=126)
        .filter(|b| !b.is_ascii_digit() && !b.is_ascii_whitespace() && *b != b'+' && *b != b'-')
        .collect();
    let ops: Vec<Op> = (0..600)
        .map(|_| {
            let n = rng.range_i64(1, 24) as usize;
            let s: Vec<u8> = (0..n).map(|_| *rng.pick(&alphabet)).collect();
            Op::Driver(s)
        })
        .collect();
    diff_sequence("row02/fuzz", &ops);
}

// ------------------------------------------------------------------ row 3

fn errors_row03_whitespace_or_sign_only() {
    guard();
    let inputs = [
        " ", "  ", "\t", "\n", "\t\n", "\u{b}", "\u{c}", "\r", " \t\n\u{b}\u{c}\r", "+", "-", "   +",
        "\t-", "+ 1", "- 1", "+\t", "-\n", "++", "--", "+-", "-+",
    ];
    diff_rejection_leaves_state_intact("row03", &inputs);
}

// ------------------------------------------------------------------ row 4

fn errors_row04_erange_positive() {
    guard();
    let big = "9".repeat(400);
    let inputs = [
        "9223372036854775808",
        "9223372036854775809",
        "99999999999999999999",
        "18446744073709551616",
        "+9223372036854775808",
        big.as_str(),
    ];
    diff_rejection_leaves_state_intact("row04", &inputs);

    // Randomized: 20..80 digit numbers are always past LONG_MAX.
    let mut rng = Rng::new(SEED ^ 104);
    let ops: Vec<Op> = (0..300)
        .map(|_| {
            let n = rng.range_i64(20, 80) as usize;
            let mut s = String::new();
            s.push((b'1' + (rng.next_u64() % 9) as u8) as char);
            for _ in 1..n {
                s.push((b'0' + (rng.next_u64() % 10) as u8) as char);
            }
            Op::Driver(s.into_bytes())
        })
        .collect();
    diff_sequence("row04/fuzz", &ops);
}

// ------------------------------------------------------------------ row 5

fn errors_row05_erange_negative() {
    guard();
    let big = format!("-{}", "9".repeat(400));
    let inputs = [
        "-9223372036854775809",
        "-9223372036854775810",
        "-99999999999999999999",
        "-18446744073709551616",
        big.as_str(),
    ];
    diff_rejection_leaves_state_intact("row05", &inputs);

    let mut rng = Rng::new(SEED ^ 105);
    let ops: Vec<Op> = (0..300)
        .map(|_| {
            let n = rng.range_i64(20, 80) as usize;
            let mut s = String::from("-");
            s.push((b'1' + (rng.next_u64() % 9) as u8) as char);
            for _ in 1..n {
                s.push((b'0' + (rng.next_u64() % 10) as u8) as char);
            }
            Op::Driver(s.into_bytes())
        })
        .collect();
    diff_sequence("row05/fuzz", &ops);
}

// ------------------------------------------------------------------ row 6

fn errors_row06_above_int_max_within_long() {
    guard();
    let inputs = [
        "2147483648",
        "2147483649",
        "4294967295",
        "4294967296",
        "9223372036854775807",
        "+2147483648",
        "0002147483648",
        "  2147483648",
    ];
    diff_rejection_leaves_state_intact("row06", &inputs);

    // Randomized in (INT_MAX, LONG_MAX] — no errno, rejected by the range test.
    let mut rng = Rng::new(SEED ^ 106);
    let ops: Vec<Op> = (0..400)
        .map(|_| {
            let v = rng.range_i64(i32::MAX as i64 + 1, i64::MAX);
            Op::Driver(format!("{v}").into_bytes())
        })
        .collect();
    diff_sequence("row06/fuzz", &ops);
}

// ------------------------------------------------------------------ row 7

fn errors_row07_below_int_min_within_long() {
    guard();
    let inputs = [
        "-2147483649",
        "-2147483650",
        "-4294967296",
        "-9223372036854775808",
        "-0002147483649",
        "\t-2147483649",
    ];
    diff_rejection_leaves_state_intact("row07", &inputs);

    let mut rng = Rng::new(SEED ^ 107);
    let ops: Vec<Op> = (0..400)
        .map(|_| {
            let v = rng.range_i64(i64::MIN, i32::MIN as i64 - 1);
            Op::Driver(format!("{v}").into_bytes())
        })
        .collect();
    diff_sequence("row07/fuzz", &ops);
}

// ------------------------------------------------------------------ row 8

fn errors_row08_one_past_valid_range_both_sides() {
    guard();
    // Inside the window: accepted, four `run` prints twice over.
    for ok in ["2147483647", "-2147483648"] {
        common::assert_accepted_by_c(ok);
        diff_sequence(&format!("row08/accept/{ok}"), &[Op::driver(ok), Op::Run(0)]);
    }
    // Exactly one step outside: rejected.
    diff_rejection_leaves_state_intact("row08/reject", &["2147483648", "-2147483649"]);

    // Also the LONG boundary one step out (switches the *reason* from range
    // test to errno, but the observable result must be the same).
    diff_rejection_leaves_state_intact(
        "row08/long-edge",
        &["9223372036854775808", "-9223372036854775809"],
    );
    for ok_long_but_bad_int in ["9223372036854775807", "-9223372036854775808"] {
        diff_rejection_leaves_state_intact("row08/long-max", &[ok_long_but_bad_int]);
    }
}

// ------------------------------------------------------------------ row 9

fn errors_row09_null_pointer_same_signal() {
    guard();
    let c = common::null_worker_status(&common::c_so(), "zz_null_pointer_worker");
    let r = common::null_worker_status(&common::rust_so(), "zz_null_pointer_worker");

    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        let (cs, rs) = (c.status, r.status);
        assert_eq!(
            (cs.signal(), cs.code()),
            (rs.signal(), rs.code()),
            "NULL-pointer termination differs: C {cs:?} vs Rust {rs:?}\n\
             C stderr: {}\nRust stderr: {}",
            String::from_utf8_lossy(&c.stderr),
            String::from_utf8_lossy(&r.stderr),
        );
        assert_eq!(
            cs.signal(),
            Some(libc_sigsegv()),
            "expected the C library to fault on driver(NULL); got {cs:?}"
        );
    }
}

fn libc_sigsegv() -> i32 {
    11
}

// ----------------------------------------------------------------- row 10

fn errors_row10_run_extreme_ints() {
    guard();
    // `run` has no rejection path: every `int` — including the values an
    // out-of-range C enum would deliver across the FFI boundary — must be
    // accepted and produce identical two's-complement wrapped output.
    let mut ops = Vec::new();
    for v in [
        i32::MAX,
        i32::MIN,
        -1,
        0,
        1,
        i32::MAX - 4,
        i32::MIN + 4,
        0x7fff_fffe,
        -0x7fff_ffff,
        1 << 30,
        -(1 << 30),
        // "enum-shaped" invalid discriminants
        999_999,
        -999_999,
        0xDEAD_BEEFu32 as i32,
        0x0BAD_F00Du32 as i32,
    ] {
        ops.push(Op::Run(v));
    }
    common::diff_each_from_pristine("row10/pristine", &ops);
    diff_sequence("row10/accumulated", &ops);

    // Drive `bedrooms` deliberately through `int` overflow, repeatedly.
    let overflow: Vec<Op> = (0..64).map(|_| Op::Run(i32::MAX)).collect();
    diff_sequence("row10/overflow-max", &overflow);
    let underflow: Vec<Op> = (0..64).map(|_| Op::Run(i32::MIN)).collect();
    diff_sequence("row10/overflow-min", &underflow);
}

// ----------------------------------------------------------------- row 11

fn errors_row11_trailing_garbage_accepted() {
    guard();
    // Documented quirk: the C never requires `endp` to reach the NUL, so these
    // are ACCEPTED. A "fixed" translation that rejects them diverges here.
    let inputs = [
        "12abc",
        "5 5",
        "0x10",
        "1.9",
        "-3junk",
        "  +7 rest",
        "0b101",
        "42\n",
        "42\t",
        "007tail",
        "2147483647!",
        "-2147483648!",
    ];
    for s in inputs {
        common::assert_accepted_by_c(s);
    }
    let ops: Vec<Op> = inputs.iter().map(|s| Op::driver(s)).collect();
    common::diff_each_from_pristine("row11/pristine", &ops);
    diff_sequence("row11/accumulated", &ops);
}

// ----------------------------------------------------------------- row 12

fn errors_row12_oversized_inputs() {
    guard();
    let digits_4096 = "1".repeat(4096);
    let junk_4096 = "z".repeat(4096);
    let ws_4096 = " ".repeat(4096);
    let ws_then_num = format!("{}{}", " ".repeat(4096), 12345);
    let zeros_then_num = format!("{}{}", "0".repeat(4096), 7);

    let neg_digits_4096 = format!("-{digits_4096}");

    // ERANGE / non-numeric giants are rejected...
    diff_rejection_leaves_state_intact(
        "row12/reject",
        &[
            digits_4096.as_str(),
            junk_4096.as_str(),
            ws_4096.as_str(),
            neg_digits_4096.as_str(),
        ],
    );
    // ...but 4096 spaces or 4096 leading zeros followed by a small number are
    // valid and must run.
    for ok in [ws_then_num.as_str(), zeros_then_num.as_str()] {
        common::assert_accepted_by_c(ok);
        diff_sequence("row12/accept", &[Op::Driver(ok.as_bytes().to_vec()), Op::Run(0)]);
    }
    // Zero length is the degenerate "oversized" boundary on the other end.
    diff_rejection_leaves_state_intact("row12/zero-length", &[""]);
}

// ------------------------------------------------------------------- runner

fn main() -> ! {
    common::run_suite(
        "phase C (ERRORS.md rows)",
        &[
            ("zz_null_pointer_worker", zz_null_pointer_worker as common::TestFn),
            ("errors_row01_empty_string", errors_row01_empty_string as common::TestFn),
            ("errors_row02_non_numeric", errors_row02_non_numeric as common::TestFn),
            ("errors_row03_whitespace_or_sign_only", errors_row03_whitespace_or_sign_only as common::TestFn),
            ("errors_row04_erange_positive", errors_row04_erange_positive as common::TestFn),
            ("errors_row05_erange_negative", errors_row05_erange_negative as common::TestFn),
            ("errors_row06_above_int_max_within_long", errors_row06_above_int_max_within_long as common::TestFn),
            ("errors_row07_below_int_min_within_long", errors_row07_below_int_min_within_long as common::TestFn),
            ("errors_row08_one_past_valid_range_both_sides", errors_row08_one_past_valid_range_both_sides as common::TestFn),
            ("errors_row09_null_pointer_same_signal", errors_row09_null_pointer_same_signal as common::TestFn),
            ("errors_row10_run_extreme_ints", errors_row10_run_extreme_ints as common::TestFn),
            ("errors_row11_trailing_garbage_accepted", errors_row11_trailing_garbage_accepted as common::TestFn),
            ("errors_row12_oversized_inputs", errors_row12_oversized_inputs as common::TestFn),
        ],
    )
}
