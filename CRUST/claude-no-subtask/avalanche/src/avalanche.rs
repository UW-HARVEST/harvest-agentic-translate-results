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
        // Read exactly `key_size` bytes; on short read, break.
        let n_in = read_full(ins, &mut key);
        if n_in < key_size {
            break;
        }

        hash(&key, &mut hvalue);
        key_count += 1;

        for i_byte in 0..key_size {
            for i_bit in 0..8usize {
                let row = i_byte * 8 + i_bit;

                // Flip the i-th bit and re-hash.
                let i_mask: u8 = 0x80u8 >> i_bit;
                key[i_byte] ^= i_mask;
                hash(&key, &mut htemp);
                key[i_byte] ^= i_mask;

                for j_word in 0..hash_words {
                    let xor = hvalue[j_word] ^ htemp[j_word];
                    if xor == 0 {
                        continue;
                    }
                    for j_bit in 0..32usize {
                        let col = j_word * 32 + j_bit;
                        let j_mask: u32 = 0x80000000u32 >> j_bit;
                        if xor & j_mask != 0 {
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

/// Read up to `buf.len()` bytes from `ins` into `buf`. Returns the number of
/// bytes actually read; returns less than `buf.len()` only if EOF is hit.
fn read_full(ins: &mut dyn Read, buf: &mut [u8]) -> usize {
    let mut total: usize = 0;
    while total < buf.len() {
        match ins.read(&mut buf[total..]) {
            Ok(0) => break,
            Ok(n) => total += n,
            Err(_) => break,
        }
    }
    total
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

/// Minimal printf-style formatter sufficient for floating-point conversions
/// used by the avalanche tests/binaries (e.g. `%8.4f`, `% 6.4g`).
/// Supports flags: ' ', '+', '-', '0', '#'; optional width, optional precision,
/// and conversion specifiers 'f', 'F', 'e', 'E', 'g', 'G'. Unknown specifiers
/// fall through with a default `{}` rendering.
fn format_double(fmt: &str, value: f64) -> String {
    let bytes = fmt.as_bytes();
    let mut i = 0;
    let mut out = String::new();
    while i < bytes.len() {
        let b = bytes[i];
        if b != b'%' {
            out.push(b as char);
            i += 1;
            continue;
        }
        // Found '%'
        i += 1;
        if i < bytes.len() && bytes[i] == b'%' {
            out.push('%');
            i += 1;
            continue;
        }

        // Parse flags
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

        // Parse width
        let mut width: Option<usize> = None;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            let d = (bytes[i] - b'0') as usize;
            width = Some(width.unwrap_or(0) * 10 + d);
            i += 1;
        }

        // Parse precision
        let mut precision: Option<usize> = None;
        if i < bytes.len() && bytes[i] == b'.' {
            i += 1;
            let mut p: usize = 0;
            let mut has_digits = false;
            while i < bytes.len() && bytes[i].is_ascii_digit() {
                p = p * 10 + (bytes[i] - b'0') as usize;
                i += 1;
                has_digits = true;
            }
            precision = Some(if has_digits { p } else { 0 });
        }

        // Parse conversion specifier
        if i >= bytes.len() {
            break;
        }
        let spec = bytes[i] as char;
        i += 1;

        let formatted = match spec {
            'f' | 'F' => fmt_fixed(value, precision.unwrap_or(6), flag_plus, flag_space, flag_hash),
            'e' | 'E' => fmt_exponent(
                value,
                precision.unwrap_or(6),
                flag_plus,
                flag_space,
                flag_hash,
                spec == 'E',
            ),
            'g' | 'G' => fmt_general(
                value,
                precision.unwrap_or(6),
                flag_plus,
                flag_space,
                flag_hash,
                spec == 'G',
            ),
            _ => format!("{}", value),
        };

        // Apply width/padding.
        let w = width.unwrap_or(0);
        if formatted.chars().count() >= w {
            out.push_str(&formatted);
        } else {
            let pad = w - formatted.chars().count();
            if flag_minus {
                out.push_str(&formatted);
                for _ in 0..pad {
                    out.push(' ');
                }
            } else if flag_zero && !flag_minus {
                // Zero-pad after sign character.
                let mut chars = formatted.chars();
                let first = formatted.chars().next();
                let sign_char = match first {
                    Some('+') | Some('-') | Some(' ') => Some(chars.next().unwrap()),
                    _ => None,
                };
                if let Some(s) = sign_char {
                    out.push(s);
                    for _ in 0..pad {
                        out.push('0');
                    }
                    out.push_str(&formatted[1..]);
                } else {
                    for _ in 0..pad {
                        out.push('0');
                    }
                    out.push_str(&formatted);
                }
            } else {
                for _ in 0..pad {
                    out.push(' ');
                }
                out.push_str(&formatted);
            }
        }
    }
    out
}

fn sign_prefix(value: f64, plus: bool, space: bool) -> &'static str {
    if value.is_sign_negative() && !value.is_nan() {
        "" // negative sign already part of formatted body
    } else if plus {
        "+"
    } else if space {
        " "
    } else {
        ""
    }
}

fn fmt_fixed(value: f64, precision: usize, plus: bool, space: bool, _hash: bool) -> String {
    let body = format!("{:.*}", precision, value);
    let prefix = sign_prefix(value, plus, space);
    format!("{}{}", prefix, body)
}

fn fmt_exponent(
    value: f64,
    precision: usize,
    plus: bool,
    space: bool,
    _hash: bool,
    upper: bool,
) -> String {
    // Use Rust's exponent formatting then post-process to a C-like form.
    let body = if upper {
        format!("{:.*E}", precision, value)
    } else {
        format!("{:.*e}", precision, value)
    };
    let prefix = sign_prefix(value, plus, space);
    let body = c_style_exponent(&body, upper);
    format!("{}{}", prefix, body)
}

/// Convert Rust's exponent form (e.g. "1.2e3") into a C-style form ("1.2e+03").
fn c_style_exponent(s: &str, upper: bool) -> String {
    let exp_char = if upper { 'E' } else { 'e' };
    if let Some(pos) = s.find(exp_char) {
        let (mantissa, exp_part) = s.split_at(pos);
        let exp_part = &exp_part[1..];
        let (sign, digits) = if let Some(stripped) = exp_part.strip_prefix('-') {
            ('-', stripped)
        } else if let Some(stripped) = exp_part.strip_prefix('+') {
            ('+', stripped)
        } else {
            ('+', exp_part)
        };
        let padded = if digits.len() < 2 {
            format!("0{}", digits)
        } else {
            digits.to_string()
        };
        format!("{}{}{}{}", mantissa, exp_char, sign, padded)
    } else {
        s.to_string()
    }
}

fn fmt_general(
    value: f64,
    precision: usize,
    plus: bool,
    space: bool,
    hash: bool,
    upper: bool,
) -> String {
    // %g uses precision = number of significant digits (default 6, min 1).
    let p = if precision == 0 { 1 } else { precision };
    let prefix = sign_prefix(value, plus, space);

    if value == 0.0 {
        let body = if hash {
            // # flag: keep trailing zeros and decimal point.
            let zeros = if p > 1 { "0".repeat(p - 1) } else { String::new() };
            if zeros.is_empty() {
                "0.".to_string()
            } else {
                format!("0.{}", zeros)
            }
        } else {
            "0".to_string()
        };
        return format!("{}{}", prefix, body);
    }

    if !value.is_finite() {
        return format!("{}{}", prefix, value);
    }

    // Determine the exponent X of the value when written as d.ddd...e±X.
    let abs = value.abs();
    let exp_x = abs.log10().floor() as i32;

    // C's choice: use exponential form when X < -4 or X >= precision; otherwise
    // use fixed form with precision = p - 1 - X decimal places.
    let use_exp = exp_x < -4 || exp_x >= p as i32;

    let body = if use_exp {
        // Format with precision-1 fractional digits in exponential form.
        let frac = if p == 0 { 0 } else { p - 1 };
        let raw = if upper {
            format!("{:.*E}", frac, value)
        } else {
            format!("{:.*e}", frac, value)
        };
        let mut s = c_style_exponent(&raw, upper);
        if !hash {
            s = strip_trailing_zeros_g(&s, upper);
        }
        s
    } else {
        // Fixed form
        let frac = (p as i32 - 1 - exp_x).max(0) as usize;
        let raw = format!("{:.*}", frac, value);
        if hash {
            raw
        } else {
            strip_trailing_zeros_fixed(&raw)
        }
    };

    format!("{}{}", prefix, body)
}

/// Strip trailing zeros from the fractional part of a fixed-format number,
/// also stripping a trailing decimal point. Used for `%g` post-processing.
fn strip_trailing_zeros_fixed(s: &str) -> String {
    if !s.contains('.') {
        return s.to_string();
    }
    let trimmed = s.trim_end_matches('0');
    let trimmed = trimmed.trim_end_matches('.');
    trimmed.to_string()
}

/// Strip trailing zeros from the mantissa of an exponential-format number.
fn strip_trailing_zeros_g(s: &str, upper: bool) -> String {
    let exp_char = if upper { 'E' } else { 'e' };
    if let Some(pos) = s.find(exp_char) {
        let (mantissa, exp_part) = s.split_at(pos);
        let mantissa = strip_trailing_zeros_fixed(mantissa);
        format!("{}{}", mantissa, exp_part)
    } else {
        strip_trailing_zeros_fixed(s)
    }
}
