// Rust translation of c_src/src/main.c (StaticAlias driver).
//
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

use std::ffi::OsString;
use std::io::Write;

/// Which object the `running_sum` pointer currently aliases.
///
/// The C code juggles a pointer that points either at `main`'s local
/// `initial_value` or at the function-local `static int inner`.  Modelling the
/// two possible targets explicitly keeps the Rust side safe while reproducing
/// the aliasing behaviour exactly.
#[derive(Copy, Clone, PartialEq, Eq)]
enum Target {
    /// `&initial_value` in `main`
    Outer,
    /// `&inner`, the `static` inside `static_alias`
    Inner,
}

/// State of the two `int` objects the pointer can refer to.
struct Cells {
    /// `main`'s `initial_value`
    outer: i32,
    /// `static int inner = 1;` inside `static_alias`
    inner: i32,
}

impl Cells {
    fn load(&self, t: Target) -> i32 {
        match t {
            Target::Outer => self.outer,
            Target::Inner => self.inner,
        }
    }

    fn store(&mut self, t: Target, v: i32) {
        match t {
            Target::Outer => self.outer = v,
            Target::Inner => self.inner = v,
        }
    }
}

/// ```c
/// int* static_alias(int *outer) {
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
/// `outer` is the target the incoming pointer aliases; the returned `Target`
/// is the target of the returned pointer.  Signed overflow (UB in C) is
/// reproduced as the two's-complement wraparound that real compilers emit.
fn static_alias(cells: &mut Cells, outer: Target) -> Target {
    let outer_val = cells.load(outer);
    if outer_val >= cells.inner {
        cells.inner = cells.inner.wrapping_add(outer_val);
        Target::Inner
    } else {
        cells.store(outer, outer_val.wrapping_add(cells.inner));
        outer
    }
}

fn is_c_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | b'\x0b' | b'\x0c' | b'\r')
}

/// `strtol(nptr, &end, 10)` for the subset of behaviour this program can hit.
///
/// Returns `(value, consumed)` where `consumed` is the offset of the `end`
/// pointer relative to the start of the string; `consumed == 0` mirrors C's
/// "`end` is set to the start of the string if nothing was parsed".
/// Out-of-range results saturate to `LONG_MIN` / `LONG_MAX`, as C requires.
fn strtol_base10(s: &[u8]) -> (i64, usize) {
    let mut i = 0usize;

    // Skip leading whitespace.
    while i < s.len() && is_c_space(s[i]) {
        i += 1;
    }

    // Optional sign.
    let negative = match s.get(i) {
        Some(&b'-') => {
            i += 1;
            true
        }
        Some(&b'+') => {
            i += 1;
            false
        }
        _ => false,
    };

    let digits_start = i;
    let mut acc: i64 = 0;
    let mut overflow = false;
    while i < s.len() && s[i].is_ascii_digit() {
        let d = i64::from(s[i] - b'0');
        if !overflow {
            // Accumulate the magnitude negatively so that LONG_MIN is
            // representable without overflowing i64.
            match acc.checked_mul(10).and_then(|v| v.checked_sub(d)) {
                Some(v) => acc = v,
                None => overflow = true,
            }
        }
        i += 1;
    }

    if i == digits_start {
        // No conversion performed: value is 0 and end == nptr.
        return (0, 0);
    }

    let value = if overflow {
        if negative {
            i64::MIN
        } else {
            i64::MAX
        }
    } else if negative {
        acc
    } else {
        // `acc` holds the negated magnitude; -LONG_MIN would overflow, which
        // can only happen when the magnitude is exactly LONG_MIN's, i.e. the
        // positive value is out of range and must saturate to LONG_MAX.
        match acc.checked_neg() {
            Some(v) => v,
            None => i64::MAX,
        }
    };

    (value, i)
}

fn arg_bytes(arg: &OsString) -> Vec<u8> {
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

fn main() {
    let argv: Vec<Vec<u8>> = std::env::args_os().map(|a| arg_bytes(&a)).collect();
    let argc = argv.len();

    let stdout = std::io::stdout();
    let mut out = std::io::BufWriter::new(stdout.lock());

    if argc != 3 {
        let _ = out.write_all(b"Error: should only be two (integer) arguments!\n");
        let _ = out.flush();
        std::process::exit(1);
    }

    // int initial_value = strtol(argv[1], &end, 10);
    let (v1, consumed1) = strtol_base10(&argv[1]);
    if consumed1 == 0 {
        // end is set to start of string if nothing parsed
        let _ = out.write_all(b"Error: first argument must be an integer!\n");
        let _ = out.flush();
        std::process::exit(1);
    }
    let initial_value = v1 as i32; // long -> int truncation

    // int iterations = strtol(argv[2], &end, 10);
    let (v2, consumed2) = strtol_base10(&argv[2]);
    if consumed2 == 0 {
        // end is set to start of string if nothing parsed
        let _ = out.write_all(b"Error: second argument must be an integer!\n");
        let _ = out.flush();
        std::process::exit(1);
    }
    let iterations = v2 as i32; // long -> int truncation

    let mut cells = Cells {
        outer: initial_value,
        inner: 1,
    };
    let mut running_sum = Target::Outer;

    let mut i: i32 = 0;
    while i < iterations {
        running_sum = static_alias(&mut cells, running_sum);
        let _ = writeln!(out, "{}", cells.load(running_sum));
        i = i.wrapping_add(1);
    }

    let _ = out.flush();
    std::process::exit(0);
}
