// Copyright 2025 MIT Lincoln Laboratory
// Permission is hereby granted, free of charge,
// to any person obtaining a copy of this software
// and associated documentation files (the "Software"),
// to deal in the Software without restriction,
// including without limitation the rights to use, copy,
// modify, merge, publish, distribute, sublicense,
// and/or sell copies of the Software,
// and to permit persons to whom the Software is furnished to do so,
// subject to the following conditions:
//
// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
// EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
// THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
// IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE
// FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT,
// TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE
// OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
//
// Rust translation of c_src/src/main.c -- behaviour-preserving, including
// the original's quirks (no trailing-garbage rejection, integer truncation
// of strtol results, wrapping arithmetic).

use std::io::Write;

/// Identifies which of the two `int` objects a "pointer" currently designates.
///
/// The C program passes around a raw `int *` that may alias either the
/// function-local `initial_value` in `main` or the `static int inner` inside
/// `static_alias`. Rather than reproducing that with raw pointers, we model the
/// pointee identity explicitly so the aliasing effects stay observable while
/// the code remains safe.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
enum Target {
    /// `&inner` (the static inside `static_alias`)
    Inner,
    /// `&initial_value` (the local in `main`)
    Initial,
}

/// The two mutable `int` objects the program manipulates.
struct Vars {
    /// `static int inner = 1;`
    inner: i32,
    /// `int initial_value = strtol(argv[1], &end, 10);`
    initial_value: i32,
}

impl Vars {
    fn load(&self, t: Target) -> i32 {
        match t {
            Target::Inner => self.inner,
            Target::Initial => self.initial_value,
        }
    }

    fn store(&mut self, t: Target, v: i32) {
        match t {
            Target::Inner => self.inner = v,
            Target::Initial => self.initial_value = v,
        }
    }
}

/// ```c
/// int *
/// static_alias(int *outer) {
///   static int inner = 1;
///   if(*outer >= inner) {
///     inner += *outer;
///     return &inner;
///   } else {
///     *outer += inner;
///     return outer;
///   }
/// }
/// ```
///
/// `outer` names the object the incoming pointer designates; the return value
/// names the object the returned pointer designates. When `outer` is
/// `Target::Inner` the comparison `*outer >= inner` is trivially true, so the
/// `else` branch is only reachable for `Target::Initial`; it is still written
/// generically so the translation mirrors the C statement for statement.
fn static_alias(vars: &mut Vars, outer: Target) -> Target {
    let outer_val = vars.load(outer);
    if outer_val >= vars.inner {
        // inner += *outer;  (wrapping: C would be signed overflow / UB, gcc wraps)
        vars.inner = vars.inner.wrapping_add(outer_val);
        Target::Inner
    } else {
        // *outer += inner;
        let sum = outer_val.wrapping_add(vars.inner);
        vars.store(outer, sum);
        outer
    }
}

/// True for the characters `isspace` accepts in the C locale.
fn is_c_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

/// `strtol(nptr, &end, 10)`.
///
/// Returns the parsed `long` value plus the byte offset of `end` within
/// `nptr`. When no conversion is performed the offset is 0, which is how the C
/// code detects failure (`end == argv[1]`). Out-of-range input saturates to
/// `LONG_MAX` / `LONG_MIN`, as required of `strtol`.
fn strtol_base10(nptr: &[u8]) -> (i64, usize) {
    let mut i = 0usize;

    // Skip leading whitespace.
    while i < nptr.len() && is_c_space(nptr[i]) {
        i += 1;
    }

    // Optional sign.
    let mut negative = false;
    if i < nptr.len() && (nptr[i] == b'+' || nptr[i] == b'-') {
        negative = nptr[i] == b'-';
        i += 1;
    }

    let digits_start = i;
    let mut acc: u64 = 0;
    let mut overflowed = false;
    while i < nptr.len() && nptr[i].is_ascii_digit() {
        if !overflowed {
            let digit = u64::from(nptr[i] - b'0');
            match acc.checked_mul(10).and_then(|v| v.checked_add(digit)) {
                Some(v) => acc = v,
                None => overflowed = true,
            }
        }
        i += 1;
    }

    if i == digits_start {
        // No digits consumed: strtol stores nptr in *endptr and returns 0.
        return (0, 0);
    }

    // Magnitude limit for LONG_MIN is 2^63, for LONG_MAX it is 2^63 - 1.
    let limit: u64 = if negative {
        1u64 << 63
    } else {
        i64::MAX as u64
    };
    if overflowed || acc > limit {
        return (if negative { i64::MIN } else { i64::MAX }, i);
    }

    // `acc <= 2^63` here, so the negative case relies on wrapping to reach
    // LONG_MIN exactly.
    let value = if negative {
        (acc as i64).wrapping_neg()
    } else {
        acc as i64
    };
    (value, i)
}

/// Argument bytes exactly as the OS supplied them, so that non-UTF-8 arguments
/// behave the way `char **argv` would in C.
fn arg_bytes() -> Vec<Vec<u8>> {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        std::env::args_os()
            .map(|a| a.as_os_str().as_bytes().to_vec())
            .collect()
    }
    #[cfg(not(unix))]
    {
        std::env::args_os()
            .map(|a| a.to_string_lossy().into_owned().into_bytes())
            .collect()
    }
}

/// Maintain a sum leveraging multiple references to a static variable
fn main() {
    let stdout = std::io::stdout();
    let mut out = stdout.lock();

    let argv = arg_bytes();
    let argc = argv.len();

    if argc != 3 {
        let _ = write!(out, "Error: should only be two (integer) arguments!\n");
        let _ = out.flush();
        std::process::exit(1);
    }

    let (raw_initial, end) = strtol_base10(&argv[1]);
    if end == 0 {
        // end is set to start of string if nothing parsed
        let _ = write!(out, "Error: first argument must be an integer!\n");
        let _ = out.flush();
        std::process::exit(1);
    }
    // Implicit long -> int conversion (truncating, as on gcc/x86-64).
    let initial_value = raw_initial as i32;

    let (raw_iterations, end) = strtol_base10(&argv[2]);
    if end == 0 {
        // end is set to start of string if nothing parsed
        let _ = write!(out, "Error: second argument must be an integer!\n");
        let _ = out.flush();
        std::process::exit(1);
    }
    let iterations = raw_iterations as i32;

    let mut vars = Vars {
        inner: 1,
        initial_value,
    };

    let mut running_sum = Target::Initial;
    let mut i: i32 = 0;
    while i < iterations {
        running_sum = static_alias(&mut vars, running_sum);
        let _ = write!(out, "{}\n", vars.load(running_sum));
        i += 1;
    }

    let _ = out.flush();
    std::process::exit(0);
}
