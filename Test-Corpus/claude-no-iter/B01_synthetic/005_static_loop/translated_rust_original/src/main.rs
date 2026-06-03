use std::process::ExitCode;
use std::sync::atomic::{AtomicI32, Ordering};

static SUM: AtomicI32 = AtomicI32::new(0);

fn static_sum(update: i32) -> i32 {
    // Mimic C `static int sum = 0; sum += update; return sum;`
    // Use wrapping add to mirror C's two's complement int arithmetic.
    let prev = SUM.load(Ordering::Relaxed);
    let new = prev.wrapping_add(update);
    SUM.store(new, Ordering::Relaxed);
    new
}

/// Mimic C's `strtol(s, &end, 10)` behavior closely enough for this program:
/// - Skip leading whitespace
/// - Optional '+' or '-' sign
/// - Consume decimal digits
/// - Returns the parsed value (clamped to long range; we use i64 here) and
///   the number of bytes consumed from the input. If no digits are consumed,
///   `consumed` is 0 (matching `end == argv[1]` test in the C code).
fn c_strtol(s: &[u8]) -> (i64, usize) {
    let mut i = 0usize;
    // Skip leading whitespace (C isspace: ' ', \t, \n, \v, \f, \r)
    while i < s.len() {
        match s[i] {
            b' ' | b'\t' | b'\n' | 0x0B | 0x0C | b'\r' => i += 1,
            _ => break,
        }
    }

    let sign_start = i;
    let negative = if i < s.len() && (s[i] == b'+' || s[i] == b'-') {
        let neg = s[i] == b'-';
        i += 1;
        neg
    } else {
        false
    };

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
            match value.checked_mul(10).and_then(|v| {
                if negative {
                    v.checked_sub(d)
                } else {
                    v.checked_add(d)
                }
            }) {
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
        // No digits parsed: per C strtol, endptr is set to the original
        // string (not past the sign). Report 0 consumed bytes so the
        // C-style `end == argv[1]` check works.
        let _ = sign_start;
        (0, 0)
    } else {
        (value, i)
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    let argc = args.len();

    if argc != 2 {
        println!("Error: should only be a single (integer) argument!");
        return ExitCode::from(1);
    }

    let arg = args[1].as_bytes();
    let (parsed, consumed) = c_strtol(arg);
    if consumed == 0 {
        // end is set to start of string if nothing parsed
        println!("Error: first argument must be an integer!");
        return ExitCode::from(1);
    }

    // Mirror C's `int stride = strtol(...)` truncation to 32-bit int.
    let stride: i32 = parsed as i32;

    for i in 0..10i32 {
        let update = i.wrapping_mul(stride);
        println!("{}", static_sum(update));
    }

    ExitCode::from(0)
}
