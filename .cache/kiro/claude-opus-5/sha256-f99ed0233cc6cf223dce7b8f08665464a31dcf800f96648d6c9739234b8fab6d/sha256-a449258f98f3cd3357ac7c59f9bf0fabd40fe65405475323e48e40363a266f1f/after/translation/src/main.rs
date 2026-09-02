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

//! Rust translation of `c_src/src/main.c`.
//!
//! Behaviour is preserved bit-for-bit, including:
//!   * glibc's `srand`/`rand` (TYPE_3 additive-feedback generator) sequence,
//!   * C's `strtoul` parsing semantics (leading whitespace, sign, ERANGE),
//!   * wrapping signed arithmetic, arithmetic right shift, truncating
//!     division and C's `%` sign rules in the hot loop,
//!   * the exact stdout/stderr text and exit statuses.

use std::io::Write;

/// 1MB assuming sizeof(int) = 4
const ARRAY_SIZE: usize = 256 * 1024;
const ITERATIONS: i32 = 2000;

const UINT_MAX: u64 = u32::MAX as u64;

// ---------------------------------------------------------------------------
// glibc `srand` / `rand`
// ---------------------------------------------------------------------------

/// Re-implementation of glibc's default `random_r` generator (TYPE_3:
/// degree 31, separation 3), which is what `rand()` uses.
struct GlibcRand {
    /// The 31 `int32_t` state words.
    state: [i32; DEG],
    /// Index of the "front" pointer.
    fptr: usize,
    /// Index of the "rear" pointer.
    rptr: usize,
}

const DEG: usize = 31;
const SEP: usize = 3;

impl GlibcRand {
    /// Equivalent of `srand(seed)` (i.e. `__srandom_r`).
    fn new(seed: u32) -> Self {
        // glibc: "We must make sure the seed is not 0."
        let seed = if seed == 0 { 1 } else { seed };

        let mut state = [0i32; DEG];
        state[0] = seed as i32;

        let mut word = seed as i32;
        for i in 1..DEG {
            // state[i] = (16807 * state[i - 1]) % 2147483647, computed
            // without overflowing 31 bits (Schrage's method).
            let hi = (word / 127773) as i64;
            let lo = (word % 127773) as i64;
            let mut w = 16807 * lo - 2836 * hi;
            if w < 0 {
                w += 2147483647;
            }
            word = w as i32;
            state[i] = word;
        }

        let mut rng = GlibcRand {
            state,
            fptr: SEP,
            rptr: 0,
        };

        // Discard the first `deg * 10` outputs.
        for _ in 0..(DEG * 10) {
            rng.next_i32();
        }

        rng
    }

    /// Equivalent of `rand()` (i.e. `__random_r`).
    fn next_i32(&mut self) -> i32 {
        let val = (self.state[self.fptr] as u32).wrapping_add(self.state[self.rptr] as u32);
        self.state[self.fptr] = val as i32;

        // "Chucking least random bit."
        let result = (val >> 1) as i32;

        self.fptr += 1;
        if self.fptr >= DEG {
            self.fptr = 0;
        }
        self.rptr += 1;
        if self.rptr >= DEG {
            self.rptr = 0;
        }

        result
    }
}

// ---------------------------------------------------------------------------
// C `strtoul` (base 10)
// ---------------------------------------------------------------------------

fn is_c_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

/// Result of `strtoul(nptr, &endptr, 10)`.
struct StrToUlResult {
    value: u64,
    /// Offset of `endptr` within the input.
    end: usize,
    /// Whether `errno` was set to `ERANGE`.
    erange: bool,
}

fn strtoul_base10(s: &[u8]) -> StrToUlResult {
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
    let mut value: u64 = 0;
    let mut overflow = false;

    while i < s.len() && s[i].is_ascii_digit() {
        let digit = u64::from(s[i] - b'0');
        match value.checked_mul(10).and_then(|v| v.checked_add(digit)) {
            Some(v) => value = v,
            None => overflow = true,
        }
        i += 1;
    }

    if i == digits_start {
        // No conversion performed: endptr is set back to nptr and 0 returned.
        return StrToUlResult {
            value: 0,
            end: 0,
            erange: false,
        };
    }

    if overflow {
        // glibc returns ULONG_MAX and sets ERANGE, regardless of sign.
        return StrToUlResult {
            value: u64::MAX,
            end: i,
            erange: true,
        };
    }

    let value = if negative { value.wrapping_neg() } else { value };

    StrToUlResult {
        value,
        end: i,
        erange: false,
    }
}

// ---------------------------------------------------------------------------
// Workload
// ---------------------------------------------------------------------------

/// Perform expensive arithmetic on each element.
fn perform_expensive_operations(array: &mut [i32]) {
    for slot in array.iter_mut() {
        let mut x = *slot;
        for _ in 0..100 {
            x = x.wrapping_mul(3).wrapping_add(7);
            x ^= x >> 3;
            x = x.wrapping_sub(x.wrapping_shl(1));
            x = (x / 2).wrapping_add(x % 7);
        }
        *slot = x;
    }
}

// ---------------------------------------------------------------------------
// main
// ---------------------------------------------------------------------------

fn main() {
    restore_default_sigpipe();
    std::process::exit(run());
}

/// The Rust runtime installs `SIG_IGN` for `SIGPIPE` before `main` runs, but a
/// C program starts with the default disposition.  Without this, a vanished
/// stdout reader makes the C program die from `SIGPIPE` (wait status 141)
/// while this program's `write!` would merely fail with `EPIPE` and exit 0.
/// Restore the C behaviour so the exit status matches in that case too.
fn restore_default_sigpipe() {
    const SIGPIPE: i32 = 13;
    const SIG_DFL: usize = 0;
    extern "C" {
        fn signal(signum: i32, handler: usize) -> usize;
    }
    unsafe {
        signal(SIGPIPE, SIG_DFL);
    }
}

fn run() -> i32 {
    let argv: Vec<Vec<u8>> = argv_bytes();
    let argc = argv.len();

    if argc != 2 {
        let prog: &[u8] = argv.first().map(|a| a.as_slice()).unwrap_or(b"");
        let mut msg = Vec::new();
        msg.extend_from_slice(b"Usage: ");
        msg.extend_from_slice(prog);
        msg.extend_from_slice(b" <seed>\n");
        write_stderr(&msg);
        return 1;
    }

    let arg = &argv[1];
    let parsed = strtoul_base10(arg);
    let temp_seed = parsed.value;

    if parsed.end != arg.len() || parsed.erange || temp_seed > UINT_MAX {
        let mut msg = Vec::new();
        msg.extend_from_slice(b"Invalid seed: '");
        msg.extend_from_slice(arg);
        msg.extend_from_slice(b"'\n");
        write_stderr(&msg);
        return 1;
    }

    let seed = temp_seed as u32;
    let mut rng = GlibcRand::new(seed);

    let mut array: Vec<i32> = vec![0; ARRAY_SIZE];
    for slot in array.iter_mut() {
        *slot = rng.next_i32();
    }

    for _ in 0..ITERATIONS {
        perform_expensive_operations(&mut array);
    }

    let mut xor_result: i32 = 0;
    for &v in array.iter() {
        xor_result ^= v;
    }

    let mut out = std::io::stdout();
    let _ = write!(out, "{}\n", xor_result);
    let _ = out.flush();

    0
}

/// The raw bytes of the process arguments, mirroring C's `argv`.
fn argv_bytes() -> Vec<Vec<u8>> {
    use std::os::unix::ffi::OsStrExt;
    std::env::args_os()
        .map(|a| a.as_os_str().as_bytes().to_vec())
        .collect()
}

fn write_stderr(bytes: &[u8]) {
    let mut err = std::io::stderr();
    let _ = err.write_all(bytes);
    let _ = err.flush();
}
