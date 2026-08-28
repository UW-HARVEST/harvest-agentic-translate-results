//! Operation 3 (`validate_sequence`). Four rules run in order (start with
//! `y`, end with `n` when longer than one character, no more than three
//! consecutive equal values, then a transition count), and the final verdict
//! depends on which of the three length bands the string falls into:
//! `len <= 3`, `len <= 10`, and longer.

mod harness;
use harness::{all_patterns, case, compare_all};

/// Repeat `unit` until it is at least `len` characters, then cut to `len`.
fn fill(unit: &str, len: usize) -> String {
    let mut s = String::new();
    while s.len() < len {
        s.push_str(unit);
    }
    s.truncate(len);
    s
}

/// Every pattern of length 1 through 10: the whole `len <= 3` band and the
/// whole `len <= 10` band, including all the early rejections.
#[test]
fn operation_three_exhaustive_short() {
    let mut inputs = Vec::new();
    for len in 1..=10u32 {
        for s in all_patterns(len) {
            inputs.push(case("3", "0", &s));
        }
    }
    compare_all("operation_three_exhaustive_short", inputs);
}

/// Every pattern of length 11 and 12: the start of the long band, where the
/// verdict is 45 or 50 depending on the transition count. (Length 13 was
/// swept exhaustively out of band as well; it adds no new branch.)
#[test]
fn operation_three_exhaustive_long_band() {
    let mut inputs = Vec::new();
    for len in 11..=12u32 {
        for s in all_patterns(len) {
            inputs.push(case("3", "0", &s));
        }
    }
    compare_all("operation_three_exhaustive_long_band", inputs);
}

/// Rule 1: the sequence must start with a true value, otherwise -10.
#[test]
fn operation_three_rule_one_must_start_true() {
    let mut inputs = Vec::new();
    for first in ["n", "N", "x", "X", "?", "\t", "0", " "] {
        for rest in [
            "", "n", "y", "nn", "yy", "yn", "ynynyn", "nnnnnnnnnnnn", "ynnyynnyynn",
        ] {
            inputs.push(case("3", "0", &format!("{first}{rest}")));
        }
    }
    compare_all("operation_three_rule_one_must_start_true", inputs);
}

/// Rule 2: a sequence longer than one character must end with a false
/// value, otherwise -11. A single character skips this rule.
#[test]
fn operation_three_rule_two_must_end_false() {
    let mut inputs = Vec::new();
    // Single character: rule 2 does not apply.
    for s in ["y", "Y"] {
        inputs.push(case("3", "0", s));
    }
    // Valid start, true final value, at a range of lengths.
    for len in 2..=16usize {
        let mut s = fill("yn", len - 1);
        s.push('y');
        inputs.push(case("3", "0", &s));

        let mut s = fill("ynn", len - 1);
        s.push('Y');
        inputs.push(case("3", "0", &s));
    }
    // The mirror image: same sequences ending false.
    for len in 2..=16usize {
        let mut s = fill("yn", len - 1);
        s.push('n');
        inputs.push(case("3", "0", &s));
    }
    compare_all("operation_three_rule_two_must_end_false", inputs);
}

/// Rule 3: more than three consecutive equal values gives -12. Runs of one
/// through eight, placed at the start, the middle and the end.
#[test]
fn operation_three_rule_three_consecutive_limit() {
    let mut inputs = Vec::new();
    for run in 1..=8usize {
        // A run of true values right after the leading `y`.
        inputs.push(case("3", "0", &format!("y{}n", "y".repeat(run))));
        // A run of false values in the middle.
        inputs.push(case("3", "0", &format!("y{}yn", "n".repeat(run))));
        // A run of false values at the end.
        inputs.push(case("3", "0", &format!("yn{}", "n".repeat(run))));
        // A run buried in a longer otherwise-valid sequence.
        inputs.push(case(
            "3",
            "0",
            &format!("ynnyy{}ynn", "n".repeat(run)),
        ));
        // Exactly at the boundary: three equal values is allowed, four is not.
        inputs.push(case("3", "0", &format!("yyy{}n", "n".repeat(run))));
    }
    compare_all("operation_three_rule_three_consecutive_limit", inputs);
}

/// Rule 4 and the three length bands. Only sequences with runs of at most
/// three survive rule 3, so these are the patterns that actually reach the
/// transition count.
#[test]
fn operation_three_length_bands() {
    let mut inputs = Vec::new();

    // Every pattern right at the band edges.
    for len in [1u32, 2, 3, 4, 10, 11, 12] {
        for s in all_patterns(len) {
            inputs.push(case("3", "0", &s));
        }
    }

    // Repeating units that keep runs at three or fewer, over a wide range
    // of lengths crossing every band boundary.
    for unit in [
        "yn", "ynn", "yynn", "yyynnn", "ynnyn", "yynyn", "yyn", "ynnn", "ynyn", "yyynnnyn",
    ] {
        for len in 1..=40usize {
            inputs.push(case("3", "0", &fill(unit, len)));
        }
    }

    compare_all("operation_three_length_bands", inputs);
}

/// The parameter is never read by operation 3, upper case is equivalent to
/// lower case, other characters count as false, and the string is subject to
/// the same 1023 byte `fgets` truncation as everywhere else.
#[test]
fn operation_three_long_and_mixed() {
    let mut inputs = Vec::new();

    for param in ["0", "1", "2", "3", "-1", "99", "4294967296", "abc", ""] {
        for s in ["y", "yn", "ynn", "ynynynynynyn", "nyn", "yyyyn", "ynnyynnyn"] {
            inputs.push(case("3", param, s));
        }
    }

    for s in [
        "Y", "YN", "YNN", "YnYn", "yNyN", "Yx", "yX", "y?", "?y", "yxxy", "yXXXy", "y\tn",
        "YNNYYNN", "yxyxyx", "xyxyxy",
    ] {
        inputs.push(case("3", "0", s));
    }

    // Long structured sequences well past the bands.
    for unit in ["yn", "ynn", "yynn", "yyynnn", "ynnn", "yyyn", "ynyn", "ynnyynn"] {
        for len in [14usize, 20, 33, 64, 100, 255, 511, 700, 1000] {
            inputs.push(case("3", "0", &fill(unit, len)));
        }
    }

    // Near and past the truncation point.
    for len in [1020usize, 1021, 1022, 1023, 1024, 1025, 2048] {
        inputs.push(case("3", "0", &"y".repeat(len)));
        inputs.push(case("3", "0", &"n".repeat(len)));
        inputs.push(case("3", "0", &fill("yn", len)));
        inputs.push(case("3", "0", &fill("ynnyy", len)));
    }

    compare_all("operation_three_long_and_mixed", inputs);
}
