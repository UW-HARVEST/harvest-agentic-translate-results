// Rust translation of c_src/src/main.c
//
// The C program reads one integer with scanf("%d", &x), stuffs it into a
// house_t struct along with two constants, memcpy's the struct into a raw
// char buffer, and dumps that buffer as lowercase hex followed by a newline.
//
// Faithfulness notes:
//   * house_t on the x86-64 SysV ABI is { int @0, int @4, double @8 }, size 16,
//     alignment 8, with no padding bytes. The struct is zero-initialized before
//     every field is assigned, so the dumped bytes are fully determined.
//   * print_hex uses "%02x" per byte, then a single "\n". No trailing spaces.
//   * scanf("%d") skips leading whitespace (including newlines), accepts an
//     optional +/- sign, then decimal digits. On a matching failure or EOF the
//     destination is left untouched, so x keeps its initial value of 0.
//   * glibc accumulates "%d" in a `long` and saturates at LONG_MAX / LONG_MIN
//     on overflow, then truncates that value to `int`. That means an input like
//     99999999999999999999 yields -1, and -99999999999999999999 yields 0.
//     This is reproduced rather than "fixed".

use std::io::{self, Read, Write};

/// Mirror of the C `house_t`, laid out as the C compiler lays it out.
#[repr(C)]
struct House {
    floors: i32,
    bedrooms: i32,
    bathrooms: f64,
}

impl House {
    /// Byte image of the struct, equivalent to `memcpy(raw, &house, sizeof(house))`
    /// on a little-endian x86-64 target.
    fn to_raw_bytes(&self) -> [u8; 16] {
        let mut raw = [0u8; 16];
        raw[0..4].copy_from_slice(&self.floors.to_le_bytes());
        raw[4..8].copy_from_slice(&self.bedrooms.to_le_bytes());
        raw[8..16].copy_from_slice(&self.bathrooms.to_le_bytes());
        raw
    }
}

/// Equivalent of `print_hex`: "%02x" for each byte, then a newline.
fn print_hex(out: &mut impl Write, p: &[u8]) {
    let mut s = String::with_capacity(p.len() * 2 + 1);
    for &b in p {
        s.push_str(&format!("{:02x}", b));
    }
    s.push('\n');
    let _ = out.write_all(s.as_bytes());
}

fn driver(out: &mut impl Write, floors: i32) {
    // house_t house = {0}; then every field is overwritten.
    let house = House {
        floors,
        bedrooms: 3,
        bathrooms: 2.0,
    };
    let raw = house.to_raw_bytes();
    print_hex(out, &raw);
}

/// Emulates `scanf("%d", &x)`.
///
/// Returns `Some(value)` on a successful conversion, or `None` on EOF or a
/// matching failure (in which case the caller must leave its variable alone,
/// exactly like C).
fn scanf_d(input: &[u8]) -> Option<i32> {
    let mut i = 0usize;

    // Skip leading whitespace, as the "%d" directive does.
    while i < input.len() && (input[i] as char).is_ascii_whitespace() {
        i += 1;
    }

    // Optional sign.
    let negative = match input.get(i) {
        Some(b'-') => {
            i += 1;
            true
        }
        Some(b'+') => {
            i += 1;
            false
        }
        _ => false,
    };

    // At least one digit is required, otherwise this is a matching failure.
    if !matches!(input.get(i), Some(c) if c.is_ascii_digit()) {
        return None;
    }

    // glibc accumulates into a `long` (i64 here) and saturates on overflow.
    let mut acc: i64 = 0;
    let mut overflow = false;
    while let Some(&c) = input.get(i) {
        if !c.is_ascii_digit() {
            break;
        }
        let digit = i64::from(c - b'0');
        if !overflow {
            match acc.checked_mul(10).and_then(|v| v.checked_add(digit)) {
                Some(v) => acc = v,
                None => overflow = true,
            }
        }
        i += 1;
    }

    let value: i64 = if overflow {
        // strtol clamps to LONG_MAX / LONG_MIN.
        if negative {
            i64::MIN
        } else {
            i64::MAX
        }
    } else if negative {
        -acc
    } else {
        acc
    };

    // The `long` result is stored through an `int *`, i.e. truncated.
    Some(value as i32)
}

fn main() {
    let mut buf = Vec::new();
    // scanf pulls from the stream as needed and ignores newline boundaries;
    // slurping stdin and scanning it is observationally equivalent here since
    // nothing else reads from stdin afterwards.
    let _ = io::stdin().read_to_end(&mut buf);

    let mut x: i32 = 0;
    if let Some(v) = scanf_d(&buf) {
        x = v;
    }

    let stdout = io::stdout();
    let mut out = stdout.lock();
    driver(&mut out, x);
    let _ = out.flush();
}
