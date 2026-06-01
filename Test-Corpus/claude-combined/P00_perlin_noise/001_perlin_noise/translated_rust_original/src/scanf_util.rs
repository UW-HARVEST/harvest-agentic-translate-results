// Minimal scanf-like utility for "%d" and "%f" conversions.
// Whitespace (spaces, tabs, newlines, etc.) is skipped before each read.
// Returns None on parse failure or EOF.

pub struct Scanner<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Scanner<'a> {
    pub fn new(s: &'a str) -> Self {
        Scanner { bytes: s.as_bytes(), pos: 0 }
    }

    fn skip_ws(&mut self) {
        while self.pos < self.bytes.len() {
            let c = self.bytes[self.pos];
            if c == b' ' || c == b'\t' || c == b'\n' || c == b'\r' || c == 0x0B || c == 0x0C {
                self.pos += 1;
            } else {
                break;
            }
        }
    }

    /// Read a "%d" style integer. Skips whitespace, then accepts optional +/-,
    /// then one or more decimal digits.
    pub fn read_i32(&mut self) -> Option<i32> {
        self.skip_ws();
        let start = self.pos;
        if self.pos < self.bytes.len() {
            let c = self.bytes[self.pos];
            if c == b'+' || c == b'-' {
                self.pos += 1;
            }
        }
        let digits_start = self.pos;
        while self.pos < self.bytes.len() && self.bytes[self.pos].is_ascii_digit() {
            self.pos += 1;
        }
        if self.pos == digits_start {
            // No digits parsed -> failure. Restore position.
            self.pos = start;
            return None;
        }
        let s = std::str::from_utf8(&self.bytes[start..self.pos]).ok()?;
        // Use i64 first to allow C-like overflow truncation (cast as i32).
        match s.parse::<i64>() {
            Ok(v) => Some(v as i32),
            Err(_) => {
                // Possibly out of i64 range; fall back to manual.
                let mut neg = false;
                let mut idx = 0usize;
                let bytes = s.as_bytes();
                if bytes[0] == b'+' { idx = 1; }
                else if bytes[0] == b'-' { neg = true; idx = 1; }
                let mut acc: i64 = 0;
                while idx < bytes.len() {
                    acc = acc.wrapping_mul(10).wrapping_add((bytes[idx] - b'0') as i64);
                    idx += 1;
                }
                if neg { acc = acc.wrapping_neg(); }
                Some(acc as i32)
            }
        }
    }

    /// Read a "%f" style float. Accepts optional sign, digits, optional decimal,
    /// optional exponent. Compatible with C scanf "%f" for typical inputs.
    pub fn read_f32(&mut self) -> Option<f32> {
        self.skip_ws();
        let start = self.pos;

        // Optional sign
        if self.pos < self.bytes.len() {
            let c = self.bytes[self.pos];
            if c == b'+' || c == b'-' {
                self.pos += 1;
            }
        }

        let mut has_digits = false;

        // Check for inf / infinity / nan (case-insensitive) like C scanf does.
        let rest = &self.bytes[self.pos..];
        let lower_starts = |needle: &[u8]| -> bool {
            if rest.len() < needle.len() { return false; }
            for (a, b) in rest.iter().zip(needle.iter()) {
                if a.to_ascii_lowercase() != *b { return false; }
            }
            true
        };
        if lower_starts(b"infinity") {
            self.pos += 8;
            let s = std::str::from_utf8(&self.bytes[start..self.pos]).ok()?;
            return s.parse::<f32>().ok();
        }
        if lower_starts(b"inf") {
            self.pos += 3;
            let s = std::str::from_utf8(&self.bytes[start..self.pos]).ok()?;
            return s.parse::<f32>().ok();
        }
        if lower_starts(b"nan") {
            self.pos += 3;
            let s = std::str::from_utf8(&self.bytes[start..self.pos]).ok()?;
            return s.parse::<f32>().ok();
        }

        // Integer part
        while self.pos < self.bytes.len() && self.bytes[self.pos].is_ascii_digit() {
            has_digits = true;
            self.pos += 1;
        }
        // Fractional part
        if self.pos < self.bytes.len() && self.bytes[self.pos] == b'.' {
            self.pos += 1;
            while self.pos < self.bytes.len() && self.bytes[self.pos].is_ascii_digit() {
                has_digits = true;
                self.pos += 1;
            }
        }
        if !has_digits {
            self.pos = start;
            return None;
        }
        // Exponent part
        if self.pos < self.bytes.len() && (self.bytes[self.pos] == b'e' || self.bytes[self.pos] == b'E') {
            let exp_start = self.pos;
            self.pos += 1;
            if self.pos < self.bytes.len() {
                let c = self.bytes[self.pos];
                if c == b'+' || c == b'-' {
                    self.pos += 1;
                }
            }
            let exp_digits_start = self.pos;
            while self.pos < self.bytes.len() && self.bytes[self.pos].is_ascii_digit() {
                self.pos += 1;
            }
            if self.pos == exp_digits_start {
                // No exponent digits — back out exponent part.
                self.pos = exp_start;
            }
        }
        let s = std::str::from_utf8(&self.bytes[start..self.pos]).ok()?;
        s.parse::<f32>().ok()
    }
}
