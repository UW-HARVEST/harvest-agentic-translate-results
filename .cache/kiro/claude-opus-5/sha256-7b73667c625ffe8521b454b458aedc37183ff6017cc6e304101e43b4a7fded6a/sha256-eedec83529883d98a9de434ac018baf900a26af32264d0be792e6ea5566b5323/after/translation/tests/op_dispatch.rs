//! Operation dispatch in `process_decisions`, and the `atoi` conversion in
//! `main` that decides which operation and parameter are used.

mod harness;
use harness::{case, compare_all};

/// Only 0, 1, 2 and 3 are handled; everything else falls through to the
/// `default` arm and returns -3. The `length == 0` guard runs first, so an
/// empty decision string returns -1 even for an unknown operation.
#[test]
fn unknown_operations_return_minus_three() {
    let mut inputs = Vec::new();
    for op in [
        "4", "5", "6", "10", "99", "-1", "-2", "-3", "-100", "2147483647", "-2147483648",
    ] {
        for param in ["0", "1", "2", "3", "-1", "99"] {
            for s in ["", "y", "n", "yyy", "nnn", "ynynyn"] {
                inputs.push(case(op, param, s));
            }
        }
    }
    compare_all("unknown_operations_return_minus_three", inputs);
}

/// `atoi` is glibc's `(int) strtol(s, NULL, 10)`: leading whitespace and a
/// sign are accepted, trailing junk is ignored, no digits gives 0, and an
/// out-of-range value saturates at `LONG_MAX`/`LONG_MIN` before being
/// truncated to `int`. Truncation is what makes, for example, 4294967296
/// select operation 0.
#[test]
fn atoi_conversion() {
    let numbers = [
        // Plain values, signs, padding and leading zeros.
        "0", "1", "2", "3", "4", "-0", "+0", "+1", "+2", "+3", "-1", "-2", "-3", "007", "0002",
        " 2", "  3", "\t2", "\t\t3", " -2", "  +3", "0000000000000000000000003",
        // No digits at all, or digits that do not start the string.
        "", " ", "\t", "-", "+", "--3", "++2", "- 2", "+ 3", "abc", "x2", ".5", "..", "?",
        // Trailing junk after valid digits.
        "2x", "3junk", "2 ", "3\t", "3.9", "2e3", "0x3", "1,000",
        // int boundaries.
        "2147483647", "2147483648", "2147483649", "-2147483647", "-2147483648", "-2147483649",
        // Truncation of a wider value down to int.
        "4294967296", "4294967297", "4294967298", "4294967299", "4294967300",
        "-4294967296", "-4294967295", "8589934592", "8589934595",
        // long boundaries, where strtol saturates before the cast to int.
        "9223372036854775806", "9223372036854775807", "9223372036854775808",
        "-9223372036854775808", "-9223372036854775809",
        // Far beyond long: saturates to LONG_MAX / LONG_MIN.
        "18446744073709551615", "18446744073709551616", "18446744073709551619",
        "99999999999999999999999999999", "-99999999999999999999999999999",
        "1000000000000000000000000000000", "999999999999999999999999999999",
    ];

    let mut inputs = Vec::new();
    for n in numbers {
        // As the operation.
        inputs.push(format!("{n}\n0\nyny\n").into_bytes());
        inputs.push(format!("{n}\n2\nynnyn\n").into_bytes());
        // As the parameter (operation 1 is the one that branches on it).
        inputs.push(format!("1\n{n}\nyyy\n").into_bytes());
        inputs.push(format!("1\n{n}\nynn\n").into_bytes());
        inputs.push(format!("1\n{n}\nnnn\n").into_bytes());
        // In both positions at once.
        inputs.push(format!("{n}\n{n}\nynnyn\n").into_bytes());
    }
    compare_all("atoi_conversion", inputs);
}
