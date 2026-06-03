// Translation of c_src/src/main.c into Rust.
// Preserves the original program's behavior, including the use of a
// "static" mutable variable inside `static_alias` and the aliasing
// semantics where the returned pointer can be either the caller's
// `outer` pointer or a pointer to the static `inner`.

use std::env;
use std::process::ExitCode;

/// Tracks which storage location `running_sum` is currently pointing at.
#[derive(Copy, Clone, Eq, PartialEq)]
enum PointsTo {
    Outer,
    Inner,
}

/// Mirrors the C function:
///
/// ```c
/// int* static_alias(int *outer) {
///   static int inner = 1;
///   if (*outer >= inner) {
///     inner += *outer;
///     return &inner;
///   } else {
///     *outer += inner;
///     return outer;
///   }
/// }
/// ```
///
/// Because `outer` may alias `&inner` (when the caller passes the pointer
/// returned from a previous call), we explicitly thread the aliasing
/// state via `points_to` and read/write through the appropriate slot.
fn static_alias(outer: &mut i32, inner: &mut i32, points_to: PointsTo) -> PointsTo {
    // Read *outer, accounting for possible aliasing with inner.
    let outer_val: i32 = match points_to {
        PointsTo::Outer => *outer,
        PointsTo::Inner => *inner,
    };

    if outer_val >= *inner {
        // inner += *outer;
        // If outer aliases inner, this is equivalent to inner += inner.
        // Use wrapping arithmetic to match C's two's-complement behavior
        // (and to avoid debug-mode overflow panics in Rust).
        *inner = inner.wrapping_add(outer_val);
        PointsTo::Inner
    } else {
        // *outer += inner;
        match points_to {
            PointsTo::Outer => {
                *outer = outer.wrapping_add(*inner);
            }
            PointsTo::Inner => {
                // outer aliases inner: inner += inner.
                *inner = inner.wrapping_add(*inner);
            }
        }
        points_to
    }
}

/// Mimics C's `strtol(s, &end, 10)` for the subset of behavior we need:
/// - skip leading ASCII whitespace
/// - optional leading '+' or '-'
/// - consume decimal digits
/// - return the parsed value (saturating on overflow, like glibc's strtol)
///   along with the number of bytes consumed from the start of `s`.
///
/// If no conversion could be performed, the consumed count is 0 (the
/// caller treats this the same as `end == argv[i]` in the C code).
fn c_strtol(s: &[u8]) -> (i64, usize) {
    let mut i = 0usize;
    // Skip leading whitespace (matches C isspace for default locale on ASCII).
    while i < s.len() {
        match s[i] {
            b' ' | b'\t' | b'\n' | b'\x0b' | b'\x0c' | b'\r' => i += 1,
            _ => break,
        }
    }

    // Optional sign.
    let mut neg = false;
    if i < s.len() && (s[i] == b'+' || s[i] == b'-') {
        neg = s[i] == b'-';
        i += 1;
    }

    // Digits.
    let digit_start = i;
    let mut val: i64 = 0;
    let mut overflow = false;
    while i < s.len() && s[i].is_ascii_digit() {
        let d = (s[i] - b'0') as i64;
        if !overflow {
            match val.checked_mul(10).and_then(|v| v.checked_add(d)) {
                Some(v) => val = v,
                None => {
                    overflow = true;
                    val = if neg { i64::MIN } else { i64::MAX };
                }
            }
        }
        i += 1;
    }

    if i == digit_start {
        // No digits parsed -> conversion failure: end == nptr in C.
        return (0, 0);
    }

    if neg && !overflow {
        val = -val;
    }
    (val, i)
}

fn run() -> i32 {
    // Collect args as raw bytes so we can mimic strtol byte-by-byte.
    // env::args_os gives OsStrings; on Unix we can take their bytes.
    let args: Vec<Vec<u8>> = env::args_os()
        .map(|a| os_string_to_bytes(a))
        .collect();
    let argc = args.len();

    if argc != 3 {
        println!("Error: should only be two (integer) arguments!");
        return 1;
    }

    let (initial_long, consumed) = c_strtol(&args[1]);
    if consumed == 0 {
        // end == argv[1]
        println!("Error: first argument must be an integer!");
        return 1;
    }
    // C truncates `long` to `int` on assignment.
    let mut initial_value: i32 = initial_long as i32;

    let (iter_long, consumed2) = c_strtol(&args[2]);
    if consumed2 == 0 {
        println!("Error: second argument must be an integer!");
        return 1;
    }
    let iterations: i32 = iter_long as i32;

    // Static `inner` from the C function -- carried explicitly here.
    let mut inner: i32 = 1;
    let mut points_to = PointsTo::Outer;

    let mut i: i32 = 0;
    while i < iterations {
        points_to = static_alias(&mut initial_value, &mut inner, points_to);
        let printed = match points_to {
            PointsTo::Outer => initial_value,
            PointsTo::Inner => inner,
        };
        println!("{}", printed);
        i = i.wrapping_add(1);
    }

    0
}

#[cfg(unix)]
fn os_string_to_bytes(s: std::ffi::OsString) -> Vec<u8> {
    use std::os::unix::ffi::OsStringExt;
    s.into_vec()
}

#[cfg(not(unix))]
fn os_string_to_bytes(s: std::ffi::OsString) -> Vec<u8> {
    s.to_string_lossy().into_owned().into_bytes()
}

fn main() -> ExitCode {
    ExitCode::from(run() as u8)
}
