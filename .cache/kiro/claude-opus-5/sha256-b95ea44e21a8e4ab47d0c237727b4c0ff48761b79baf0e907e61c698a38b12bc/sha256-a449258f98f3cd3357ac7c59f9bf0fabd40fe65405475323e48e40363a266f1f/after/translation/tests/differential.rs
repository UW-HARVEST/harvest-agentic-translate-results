//! Differential tests: the Rust `driver` binary vs the C `driver` binary.
//!
//! Every test spawns BOTH executables as subprocesses, feeds them identical
//! bytes on stdin, and asserts that stdout, stderr and exit status all match.
//!
//! # Branches of the C program that these tests enumerate
//!
//! `main`:
//!   M1  `scanf("%d %d")` converts both fields
//!   M2  first conversion fails (EOF or non-numeric) -> `x`/`y` keep their 0 init
//!   M3  first converts, second fails -> `y` keeps its 0 init
//!   M4  immediate EOF (empty stdin)
//!   M5  `%d` sub-paths: leading whitespace run, `+`/`-` sign, sign-then-EOF,
//!       sign-then-non-digit, leading zeros, `long` saturation, `long`->`int`
//!       truncation, `scanf` crossing newlines
//!
//! `foo`:
//!   F1  `while (x > 0 || y > 0)` false on entry              -> no output
//!   F2  loop entered because of `x > 0` only
//!   F3  loop entered because of `y > 0` only
//!   F4  `if (x == 1 && y == 4)` TRUE  -> `goto label2`, skipping `label1`
//!   F5  `if (x == 1 && y == 4)` FALSE -> fall through to `label1`
//!   F6  `label1: if (x > 0)` TRUE  -> print "x", `x--`
//!   F7  `label1: if (x > 0)` FALSE -> fall through to `label2`
//!   F8  `label2: if (y == 0) continue;` TAKEN -> re-test the loop condition
//!   F9  `if (y == 0)` FALSE -> print "y", `y--`
//!   F10 `if (x < 3) goto label1;` TAKEN -> re-enter the body without
//!       re-testing the `while` condition
//!   F11 `if (x < 3)` FALSE -> fall off the body, re-test the condition

mod harness;

use harness::{assert_identical, assert_identical_prefix, pair};

// ---------------------------------------------------------------------------
// M4 / F1 -- empty and whitespace-only input: nothing is converted, x = y = 0
// ---------------------------------------------------------------------------

#[test]
fn empty_input() {
    assert_identical("empty stdin", b"");
}

#[test]
fn whitespace_only_input() {
    // Every character `isspace` accepts, then EOF: an input failure.
    assert_identical("whitespace only", b"   \t\n\x0b\x0c\r  ");
}

#[test]
fn newline_only_input() {
    assert_identical("single newline", b"\n");
}

// ---------------------------------------------------------------------------
// M2 / M3 -- matching failures leave the C locals at their 0 initialisers
// ---------------------------------------------------------------------------

#[test]
fn first_field_not_a_number() {
    assert_identical("abc", b"abc");
    assert_identical("abc 5", b"abc 5");
    assert_identical("dot", b".");
    assert_identical("leading x", b"x 1");
}

#[test]
fn sign_with_no_digits() {
    // `-`/`+` then EOF, and sign then a non-digit: both are matching failures,
    // so `x` stays 0 and the second conversion is never attempted.
    assert_identical("minus then EOF", b"-");
    assert_identical("plus then EOF", b"+");
    assert_identical("minus space digit", b"- 5");
    assert_identical("plus space digit", b"+ 3");
    assert_identical("minus letter", b"-a");
}

#[test]
fn second_field_not_a_number() {
    // x converts, y does not -> y is 0, which drives the `y == 0` branch (F8).
    assert_identical("5 abc", b"5 abc");
    assert_identical("5 dot3", b"5 .3");
    assert_identical("3 plus", b"3 +");
    assert_identical("single field", b"5");
}

#[test]
fn hex_and_exponent_are_not_special_for_percent_d() {
    // `%d` is base 10: "0x10" converts 0 and then chokes on 'x';
    // "5e3" converts 5 and then chokes on 'e'.
    assert_identical("0x10", b"0x10");
    assert_identical("5e3 2", b"5e3 2");
    assert_identical("0b11 4", b"0b11 4");
}

// ---------------------------------------------------------------------------
// M5 -- how `%d` and the literal space in "%d %d" consume whitespace
// ---------------------------------------------------------------------------

#[test]
fn scanf_reads_across_newlines_and_odd_whitespace() {
    assert_identical("newline separator", b"3\n2");
    assert_identical("crlf separator", b"5\r\n3");
    assert_identical("vertical tab / form feed", b"\x0b5\x0c3");
    assert_identical("tabs everywhere", b"\t\t3\t\t2\t\t");
    assert_identical("leading blank lines", b"\n\n 3\t2\n");
    assert_identical("wide spacing", b"  7   2  ");
}

#[test]
fn trailing_bytes_after_the_two_fields_are_ignored() {
    assert_identical("trailing junk", b"3 2junk");
    assert_identical("trailing newline", b"3 2\n");
    assert_identical("second line ignored", b"3 2\n9 9\n");
    assert_identical("trailing space then EOF", b"5 ");
    assert_identical("embedded NUL", b"5\x00 3");
}

#[test]
fn explicit_signs_and_leading_zeros() {
    assert_identical("plus plus", b"+4 +2");
    assert_identical("leading zeros", b"0000000000000000005 2");
    assert_identical("zeros for y", b"0 00000000000000000004");
    // No whitespace before the '-': the literal space in the format matches an
    // empty whitespace run, so this converts x = 0 and y = -3.
    assert_identical("glued negative", b"0-3");
}

#[test]
fn negative_values() {
    assert_identical("-1 5", b"-1 5");
    assert_identical("-5 3", b"-5 3");
    assert_identical("-1 -1", b"-1 -1");
    assert_identical("0 -3", b"0 -3");
    assert_identical("-3 0", b"-3 0");
    assert_identical("int min", b"-2147483648 0");
}

// ---------------------------------------------------------------------------
// M5 -- overflow: glibc converts at `long` width, saturating at LONG_MAX /
// LONG_MIN, then truncates to `int`. These inputs pin that behaviour down.
// ---------------------------------------------------------------------------

#[test]
fn values_above_int_range_are_truncated_to_int() {
    // 2^31 -> (int) -2147483648, so the loop is never entered.
    assert_identical("2^31", b"2147483648 0");
    // 2^32 -> (int) 0 for both fields.
    assert_identical("2^32 twice", b"4294967296 4294967296");
    // -(2^32 - 1) -> (int) 1, and 2^32 -> (int) 0: this run prints output, so
    // it distinguishes truncation from clamping.
    assert_identical("neg 2^32-1", b"-4294967295 4294967296");
    assert_identical("2^32+1", b"4294967297 0");
}

#[test]
fn values_beyond_long_range_saturate_then_truncate() {
    // > LONG_MAX -> LONG_MAX -> (int) -1
    assert_identical("long max overflow", b"99999999999999999999999 0");
    assert_identical("long max overflow with y", b"99999999999999999999999 5");
    // < LONG_MIN -> LONG_MIN -> (int) 0
    assert_identical("long min overflow", b"-99999999999999999999999 0");
    assert_identical("long min overflow with y", b"-99999999999999999999999 7");
    // A digit string long enough to rule out any small fixed-size accumulator.
    let mut huge = vec![b'9'; 200];
    huge.extend_from_slice(b" 3");
    assert_identical("200 nines", &huge);
}

// ---------------------------------------------------------------------------
// F1 -- loop never entered
// ---------------------------------------------------------------------------

#[test]
fn loop_not_entered() {
    assert_identical("0 0", &pair(0, 0));
    assert_identical("-1 0", &pair(-1, 0));
    assert_identical("0 -1", &pair(0, -1));
    assert_identical("-7 -7", &pair(-7, -7));
}

// ---------------------------------------------------------------------------
// F4 -- the `x == 1 && y == 4` special case that jumps straight to label2
// ---------------------------------------------------------------------------

#[test]
fn goto_label2_special_case() {
    assert_identical("1 4 (goto label2)", &pair(1, 4));
    // Neighbours of the special case, which must NOT take the goto.
    assert_identical("1 3", &pair(1, 3));
    assert_identical("1 5", &pair(1, 5));
    assert_identical("2 4", &pair(2, 4));
    assert_identical("0 4", &pair(0, 4));
}

// ---------------------------------------------------------------------------
// F2 / F6 / F8 -- x drives the loop, `y == 0` continues every iteration
// ---------------------------------------------------------------------------

#[test]
fn x_only_uses_the_continue_path() {
    for x in [1, 2, 3, 4, 7, 33] {
        assert_identical(&format!("{x} 0"), &pair(x, 0));
    }
}

// ---------------------------------------------------------------------------
// F3 / F7 / F9 / F10 -- y drives the loop, label1 never prints
// ---------------------------------------------------------------------------

#[test]
fn y_only_loops_on_goto_label1() {
    for y in [1, 2, 4, 7, 18] {
        assert_identical(&format!("0 {y}"), &pair(0, y));
        assert_identical(&format!("-4 {y}"), &pair(-4, y));
    }
}

// ---------------------------------------------------------------------------
// F10 vs F11 -- `x < 3` decides between re-entering label1 and re-testing the
// while condition. x >= 3 after the decrement is the F11 side.
// ---------------------------------------------------------------------------

#[test]
fn x_less_than_three_boundary() {
    assert_identical("4 1 (x==3 at the test)", &pair(4, 1));
    assert_identical("5 1", &pair(5, 1));
    assert_identical("3 1 (x==2 at the test)", &pair(3, 1));
    assert_identical("4 2", &pair(4, 2));
    assert_identical("6 3", &pair(6, 3));
}

// ---------------------------------------------------------------------------
// Dense sweep: every combination in a window that exercises all of F1-F11
// together, including the x <= 0 column and the `1 4` cell.
// ---------------------------------------------------------------------------

#[test]
fn dense_grid_matches() {
    for x in -8i64..=40 {
        for y in 0i64..=40 {
            let input = pair(x, y);
            assert_identical(&format!("grid x={x} y={y}"), &input);
        }
    }
}

// ---------------------------------------------------------------------------
// Larger, still-terminating runs: hundreds of thousands of output lines.
// ---------------------------------------------------------------------------

#[test]
fn large_terminating_runs() {
    assert_identical("0 100000", &pair(0, 100_000));
    assert_identical("100000 0", &pair(100_000, 0));
    assert_identical("100000 100000", &pair(100_000, 100_000));
    assert_identical("1 400000", &pair(1, 400_000));
}

// ---------------------------------------------------------------------------
// The unbounded classes. `y` is decremented whenever `y != 0`, so a negative
// `y` with a positive `x` walks `y` all the way down through INT_MIN; and
// x = INT_MAX runs ~2^31 iterations. Neither terminates in test time, so the
// observable output PREFIX is compared instead. See ERRORS.md.
// ---------------------------------------------------------------------------

#[test]
fn negative_y_with_positive_x_prefix_matches() {
    const CAP: usize = 4 << 20;
    assert_identical_prefix("1 -1", &pair(1, -1), CAP);
    assert_identical_prefix("5 -2", &pair(5, -2), CAP);
    assert_identical_prefix("3 -100", &pair(3, -100), CAP);
    assert_identical_prefix("2 int_min", &pair(2, -2147483648), CAP);
}

#[test]
fn int_max_x_prefix_matches() {
    const CAP: usize = 4 << 20;
    assert_identical_prefix("int max x", b"2147483647 0", CAP);
    assert_identical_prefix("int max both", b"2147483647 2147483647", CAP);
}
