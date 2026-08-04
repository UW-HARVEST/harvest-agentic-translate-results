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
        if ins.read_exact(&mut key).is_err() {
            break;
        }
        hash(&key, &mut hvalue);
        key_count += 1;

        for i_byte in 0..key_size {
            for i_bit in 0..8u8 {
                let row = i_byte * 8 + i_bit as usize;
                let i_mask: u8 = 0x80 >> i_bit;
                key[i_byte] ^= i_mask;
                hash(&key, &mut htemp);
                key[i_byte] ^= i_mask;

                for j_word in 0..hash_words {
                    for j_bit in 0..32u32 {
                        let col = j_word * 32 + j_bit as usize;
                        let j_mask: u32 = 0x80000000 >> j_bit;
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
        for v in results.vals.iter_mut() {
            *v /= key_count as f64;
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
                let val = self.matrix_get(r, c);
                let formatted = printf_compat_format(format, val);
                let _ = write!(fout, "{}", formatted);
            }
            let _ = writeln!(fout);
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

/// Minimal C-style printf format interpreter for %f, %g, %e style formats.
fn printf_compat_format(format: &str, val: f64) -> String {
    // Parse a simple C printf format like "%8.4f", "% 6.4g", "%8.4f"
    let fmt = format.trim();
    if !fmt.contains('%') {
        return fmt.replace("{}", &val.to_string());
    }

    let after_pct = &fmt[fmt.find('%').unwrap() + 1..];
    let prefix = &fmt[..fmt.find('%').unwrap()];

    // Parse flags
    let mut pos = 0;
    let bytes = after_pct.as_bytes();
    let mut flag_space = false;
    let mut flag_minus = false;
    let mut flag_plus = false;
    while pos < bytes.len() {
        match bytes[pos] {
            b' ' => flag_space = true,
            b'-' => flag_minus = true,
            b'+' => flag_plus = true,
            _ => break,
        }
        pos += 1;
    }

    // Parse width
    let mut width: usize = 0;
    while pos < bytes.len() && bytes[pos].is_ascii_digit() {
        width = width * 10 + (bytes[pos] - b'0') as usize;
        pos += 1;
    }

    // Parse precision
    let mut precision: Option<usize> = None;
    if pos < bytes.len() && bytes[pos] == b'.' {
        pos += 1;
        let mut p = 0;
        while pos < bytes.len() && bytes[pos].is_ascii_digit() {
            p = p * 10 + (bytes[pos] - b'0') as usize;
            pos += 1;
        }
        precision = Some(p);
    }

    // Parse conversion specifier
    let spec = if pos < bytes.len() { bytes[pos] as char } else { 'f' };
    let suffix = if pos + 1 < bytes.len() { &after_pct[pos + 1..] } else { "" };

    let prec = precision.unwrap_or(6);

    let num_str = match spec {
        'f' => format!("{:.prec$}", val, prec = prec),
        'e' | 'E' => {
            let s = format!("{:.prec$e}", val, prec = prec);
            // Rust uses 'e', C uses 'e' with at least 2-digit exponent
            format_exp(&s, spec)
        }
        'g' | 'G' => format_g(val, prec, spec),
        _ => format!("{}", val),
    };

    // Apply sign/space prefix
    let (sign_char, abs_str) = if num_str.starts_with('-') {
        ("-", &num_str[1..])
    } else if flag_plus {
        ("+", num_str.as_str())
    } else if flag_space {
        (" ", num_str.as_str())
    } else {
        ("", num_str.as_str())
    };

    let content = format!("{}{}{}{}", prefix, sign_char, abs_str, suffix);

    if width > 0 && content.len() < width {
        let pad = width - content.len();
        if flag_minus {
            format!("{}{}", content, " ".repeat(pad))
        } else {
            format!("{}{}", " ".repeat(pad), content)
        }
    } else {
        content
    }
}

fn format_g(val: f64, prec: usize, spec: char) -> String {
    let prec = if prec == 0 { 1 } else { prec };
    if val == 0.0 {
        // %g with 0.0
        let s = format!("{:.prec$}", 0.0, prec = prec - 1);
        return strip_trailing_zeros_g(&s);
    }
    let exp = val.abs().log10().floor() as i32;
    if exp >= prec as i32 || exp < -4 {
        // Use scientific notation, precision P-1
        let p = if prec > 1 { prec - 1 } else { 0 };
        let s = format!("{:.p$e}", val, p = p);
        let s = format_exp(&s, if spec == 'G' { 'E' } else { 'e' });
        strip_trailing_zeros_g(&s)
    } else {
        // Use fixed notation, precision P-(X+1)
        let decimal_digits = if prec as i32 - 1 - exp > 0 {
            (prec as i32 - 1 - exp) as usize
        } else {
            0
        };
        let s = format!("{:.decimal_digits$}", val, decimal_digits = decimal_digits);
        strip_trailing_zeros_g(&s)
    }
}

fn strip_trailing_zeros_g(s: &str) -> String {
    // For %g, trailing zeros after decimal point are removed
    if let Some(dot_pos) = s.find('.') {
        // Check if there's an exponent
        let (mantissa, exp_part) = if let Some(e_pos) = s[dot_pos..].find(|c: char| c == 'e' || c == 'E') {
            (&s[..dot_pos + e_pos], &s[dot_pos + e_pos..])
        } else {
            (s, "")
        };
        let trimmed = mantissa.trim_end_matches('0');
        let trimmed = trimmed.trim_end_matches('.');
        format!("{}{}", trimmed, exp_part)
    } else {
        s.to_string()
    }
}

fn format_exp(s: &str, spec: char) -> String {
    // Convert Rust's scientific notation to C-style (e.g., "1.23e1" -> "1.23e+01")
    if let Some(e_pos) = s.find('e') {
        let mantissa = &s[..e_pos];
        let exp_str = &s[e_pos + 1..];
        let exp_val: i32 = exp_str.parse().unwrap_or(0);
        let e_char = if spec == 'E' || spec == 'G' { 'E' } else { 'e' };
        if exp_val >= 0 {
            if exp_val.abs() >= 100 {
                format!("{}{}+{:03}", mantissa, e_char, exp_val)
            } else {
                format!("{}{}+{:02}", mantissa, e_char, exp_val)
            }
        } else {
            if exp_val.abs() >= 100 {
                format!("{}{}-{:03}", mantissa, e_char, -exp_val)
            } else {
                format!("{}{}-{:02}", mantissa, e_char, -exp_val)
            }
        }
    } else {
        s.to_string()
    }
}
