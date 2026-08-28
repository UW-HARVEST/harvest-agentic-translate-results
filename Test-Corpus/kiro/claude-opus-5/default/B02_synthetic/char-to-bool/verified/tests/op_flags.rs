//! Operation 2 (`configure_flags`). All decisions are used, but the count is
//! capped at 32, so the interesting boundaries are the pattern rules
//! (all false, all true, exactly one true, exactly one false, alternating,
//! three or more consecutive true) and the 32-element cap itself.

mod harness;
use harness::{all_patterns, case, compare_all};

/// Every `y`/`n` pattern of length 1 through 10. This reaches every rule in
/// `configure_flags` many times over, including the fall-through where a
/// pattern matches none of them and the raw count of true values is
/// returned.
#[test]
fn operation_two_exhaustive_short() {
    let mut inputs = Vec::new();
    for len in 1..=10u32 {
        for s in all_patterns(len) {
            inputs.push(case("2", "0", &s));
        }
    }
    compare_all("operation_two_exhaustive_short", inputs);
}

/// Every pattern of length 11 and 12, where the "exactly one false" rule,
/// the alternating rule and the consecutive-true rule start to compete with
/// each other.
#[test]
fn operation_two_exhaustive_medium() {
    let mut inputs = Vec::new();
    for len in 11..=12u32 {
        for s in all_patterns(len) {
            inputs.push(case("2", "0", &s));
        }
    }
    compare_all("operation_two_exhaustive_medium", inputs);
}

/// The 32-decision cap: `count = min(length, 32)`. A longer string is
/// truncated, so characters from index 32 onwards cannot affect the result
/// even though they lengthen the string.
#[test]
fn operation_two_thirty_two_decision_cap() {
    let mut inputs = Vec::new();
    for len in [28usize, 29, 30, 31, 32, 33, 34, 40, 63, 64, 65, 100, 200] {
        // All true: `special_count == count`, so 1000 + count.
        inputs.push(case("2", "0", &"y".repeat(len)));
        // All false: 0.
        inputs.push(case("2", "0", &"n".repeat(len)));
        // Alternating, both phases.
        inputs.push(case("2", "0", &"yn".repeat(len / 2 + 1)[..len]));
        inputs.push(case("2", "0", &"ny".repeat(len / 2 + 1)[..len]));
        // Exactly one true, at each position.
        for i in 0..len {
            let mut s = vec!['n'; len];
            s[i] = 'y';
            inputs.push(case("2", "0", &s.iter().collect::<String>()));
        }
        // Exactly one false, at each position.
        for i in 0..len {
            let mut s = vec!['y'; len];
            s[i] = 'n';
            inputs.push(case("2", "0", &s.iter().collect::<String>()));
        }
        // Runs of true values of every length, to drive max_consecutive.
        for run in 1..=len.min(12) {
            let mut s = "y".repeat(run);
            s.push('n');
            while s.len() < len {
                s.push(if s.len() % 3 == 0 { 'y' } else { 'n' });
            }
            inputs.push(case("2", "0", &s[..len]));
        }
        // Beyond the cap the extra characters are ignored, so these must
        // agree with each other as well as with the C.
        if len > 32 {
            let mut s = "y".repeat(32);
            s.push_str(&"n".repeat(len - 32));
            inputs.push(case("2", "0", &s));
            let mut s = "n".repeat(32);
            s.push_str(&"y".repeat(len - 32));
            inputs.push(case("2", "0", &s));
        }
    }
    compare_all("operation_two_thirty_two_decision_cap", inputs);
}

/// Longer strings with structured patterns, plus the parameter being
/// irrelevant to operation 2, and non-`y`/`n` characters counting as false.
#[test]
fn operation_two_long_and_mixed() {
    let mut inputs = Vec::new();

    // The parameter is never read by operation 2.
    for param in ["0", "1", "2", "3", "-1", "99", "2147483648"] {
        for s in ["y", "n", "ynyn", "yyynnn", "ynnn", "yyyy"] {
            inputs.push(case("2", param, s));
        }
    }

    // Non-`y`/`n` characters are false, so these mirror `n` patterns.
    for s in [
        "x", "X", "?", "Y", "N", "YN", "YyNn", "yxyx", "xyxy", "y\tn", "Yxx", "xxY",
        "YYYY", "NNNN", "YYYNNN", "yXyXyX",
    ] {
        inputs.push(case("2", "0", s));
    }

    // Long structured patterns.
    for rep in [1usize, 2, 3, 5, 8, 11, 16, 20, 40] {
        for unit in ["y", "n", "yn", "ny", "yyn", "ynn", "yyyn", "ynnn", "yyynnn", "ynyn"] {
            let s = unit.repeat(rep);
            inputs.push(case("2", "0", &s));
        }
    }

    // Near the fgets truncation point.
    for len in [1022usize, 1023, 1024, 1025] {
        inputs.push(case("2", "0", &"y".repeat(len)));
        inputs.push(case("2", "0", &"n".repeat(len)));
        inputs.push(case("2", "0", &"yn".repeat(len / 2 + 1)[..len]));
    }

    compare_all("operation_two_long_and_mixed", inputs);
}
