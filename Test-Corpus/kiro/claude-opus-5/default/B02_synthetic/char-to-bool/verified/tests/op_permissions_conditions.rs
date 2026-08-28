//! Operation 0 (`apply_permissions`) and operation 1 (`evaluate_conditions`).
//! Both read only the first three characters and both reject a string
//! shorter than three characters with -2.

mod harness;
use harness::{all_patterns, case, compare_all};

/// Every one of the eight read/write/execute combinations, spelled with
/// every mixture of upper and lower case and with non-`y`/`n` filler that
/// `parse_bool` treats as false.
#[test]
fn operation_zero_all_permission_combinations() {
    let mut inputs = Vec::new();
    let spellings = ["y", "Y", "n", "N", "x", "X", "?", "\t"];

    // All 8 * 8 * 8 three-character combinations over that alphabet.
    for a in spellings {
        for b in spellings {
            for c in spellings {
                let s = format!("{a}{b}{c}");
                for param in ["0", "1", "2", "3", "-1", "99"] {
                    inputs.push(case("0", param, &s));
                }
            }
        }
    }
    // Trailing characters beyond the third are ignored.
    for tail in ["", "y", "n", "yyyy", "nnnn", "xyz"] {
        for head in all_patterns(3) {
            inputs.push(case("0", "0", &format!("{head}{tail}")));
        }
    }
    compare_all("operation_zero_all_permission_combinations", inputs);
}

/// Operation 0 needs three characters; one or two give -2, and zero gives
/// -1 from the earlier `length == 0` guard.
#[test]
fn operation_zero_short_strings() {
    let mut inputs = Vec::new();
    for param in ["0", "1", "2", "3"] {
        inputs.push(case("0", param, ""));
        for len in 1..=2u32 {
            for s in all_patterns(len) {
                inputs.push(case("0", param, &s));
            }
        }
        for s in ["x", "X", "Y", "N", "xy", "Yn", "?", "??"] {
            inputs.push(case("0", param, s));
        }
    }
    compare_all("operation_zero_short_strings", inputs);
}

/// Operation 1 with every logic operator: 0 = AND, 1 = OR, 2 = XOR,
/// 3 = NAND, and anything else returns -1 from the `default` arm.
#[test]
fn operation_one_all_logic_operators() {
    let mut inputs = Vec::new();
    let params = [
        "0", "1", "2", "3", // the handled operators
        "4", "5", "-1", "-2", "99", "2147483647", "-2147483648", // the default arm
    ];
    for param in params {
        for s in all_patterns(3) {
            inputs.push(case("1", param, &s));
        }
        for s in [
            "YYY", "NNN", "YyN", "nYy", "xxx", "yxn", "xyy", "yyx", "?y?", "\ty\t",
        ] {
            inputs.push(case("1", param, s));
        }
        // Characters past the third are ignored.
        for s in ["yyyn", "nnnyyy", "ynnnnnn", "yyyyyy"] {
            inputs.push(case("1", param, s));
        }
        // Short strings: -2, or -1 when empty.
        inputs.push(case("1", param, ""));
        for len in 1..=2u32 {
            for s in all_patterns(len) {
                inputs.push(case("1", param, &s));
            }
        }
    }
    compare_all("operation_one_all_logic_operators", inputs);
}

/// The four logic operators over all eight condition triples, checked
/// against the specific values the C returns. This pins the partial-match
/// ladder in the AND arm (50/51/52/10/11/12) and the false-combination
/// ladder in the NAND arm (200/150/151/152), where a reordered check would
/// still look plausible.
#[test]
fn operation_one_ladders() {
    let mut inputs = Vec::new();
    for param in ["0", "1", "2", "3"] {
        for s in all_patterns(3) {
            // Same triple, reached through upper case and through filler
            // characters, so the ladders are exercised more than once.
            inputs.push(case("1", param, &s));
            inputs.push(case("1", param, &s.to_uppercase()));
            inputs.push(case("1", param, &s.replace('n', "x")));
            inputs.push(case("1", param, &s.replace('n', "Q")));
        }
    }
    compare_all("operation_one_ladders", inputs);
}
