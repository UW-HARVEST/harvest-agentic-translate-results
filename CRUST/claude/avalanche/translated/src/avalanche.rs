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
        // Read key_size bytes; if fewer are available, stop.
        let mut total_read = 0usize;
        let mut ended = false;
        while total_read < key_size {
            match ins.read(&mut key[total_read..]) {
                Ok(0) => {
                    ended = true;
                    break;
                }
                Ok(n) => total_read += n,
                Err(_) => {
                    ended = true;
                    break;
                }
            }
        }
        if ended || total_read < key_size {
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
                    let diff = hvalue[j_word] ^ htemp[j_word];
                    if diff == 0 {
                        continue;
                    }
                    for j_bit in 0..32usize {
                        let col = j_word * 32 + j_bit;
                        let j_mask: u32 = 0x80000000u32 >> j_bit;
                        if diff & j_mask != 0 {
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
                let s = format_c_double(format, val);
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

/// Formats a single f64 according to a simplified C-style printf format
/// of the form "%[flags][width][.precision][specifier]".
///
/// Supported flags: ' ' (space) and '-' (left align) and '0' (zero pad) and '+'.
/// Supported specifiers: 'f', 'F', 'e', 'E', 'g', 'G'.
fn format_c_double(format: &str, val: f64) -> String {
    let bytes = format.as_bytes();
    let mut i = 0;
    let mut out = String::new();

    while i < bytes.len() {
        let b = bytes[i];
        if b != b'%' {
            out.push(b as char);
            i += 1;
            continue;
        }
        // Parse '%'
        i += 1;
        if i < bytes.len() && bytes[i] == b'%' {
            out.push('%');
            i += 1;
            continue;
        }

        // Parse flags
        let mut left_align = false;
        let mut zero_pad = false;
        let mut plus_sign = false;
        let mut space_sign = false;
        while i < bytes.len() {
            match bytes[i] {
                b'-' => {
                    left_align = true;
                    i += 1;
                }
                b'0' => {
                    zero_pad = true;
                    i += 1;
                }
                b'+' => {
                    plus_sign = true;
                    i += 1;
                }
                b' ' => {
                    space_sign = true;
                    i += 1;
                }
                b'#' => {
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
            let mut p: usize = 0;
            while i < bytes.len() && bytes[i].is_ascii_digit() {
                p = p * 10 + (bytes[i] - b'0') as usize;
                i += 1;
            }
            precision = Some(p);
        }

        // Skip length modifiers (l, ll, h, hh, L, z, j, t)
        while i < bytes.len() {
            match bytes[i] {
                b'l' | b'h' | b'L' | b'z' | b'j' | b't' => i += 1,
                _ => break,
            }
        }

        // Parse specifier
        if i >= bytes.len() {
            break;
        }
        let spec = bytes[i];
        i += 1;

        let formatted = match spec {
            b'f' | b'F' => format_fixed(val, precision.unwrap_or(6)),
            b'e' | b'E' => format_exp(val, precision.unwrap_or(6), spec == b'E'),
            b'g' | b'G' => format_g(val, precision.unwrap_or(6), spec == b'G'),
            _ => {
                // Unsupported, just push as-is
                out.push('%');
                out.push(spec as char);
                continue;
            }
        };

        // Apply sign prefix flags
        let mut s = formatted;
        if !s.starts_with('-') && !s.starts_with('+') {
            if plus_sign {
                s.insert(0, '+');
            } else if space_sign {
                s.insert(0, ' ');
            }
        }

        // Apply width / padding
        if has_width && s.len() < width {
            let pad = width - s.len();
            if left_align {
                for _ in 0..pad {
                    s.push(' ');
                }
            } else if zero_pad {
                // Insert zeros after sign
                let pad_str: String = std::iter::repeat('0').take(pad).collect();
                if let Some(first) = s.chars().next() {
                    if first == '-' || first == '+' || first == ' ' {
                        let rest: String = s.chars().skip(1).collect();
                        s = format!("{}{}{}", first, pad_str, rest);
                    } else {
                        s = format!("{}{}", pad_str, s);
                    }
                } else {
                    s = format!("{}{}", pad_str, s);
                }
            } else {
                let pad_str: String = std::iter::repeat(' ').take(pad).collect();
                s = format!("{}{}", pad_str, s);
            }
        }

        out.push_str(&s);
    }

    out
}

fn format_fixed(val: f64, precision: usize) -> String {
    format!("{:.*}", precision, val)
}

fn format_exp(val: f64, precision: usize, upper: bool) -> String {
    // Rust's {:e} produces e.g. "1.23e2"; C produces "1.230000e+02".
    if val == 0.0 {
        let mantissa = format!("{:.*}", precision, 0.0f64);
        let e = if upper { 'E' } else { 'e' };
        return format!("{}{}+00", mantissa, e);
    }
    let abs = val.abs();
    let exp = abs.log10().floor() as i32;
    let mantissa = val / 10f64.powi(exp);
    // Handle precision rounding edge cases
    let mantissa_str = format!("{:.*}", precision, mantissa);
    // After rounding mantissa might become >= 10 or < 1
    let mantissa_val: f64 = mantissa_str.parse().unwrap_or(mantissa);
    let (final_mantissa, final_exp) = if mantissa_val.abs() >= 10.0 {
        (mantissa_val / 10.0, exp + 1)
    } else if mantissa_val != 0.0 && mantissa_val.abs() < 1.0 {
        (mantissa_val * 10.0, exp - 1)
    } else {
        (mantissa_val, exp)
    };
    let mantissa_final = format!("{:.*}", precision, final_mantissa);
    let e = if upper { 'E' } else { 'e' };
    let sign = if final_exp >= 0 { '+' } else { '-' };
    let abs_exp = final_exp.abs();
    format!("{}{}{}{:02}", mantissa_final, e, sign, abs_exp)
}

fn format_g(val: f64, precision: usize, upper: bool) -> String {
    // C %g: use %e if exponent < -4 or >= precision; else %f.
    // Precision in %g is the number of significant digits (default 6, 0 treated as 1).
    let prec = if precision == 0 { 1 } else { precision };
    if val == 0.0 {
        // Trim trailing zeros after decimal point (no '#' flag handling here)
        return trim_g_zeros("0".to_string());
    }
    let abs = val.abs();
    let exp = abs.log10().floor() as i32;
    if exp < -4 || exp >= prec as i32 {
        // Use exp format with precision = prec - 1
        let s = format_exp(val, prec - 1, upper);
        trim_g_exp(s)
    } else {
        // Use fixed format with precision = prec - 1 - exp
        let f_prec = (prec as i32 - 1 - exp).max(0) as usize;
        let s = format_fixed(val, f_prec);
        trim_g_zeros(s)
    }
}

fn trim_g_zeros(s: String) -> String {
    if !s.contains('.') {
        return s;
    }
    let trimmed = s.trim_end_matches('0');
    let trimmed = trimmed.trim_end_matches('.');
    trimmed.to_string()
}

fn trim_g_exp(s: String) -> String {
    // Find 'e' or 'E'
    if let Some(idx) = s.find(|c| c == 'e' || c == 'E') {
        let (mantissa, exp_part) = s.split_at(idx);
        let trimmed = trim_g_zeros(mantissa.to_string());
        format!("{}{}", trimmed, exp_part)
    } else {
        trim_g_zeros(s)
    }
}
