// Rust translation of c_src/src/main.c (StaticAlias driver).
//
// Behavior is reproduced exactly, including:
//   * the order of the argument-count / parse validation checks,
//   * C's `strtol` semantics (leading whitespace, optional sign, saturation at
//     LONG_MAX/LONG_MIN on overflow, "end == start" when nothing is parsed),
//   * the implicit `long` -> `int` narrowing conversion (two's-complement
//     truncation, as performed by gcc/clang),
//   * wrapping `int` arithmetic,
//   * the pointer aliasing between the caller's local variable and the
//     function-local `static` variable,
//   * the exact printf output ("%d\n") and the exit statuses.

use std::io::Write;

/// Bytes of a command line argument, as the C `argv` array would see them.
fn arg_bytes(arg: &std::ffi::OsString) -> Vec<u8> {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        arg.as_os_str().as_bytes().to_vec()
    }
    #[cfg(not(unix))]
    {
        arg.to_string_lossy().into_owned().into_bytes()
    }
}

fn is_c_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

/// Faithful replica of `strtol(s, &end, 10)`.
///
/// Returns `(value, consumed)`, where `consumed` is how far `end` was advanced
/// past the start of `s`. `consumed == 0` corresponds to `end == s` in the C
/// code, i.e. no conversion could be performed.
fn c_strtol_base10(s: &[u8]) -> (i64, usize) {
    let mut i = 0usize;

    // Skip leading white space.
    while i < s.len() && is_c_space(s[i]) {
        i += 1;
    }

    // Optional sign.
    let mut negative = false;
    if i < s.len() && (s[i] == b'+' || s[i] == b'-') {
        negative = s[i] == b'-';
        i += 1;
    }

    // Digit sequence, accumulated as an exact magnitude so that the
    // LONG_MIN/LONG_MAX saturation boundaries match strtol precisely.
    let digits_start = i;
    let limit: u128 = if negative {
        u128::from(i64::MIN.unsigned_abs())
    } else {
        i64::MAX as u128
    };
    let mut acc: u128 = 0;
    let mut saturated = false;
    while i < s.len() && s[i].is_ascii_digit() {
        if !saturated {
            acc = acc * 10 + u128::from(s[i] - b'0');
            if acc > limit {
                saturated = true;
            }
        }
        i += 1;
    }

    if i == digits_start {
        // No conversion performed: strtol stores the original pointer in *endptr
        // and returns 0.
        return (0, 0);
    }

    if saturated {
        // ERANGE: strtol returns LONG_MAX / LONG_MIN.
        return (if negative { i64::MIN } else { i64::MAX }, i);
    }

    let value = if negative {
        (acc as i64).wrapping_neg()
    } else {
        acc as i64
    };
    (value, i)
}

/// `static_alias()` from the C source; `inner` is the function-local `static`.
///
/// `outer_is_inner` records the aliasing case in which the caller passed
/// `&inner` back in. Returns `true` when the returned pointer is `&inner`.
fn static_alias(inner: &mut i32, outer: &mut i32, outer_is_inner: bool) -> bool {
    if outer_is_inner {
        // `outer` and `inner` denote the same object, so `*outer >= inner` holds
        // (they are equal): `inner += *outer` doubles it and `&inner` is returned.
        *inner = inner.wrapping_add(*inner);
        return true;
    }

    if *outer >= *inner {
        *inner = inner.wrapping_add(*outer);
        true
    } else {
        *outer = outer.wrapping_add(*inner);
        false
    }
}

fn main() {
    let args: Vec<std::ffi::OsString> = std::env::args_os().collect();
    let argc = args.len();

    let stdout = std::io::stdout();
    let mut out = std::io::BufWriter::new(stdout.lock());

    if argc != 3 {
        let _ = writeln!(out, "Error: should only be two (integer) arguments!");
        let _ = out.flush();
        std::process::exit(1);
    }

    let arg1 = arg_bytes(&args[1]);
    let (raw1, consumed1) = c_strtol_base10(&arg1);
    let initial_value_parsed = raw1 as i32; // implicit long -> int narrowing
    if consumed1 == 0 {
        // end is set to start of string if nothing parsed
        let _ = writeln!(out, "Error: first argument must be an integer!");
        let _ = out.flush();
        std::process::exit(1);
    }

    let arg2 = arg_bytes(&args[2]);
    let (raw2, consumed2) = c_strtol_base10(&arg2);
    let iterations = raw2 as i32; // implicit long -> int narrowing
    if consumed2 == 0 {
        // end is set to start of string if nothing parsed
        let _ = writeln!(out, "Error: second argument must be an integer!");
        let _ = out.flush();
        std::process::exit(1);
    }

    let mut inner: i32 = 1;
    let mut initial_value: i32 = initial_value_parsed;

    // `running_sum` initially points at `initial_value`.
    let mut points_to_inner = false;

    let mut i: i32 = 0;
    while i < iterations {
        points_to_inner = if points_to_inner {
            let mut aliased = inner;
            static_alias(&mut inner, &mut aliased, true)
        } else {
            let mut local = initial_value;
            let returned_inner = static_alias(&mut inner, &mut local, false);
            initial_value = local;
            returned_inner
        };

        let current = if points_to_inner { inner } else { initial_value };
        let _ = writeln!(out, "{}", current);

        i += 1;
    }

    let _ = out.flush();
}
