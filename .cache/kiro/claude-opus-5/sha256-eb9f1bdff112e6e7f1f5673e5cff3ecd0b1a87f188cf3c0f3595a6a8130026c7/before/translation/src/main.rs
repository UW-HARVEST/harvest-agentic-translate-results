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
//! The program seeds glibc's `rand()`, fills a 1 MiB array of `int`s, mangles
//! every element with a fixed amount of wrapping integer arithmetic, and prints
//! the XOR of the whole array. Output must be byte identical to the C original,
//! so both `strtoul` validation and the glibc PRNG are reimplemented faithfully.

use std::io::Write;
use std::os::unix::ffi::OsStrExt;

const ARRAY_SIZE: usize = 256 * 1024; // 1MB assuming sizeof(int) = 4
const ITERATIONS: usize = 2000;

// ---------------------------------------------------------------------------
// glibc `rand()` / `srand()`
// ---------------------------------------------------------------------------

/// Reimplementation of glibc's default `random()` generator (TYPE_3: a lagged
/// Fibonacci / additive-feedback generator with degree 31 and separation 3),
/// which is what `rand()` forwards to. `rand()` returns `state[f] + state[r]`
/// with the lowest bit chucked.
struct GlibcRand {
    state: [i32; GlibcRand::DEG],
    fptr: usize,
    rptr: usize,
}

impl GlibcRand {
    const DEG: usize = 31;
    const SEP: usize = 3;

    /// Equivalent of `srand(seed)` (`__srandom_r`).
    fn new(seed: u32) -> Self {
        // "We must make sure the seed is not 0. Take arbitrarily 1 in this case."
        let seed = if seed == 0 { 1 } else { seed };

        let mut state = [0i32; Self::DEG];
        state[0] = seed as i32;

        // state[i] = (16807 * state[i - 1]) % 2147483647, computed with
        // Schrage's trick. glibc keeps `word` in an int32_t while the
        // intermediate products are 64 bit, so the store back to `word`
        // truncates.
        let mut word = seed as i32;
        for slot in state.iter_mut().skip(1) {
            let hi = (word as i64) / 127773;
            let lo = (word as i64) % 127773;
            let mut next = (16807i64 * lo - 2836i64 * hi) as i32;
            if next < 0 {
                next = next.wrapping_add(2147483647);
            }
            *slot = next;
            word = next;
        }

        let mut rng = GlibcRand {
            state,
            fptr: Self::SEP,
            rptr: 0,
        };

        // glibc discards `rand_deg * 10` outputs to warm the state up.
        for _ in 0..(Self::DEG * 10) {
            rng.rand();
        }
        rng
    }

    /// Equivalent of `rand()` (`__random_r`).
    fn rand(&mut self) -> i32 {
        let sum = (self.state[self.fptr] as u32).wrapping_add(self.state[self.rptr] as u32);
        self.state[self.fptr] = sum as i32;
        let result = (sum >> 1) as i32; // chuck the least random bit

        self.fptr += 1;
        if self.fptr >= Self::DEG {
            self.fptr = 0;
        }
        self.rptr += 1;
        if self.rptr >= Self::DEG {
            self.rptr = 0;
        }

        result
    }
}

// ---------------------------------------------------------------------------
// `strtoul` emulation
// ---------------------------------------------------------------------------

struct StrToUl {
    value: u64,
    /// Index of the byte `endptr` would point at.
    end: usize,
    /// Whether `errno` would have been set to `ERANGE`.
    erange: bool,
}

/// Base-10 `strtoul` for a NUL-terminated byte string, matching glibc:
/// leading whitespace is skipped, an optional sign is honoured (a negated
/// result wraps modulo 2^64), an out-of-range magnitude yields `ULONG_MAX`
/// plus `ERANGE`, and when no digits are consumed the end pointer is reset to
/// the start of the input without touching `errno`.
fn strtoul(s: &[u8]) -> StrToUl {
    let mut i = 0usize;

    while i < s.len() && matches!(s[i], b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r') {
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
        // No conversion performed: endptr is restored to the original string.
        return StrToUl {
            value: 0,
            end: 0,
            erange: false,
        };
    }

    if overflow {
        return StrToUl {
            value: u64::MAX,
            end: i,
            erange: true,
        };
    }

    StrToUl {
        value: if negative { value.wrapping_neg() } else { value },
        end: i,
        erange: false,
    }
}

// ---------------------------------------------------------------------------
// The workload
// ---------------------------------------------------------------------------

/// One pass of the inner 100-step mangling for a single element.
///
/// All arithmetic wraps and the shifts/divisions keep C's semantics:
/// `>>` on a signed value is arithmetic, `/` truncates toward zero and `%`
/// takes the sign of the dividend.
#[inline(always)]
fn mangle(mut x: i32) -> i32 {
    for _ in 0..100 {
        x = x.wrapping_mul(3).wrapping_add(7);
        x ^= x >> 3;
        x = x.wrapping_sub(x.wrapping_shl(1));
        x = (x / 2).wrapping_add(x % 7);
    }
    x
}

/// Perform expensive arithmetic on each element.
///
/// Elements are independent, so they are handled in small chunks purely to give
/// the CPU some instruction-level parallelism across the otherwise serial
/// dependency chains. The arithmetic performed per element is unchanged.
fn perform_expensive_operations(array: &mut [i32]) {
    const LANES: usize = 8;
    let mut chunks = array.chunks_exact_mut(LANES);
    for chunk in &mut chunks {
        let mut x = [0i32; LANES];
        x.copy_from_slice(chunk);
        for _ in 0..100 {
            for v in x.iter_mut() {
                let mut t = v.wrapping_mul(3).wrapping_add(7);
                t ^= t >> 3;
                t = t.wrapping_sub(t.wrapping_shl(1));
                *v = (t / 2).wrapping_add(t % 7);
            }
        }
        chunk.copy_from_slice(&x);
    }
    for slot in chunks.into_remainder() {
        *slot = mangle(*slot);
    }
}

fn main() {
    let argv: Vec<Vec<u8>> = std::env::args_os()
        .map(|a| {
            let mut bytes = a.as_bytes().to_vec();
            bytes.push(0); // emulate C's NUL terminator
            bytes
        })
        .collect();
    let argc = argv.len();
    let program = argv
        .first()
        .map(|a| &a[..a.len() - 1])
        .unwrap_or(b"" as &[u8]);

    if argc != 2 {
        let mut stderr = std::io::stderr();
        let _ = stderr.write_all(b"Usage: ");
        let _ = stderr.write_all(program);
        let _ = stderr.write_all(b" <seed>\n");
        std::process::exit(1);
    }

    let arg = &argv[1];
    let parsed = strtoul(arg);
    // Same short-circuited order of checks as the C original.
    if arg[parsed.end] != 0 || parsed.erange || parsed.value > u64::from(u32::MAX) {
        let mut stderr = std::io::stderr();
        let _ = stderr.write_all(b"Invalid seed: '");
        let _ = stderr.write_all(&arg[..arg.len() - 1]);
        let _ = stderr.write_all(b"'\n");
        std::process::exit(1);
    }

    let seed = parsed.value as u32;
    let mut rng = GlibcRand::new(seed);

    let mut array = vec![0i32; ARRAY_SIZE];
    for slot in array.iter_mut() {
        *slot = rng.rand();
    }

    for _ in 0..ITERATIONS {
        perform_expensive_operations(&mut array);
    }

    let mut xor_result: i32 = 0;
    for &v in array.iter() {
        xor_result ^= v;
    }

    let mut stdout = std::io::stdout();
    let _ = writeln!(stdout, "{}", xor_result);
    let _ = stdout.flush();
}
