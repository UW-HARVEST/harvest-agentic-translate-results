// Rust translation of c_src/src/main.c
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

use std::io::{Read, Write};

/// typedef struct {
///     int floors;
///     int bedrooms;
///     double bathrooms;
/// } house_t;
///
/// On the x86-64 SysV ABI (the platform the C targets) this lays out as:
///   offset 0..4   -> floors    (int,    4 bytes, little endian)
///   offset 4..8   -> bedrooms  (int,    4 bytes, little endian)
///   offset 8..16  -> bathrooms (double, 8 bytes, little endian)
/// sizeof(house_t) == 16, with no padding bytes.
struct HouseT {
    floors: i32,
    bedrooms: i32,
    bathrooms: f64,
}

const SIZEOF_HOUSE_T: usize = 16;

impl HouseT {
    /// `house_t house = {0};` -- every byte of the object starts out zero.
    fn zeroed() -> Self {
        HouseT {
            floors: 0,
            bedrooms: 0,
            bathrooms: 0.0,
        }
    }

    /// Equivalent of `memcpy(raw, &house, sizeof(house))`: reproduce the exact
    /// in-memory byte image of the struct.
    fn to_raw_bytes(&self) -> [u8; SIZEOF_HOUSE_T] {
        let mut raw = [0u8; SIZEOF_HOUSE_T];
        raw[0..4].copy_from_slice(&self.floors.to_le_bytes());
        raw[4..8].copy_from_slice(&self.bedrooms.to_le_bytes());
        raw[8..16].copy_from_slice(&self.bathrooms.to_le_bytes());
        raw
    }
}

/// static void print_hex(unsigned char *p, int len)
fn print_hex(p: &[u8], len: i32) {
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    let mut buf = String::new();
    let mut i: i32 = 0;
    while i < len {
        // printf("%02x", p[i]);
        buf.push_str(&format!("{:02x}", p[i as usize]));
        i += 1;
    }
    // printf("\n");
    buf.push('\n');
    let _ = out.write_all(buf.as_bytes());
    let _ = out.flush();
}

/// void driver(int floors)
fn driver(floors: i32) {
    let mut house = HouseT::zeroed();
    house.floors = floors;
    house.bedrooms = 3;
    house.bathrooms = 2.0;
    let raw: [u8; SIZEOF_HOUSE_T] = house.to_raw_bytes();
    print_hex(&raw, raw.len() as i32);
}

/// Emulation of `scanf("%d", &x)`.
///
/// Skips leading whitespace (including newlines), then accepts an optional
/// sign followed by one or more decimal digits.  glibc accumulates the value
/// into a `long` (saturating at LONG_MIN/LONG_MAX like strtol) and then stores
/// the truncated low 32 bits into the `int` destination.  Returns `None` when
/// the conversion fails (matching-failure or EOF), in which case the caller
/// leaves its variable untouched, exactly as C does.
fn scanf_d(input: &[u8], pos: &mut usize) -> Option<i32> {
    // Skip whitespace as isspace() does.
    while *pos < input.len() {
        match input[*pos] {
            b' ' | b'\t' | b'\n' | b'\x0b' | b'\x0c' | b'\r' => *pos += 1,
            _ => break,
        }
    }

    let start = *pos;
    let mut negative = false;
    if *pos < input.len() && (input[*pos] == b'+' || input[*pos] == b'-') {
        negative = input[*pos] == b'-';
        *pos += 1;
    }

    let digits_start = *pos;
    let mut acc: i64 = 0;
    let mut saturated = false;
    while *pos < input.len() && input[*pos].is_ascii_digit() {
        let digit = i64::from(input[*pos] - b'0');
        if !saturated {
            match acc.checked_mul(10).and_then(|v| v.checked_add(digit)) {
                Some(v) => acc = v,
                None => saturated = true,
            }
        }
        *pos += 1;
    }

    if *pos == digits_start {
        // No digits consumed: matching failure, nothing is stored.
        *pos = start;
        return None;
    }

    let value: i64 = if saturated {
        if negative {
            i64::MIN
        } else {
            i64::MAX
        }
    } else if negative {
        acc.wrapping_neg()
    } else {
        acc
    };

    // Truncation of long -> int.
    Some(value as i32)
}

/// The Rust standard library sets `SIGPIPE` to `SIG_IGN` before `main` runs,
/// so a Rust program whose stdout is a closed pipe merely gets `EPIPE` from
/// `write` and exits 0.  A C program keeps the default disposition and is
/// therefore *killed* by `SIGPIPE` (shells report status 141).  Restore the
/// default so the exit status matches the C for `driver | closed-pipe`.
#[cfg(unix)]
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

#[cfg(not(unix))]
fn restore_default_sigpipe() {}

/// int main()
fn main() {
    restore_default_sigpipe();

    let mut x: i32 = 0;

    let mut input = Vec::new();
    let _ = std::io::stdin().read_to_end(&mut input);
    let mut pos = 0usize;
    if let Some(v) = scanf_d(&input, &mut pos) {
        x = v;
    }

    driver(x);
}
