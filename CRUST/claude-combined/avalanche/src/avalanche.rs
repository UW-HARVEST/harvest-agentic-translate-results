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
        // Read exactly key_size bytes; if fewer available, break.
        let mut total_read = 0usize;
        let mut eof = false;
        while total_read < key_size {
            match ins.read(&mut key[total_read..]) {
                Ok(0) => {
                    eof = true;
                    break;
                }
                Ok(n) => total_read += n,
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
                let v = self.matrix_get(r, c);
                let s = format_double(format, v);
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

/// Format a double according to a printf-style format string.
///
/// Supports a single conversion of the form
///   %[flags][width][.precision]<conv>
/// where flags ∈ {' ', '+', '-', '0', '#'}, conv ∈ {'f', 'g', 'G', 'e', 'E'},
/// surrounded by literal text. Multiple conversions in the same format string
/// are not used by the C tests (each format string contains exactly one).
fn format_double(format: &str, value: f64) -> String {
    let bytes = format.as_bytes();
    let mut out = String::new();
    let mut i = 0;
    let mut value_used = false;
    while i < bytes.len() {
        if bytes[i] != b'%' {
            out.push(bytes[i] as char);
            i += 1;
            continue;
        }
        // start of conversion
        i += 1;
        if i < bytes.len() && bytes[i] == b'%' {
            out.push('%');
            i += 1;
            continue;
        }

        // parse flags
        let mut flag_space = false;
        let mut flag_plus = false;
        let mut flag_minus = false;
        let mut flag_zero = false;
        let mut flag_hash = false;
        while i < bytes.len() {
            match bytes[i] {
                b' ' => flag_space = true,
                b'+' => flag_plus = true,
                b'-' => flag_minus = true,
                b'0' => flag_zero = true,
                b'#' => flag_hash = true,
                _ => break,
            }
            i += 1;
        }

        // parse width
        let mut width: usize = 0;
        let mut has_width = false;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            has_width = true;
            width = width * 10 + (bytes[i] - b'0') as usize;
            i += 1;
        }

        // parse precision
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

        // conversion char
        if i >= bytes.len() {
            break;
        }
        let conv = bytes[i];
        i += 1;

        // produce the formatted number
        let formatted = match conv {
            b'f' | b'F' => format_f(value, precision.unwrap_or(6)),
            b'e' => format_e(value, precision.unwrap_or(6), false),
            b'E' => format_e(value, precision.unwrap_or(6), true),
            b'g' => format_g(value, precision.unwrap_or(6), false, flag_hash),
            b'G' => format_g(value, precision.unwrap_or(6), true, flag_hash),
            _ => {
                // Unsupported conversion — emit literal placeholder
                String::new()
            }
        };

        // apply sign flags (' ' and '+' for positive numbers)
        let signed = if !formatted.is_empty()
            && !formatted.starts_with('-')
            && !formatted.starts_with('+')
            && !formatted.starts_with(' ')
            && (value.is_finite() || value.is_nan())
        {
            if flag_plus {
                let mut s = String::with_capacity(formatted.len() + 1);
                s.push('+');
                s.push_str(&formatted);
                s
            } else if flag_space {
                let mut s = String::with_capacity(formatted.len() + 1);
                s.push(' ');
                s.push_str(&formatted);
                s
            } else {
                formatted
            }
        } else {
            formatted
        };

        // apply width
        let padded = if has_width && signed.chars().count() < width {
            let pad_count = width - signed.chars().count();
            if flag_minus {
                let mut s = signed.clone();
                for _ in 0..pad_count {
                    s.push(' ');
                }
                s
            } else if flag_zero
                && (conv == b'f'
                    || conv == b'F'
                    || conv == b'e'
                    || conv == b'E'
                    || conv == b'g'
                    || conv == b'G')
            {
                // zero-padding: place zeros after any sign char
                let mut s = String::with_capacity(width);
                let mut chars = signed.chars();
                if let Some(first) = signed.chars().next() {
                    if first == '-' || first == '+' || first == ' ' {
                        s.push(first);
                        chars.next();
                    }
                }
                for _ in 0..pad_count {
                    s.push('0');
                }
                for c in chars {
                    s.push(c);
                }
                s
            } else {
                let mut s = String::with_capacity(width);
                for _ in 0..pad_count {
                    s.push(' ');
                }
                s.push_str(&signed);
                s
            }
        } else {
            signed
        };

        out.push_str(&padded);
        value_used = true;
    }
    let _ = value_used;
    out
}

/// %f: fixed-point with given precision.
fn format_f(value: f64, precision: usize) -> String {
    if value.is_nan() {
        return "nan".to_string();
    }
    if value.is_infinite() {
        return if value < 0.0 {
            "-inf".to_string()
        } else {
            "inf".to_string()
        };
    }
    format!("{:.*}", precision, value)
}

/// %e: scientific.
fn format_e(value: f64, precision: usize, upper: bool) -> String {
    if value.is_nan() {
        return if upper {
            "NAN".to_string()
        } else {
            "nan".to_string()
        };
    }
    if value.is_infinite() {
        let s = if upper { "INF" } else { "inf" };
        return if value < 0.0 {
            format!("-{}", s)
        } else {
            s.to_string()
        };
    }
    // Use Rust's exponent formatting then tweak to printf's shape (e.g. e+01).
    let raw = format!("{:.*e}", precision, value);
    // raw is like "1.2345e2" or "1.2345e-2"
    // Convert to "1.2345e+02" with at least 2-digit exponent.
    if let Some(epos) = raw.find('e') {
        let mantissa = &raw[..epos];
        let exp_part = &raw[epos + 1..];
        let (sign, digits) = if let Some(stripped) = exp_part.strip_prefix('-') {
            ('-', stripped)
        } else if let Some(stripped) = exp_part.strip_prefix('+') {
            ('+', stripped)
        } else {
            ('+', exp_part)
        };
        let digits = if digits.len() < 2 {
            format!("0{}", digits)
        } else {
            digits.to_string()
        };
        let echar = if upper { 'E' } else { 'e' };
        format!("{}{}{}{}", mantissa, echar, sign, digits)
    } else {
        raw
    }
}

/// %g: shortest of %e or %f. Trailing zeros are trimmed unless `#` flag.
fn format_g(value: f64, precision: usize, upper: bool, hash: bool) -> String {
    if value.is_nan() {
        return if upper {
            "NAN".to_string()
        } else {
            "nan".to_string()
        };
    }
    if value.is_infinite() {
        let s = if upper { "INF" } else { "inf" };
        return if value < 0.0 {
            format!("-{}", s)
        } else {
            s.to_string()
        };
    }
    let p = if precision == 0 { 1 } else { precision };

    // Determine exponent X if value were displayed in scientific.
    let x = if value == 0.0 {
        0
    } else {
        value.abs().log10().floor() as i32
    };

    // %g uses %e when X < -4 or X >= precision, else %f.
    if x < -4 || (x as i64) >= p as i64 {
        // Use %e style with precision p-1.
        let prec = p - 1;
        let s = format_e(value, prec, upper);
        if !hash {
            trim_g_trailing(&s, true)
        } else {
            s
        }
    } else {
        // Use %f style with precision p - 1 - x.
        let prec = (p as i64 - 1 - x as i64).max(0) as usize;
        let s = format_f(value, prec);
        if !hash {
            trim_g_trailing(&s, false)
        } else {
            s
        }
    }
}

/// Trim trailing zeros (and a trailing '.') from %g output.
/// If `is_e` is true, the exponent suffix must be preserved.
fn trim_g_trailing(s: &str, is_e: bool) -> String {
    if is_e {
        // Split mantissa and exponent.
        let lower = s.to_ascii_lowercase();
        let epos = lower.rfind('e');
        if let Some(epos) = epos {
            let mantissa = &s[..epos];
            let exp_part = &s[epos..];
            let trimmed = trim_zeros_dot(mantissa);
            format!("{}{}", trimmed, exp_part)
        } else {
            s.to_string()
        }
    } else {
        trim_zeros_dot(s)
    }
}

fn trim_zeros_dot(s: &str) -> String {
    if !s.contains('.') {
        return s.to_string();
    }
    let bytes = s.as_bytes();
    let mut end = bytes.len();
    while end > 0 && bytes[end - 1] == b'0' {
        end -= 1;
    }
    if end > 0 && bytes[end - 1] == b'.' {
        end -= 1;
    }
    s[..end].to_string()
}
