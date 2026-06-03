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

    let mut key = vec![0u8; key_size];
    let mut hvalue = vec![0u32; hash_words];
    let mut htemp = vec![0u32; hash_words];

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

                // Flip the i-th bit of this byte and re-hash.
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
            vals: vec![0.0f64; n_rows * n_cols],
        }
    }
    /// Print the matrix to the given writer using the specified format.
    pub fn matrix_fprintf(&self, fout: &mut dyn Write, format: &str) {
        for r in 0..self.n_rows {
            for c in 0..self.n_cols {
                let val = self.matrix_get(r, c);
                let s = format_double(format, val);
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

/// Minimal printf-style formatter for a single f64 value.
///
/// Supports format specifiers of the form: %[flags][width][.precision]conv
/// where flags include '-', '+', ' ', '0', '#'; conv is one of 'f', 'F', 'e',
/// 'E', 'g', 'G'. Any text outside the specifier is emitted verbatim. Only
/// the first conversion specifier in the format string consumes the value.
fn format_double(format: &str, val: f64) -> String {
    let bytes = format.as_bytes();
    let mut out = String::new();
    let mut i = 0;
    let mut consumed = false;

    while i < bytes.len() {
        let b = bytes[i];
        if b == b'%' && i + 1 < bytes.len() && bytes[i + 1] == b'%' {
            out.push('%');
            i += 2;
            continue;
        }
        if b == b'%' && !consumed {
            // Parse flags
            i += 1;
            let mut flag_minus = false;
            let mut flag_plus = false;
            let mut flag_space = false;
            let mut flag_zero = false;
            let mut flag_hash = false;
            while i < bytes.len() {
                match bytes[i] {
                    b'-' => {
                        flag_minus = true;
                        i += 1;
                    }
                    b'+' => {
                        flag_plus = true;
                        i += 1;
                    }
                    b' ' => {
                        flag_space = true;
                        i += 1;
                    }
                    b'0' => {
                        flag_zero = true;
                        i += 1;
                    }
                    b'#' => {
                        flag_hash = true;
                        i += 1;
                    }
                    _ => break,
                }
            }

            // Parse width
            let mut width: usize = 0;
            let mut has_width = false;
            while i < bytes.len() && bytes[i].is_ascii_digit() {
                has_width = true;
                width = width * 10 + (bytes[i] - b'0') as usize;
                i += 1;
            }

            // Parse precision
            let mut precision: Option<usize> = None;
            if i < bytes.len() && bytes[i] == b'.' {
                i += 1;
                let mut prec: usize = 0;
                while i < bytes.len() && bytes[i].is_ascii_digit() {
                    prec = prec * 10 + (bytes[i] - b'0') as usize;
                    i += 1;
                }
                precision = Some(prec);
            }

            // Skip optional length modifiers (l, ll, h, hh, L, z, j, t)
            while i < bytes.len()
                && matches!(
                    bytes[i],
                    b'l' | b'h' | b'L' | b'z' | b'j' | b't'
                )
            {
                i += 1;
            }

            if i >= bytes.len() {
                break;
            }
            let conv = bytes[i] as char;
            i += 1;

            // Format value as a string according to conv and precision.
            let prec = precision.unwrap_or(6);
            let mut formatted = match conv {
                'f' | 'F' => format_f(val, prec, flag_hash),
                'e' => format_e(val, prec, false, flag_hash),
                'E' => format_e(val, prec, true, flag_hash),
                'g' | 'G' => format_g(val, precision, conv == 'G', flag_hash),
                _ => {
                    // Fallback: use Rust's debug formatting.
                    format!("{}", val)
                }
            };

            // Apply sign flags.
            let is_neg = formatted.starts_with('-');
            if !is_neg {
                if flag_plus {
                    formatted.insert(0, '+');
                } else if flag_space {
                    formatted.insert(0, ' ');
                }
            }

            // Apply width and padding.
            let len = formatted.chars().count();
            if has_width && len < width {
                let pad = width - len;
                if flag_minus {
                    // Left-justify: pad with spaces on right.
                    out.push_str(&formatted);
                    for _ in 0..pad {
                        out.push(' ');
                    }
                } else if flag_zero
                    && (conv == 'f'
                        || conv == 'F'
                        || conv == 'e'
                        || conv == 'E'
                        || conv == 'g'
                        || conv == 'G')
                    && val.is_finite()
                {
                    // Zero-pad: insert zeros after sign.
                    let mut chars: Vec<char> = formatted.chars().collect();
                    let sign_idx = if !chars.is_empty()
                        && (chars[0] == '+' || chars[0] == '-' || chars[0] == ' ')
                    {
                        1
                    } else {
                        0
                    };
                    for _ in 0..pad {
                        chars.insert(sign_idx, '0');
                    }
                    let s: String = chars.into_iter().collect();
                    out.push_str(&s);
                } else {
                    for _ in 0..pad {
                        out.push(' ');
                    }
                    out.push_str(&formatted);
                }
            } else {
                out.push_str(&formatted);
            }
            consumed = true;
            continue;
        }
        out.push(b as char);
        i += 1;
    }
    out
}

fn format_f(val: f64, precision: usize, _hash: bool) -> String {
    if val.is_nan() {
        return "nan".to_string();
    }
    if val.is_infinite() {
        return if val < 0.0 { "-inf".to_string() } else { "inf".to_string() };
    }
    format!("{:.*}", precision, val)
}

fn format_e(val: f64, precision: usize, upper: bool, _hash: bool) -> String {
    if val.is_nan() {
        return if upper { "NAN".to_string() } else { "nan".to_string() };
    }
    if val.is_infinite() {
        let s = if upper { "INF" } else { "inf" };
        return if val < 0.0 { format!("-{}", s) } else { s.to_string() };
    }
    // Use Rust's exponent formatting then reformat exponent to match C (e±NN with at least 2 digits).
    let raw = format!("{:.*e}", precision, val);
    reformat_exponent(&raw, upper)
}

fn reformat_exponent(s: &str, upper: bool) -> String {
    // Rust's format produces "1.234e5" or "1.234e-5"; C's produces "1.234e+05".
    if let Some(e_idx) = s.find('e') {
        let mantissa = &s[..e_idx];
        let exp_part = &s[e_idx + 1..];
        let (sign, digits) = if let Some(rest) = exp_part.strip_prefix('-') {
            ("-", rest)
        } else if let Some(rest) = exp_part.strip_prefix('+') {
            ("+", rest)
        } else {
            ("+", exp_part)
        };
        let padded = if digits.len() < 2 {
            format!("0{}", digits)
        } else {
            digits.to_string()
        };
        let e_char = if upper { 'E' } else { 'e' };
        format!("{}{}{}{}", mantissa, e_char, sign, padded)
    } else {
        s.to_string()
    }
}

fn format_g(val: f64, precision: Option<usize>, upper: bool, hash: bool) -> String {
    if val.is_nan() {
        return if upper { "NAN".to_string() } else { "nan".to_string() };
    }
    if val.is_infinite() {
        let s = if upper { "INF" } else { "inf" };
        return if val < 0.0 { format!("-{}", s) } else { s.to_string() };
    }
    // For %g: precision is the number of significant digits; default is 6.
    // Precision of 0 is treated as 1.
    let mut p = precision.unwrap_or(6);
    if p == 0 {
        p = 1;
    }

    // Determine exponent of the value.
    let exp: i32 = if val == 0.0 {
        0
    } else {
        val.abs().log10().floor() as i32
    };

    let use_e = exp < -4 || exp >= p as i32;

    let result = if use_e {
        // Use %e style with precision p-1.
        let prec_e = p - 1;
        let raw = format!("{:.*e}", prec_e, val);
        reformat_exponent(&raw, upper)
    } else {
        // Use %f style with precision p - 1 - exp.
        let prec_f = (p as i32 - 1 - exp).max(0) as usize;
        format!("{:.*}", prec_f, val)
    };

    if hash {
        result
    } else {
        // Strip trailing zeros after decimal point and a trailing decimal point.
        strip_trailing_zeros(&result)
    }
}

fn strip_trailing_zeros(s: &str) -> String {
    // Split mantissa and exponent if present.
    let (mantissa, exp_part) = if let Some(idx) = s.find(|c| c == 'e' || c == 'E') {
        (&s[..idx], Some(&s[idx..]))
    } else {
        (s, None)
    };
    let trimmed = if mantissa.contains('.') {
        let m = mantissa.trim_end_matches('0');
        let m = m.trim_end_matches('.');
        m.to_string()
    } else {
        mantissa.to_string()
    };
    if let Some(e) = exp_part {
        format!("{}{}", trimmed, e)
    } else {
        trimmed
    }
}
