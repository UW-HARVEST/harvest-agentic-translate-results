use std::env;
use std::process::ExitCode;

/// Mimic C's `strtol(s, &end, 10)` behavior closely enough for this program.
/// Returns (parsed_value_as_i64, number_of_bytes_consumed_from_start).
/// - Skips leading ASCII whitespace.
/// - Accepts optional leading '+' or '-'.
/// - Parses decimal digits.
/// - On overflow, saturates to i64::MAX or i64::MIN (matches glibc semantics
///   for the values that fit in int after truncation; for this program the
///   subsequent cast to i32 would yield a wrapped/truncated value).
/// - If no digits are parsed, the consumed count is 0 (equivalent to
///   `end == argv[1]` in C).
fn strtol_base10(s: &[u8]) -> (i64, usize) {
    let mut i = 0usize;
    // Skip leading whitespace (matches isspace for ASCII)
    while i < s.len() {
        let c = s[i];
        if c == b' ' || c == b'\t' || c == b'\n' || c == b'\r' || c == 0x0b || c == 0x0c {
            i += 1;
        } else {
            break;
        }
    }

    let mut negative = false;
    if i < s.len() {
        if s[i] == b'+' {
            i += 1;
        } else if s[i] == b'-' {
            negative = true;
            i += 1;
        }
    }

    let digits_start = i;
    let mut value: i64 = 0;
    let mut overflow = false;
    while i < s.len() {
        let c = s[i];
        if !c.is_ascii_digit() {
            break;
        }
        let d = (c - b'0') as i64;
        if !overflow {
            // Detect overflow by working in i64 and checking bounds.
            let next = value.checked_mul(10).and_then(|v| {
                if negative {
                    v.checked_sub(d)
                } else {
                    v.checked_add(d)
                }
            });
            match next {
                Some(v) => value = v,
                None => {
                    overflow = true;
                    value = if negative { i64::MIN } else { i64::MAX };
                }
            }
        }
        i += 1;
    }

    if i == digits_start {
        // No digits were consumed; C's strtol treats this as "no conversion"
        // and sets endptr to the original string (consumed = 0).
        (0, 0)
    } else {
        (value, i)
    }
}

fn static_sum(state: &mut i32, update: i32) -> i32 {
    *state = state.wrapping_add(update);
    *state
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    let argc = args.len();

    if argc != 2 {
        println!("Error: should only be a single (integer) argument!");
        return ExitCode::from(1);
    }

    let arg_bytes = args[1].as_bytes();
    let (parsed, consumed) = strtol_base10(arg_bytes);
    if consumed == 0 {
        // end == argv[1] in C terms -> nothing was parsed
        println!("Error: first argument must be an integer!");
        return ExitCode::from(1);
    }

    // Truncate long -> int as in C (implementation-defined but typically wraps).
    let stride: i32 = parsed as i32;

    let mut sum: i32 = 0;
    for i in 0..10i32 {
        let update = i.wrapping_mul(stride);
        let s = static_sum(&mut sum, update);
        println!("{}", s);
    }

    ExitCode::from(0)
}
