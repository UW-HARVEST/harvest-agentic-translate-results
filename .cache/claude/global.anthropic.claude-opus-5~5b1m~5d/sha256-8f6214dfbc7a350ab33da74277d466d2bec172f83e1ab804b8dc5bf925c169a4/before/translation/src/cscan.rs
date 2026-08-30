//! Minimal emulation of C's `scanf` conversions used by the original program
//! (`%d` and `%f`), operating over the whole of stdin.

pub struct Scanner {
    buf: Vec<u8>,
    pos: usize,
}

fn is_space(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | b'\n' | b'\x0b' | b'\x0c' | b'\r')
}

impl Scanner {
    pub fn new(buf: Vec<u8>) -> Self {
        Scanner { buf, pos: 0 }
    }

    fn peek(&self, off: usize) -> Option<u8> {
        self.buf.get(self.pos + off).copied()
    }

    fn at(&self, i: usize) -> Option<u8> {
        self.buf.get(i).copied()
    }

    /// Directives `%d` / `%f` skip leading whitespace (across newlines).
    fn skip_ws(&mut self) {
        while let Some(c) = self.peek(0) {
            if is_space(c) {
                self.pos += 1;
            } else {
                break;
            }
        }
    }

    /// `%d`: strtol-like base-10 parse, result stored into an `int`.
    pub fn scan_int(&mut self) -> Option<i32> {
        self.skip_ws();
        let start = self.pos;
        let mut i = start;
        let mut neg = false;
        match self.at(i) {
            Some(b'+') => i += 1,
            Some(b'-') => {
                neg = true;
                i += 1;
            }
            _ => {}
        }
        let digits_start = i;
        let mut acc: i128 = 0;
        let mut overflow = false;
        while let Some(c) = self.at(i) {
            if !c.is_ascii_digit() {
                break;
            }
            if !overflow {
                acc = acc * 10 + (c - b'0') as i128;
                if acc > (i64::MAX as i128) + 1 {
                    overflow = true;
                }
            }
            i += 1;
        }
        if i == digits_start {
            // matching failure: nothing consumed beyond whitespace
            self.pos = start;
            return None;
        }
        self.pos = i;
        // strtol saturates at LONG_MAX / LONG_MIN, the value is then
        // truncated to int on assignment.
        let long_val: i64 = if neg {
            if overflow || -acc < i64::MIN as i128 {
                i64::MIN
            } else {
                (-acc) as i64
            }
        } else if overflow || acc > i64::MAX as i128 {
            i64::MAX
        } else {
            acc as i64
        };
        Some(long_val as i32)
    }

    /// `%f`: glibc's `strtof`-style scanf conversion, including its exact
    /// failure cases and how much input it consumes (glibc can only push one
    /// character back, so a malformed exponent leaves `e`/`p` and its sign
    /// consumed).
    pub fn scan_f32(&mut self) -> Option<f32> {
        self.skip_ws();
        let start = self.pos;
        let mut i = start;
        let mut neg = false;
        match self.at(i) {
            Some(b'+') => i += 1,
            Some(b'-') => {
                neg = true;
                i += 1;
            }
            _ => {}
        }

        let signed_inf = || Some(if neg { f32::NEG_INFINITY } else { f32::INFINITY });

        // "inf" / "infinity". A partial "infinity" longer than "inf" is a
        // matching failure in glibc ("infi", "infin", "infix", ...).
        if match_ci(&self.buf, i, b"inf").is_some() {
            if matches!(self.at(i + 3), Some(b'i') | Some(b'I')) {
                if match_ci(&self.buf, i, b"infinity").is_some() {
                    self.pos = i + 8;
                    return signed_inf();
                }
                self.pos = self.buf.len().min(i + 8);
                return None;
            }
            self.pos = i + 3;
            return signed_inf();
        }
        // "nan"; glibc's scanf does not accept an "nan(chars)" payload here.
        if match_ci(&self.buf, i, b"nan").is_some() {
            self.pos = i + 3;
            return Some(if neg { -f32::NAN } else { f32::NAN });
        }

        // Hexadecimal floating point: 0x<hexdigits>[.<hexdigits>][p[+-]digits].
        // Once "0x" has been seen the conversion is committed to hex and fails
        // if no hex digits follow.
        if self.at(i) == Some(b'0') && matches!(self.at(i + 1), Some(b'x') | Some(b'X')) {
            let mut j = i + 2;
            let mut mant: f64 = 0.0;
            let mut exp: i64 = 0;
            let mut any_digit = false;
            while let Some(d) = self.at(j).and_then(hex_val) {
                mant = mant * 16.0 + d as f64;
                any_digit = true;
                j += 1;
            }
            if self.at(j) == Some(b'.') {
                j += 1;
                while let Some(d) = self.at(j).and_then(hex_val) {
                    mant = mant * 16.0 + d as f64;
                    exp -= 4;
                    any_digit = true;
                    j += 1;
                }
            }
            if !any_digit {
                self.pos = j;
                return None;
            }
            let mut end = j;
            if matches!(self.at(j), Some(b'p') | Some(b'P')) {
                let mut k = j + 1;
                let mut eneg = false;
                if matches!(self.at(k), Some(b'+') | Some(b'-')) {
                    eneg = self.at(k) == Some(b'-');
                    k += 1;
                }
                if self.at(k).map(|c| c.is_ascii_digit()).unwrap_or(false) {
                    let mut e: i64 = 0;
                    while let Some(c) = self.at(k) {
                        if !c.is_ascii_digit() {
                            break;
                        }
                        e = (e * 10 + (c - b'0') as i64).min(1_000_000);
                        k += 1;
                    }
                    exp += if eneg { -e } else { e };
                }
                // The 'p' and its sign stay consumed even without digits.
                end = k;
            }
            self.pos = end;
            let v = mant * (exp as f64).exp2();
            let v = if neg { -v } else { v };
            return Some(v as f32);
        }

        // Decimal.
        let mut j = i;
        let mut any_digit = false;
        while self.at(j).map(|c| c.is_ascii_digit()).unwrap_or(false) {
            any_digit = true;
            j += 1;
        }
        if self.at(j) == Some(b'.') {
            j += 1;
            while self.at(j).map(|c| c.is_ascii_digit()).unwrap_or(false) {
                any_digit = true;
                j += 1;
            }
        }
        if !any_digit {
            self.pos = j;
            return None;
        }
        let mut end = j;
        let mut exp_end = j;
        if matches!(self.at(j), Some(b'e') | Some(b'E')) {
            let mut k = j + 1;
            if matches!(self.at(k), Some(b'+') | Some(b'-')) {
                k += 1;
            }
            if self.at(k).map(|c| c.is_ascii_digit()).unwrap_or(false) {
                while self.at(k).map(|c| c.is_ascii_digit()).unwrap_or(false) {
                    k += 1;
                }
                end = k;
                exp_end = k;
            } else {
                // The 'e' and its sign stay consumed, but are not part of the
                // number that gets converted.
                end = k;
            }
        }
        self.pos = end;
        let text = std::str::from_utf8(&self.buf[start..exp_end]).ok()?;
        text.parse::<f32>().ok()
    }
}

fn hex_val(c: u8) -> Option<u32> {
    match c {
        b'0'..=b'9' => Some((c - b'0') as u32),
        b'a'..=b'f' => Some((c - b'a') as u32 + 10),
        b'A'..=b'F' => Some((c - b'A') as u32 + 10),
        _ => None,
    }
}

/// Case-insensitive prefix match, returns the matched length.
fn match_ci(buf: &[u8], at: usize, pat: &[u8]) -> Option<usize> {
    if at + pat.len() > buf.len() {
        return None;
    }
    for (k, p) in pat.iter().enumerate() {
        if buf[at + k].to_ascii_lowercase() != *p {
            return None;
        }
    }
    Some(pat.len())
}
