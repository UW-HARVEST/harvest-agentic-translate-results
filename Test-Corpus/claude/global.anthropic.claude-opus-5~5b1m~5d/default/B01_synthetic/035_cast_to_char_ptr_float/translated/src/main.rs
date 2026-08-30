// Rust translation of c_src/src/main.c
//
// Original C:
//   static void print_hex(unsigned char *p, int len) {
//       for (int i = 0; i < len; i++) printf("%02x", p[i]);
//       printf("\n");
//   }
//   void driver(float x) { print_hex((unsigned char *)&x, sizeof(x)); }
//   int main() { float x = 0.f; scanf("%f", &x); driver(x); return 0; }
//
// The program reads one float with scanf("%f") (leaving x == 0.0f on matching
// failure or EOF) and dumps the raw object representation of the float in
// memory order as lowercase hex, followed by a newline.

use std::io::{self, Read, Write};

/// A byte reader over stdin with a single byte of push-back, mimicking the
/// character-at-a-time consumption performed by scanf.
struct Reader {
    inner: io::Stdin,
    buf: [u8; 1],
    pending: Option<u8>,
    eof: bool,
}

impl Reader {
    fn new() -> Self {
        Reader {
            inner: io::stdin(),
            buf: [0u8; 1],
            pending: None,
            eof: false,
        }
    }

    fn peek(&mut self) -> Option<u8> {
        if let Some(c) = self.pending {
            return Some(c);
        }
        if self.eof {
            return None;
        }
        match self.inner.read(&mut self.buf) {
            Ok(0) => {
                self.eof = true;
                None
            }
            Ok(_) => {
                self.pending = Some(self.buf[0]);
                self.pending
            }
            Err(_) => {
                self.eof = true;
                None
            }
        }
    }

    fn next_byte(&mut self) -> Option<u8> {
        let c = self.peek();
        if c.is_some() {
            self.pending = None;
        }
        c
    }

    /// Consume the next byte if it equals (case-insensitively) `want`.
    fn eat_ci(&mut self, want: u8) -> bool {
        match self.peek() {
            Some(c) if c.to_ascii_lowercase() == want.to_ascii_lowercase() => {
                self.pending = None;
                true
            }
            _ => false,
        }
    }
}

fn is_c_space(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

fn hex_val(c: u8) -> Option<u32> {
    match c {
        b'0'..=b'9' => Some((c - b'0') as u32),
        b'a'..=b'f' => Some((c - b'a') as u32 + 10),
        b'A'..=b'F' => Some((c - b'A') as u32 + 10),
        _ => None,
    }
}

/// Build an f32 from `m * 2^e` (plus a sticky bit indicating that further
/// non-zero, less significant bits were discarded), with round-to-nearest-even.
fn assemble(m: u128, sticky: bool, e: i32, neg: bool) -> f32 {
    let sign = if neg { -1.0f64 } else { 1.0f64 };
    if m == 0 {
        // A sticky-only value is smaller than the smallest subnormal.
        let v = if sticky { 1.0e-60f64 * 1.0e-60f64 * 1.0e-60f64 } else { 0.0f64 };
        return (sign * v) as f32;
    }
    let bits = 128 - m.leading_zeros() as i32; // bit length of m
    let top_exp = e + bits - 1; // exponent of the most significant bit
    if top_exp > 200 {
        return (sign * f64::INFINITY) as f32;
    }
    if top_exp < -200 {
        // Far below the smallest subnormal.
        return (sign * 0.0f64) as f32;
    }
    // Quantum exponent: 24 significand bits, but never finer than 2^-149.
    let q = std::cmp::max(e + bits - 24, -149);
    let shift = q - e;
    let rounded: u128 = if shift <= 0 {
        m << ((-shift) as u32)
    } else if shift >= 128 {
        // Everything is shifted out; only rounding information remains.
        let half_pos = shift - 1;
        let half_bit = if half_pos < 128 { (m >> (half_pos as u32)) & 1 } else { 0 };
        let rest = if half_pos >= 128 {
            true
        } else {
            sticky || (m & ((1u128 << (half_pos as u32)) - 1)) != 0
        };
        if half_bit == 1 && rest {
            1
        } else {
            0
        }
    } else {
        let s = shift as u32;
        let q_val = m >> s;
        let mask = (1u128 << s) - 1;
        let rem = m & mask;
        let half = 1u128 << (s - 1);
        if rem > half || (rem == half && (sticky || (q_val & 1) == 1)) {
            q_val + 1
        } else {
            q_val
        }
    };
    if rounded == 0 {
        return (sign * 0.0f64) as f32;
    }
    // `rounded` has at most 25 bits and is exactly representable in f64, and
    // 2^q is exactly representable in f64 for q in [-149, 200].
    let scale = exp2_f64(q);
    (sign * (rounded as f64) * scale) as f32
}

fn exp2_f64(q: i32) -> f64 {
    // Exact power of two as f64 for q within f64's normal/subnormal range.
    if q >= -1022 && q <= 1023 {
        f64::from_bits((((q + 1023) as u64) & 0x7ff) << 52)
    } else if q > 1023 {
        f64::INFINITY
    } else {
        let mut v = f64::from_bits(1); // 2^-1074
        let mut k = q + 1074;
        if k < 0 {
            return 0.0;
        }
        while k > 0 {
            let step = std::cmp::min(k, 1000);
            v *= exp2_f64(step);
            k -= step;
        }
        v
    }
}

/// Parse a float the way scanf("%f") / strtof does. Returns None on matching
/// failure (in which case the C code leaves x untouched).
fn scan_float(rd: &mut Reader) -> Option<f32> {
    // Skip leading whitespace.
    loop {
        match rd.peek() {
            Some(c) if is_c_space(c) => {
                rd.next_byte();
            }
            Some(_) => break,
            None => return None,
        }
    }

    let mut neg = false;
    match rd.peek() {
        Some(b'+') => {
            rd.next_byte();
        }
        Some(b'-') => {
            rd.next_byte();
            neg = true;
        }
        _ => {}
    }

    match rd.peek().map(|c| c.to_ascii_lowercase()) {
        Some(b'i') => {
            // inf / infinity
            for want in b"inf" {
                if !rd.eat_ci(*want) {
                    return None;
                }
            }
            // If an 'i' follows "inf", glibc commits to matching the whole
            // "infinity" spelling; a partial match is a matching failure.
            if let Some(c) = rd.peek() {
                if c.to_ascii_lowercase() == b'i' {
                    for want in b"inity" {
                        if !rd.eat_ci(*want) {
                            return None;
                        }
                    }
                }
            }
            return Some(if neg { f32::NEG_INFINITY } else { f32::INFINITY });
        }
        Some(b'n') => {
            for want in b"nan" {
                if !rd.eat_ci(*want) {
                    return None;
                }
            }
            if rd.peek() == Some(b'(') {
                rd.next_byte();
                loop {
                    match rd.next_byte() {
                        Some(b')') | None => break,
                        Some(_) => {}
                    }
                }
            }
            let nan = f32::from_bits(0x7fc0_0000);
            return Some(if neg { -nan } else { nan });
        }
        Some(_) => {}
        None => return None,
    }

    // Possible hex prefix.
    let mut leading_zero = false;
    if rd.peek() == Some(b'0') {
        rd.next_byte();
        leading_zero = true;
        if rd.peek() == Some(b'x') || rd.peek() == Some(b'X') {
            rd.next_byte();
            return Some(scan_hex(rd, neg));
        }
    }

    // Decimal form.
    let mut int_digits = String::new();
    if leading_zero {
        int_digits.push('0');
    }
    while let Some(c) = rd.peek() {
        if c.is_ascii_digit() {
            int_digits.push(c as char);
            rd.next_byte();
        } else {
            break;
        }
    }
    let mut frac_digits = String::new();
    if rd.peek() == Some(b'.') {
        rd.next_byte();
        while let Some(c) = rd.peek() {
            if c.is_ascii_digit() {
                frac_digits.push(c as char);
                rd.next_byte();
            } else {
                break;
            }
        }
    }
    if int_digits.is_empty() && frac_digits.is_empty() {
        return None;
    }

    let mut exp: i64 = 0;
    if rd.peek() == Some(b'e') || rd.peek() == Some(b'E') {
        rd.next_byte();
        let mut eneg = false;
        match rd.peek() {
            Some(b'+') => {
                rd.next_byte();
            }
            Some(b'-') => {
                rd.next_byte();
                eneg = true;
            }
            _ => {}
        }
        let mut edigits = String::new();
        while let Some(c) = rd.peek() {
            if c.is_ascii_digit() {
                edigits.push(c as char);
                rd.next_byte();
            } else {
                break;
            }
        }
        if !edigits.is_empty() {
            let mut v: i64 = 0;
            for ch in edigits.bytes() {
                v = v.saturating_mul(10).saturating_add((ch - b'0') as i64);
                if v > 1_000_000 {
                    v = 1_000_000;
                }
            }
            exp = if eneg { -v } else { v };
        }
    }

    let text = format!(
        "{}.{}e{}",
        if int_digits.is_empty() { "0" } else { &int_digits },
        if frac_digits.is_empty() { "0" } else { &frac_digits },
        exp
    );
    let mag: f32 = text.parse::<f32>().unwrap_or(0.0);
    Some(if neg { -mag } else { mag })
}

/// Parse the part of a hexadecimal float after the "0x" prefix.
fn scan_hex(rd: &mut Reader, neg: bool) -> f32 {
    let mut m: u128 = 0;
    let mut sticky = false;
    let mut exp_adj: i64 = 0; // power-of-two contribution of dropped digits
    let mut frac_count: i64 = 0;
    let mut any_digit = false;

    let push = |d: u32, m: &mut u128, sticky: &mut bool, exp_adj: &mut i64| {
        if m.leading_zeros() >= 8 {
            *m = (*m << 4) | d as u128;
        } else {
            *exp_adj += 4;
            if d != 0 {
                *sticky = true;
            }
        }
    };

    while let Some(c) = rd.peek() {
        if let Some(d) = hex_val(c) {
            rd.next_byte();
            any_digit = true;
            push(d, &mut m, &mut sticky, &mut exp_adj);
        } else {
            break;
        }
    }
    if rd.peek() == Some(b'.') {
        rd.next_byte();
        while let Some(c) = rd.peek() {
            if let Some(d) = hex_val(c) {
                rd.next_byte();
                any_digit = true;
                frac_count += 1;
                push(d, &mut m, &mut sticky, &mut exp_adj);
            } else {
                break;
            }
        }
    }
    if !any_digit {
        // "0x" with no hex digits: the leading '0' converts to zero.
        return if neg { -0.0f32 } else { 0.0f32 };
    }

    let mut pexp: i64 = 0;
    if rd.peek() == Some(b'p') || rd.peek() == Some(b'P') {
        rd.next_byte();
        let mut eneg = false;
        match rd.peek() {
            Some(b'+') => {
                rd.next_byte();
            }
            Some(b'-') => {
                rd.next_byte();
                eneg = true;
            }
            _ => {}
        }
        let mut edigits = String::new();
        while let Some(c) = rd.peek() {
            if c.is_ascii_digit() {
                edigits.push(c as char);
                rd.next_byte();
            } else {
                break;
            }
        }
        if !edigits.is_empty() {
            let mut v: i64 = 0;
            for ch in edigits.bytes() {
                v = v.saturating_mul(10).saturating_add((ch - b'0') as i64);
                if v > 1_000_000 {
                    v = 1_000_000;
                }
            }
            pexp = if eneg { -v } else { v };
        }
    }

    let total = exp_adj - 4 * frac_count + pexp;
    let e = total.clamp(-100_000, 100_000) as i32;
    assemble(m, sticky, e, neg)
}

fn print_hex(bytes: &[u8], out: &mut impl Write) {
    let mut s = String::with_capacity(bytes.len() * 2 + 1);
    for b in bytes {
        s.push_str(&format!("{:02x}", b));
    }
    s.push('\n');
    let _ = out.write_all(s.as_bytes());
}

fn driver(x: f32, out: &mut impl Write) {
    print_hex(&x.to_ne_bytes(), out);
}

fn main() {
    let mut x: f32 = 0.0;
    let mut rd = Reader::new();
    if let Some(v) = scan_float(&mut rd) {
        x = v;
    }
    let stdout = io::stdout();
    let mut lock = stdout.lock();
    driver(x, &mut lock);
    let _ = lock.flush();
}
