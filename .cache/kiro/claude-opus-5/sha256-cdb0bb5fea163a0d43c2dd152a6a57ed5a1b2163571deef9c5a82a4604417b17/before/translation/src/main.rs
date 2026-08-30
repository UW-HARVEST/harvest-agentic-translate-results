// Rust translation of c_src/src/main.c (StaticAlias `driver`).
//
// Behavior is reproduced byte-for-byte, including the quirks of the original:
//   * all messages go to stdout (the C code uses printf, not fprintf(stderr, ...))
//   * `strtol` results are truncated to `int` (gcc/clang wrap-around semantics)
//   * signed overflow inside the accumulator wraps instead of aborting
//   * once the returned pointer aliases the `static` variable, `outer` and
//     `inner` are the *same* object, so the sum starts doubling forever
//
// The original relies on two aliasing raw pointers into (a) an automatic
// variable in `main` and (b) a function-local `static`. Instead of using raw
// pointers, the two objects are kept as plain locals and the "pointer" is
// modelled as a tag saying which of them is currently referenced. This is
// observationally identical while staying in safe Rust.

use std::io::{self, Write};
use std::os::unix::ffi::OsStrExt;

/// Which object `running_sum` currently points at.
#[derive(Copy, Clone, PartialEq, Eq)]
enum Ref {
    /// `&initial_value` (the automatic variable in `main`)
    Outer,
    /// `&inner` (the function-local `static`)
    Inner,
}

/// Mirrors the C translation unit's mutable state:
/// `initial_value` in `main` and `static int inner = 1;` in `static_alias`.
struct State {
    initial_value: i32,
    inner: i32,
}

impl State {
    /// int *static_alias(int *outer)
    ///
    /// `current` identifies the object `outer` points to; the return value
    /// identifies the object the returned pointer points to.
    fn static_alias(&mut self, current: Ref) -> Ref {
        match current {
            // `outer` and `inner` are distinct objects.
            Ref::Outer => {
                if self.initial_value >= self.inner {
                    // inner += *outer;  return &inner;
                    self.inner = self.inner.wrapping_add(self.initial_value);
                    Ref::Inner
                } else {
                    // *outer += inner;  return outer;
                    self.initial_value = self.initial_value.wrapping_add(self.inner);
                    Ref::Outer
                }
            }
            // `outer` aliases `inner`, so `*outer >= inner` compares the
            // variable with itself and is always true: inner += inner.
            Ref::Inner => {
                self.inner = self.inner.wrapping_add(self.inner);
                Ref::Inner
            }
        }
    }
}

/// True for the characters `isspace` accepts in the "C" locale.
fn is_c_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | b'\x0b' | b'\x0c' | b'\r')
}

/// `strtol(nptr, &end, 10)` for a NUL-free byte string.
///
/// Returns the converted `long` value together with the offset that `end`
/// would be set to. An offset of 0 means no conversion was performed (C sets
/// `*endptr == nptr` in that case), which is exactly what the caller tests.
/// Out-of-range conversions saturate to `LONG_MAX` / `LONG_MIN` like the libc
/// function does, so the subsequent narrowing to `int` matches.
fn strtol_base10(s: &[u8]) -> (i64, usize) {
    let mut i = 0usize;

    while i < s.len() && is_c_space(s[i]) {
        i += 1;
    }

    let mut negative = false;
    if i < s.len() && (s[i] == b'+' || s[i] == b'-') {
        negative = s[i] == b'-';
        i += 1;
    }

    let digits_start = i;
    // Largest representable magnitude for the requested sign.
    let limit: u64 = if negative {
        i64::MAX as u64 + 1
    } else {
        i64::MAX as u64
    };

    let mut acc: u64 = 0;
    let mut out_of_range = false;

    while i < s.len() && s[i].is_ascii_digit() {
        if !out_of_range {
            let digit = u64::from(s[i] - b'0');
            match acc.checked_mul(10).and_then(|v| v.checked_add(digit)) {
                Some(v) if v <= limit => acc = v,
                _ => out_of_range = true,
            }
        }
        // All digits are consumed even when the value is out of range.
        i += 1;
    }

    if i == digits_start {
        // No digits: nothing was converted, endptr goes back to the start.
        return (0, 0);
    }

    let value = if out_of_range {
        if negative {
            i64::MIN
        } else {
            i64::MAX
        }
    } else if negative {
        (acc as i64).wrapping_neg()
    } else {
        acc as i64
    };

    (value, i)
}

fn main() {
    let argv: Vec<Vec<u8>> = std::env::args_os()
        .map(|a| a.as_bytes().to_vec())
        .collect();
    let argc = argv.len();

    let stdout = io::stdout();
    let mut out = stdout.lock();

    if argc != 3 {
        let _ = out.write_all(b"Error: should only be two (integer) arguments!\n");
        let _ = out.flush();
        std::process::exit(1);
    }

    // int initial_value = strtol(argv[1], &end, 10);
    let (raw_initial, end) = strtol_base10(&argv[1]);
    if end == 0 {
        // end is set to start of string if nothing parsed
        let _ = out.write_all(b"Error: first argument must be an integer!\n");
        let _ = out.flush();
        std::process::exit(1);
    }
    let initial_value = raw_initial as i32; // long -> int narrowing

    // int iterations = strtol(argv[2], &end, 10);
    let (raw_iterations, end) = strtol_base10(&argv[2]);
    if end == 0 {
        // end is set to start of string if nothing parsed
        let _ = out.write_all(b"Error: second argument must be an integer!\n");
        let _ = out.flush();
        std::process::exit(1);
    }
    let iterations = raw_iterations as i32; // long -> int narrowing

    let mut state = State {
        initial_value,
        inner: 1,
    };

    // int *running_sum = &initial_value;
    let mut running_sum = Ref::Outer;
    for _ in 0..iterations {
        running_sum = state.static_alias(running_sum);
        let value = match running_sum {
            Ref::Outer => state.initial_value,
            Ref::Inner => state.inner,
        };
        let _ = writeln!(out, "{}", value);
    }

    let _ = out.flush();
}
