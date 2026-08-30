// Rust translation of c_src/src/main.c
//
// Original C:
//     static void print_hex(unsigned char *p, int len);
//     void driver(float x) { print_hex((unsigned char *)&x, sizeof(x)); }
//     int main() { float x = 0.f; scanf("%f", &x); driver(x); return 0; }
//
// The program reads a single float with scanf("%f") and dumps the raw object
// representation of that float as lowercase hex bytes, in memory order,
// followed by a newline.
//
// Notes on fidelity:
//  * scanf("%f") skips leading whitespace (including newlines), then matches
//    the longest prefix of the input that can begin a strtof subject sequence.
//    On a matching failure (or EOF) no assignment happens, so `x` keeps its
//    initial value of 0.f and the program prints "00000000".
//  * strtof accepts decimal forms, C99 hexadecimal forms (0x1.8p3), and
//    "inf"/"infinity"/"nan"/"nan(chars)" case-insensitively, with an optional
//    leading sign. All of that is reimplemented here, including
//    round-to-nearest-even rounding of hex significands to f32.
//  * The bytes are emitted with to_ne_bytes() so the endianness of the dump
//    matches what C would produce on the same machine.

use std::io::{ErrorKind, Read, StdinLock, Write};

// ---------------------------------------------------------------------------
// Buffered stdin with arbitrary lookahead.
//
// Bytes are pulled from stdin only as far as the scanner needs to look, which
// mirrors the incremental way scanf consumes a stream.
// ---------------------------------------------------------------------------

struct Input {
    inner: StdinLock<'static>,
    buf: Vec<u8>,
    pos: usize,
    eof: bool,
}

impl Input {
    fn new() -> Self {
        Input {
            inner: std::io::stdin().lock(),
            buf: Vec::new(),
            pos: 0,
            eof: false,
        }
    }

    /// Make sure at least `need` unconsumed bytes are buffered, unless EOF.
    fn fill(&mut self, need: usize) {
        while !self.eof && self.buf.len() - self.pos < need {
            let mut b = [0u8; 1];
            match self.inner.read(&mut b) {
                Ok(0) => self.eof = true,
                Ok(_) => self.buf.push(b[0]),
                Err(ref e) if e.kind() == ErrorKind::Interrupted => continue,
                Err(_) => self.eof = true,
            }
        }
    }

    /// Look at the byte `off` positions past the current cursor.
    fn peek_at(&mut self, off: usize) -> Option<u8> {
        self.fill(off + 1);
        self.buf.get(self.pos + off).copied()
    }

    fn advance(&mut self, n: usize) {
        self.pos += n;
    }

    /// Case-insensitive ASCII match of `pat` starting `off` past the cursor.
    fn match_ci(&mut self, off: usize, pat: &[u8]) -> bool {
        for (i, want) in pat.iter().enumerate() {
            match self.peek_at(off + i) {
                Some(got) if got.to_ascii_lowercase() == want.to_ascii_lowercase() => {}
                _ => return false,
            }
        }
        true
    }
}

// ---------------------------------------------------------------------------
// Character classification (C locale)
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Assemble a correctly rounded f32 from an exact binary significand.
//
// The represented value is `m * 2^e`, plus an infinitesimal positive residue
// when `sticky` is set (bits that were dropped off the bottom of `m`).
// Rounding is round-to-nearest, ties-to-even, matching IEEE-754 / strtof.
// ---------------------------------------------------------------------------

fn compose_f32(neg: bool, m: u128, sticky: bool, e: i64) -> f32 {
    let sign_bit: u32 = if neg { 0x8000_0000 } else { 0 };

    if m == 0 {
        // Signed zero; nothing was dropped below an all-zero significand.
        return f32::from_bits(sign_bit);
    }

    let msb = 127 - m.leading_zeros() as i64; // index of the top set bit
    let value_exp = e + msb; // unbiased exponent of the value

    // Exponent of the least significant bit we are allowed to keep: 24 bits of
    // significand for normals, clamped at 2^-149 for subnormals.
    let lsb_exp = std::cmp::max(-149i64, value_exp - 23);
    let shift = lsb_exp - e;

    let (mut q, round_bit, rest_nonzero) = if shift > 0 {
        let s = shift as u32;
        let kept = if s >= 128 { 0u128 } else { m >> s };
        let round = if s - 1 >= 128 {
            false
        } else {
            (m >> (s - 1)) & 1 == 1
        };
        let below = if s - 1 == 0 {
            false
        } else if s - 1 >= 128 {
            m != 0
        } else {
            (m & ((1u128 << (s - 1)) - 1)) != 0
        };
        (kept, round, below || sticky)
    } else {
        // Shifting left is exact; `sticky` can only be set once `m` is full,
        // which forces shift > 0, so there is nothing pending here.
        (m << ((-shift) as u32), false, false)
    };

    if round_bit && (rest_nonzero || (q & 1) == 1) {
        q += 1; // may carry into an extra bit; handled below
    }

    if q == 0 {
        return f32::from_bits(sign_bit); // rounded away to signed zero
    }

    let q_bits = 128 - q.leading_zeros() as i64;
    let final_exp = lsb_exp + q_bits - 1;
    if final_exp > 127 {
        // Overflow: strtof reports ERANGE and returns HUGE_VALF.
        return f32::from_bits(sign_bit | 0x7f80_0000);
    }

    // q has at most 24 significant bits and lsb_exp is in [-149, 127], so the
    // product is exactly representable in f64 and the narrowing cast is exact.
    let two_pow = f64::from_bits(((lsb_exp + 1023) as u64) << 52);
    let magnitude = (q as f64) * two_pow;
    let bits = (magnitude as f32).to_bits();
    f32::from_bits(bits | sign_bit)
}

// ---------------------------------------------------------------------------
// scanf("%f") for a single float.
//
// Returns None on a matching failure or EOF (in which case the caller leaves
// its variable untouched, exactly like C).
// ---------------------------------------------------------------------------

fn scan_float(input: &mut Input) -> Option<f32> {
    // Leading whitespace is consumed and discarded.
    while let Some(c) = input.peek_at(0) {
        if is_c_space(c) {
            input.advance(1);
        } else {
            break;
        }
    }

    // Optional sign.
    let mut k = 0usize;
    let mut neg = false;
    match input.peek_at(0) {
        Some(b'+') => k = 1,
        Some(b'-') => {
            neg = true;
            k = 1;
        }
        _ => {}
    }

    // "inf" / "infinity".
    //
    // glibc's scanf does not back up here: once it has read "inf" and the next
    // character is another 'i', it commits to reading "infinity" and reports a
    // matching failure if the rest does not follow. So "infin" yields no
    // assignment at all, even though strtof() alone would return inf for it.
    // That quirk is reproduced deliberately.
    if input.match_ci(k, b"inf") {
        let end = if matches!(input.peek_at(k + 3), Some(b'i') | Some(b'I')) {
            if input.match_ci(k, b"infinity") {
                k + 8
            } else {
                input.advance(k + 4);
                return None; // matching failure, nothing assigned
            }
        } else {
            k + 3
        };
        input.advance(end);
        let sign_bit: u32 = if neg { 0x8000_0000 } else { 0 };
        return Some(f32::from_bits(sign_bit | 0x7f80_0000));
    }

    // "nan" / "nan(n-char-sequence)" -- the char sequence is ignored.
    if input.match_ci(k, b"nan") {
        let mut end = k + 3;
        if input.peek_at(end) == Some(b'(') {
            let mut j = end + 1;
            loop {
                match input.peek_at(j) {
                    Some(b')') => {
                        end = j + 1;
                        break;
                    }
                    Some(c) if c == b'_' || c.is_ascii_alphanumeric() => j += 1,
                    _ => break, // unterminated: subject sequence stays "nan"
                }
            }
        }
        input.advance(end);
        let sign_bit: u32 = if neg { 0x8000_0000 } else { 0 };
        return Some(f32::from_bits(sign_bit | 0x7fc0_0000));
    }

    // Hexadecimal form: 0x / 0X followed by hex digits with optional '.' and
    // optional binary exponent.
    if input.peek_at(k) == Some(b'0')
        && matches!(input.peek_at(k + 1), Some(b'x') | Some(b'X'))
    {
        let mut m: u128 = 0;
        let mut sticky = false;
        let mut exp: i64 = 0;
        let mut ndigits = 0usize;

        let mut j = k + 2;
        // Integer hex digits.
        while let Some(c) = input.peek_at(j) {
            match hex_val(c) {
                Some(d) => {
                    if m.leading_zeros() >= 4 {
                        m = (m << 4) | d as u128;
                    } else {
                        sticky |= d != 0;
                        exp += 4; // digit dropped, but the scale still grows
                    }
                    ndigits += 1;
                    j += 1;
                }
                None => break,
            }
        }
        // Optional fraction.
        let mut saw_dot = false;
        if input.peek_at(j) == Some(b'.') {
            saw_dot = true;
            let mut f = j + 1;
            while let Some(c) = input.peek_at(f) {
                match hex_val(c) {
                    Some(d) => {
                        if m.leading_zeros() >= 4 {
                            m = (m << 4) | d as u128;
                            exp -= 4;
                        } else {
                            sticky |= d != 0;
                        }
                        ndigits += 1;
                        f += 1;
                    }
                    None => break,
                }
            }
            j = f;
        }

        if ndigits == 0 {
            // glibc's scanf accumulates the token in a buffer with the sign
            // stripped off, then rejects it outright when the buffer is exactly
            // "0x". A radix point makes the buffer "0x.", which is handed to
            // strtof instead and converts the leading "0", so the sign
            // survives. Hence "-0x" prints +0 while "-0x." prints -0.
            input.advance(j);
            if saw_dot {
                let sign_bit: u32 = if neg { 0x8000_0000 } else { 0 };
                return Some(f32::from_bits(sign_bit));
            }
            return None; // matching failure, nothing assigned
        }

        {
            let mut end = j;
            if matches!(input.peek_at(j), Some(b'p') | Some(b'P')) {
                let mut e = j + 1;
                let mut esign = 1i64;
                match input.peek_at(e) {
                    Some(b'+') => e += 1,
                    Some(b'-') => {
                        esign = -1;
                        e += 1;
                    }
                    _ => {}
                }
                let mut evalue: i64 = 0;
                let mut edigits = 0usize;
                while let Some(c) = input.peek_at(e) {
                    if c.is_ascii_digit() {
                        evalue = (evalue * 10 + (c - b'0') as i64).min(1_000_000);
                        edigits += 1;
                        e += 1;
                    } else {
                        break;
                    }
                }
                if edigits > 0 {
                    exp += esign * evalue;
                    end = e;
                }
            }
            input.advance(end);
            return Some(compose_f32(neg, m, sticky, exp));
        }
    }

    // Decimal form.
    let mut j = k;
    let mut ndigits = 0usize;
    while let Some(c) = input.peek_at(j) {
        if c.is_ascii_digit() {
            ndigits += 1;
            j += 1;
        } else {
            break;
        }
    }
    if input.peek_at(j) == Some(b'.') {
        let mut f = j + 1;
        let mut frac = 0usize;
        while let Some(c) = input.peek_at(f) {
            if c.is_ascii_digit() {
                frac += 1;
                f += 1;
            } else {
                break;
            }
        }
        if ndigits + frac > 0 {
            ndigits += frac;
            j = f; // the '.' belongs to the subject sequence
        }
    }
    if ndigits == 0 {
        return None; // matching failure: nothing assigned
    }

    let mut end = j;
    if matches!(input.peek_at(j), Some(b'e') | Some(b'E')) {
        let mut e = j + 1;
        if matches!(input.peek_at(e), Some(b'+') | Some(b'-')) {
            e += 1;
        }
        let mut edigits = 0usize;
        while let Some(c) = input.peek_at(e) {
            if c.is_ascii_digit() {
                edigits += 1;
                e += 1;
            } else {
                break;
            }
        }
        if edigits > 0 {
            end = e; // exponent only counts when it has digits
        }
    }

    let mut token = String::with_capacity(end);
    for i in 0..end {
        token.push(input.peek_at(i).unwrap() as char);
    }
    input.advance(end);

    // Rust's f32 parser is correctly rounded, matching strtof for this grammar.
    Some(token.parse::<f32>().unwrap_or(0.0))
}

// ---------------------------------------------------------------------------
// Translation of the C functions
// ---------------------------------------------------------------------------

fn print_hex(p: &[u8]) {
    let mut out = String::with_capacity(p.len() * 2 + 1);
    for b in p {
        out.push_str(&format!("{:02x}", b));
    }
    out.push('\n');
    let stdout = std::io::stdout();
    let mut lock = stdout.lock();
    let _ = lock.write_all(out.as_bytes());
    let _ = lock.flush();
}

fn driver(x: f32) {
    print_hex(&x.to_ne_bytes());
}

fn main() {
    let mut x: f32 = 0.0;
    let mut input = Input::new();
    if let Some(v) = scan_float(&mut input) {
        x = v;
    }
    driver(x);
}
