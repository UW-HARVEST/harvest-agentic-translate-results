// Rust translation of c_src/src/main.c
//
// Original C:
//     typedef union { uint64_t x; double f; } raw_double_t;
//     void driver(double f) {
//         raw_double_t u = {.f = f};
//         printf("%llx %a %.4f\n", u.x, f, f);
//     }
//     int main() { double f = 0.0f; scanf("%lf", &f); driver(f); return 0; }
//
// The translation reproduces glibc's behaviour for `scanf("%lf", ...)`,
// `%llx`, `%a` and `%.4f` byte for byte, including the quirks of the
// scanf float scanner (e.g. a bare "0x" prefix is a matching failure, while
// "0x." parses as zero, and a partially spelled "infinity" fails).

use std::io::{ErrorKind, Read, Write};

// ---------------------------------------------------------------------------
// Incremental stdin reader
// ---------------------------------------------------------------------------

/// A lazily-filled view of stdin with random access by absolute index.
///
/// `scanf` stops at the first byte that cannot extend the conversion and
/// leaves the rest of the stream alone; it never waits for EOF.  Slurping all
/// of stdin up front would therefore hang on an endless producer
/// (`yes 1.5 | driver`) where the C program exits immediately, so bytes are
/// pulled in only as the scanner asks for them.
struct Input<R: Read> {
    src: R,
    buf: Vec<u8>,
    eof: bool,
}

impl<R: Read> Input<R> {
    const CHUNK: usize = 4096;

    fn new(src: R) -> Self {
        Input {
            src,
            buf: Vec::new(),
            eof: false,
        }
    }

    /// The byte at absolute index `i`, reading more of the stream if needed.
    /// `None` means the stream ended before that index.
    fn at(&mut self, i: usize) -> Option<u8> {
        while self.buf.len() <= i && !self.eof {
            let filled = self.buf.len();
            self.buf.resize(filled + Self::CHUNK, 0);
            match self.src.read(&mut self.buf[filled..]) {
                Ok(0) => {
                    self.buf.truncate(filled);
                    self.eof = true;
                }
                Ok(n) => self.buf.truncate(filled + n),
                Err(ref e) if e.kind() == ErrorKind::Interrupted => {
                    self.buf.truncate(filled);
                }
                Err(_) => {
                    // A read error ends the conversion, exactly as EOF does.
                    self.buf.truncate(filled);
                    self.eof = true;
                }
            }
        }
        self.buf.get(i).copied()
    }

    fn matches(&mut self, i: usize, byte: u8) -> bool {
        self.at(i) == Some(byte)
    }

    /// True when the byte at `i` equals `byte` after ASCII case folding.
    /// `byte` must be lower case.
    fn matches_ci(&mut self, i: usize, byte: u8) -> bool {
        self.at(i).is_some_and(|c| c | 0x20 == byte)
    }

    fn is_digit(&mut self, i: usize) -> bool {
        self.at(i).is_some_and(|c| c.is_ascii_digit())
    }

    /// Bytes already read, in `a..b`.  Only valid for indices that `at` has
    /// already returned `Some` for.
    fn seen(&self, a: usize, b: usize) -> &[u8] {
        &self.buf[a..b]
    }
}

// ---------------------------------------------------------------------------
// scanf("%lf") emulation
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

/// Length of the longest case-insensitive prefix of `word` found at `pos`.
fn ci_prefix_len<R: Read>(input: &mut Input<R>, pos: usize, word: &[u8]) -> usize {
    let mut n = 0;
    while n < word.len() && input.matches_ci(pos + n, word[n]) {
        n += 1;
    }
    n
}

fn signed(neg: bool, v: f64) -> f64 {
    if neg {
        -v
    } else {
        v
    }
}

fn quiet_nan(neg: bool) -> f64 {
    let bits: u64 = 0x7ff8_0000_0000_0000 | if neg { 1u64 << 63 } else { 0 };
    f64::from_bits(bits)
}

/// Emulates one `%lf` conversion.  `None` means the conversion failed (input
/// or matching failure), in which case the C code leaves its variable at 0.0.
fn scan_double<R: Read>(input: &mut Input<R>) -> Option<f64> {
    let mut i = 0usize;

    // %lf skips leading white space, newlines included.
    while input.at(i).is_some_and(is_c_space) {
        i += 1;
    }
    let c = input.at(i)?; // None here is an input failure (EOF)

    let mut neg = false;
    if c == b'+' || c == b'-' {
        neg = c == b'-';
        i += 1;
    }

    // "inf" / "infinity": glibc commits to the long spelling once a 4th
    // matching character shows up, so "infi".."infinit" are failures.
    let n = ci_prefix_len(input, i, b"infinity");
    if n >= 8 {
        return Some(signed(neg, f64::INFINITY));
    }
    if n == 3 {
        return Some(signed(neg, f64::INFINITY));
    }
    if n > 3 {
        return None;
    }

    // "nan", optionally followed by a parenthesised char sequence, which
    // glibc ignores; the result is always the default quiet NaN.
    if ci_prefix_len(input, i, b"nan") == 3 {
        return Some(quiet_nan(neg));
    }

    // Hexadecimal form: 0x / 0X prefix.
    if input.matches(i, b'0') && input.matches_ci(i + 1, b'x') {
        return scan_hex(input, i + 2, neg);
    }

    scan_decimal(input, i, neg)
}

fn scan_hex<R: Read>(input: &mut Input<R>, mut j: usize, neg: bool) -> Option<f64> {
    let mut digits: Vec<u32> = Vec::new();
    let mut frac_digits: i64 = 0;
    let mut seen_dot = false;
    let mut any_digit = false;

    while let Some(c) = input.at(j) {
        if c == b'.' && !seen_dot {
            seen_dot = true;
            j += 1;
            continue;
        }
        match hex_val(c) {
            Some(v) => {
                digits.push(v);
                if seen_dot {
                    frac_digits += 1;
                }
                any_digit = true;
                j += 1;
            }
            None => break,
        }
    }

    if !any_digit {
        if seen_dot {
            // glibc hands e.g. "-0x." to strtod, which converts the leading
            // "0" and stops at 'x' => signed zero, conversion succeeds.
            return Some(signed(neg, 0.0));
        }
        // Nothing but the "0x" prefix: matching failure.
        return None;
    }

    // Optional binary exponent; ignored when no digits follow it.
    let mut pexp: i64 = 0;
    if input.matches_ci(j, b'p') {
        let mut k = j + 1;
        let mut eneg = false;
        if input.matches(k, b'+') || input.matches(k, b'-') {
            eneg = input.matches(k, b'-');
            k += 1;
        }
        if input.is_digit(k) {
            let mut v: i64 = 0;
            while input.is_digit(k) {
                if v < 1 << 40 {
                    v = v * 10 + (input.at(k).unwrap() - b'0') as i64;
                }
                k += 1;
            }
            pexp = if eneg { -v } else { v };
        }
    }

    // value = mantissa * 16^(-frac_digits) * 2^pexp
    let mut m: u128 = 0;
    let mut sticky = false;
    let mut taken = 0;
    let mut extra_exp: i64 = 0;
    let mut started = false;
    for d in digits {
        if !started {
            if d == 0 {
                continue;
            }
            started = true;
        }
        if taken < 30 {
            m = (m << 4) | d as u128;
            taken += 1;
        } else {
            if d != 0 {
                sticky = true;
            }
            extra_exp += 4;
        }
    }
    if !started {
        return Some(signed(neg, 0.0));
    }

    let e2 = pexp
        .saturating_sub(frac_digits.saturating_mul(4))
        .saturating_add(extra_exp);
    Some(signed(neg, compose_f64(m, sticky, e2)))
}

/// Rounds `m * 2^e2` (plus a non-zero tail when `sticky`) to the nearest
/// double, ties to even, matching strtod.
fn compose_f64(mut m: u128, mut sticky: bool, mut e2: i64) -> f64 {
    if m == 0 {
        return 0.0;
    }

    let mut bl = (128 - m.leading_zeros()) as i64;
    if bl > 64 {
        let sh = (bl - 64) as u32;
        if m & ((1u128 << sh) - 1) != 0 {
            sticky = true;
        }
        m >>= sh;
        e2 = e2.saturating_add(sh as i64);
        bl = (128 - m.leading_zeros()) as i64;
    }

    let e = e2.saturating_add(bl - 1); // value == 1.f * 2^e
    if e > 1023 {
        return f64::INFINITY;
    }
    if e < -1080 {
        return 0.0;
    }

    // Target position of the least significant retained bit.
    let mut target = std::cmp::max(-1074i64, e - 52);
    let shift = target - e2;
    if shift > 0 {
        let sh = shift as u32;
        let rem = m & ((1u128 << sh) - 1);
        let half = 1u128 << (sh - 1);
        m >>= sh;
        let round_up = rem > half || (rem == half && (sticky || (m & 1) == 1));
        if round_up {
            m += 1;
        }
    } else if shift < 0 {
        m <<= (-shift) as u32;
    }
    if m == 0 {
        return 0.0;
    }

    let bl2 = (128 - m.leading_zeros()) as i64;
    let e_final = target + bl2 - 1;
    if e_final > 1023 {
        return f64::INFINITY;
    }
    if e_final < -1022 {
        // Subnormal: target == -1074, so m is the raw mantissa.
        return f64::from_bits(m as u64);
    }

    let sh = 53 - bl2;
    if sh > 0 {
        m <<= sh as u32;
        target -= sh;
    } else if sh < 0 {
        m >>= (-sh) as u32;
        target += -sh;
    }
    let _ = target;
    let bits = (((e_final + 1023) as u64) << 52) | ((m as u64) & 0x000f_ffff_ffff_ffff);
    f64::from_bits(bits)
}

fn scan_decimal<R: Read>(input: &mut Input<R>, i: usize, neg: bool) -> Option<f64> {
    let mut j = i;
    let mut any_digit = false;
    let mut seen_dot = false;

    while let Some(c) = input.at(j) {
        if c.is_ascii_digit() {
            any_digit = true;
            j += 1;
        } else if c == b'.' && !seen_dot {
            seen_dot = true;
            j += 1;
        } else {
            break;
        }
    }
    if !any_digit {
        return None; // matching failure
    }

    let mantissa_end = j;
    let mut end = j;
    if input.matches_ci(j, b'e') {
        let mut k = j + 1;
        if input.matches(k, b'+') || input.matches(k, b'-') {
            k += 1;
        }
        if input.is_digit(k) {
            while input.is_digit(k) {
                k += 1;
            }
            end = k;
        }
    }

    // Normalise the token into a form Rust's parser accepts ("5." => "5.0",
    // ".5" => "0.5"), then rely on its correctly rounded conversion.
    let mut tok = String::new();
    if neg {
        tok.push('-');
    }
    let mant = input.seen(i, mantissa_end);
    if mant.first() == Some(&b'.') {
        tok.push('0');
    }
    tok.push_str(std::str::from_utf8(mant).unwrap());
    if mant.last() == Some(&b'.') {
        tok.push('0');
    }
    if end > mantissa_end {
        tok.push_str(std::str::from_utf8(input.seen(mantissa_end, end)).unwrap());
    }

    match tok.parse::<f64>() {
        Ok(v) => Some(v),
        Err(_) => None,
    }
}

// ---------------------------------------------------------------------------
// printf formatting
// ---------------------------------------------------------------------------

/// glibc's "%a".
fn format_hex_float(f: f64) -> String {
    let bits = f.to_bits();
    let sign = (bits >> 63) != 0;
    let exp_field = ((bits >> 52) & 0x7ff) as i64;
    let mantissa = bits & 0x000f_ffff_ffff_ffff;

    let mut out = String::new();
    if sign {
        out.push('-');
    }

    if exp_field == 0x7ff {
        out.push_str(if mantissa == 0 { "inf" } else { "nan" });
        return out;
    }

    let leading = if exp_field == 0 { '0' } else { '1' };
    let exponent: i64 = if exp_field == 0 {
        if mantissa == 0 {
            0
        } else {
            -1022
        }
    } else {
        exp_field - 1023
    };

    out.push_str("0x");
    out.push(leading);

    let digits = format!("{:013x}", mantissa);
    let trimmed = digits.trim_end_matches('0');
    if !trimmed.is_empty() {
        out.push('.');
        out.push_str(trimmed);
    }

    out.push('p');
    if exponent < 0 {
        out.push('-');
    } else {
        out.push('+');
    }
    out.push_str(&exponent.unsigned_abs().to_string());
    out
}

/// glibc's "%.4f".
fn format_fixed4(f: f64) -> String {
    let bits = f.to_bits();
    let sign = (bits >> 63) != 0;
    if f.is_nan() {
        return if sign {
            "-nan".to_string()
        } else {
            "nan".to_string()
        };
    }
    if f.is_infinite() {
        return if sign {
            "-inf".to_string()
        } else {
            "inf".to_string()
        };
    }
    format!("{:.4}", f)
}

fn driver(f: f64, out: &mut impl Write) {
    // The union reinterprets the double's bits; "%llx" prints them unpadded.
    let x = f.to_bits();
    let _ = write!(
        out,
        "{:x} {} {}\n",
        x,
        format_hex_float(f),
        format_fixed4(f)
    );
}

/// Restores the default disposition of `SIGPIPE`.
///
/// The Rust runtime sets `SIGPIPE` to `SIG_IGN` before `main`, so a write to a
/// closed pipe would return `EPIPE` and this program would exit 0.  The C
/// program has the default disposition and dies from the signal instead
/// (`$? == 141`), so the signal handling has to be put back to match.
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

fn main() {
    restore_default_sigpipe();

    // `scanf` pulls from the stream only as far as the conversion needs, so
    // the reader is lazy: an endless producer must not stop this program from
    // terminating.
    let stdin = std::io::stdin();
    let mut input = Input::new(stdin.lock());

    let mut f: f64 = 0.0;
    if let Some(v) = scan_double(&mut input) {
        f = v;
    }

    let stdout = std::io::stdout();
    let mut lock = stdout.lock();
    driver(f, &mut lock);
    let _ = lock.flush();
}
