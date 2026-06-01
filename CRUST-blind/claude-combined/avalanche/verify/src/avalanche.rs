use std::io::{Read, Write};
/// Prototypical hash function: converts a key of bytes to a hash value,
/// represented as an array of 32-bit blocks.
pub type HashFn = fn(key: &[u8], hash_value: &mut [u32]);
/// Simple matrix structure.
pub struct Matrix {
    pub n_rows: usize,
    pub n_cols: usize,
    pub vals: Vec<f64>,
}
/// Avalanche test for hash function.
///
/// All test keys are read successively from the stream `ins`, and the probability
/// that flipping i-th input bit affects the j-th output bit is recorded as the
/// ij-th entry of the matrix.
///
/// All keys read have the same length, so that the key length and size of the
/// hash value (in words) are parameterized by the row and columns of the matrix.
pub fn avalanche(hash: HashFn, ins: &mut dyn Read, max_iter: u64, results: &mut Matrix) {
    let key_size = results.n_rows / 8;
    let hash_words = results.n_cols / 32;

    let mut key: Vec<u8> = vec![0u8; key_size];
    let mut hvalue: Vec<u32> = vec![0u32; hash_words];
    let mut htemp: Vec<u32> = vec![0u32; hash_words];

    let mut key_count: u64 = 0;
    while key_count < max_iter {
        // read exactly key_size bytes; if fewer are read, break.
        let mut total_read = 0usize;
        let mut short = false;
        while total_read < key_size {
            match ins.read(&mut key[total_read..]) {
                Ok(0) => {
                    short = true;
                    break;
                }
                Ok(n) => {
                    total_read += n;
                }
                Err(_) => {
                    short = true;
                    break;
                }
            }
        }
        if short || total_read < key_size {
            break;
        }

        hash(&key, &mut hvalue);
        key_count += 1;

        for i_byte in 0..key_size {
            for i_bit in 0..8usize {
                let row = i_byte * 8 + i_bit;

                // flip the i-th bit of this byte and re-hash
                let i_mask: u8 = 0x80u8 >> i_bit;
                key[i_byte] ^= i_mask;
                hash(&key, &mut htemp);
                key[i_byte] ^= i_mask;

                for j_word in 0..hash_words {
                    for j_bit in 0..32usize {
                        let col = j_word * 32 + j_bit;

                        // test whether hvalue & htemp differ at j-th bit.
                        let j_mask: u32 = 0x80000000u32 >> j_bit;
                        if (hvalue[j_word] ^ htemp[j_word]) & j_mask != 0 {
                            let curr = results.matrix_get(row, col);
                            results.matrix_set(row, col, curr + 1.0);
                        }
                    }
                }
            }
        }
    }

    if key_count > 0 {
        let n_vals = results.n_cols * results.n_rows;
        let kc = key_count as f64;
        for k in 0..n_vals {
            results.vals[k] /= kc;
        }
    }
}

impl Matrix {
    /// Allocate a matrix with the given number of rows and columns.
    pub fn matrix_alloc(n_rows: usize, n_cols: usize) -> Self {
        Matrix {
            n_rows,
            n_cols,
            vals: vec![0.0f64; n_rows * n_cols],
        }
    }
    /// Print the matrix to the given writer using the specified format.
    pub fn matrix_fprintf(&self, fout: &mut dyn Write, format: &str) {
        for r in 0..self.n_rows {
            for c in 0..self.n_cols {
                let s = format_double(format, self.matrix_get(r, c));
                let _ = fout.write_all(s.as_bytes());
            }
            let _ = fout.write_all(b"\n");
        }
    }
    /// Get value from the matrix at (row, col).
    pub fn matrix_get(&self, row: usize, col: usize) -> f64 {
        self.vals[row * self.n_cols + col]
    }
    /// Set value in the matrix at (row, col).
    pub fn matrix_set(&mut self, row: usize, col: usize, val: f64) {
        self.vals[row * self.n_cols + col] = val;
    }
}

/// Minimal printf-style formatter for a single double `%[flags][width][.precision][specifier]`.
/// Supports flags: ' ' (space), '+', '-' (left-justify), '0' (zero pad).
/// Specifiers: f, F, e, E, g, G.
fn format_double(format: &str, val: f64) -> String {
    let bytes = format.as_bytes();
    // find '%' that isn't '%%'
    let mut i = 0usize;
    let mut prefix = String::new();
    let mut suffix = String::new();
    let mut found = false;
    let (mut flags, mut width, mut precision, mut spec) =
        (String::new(), None::<usize>, None::<usize>, ' ');
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 1 < bytes.len() && bytes[i + 1] == b'%' {
            prefix.push('%');
            i += 2;
        } else if bytes[i] == b'%' {
            // parse format spec
            i += 1;
            // flags
            while i < bytes.len() {
                match bytes[i] {
                    b' ' | b'+' | b'-' | b'0' | b'#' => {
                        flags.push(bytes[i] as char);
                        i += 1;
                    }
                    _ => break,
                }
            }
            // width
            let mut w = 0usize;
            let mut has_w = false;
            while i < bytes.len() && bytes[i].is_ascii_digit() {
                has_w = true;
                w = w * 10 + (bytes[i] - b'0') as usize;
                i += 1;
            }
            if has_w {
                width = Some(w);
            }
            // precision
            if i < bytes.len() && bytes[i] == b'.' {
                i += 1;
                let mut p = 0usize;
                while i < bytes.len() && bytes[i].is_ascii_digit() {
                    p = p * 10 + (bytes[i] - b'0') as usize;
                    i += 1;
                }
                precision = Some(p);
            }
            // specifier
            if i < bytes.len() {
                spec = bytes[i] as char;
                i += 1;
            }
            found = true;
            // remainder is suffix
            while i < bytes.len() {
                if bytes[i] == b'%' && i + 1 < bytes.len() && bytes[i + 1] == b'%' {
                    suffix.push('%');
                    i += 2;
                } else {
                    suffix.push(bytes[i] as char);
                    i += 1;
                }
            }
            break;
        } else {
            prefix.push(bytes[i] as char);
            i += 1;
        }
    }
    if !found {
        return format.to_string();
    }
    let mut out = String::new();
    out.push_str(&prefix);
    out.push_str(&printf_double(&flags, width, precision, spec, val));
    out.push_str(&suffix);
    out
}

/// Formats a double matching C printf semantics for f/e/g specifiers.
fn printf_double(flags: &str, width: Option<usize>, precision: Option<usize>, spec: char, val: f64) -> String {
    let space = flags.contains(' ');
    let plus = flags.contains('+');
    let left = flags.contains('-');
    let zero = flags.contains('0') && !left;
    let alt = flags.contains('#');

    let prec = precision.unwrap_or(6);

    // Format based on spec
    let body_no_sign = match spec {
        'f' | 'F' => format_f(val.abs(), prec),
        'e' => format_e(val.abs(), prec, false, alt),
        'E' => format_e(val.abs(), prec, true, alt),
        'g' | 'G' => format_g(val.abs(), prec, spec == 'G', alt),
        _ => format!("{}", val),
    };

    let neg = val.is_sign_negative() && val == val; // not NaN signaling; treat NaN simply
    let sign_char = if neg {
        "-"
    } else if plus {
        "+"
    } else if space {
        " "
    } else {
        ""
    };

    let total_len = sign_char.len() + body_no_sign.len();
    let w = width.unwrap_or(0);
    if total_len >= w {
        let mut s = String::new();
        s.push_str(sign_char);
        s.push_str(&body_no_sign);
        return s;
    }
    let pad = w - total_len;
    if left {
        let mut s = String::new();
        s.push_str(sign_char);
        s.push_str(&body_no_sign);
        for _ in 0..pad {
            s.push(' ');
        }
        s
    } else if zero {
        let mut s = String::new();
        s.push_str(sign_char);
        for _ in 0..pad {
            s.push('0');
        }
        s.push_str(&body_no_sign);
        s
    } else {
        let mut s = String::new();
        for _ in 0..pad {
            s.push(' ');
        }
        s.push_str(sign_char);
        s.push_str(&body_no_sign);
        s
    }
}

fn format_f(absval: f64, prec: usize) -> String {
    // Standard fixed-point, banker's rounding... C uses round-half-away-from-zero typically.
    // Rust's format!("{:.prec$}") uses round-half-to-even on most platforms.
    // For test outputs, this should still be fine for our use cases.
    format!("{:.*}", prec, absval)
}

fn format_e(absval: f64, prec: usize, upper: bool, _alt: bool) -> String {
    if absval == 0.0 {
        let mut s = format!("{:.*}", prec, 0.0f64);
        s.push_str(if upper { "E+00" } else { "e+00" });
        return s;
    }
    // normalize
    let exp = absval.abs().log10().floor() as i32;
    let mantissa = absval / 10f64.powi(exp);
    // Handle rounding edge case where mantissa rounds up to 10
    let rounded_mantissa_str = format!("{:.*}", prec, mantissa);
    let parts: Vec<&str> = rounded_mantissa_str.split('.').collect();
    let int_part: i32 = parts[0].parse().unwrap_or(0);
    let (final_mantissa, final_exp) = if int_part >= 10 {
        let m = mantissa / 10.0;
        (format!("{:.*}", prec, m), exp + 1)
    } else {
        (rounded_mantissa_str, exp)
    };
    let exp_sign = if final_exp < 0 { '-' } else { '+' };
    let abs_exp = final_exp.abs();
    let exp_str = if abs_exp < 10 {
        format!("0{}", abs_exp)
    } else {
        format!("{}", abs_exp)
    };
    format!(
        "{}{}{}{}",
        final_mantissa,
        if upper { "E" } else { "e" },
        exp_sign,
        exp_str
    )
}

fn format_g(absval: f64, prec: usize, upper: bool, alt: bool) -> String {
    // For %g: precision specifies the number of significant digits.
    // P = precision; if P == 0, treated as 1.
    let p = if prec == 0 { 1 } else { prec };

    if absval == 0.0 {
        // exponent is 0, so use %f with precision p-1
        let mut s = format!("{:.*}", p - 1, 0.0f64);
        if !alt {
            s = strip_trailing_zeros(&s);
        }
        return s;
    }

    let x = absval.log10().floor() as i32;
    // C: Let X be exponent. If P > X >= -4, use style f with precision = P-1-X. Otherwise use style e with precision = P-1.
    if (x as i64) < (p as i64) && x >= -4 {
        let prec_f = (p as i32 - 1 - x).max(0) as usize;
        let mut s = format!("{:.*}", prec_f, absval);
        if !alt {
            s = strip_trailing_zeros(&s);
        }
        s
    } else {
        let mut s = format_e(absval, p - 1, upper, alt);
        if !alt {
            s = strip_trailing_zeros_e(&s, upper);
        }
        s
    }
}

fn strip_trailing_zeros(s: &str) -> String {
    if !s.contains('.') {
        return s.to_string();
    }
    let trimmed = s.trim_end_matches('0');
    let trimmed = trimmed.trim_end_matches('.');
    trimmed.to_string()
}

fn strip_trailing_zeros_e(s: &str, upper: bool) -> String {
    let sep = if upper { 'E' } else { 'e' };
    if let Some(idx) = s.find(sep) {
        let (mantissa, exp) = s.split_at(idx);
        let mut m = mantissa.to_string();
        if m.contains('.') {
            m = m.trim_end_matches('0').to_string();
            m = m.trim_end_matches('.').to_string();
        }
        format!("{}{}", m, exp)
    } else {
        s.to_string()
    }
}
