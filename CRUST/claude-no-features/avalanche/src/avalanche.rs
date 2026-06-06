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
        // Read exactly key_size bytes; if fewer are available, break.
        let mut total_read = 0usize;
        let mut eof = false;
        while total_read < key_size {
            match ins.read(&mut key[total_read..]) {
                Ok(0) => {
                    eof = true;
                    break;
                }
                Ok(n) => {
                    total_read += n;
                }
                Err(_) => {
                    eof = true;
                    break;
                }
            }
        }
        if total_read < key_size {
            let _ = eof;
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
        let denom = key_count as f64;
        for v in results.vals.iter_mut() {
            *v /= denom;
        }
    }
}
impl Matrix {
    /// Allocate a matrix with the given number of rows and columns.
    pub fn matrix_alloc(n_rows: usize, n_cols: usize) -> Self {
        Matrix {
            n_rows,
            n_cols,
            vals: vec![0.0; n_rows * n_cols],
        }
    }
    /// Print the matrix to the given writer using the specified format.
    pub fn matrix_fprintf(&self, fout: &mut dyn Write, format: &str) {
        for r in 0..self.n_rows {
            for c in 0..self.n_cols {
                let s = format_double_c(format, self.matrix_get(r, c));
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

/// Minimal C-style printf format helper for double values.
/// Supports a subset of the C printf format spec sufficient to handle
/// patterns like "%8.4f", "% 6.4g", "%-10.3e", "%f", etc.
fn format_double_c(fmt: &str, value: f64) -> String {
    let bytes = fmt.as_bytes();
    // Find the first '%' that is not "%%".
    let mut i = 0;
    let mut prefix = String::new();
    let mut spec_start: Option<usize> = None;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            if i + 1 < bytes.len() && bytes[i + 1] == b'%' {
                prefix.push('%');
                i += 2;
                continue;
            }
            spec_start = Some(i);
            break;
        }
        prefix.push(bytes[i] as char);
        i += 1;
    }
    if spec_start.is_none() {
        // No format spec; just return the prefix.
        return prefix;
    }

    let mut idx = spec_start.unwrap() + 1;

    // Parse flags
    let mut flag_minus = false;
    let mut flag_plus = false;
    let mut flag_space = false;
    let mut flag_zero = false;
    let mut flag_hash = false;
    while idx < bytes.len() {
        match bytes[idx] {
            b'-' => flag_minus = true,
            b'+' => flag_plus = true,
            b' ' => flag_space = true,
            b'0' => flag_zero = true,
            b'#' => flag_hash = true,
            _ => break,
        }
        idx += 1;
    }

    // Parse width
    let mut width: Option<usize> = None;
    while idx < bytes.len() && bytes[idx].is_ascii_digit() {
        let d = (bytes[idx] - b'0') as usize;
        width = Some(width.unwrap_or(0) * 10 + d);
        idx += 1;
    }

    // Parse precision
    let mut precision: Option<usize> = None;
    if idx < bytes.len() && bytes[idx] == b'.' {
        idx += 1;
        let mut p = 0usize;
        while idx < bytes.len() && bytes[idx].is_ascii_digit() {
            p = p * 10 + (bytes[idx] - b'0') as usize;
            idx += 1;
        }
        precision = Some(p);
    }

    // Specifier
    if idx >= bytes.len() {
        return prefix;
    }
    let spec = bytes[idx] as char;
    idx += 1;

    // Trailing literal text
    let suffix: String = std::str::from_utf8(&bytes[idx..])
        .unwrap_or("")
        .to_string();

    // Format the number into a body string.
    let body = format_number(value, spec, precision, flag_plus, flag_space, flag_hash);

    // Apply width / padding.
    let padded = apply_width(&body, width, flag_minus, flag_zero, value, spec);

    let mut out = prefix;
    out.push_str(&padded);
    out.push_str(&suffix);
    out
}

fn format_number(
    value: f64,
    spec: char,
    precision: Option<usize>,
    plus: bool,
    space: bool,
    _hash: bool,
) -> String {
    let prec = precision;
    let abs_str = match spec {
        'f' | 'F' => {
            let p = prec.unwrap_or(6);
            format!("{:.*}", p, value.abs())
        }
        'e' => {
            let p = prec.unwrap_or(6);
            format_scientific(value.abs(), p, false)
        }
        'E' => {
            let p = prec.unwrap_or(6);
            format_scientific(value.abs(), p, true)
        }
        'g' | 'G' => {
            let p = prec.unwrap_or(6);
            let p = if p == 0 { 1 } else { p };
            format_general(value.abs(), p, spec == 'G')
        }
        _ => {
            // Fallback: default Display
            format!("{}", value.abs())
        }
    };

    let mut out = String::new();
    if value.is_sign_negative() && !value.is_nan() {
        out.push('-');
    } else if plus {
        out.push('+');
    } else if space {
        out.push(' ');
    }
    out.push_str(&abs_str);
    out
}

fn apply_width(
    body: &str,
    width: Option<usize>,
    minus: bool,
    zero: bool,
    value: f64,
    spec: char,
) -> String {
    let w = match width {
        Some(w) => w,
        None => return body.to_string(),
    };
    let len = body.chars().count();
    if len >= w {
        return body.to_string();
    }
    let pad = w - len;
    if minus {
        let mut s = String::from(body);
        s.extend(std::iter::repeat(' ').take(pad));
        s
    } else if zero && (spec == 'f' || spec == 'F' || spec == 'e' || spec == 'E' || spec == 'g' || spec == 'G') {
        // Zero-pad after sign
        let (sign, rest) = if let Some(stripped) = body.strip_prefix('-') {
            ("-", stripped)
        } else if let Some(stripped) = body.strip_prefix('+') {
            ("+", stripped)
        } else if let Some(stripped) = body.strip_prefix(' ') {
            (" ", stripped)
        } else {
            ("", body)
        };
        let _ = value;
        let mut s = String::from(sign);
        s.extend(std::iter::repeat('0').take(pad));
        s.push_str(rest);
        s
    } else {
        let mut s = String::new();
        s.extend(std::iter::repeat(' ').take(pad));
        s.push_str(body);
        s
    }
}

fn format_scientific(value: f64, precision: usize, upper: bool) -> String {
    if value == 0.0 || !value.is_finite() {
        let mantissa = format!("{:.*}", precision, value);
        let e = if upper { "E" } else { "e" };
        return format!("{}{}+00", mantissa, e);
    }
    let exp = value.log10().floor() as i32;
    let mantissa = value / 10f64.powi(exp);
    // Handle rounding overflow: if mantissa rounds to 10.0 at the given precision.
    let mantissa_str = format!("{:.*}", precision, mantissa);
    let (mantissa_str, exp) = if mantissa_str.starts_with("10") {
        (format!("{:.*}", precision, mantissa / 10.0), exp + 1)
    } else {
        (mantissa_str, exp)
    };
    let e_char = if upper { 'E' } else { 'e' };
    let sign_char = if exp < 0 { '-' } else { '+' };
    format!("{}{}{}{:02}", mantissa_str, e_char, sign_char, exp.abs())
}

fn format_general(value: f64, precision: usize, upper: bool) -> String {
    // Replicate %g semantics: use %e if exponent < -4 or >= precision, else %f.
    // Trailing zeros and trailing '.' are removed (since '#' flag is not supported here).
    if value == 0.0 {
        // %g with value 0 produces "0"
        return "0".to_string();
    }
    let exp = value.log10().floor() as i32;
    let use_e = exp < -4 || exp >= precision as i32;
    let s = if use_e {
        let p = if precision == 0 { 0 } else { precision - 1 };
        format_scientific(value, p, upper)
    } else {
        let p = (precision as i32 - 1 - exp).max(0) as usize;
        format!("{:.*}", p, value)
    };
    // Strip trailing zeros after a decimal point, then trailing '.'.
    strip_trailing_zeros(&s)
}

fn strip_trailing_zeros(s: &str) -> String {
    // Split off exponent if any.
    let (num_part, exp_part) = if let Some(idx) = s.find(|c: char| c == 'e' || c == 'E') {
        (&s[..idx], &s[idx..])
    } else {
        (s, "")
    };
    if !num_part.contains('.') {
        return s.to_string();
    }
    let trimmed = num_part.trim_end_matches('0');
    let trimmed = trimmed.trim_end_matches('.');
    let mut out = String::from(trimmed);
    out.push_str(exp_part);
    out
}
