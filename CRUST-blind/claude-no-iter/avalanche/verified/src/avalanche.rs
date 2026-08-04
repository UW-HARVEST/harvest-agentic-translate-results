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
        // Read exactly key_size bytes; if we can't get a full key, stop.
        if !read_exact_or_stop(ins, &mut key) {
            break;
        }

        hash(&key, &mut hvalue);
        key_count += 1;

        for i_byte in 0..key_size {
            for i_bit in 0..8u32 {
                let row = i_byte * 8 + i_bit as usize;

                // flip the i-th bit of this byte and re-hash
                let i_mask: u8 = 0x80u8 >> i_bit;
                key[i_byte] ^= i_mask;
                hash(&key, &mut htemp);
                key[i_byte] ^= i_mask;

                for j_word in 0..hash_words {
                    for j_bit in 0..32u32 {
                        let col = j_word * 32 + j_bit as usize;

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
        let denom = key_count as f64;
        for v in results.vals.iter_mut() {
            *v /= denom;
        }
    }
}

/// Read exactly `buf.len()` bytes from `ins`. Returns true if filled; false if
/// the stream ended before the buffer could be filled (matching the C
/// behavior where `fread` returns less than requested at EOF).
fn read_exact_or_stop(ins: &mut dyn Read, buf: &mut [u8]) -> bool {
    let mut filled = 0usize;
    while filled < buf.len() {
        match ins.read(&mut buf[filled..]) {
            Ok(0) => return false,
            Ok(n) => filled += n,
            Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(_) => return false,
        }
    }
    true
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

/// Minimal printf-style formatter for a single double value, supporting the
/// format specifiers used by this project (e.g. "% 6.4g", "%8.4f", "%e",
/// "%-8.4f"). Anything outside the conversion specifier is copied verbatim.
fn format_double(fmt: &str, val: f64) -> String {
    let bytes = fmt.as_bytes();
    let mut out = String::new();
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 1 < bytes.len() {
            // Parse a conversion specifier: %[flags][width][.precision]conv
            let mut j = i + 1;

            // flags
            let mut left_align = false;
            let mut plus = false;
            let mut space = false;
            let mut hash = false;
            let mut zero = false;
            while j < bytes.len() {
                match bytes[j] {
                    b'-' => left_align = true,
                    b'+' => plus = true,
                    b' ' => space = true,
                    b'#' => hash = true,
                    b'0' => zero = true,
                    _ => break,
                }
                j += 1;
            }

            // width
            let mut width: Option<usize> = None;
            let mut width_val: usize = 0;
            let mut have_width = false;
            while j < bytes.len() && (b'0'..=b'9').contains(&bytes[j]) {
                width_val = width_val * 10 + (bytes[j] - b'0') as usize;
                have_width = true;
                j += 1;
            }
            if have_width {
                width = Some(width_val);
            }

            // precision
            let mut precision: Option<usize> = None;
            if j < bytes.len() && bytes[j] == b'.' {
                j += 1;
                let mut p: usize = 0;
                while j < bytes.len() && (b'0'..=b'9').contains(&bytes[j]) {
                    p = p * 10 + (bytes[j] - b'0') as usize;
                    j += 1;
                }
                precision = Some(p);
            }

            // conversion specifier
            if j < bytes.len() {
                let conv = bytes[j];
                j += 1;
                if matches!(conv, b'f' | b'F' | b'e' | b'E' | b'g' | b'G') {
                    let formatted =
                        format_conv(val, conv, precision, plus, space, hash, zero, left_align, width);
                    out.push_str(&formatted);
                } else if conv == b'%' {
                    out.push('%');
                } else {
                    // Unknown conversion: emit raw text
                    out.push_str(std::str::from_utf8(&bytes[i..j]).unwrap_or(""));
                }
                i = j;
                continue;
            } else {
                // Trailing '%': emit as-is
                out.push('%');
                i = j;
                continue;
            }
        } else {
            out.push(bytes[i] as char);
            i += 1;
        }
    }
    out
}

#[allow(clippy::too_many_arguments)]
fn format_conv(
    val: f64,
    conv: u8,
    precision: Option<usize>,
    plus: bool,
    space: bool,
    hash: bool,
    zero: bool,
    left_align: bool,
    width: Option<usize>,
) -> String {
    // Handle special values consistently with printf.
    let body: String;
    let is_negative = val.is_sign_negative() && !val.is_nan();

    if val.is_nan() {
        body = if conv.is_ascii_uppercase() { "NAN".to_string() } else { "nan".to_string() };
    } else if val.is_infinite() {
        body = if conv.is_ascii_uppercase() { "INF".to_string() } else { "inf".to_string() };
    } else {
        let abs_val = val.abs();
        match conv {
            b'f' | b'F' => {
                let p = precision.unwrap_or(6);
                body = format!("{:.*}", p, abs_val);
            }
            b'e' | b'E' => {
                let p = precision.unwrap_or(6);
                body = format_e(abs_val, p, conv == b'E');
            }
            b'g' | b'G' => {
                // %g: precision = number of significant digits (default 6,
                // 0 is treated as 1).
                let mut p = precision.unwrap_or(6);
                if p == 0 {
                    p = 1;
                }
                body = format_g(abs_val, p, conv == b'G', hash);
            }
            _ => body = format!("{}", abs_val),
        }
    }

    // Sign handling
    let sign = if is_negative {
        "-"
    } else if plus {
        "+"
    } else if space {
        " "
    } else {
        ""
    };

    let mut result = String::new();
    result.push_str(sign);
    result.push_str(&body);

    // Apply width / padding.
    if let Some(w) = width {
        if result.len() < w {
            let pad_len = w - result.len();
            if left_align {
                result.push_str(&" ".repeat(pad_len));
            } else if zero && !val.is_nan() && !val.is_infinite() {
                // zero-pad after the sign
                let mut padded = String::new();
                padded.push_str(sign);
                padded.push_str(&"0".repeat(pad_len));
                padded.push_str(&result[sign.len()..]);
                result = padded;
            } else {
                let mut padded = String::new();
                padded.push_str(&" ".repeat(pad_len));
                padded.push_str(&result);
                result = padded;
            }
        }
    }

    result
}

/// Format a non-negative finite f64 in scientific notation with a fixed
/// 2-digit (or more) exponent, matching printf's %e semantics.
fn format_e(abs_val: f64, precision: usize, upper: bool) -> String {
    if abs_val == 0.0 {
        let mantissa = if precision == 0 {
            "0".to_string()
        } else {
            format!("0.{}", "0".repeat(precision))
        };
        return format!("{}{}+00", mantissa, if upper { "E" } else { "e" });
    }
    let exp = abs_val.log10().floor() as i32;
    let mantissa = abs_val / 10f64.powi(exp);
    // Re-check after rounding: if rounding pushes mantissa to 10, bump exponent.
    let rounded = format!("{:.*}", precision, mantissa);
    let (mantissa_str, final_exp) = if rounded.starts_with("10") {
        let m2 = mantissa / 10.0;
        (format!("{:.*}", precision, m2), exp + 1)
    } else {
        (rounded, exp)
    };
    let exp_char = if upper { 'E' } else { 'e' };
    let exp_sign = if final_exp < 0 { '-' } else { '+' };
    let abs_exp = final_exp.unsigned_abs();
    format!("{}{}{}{:02}", mantissa_str, exp_char, exp_sign, abs_exp)
}

/// Format a non-negative finite f64 using the rules of printf's %g.
fn format_g(abs_val: f64, precision: usize, upper: bool, hash: bool) -> String {
    // Choose between %e and %f based on the exponent. For zero, the exponent
    // is treated as 0 so %f form is used.
    let exp = if abs_val == 0.0 {
        0
    } else {
        abs_val.log10().floor() as i32
    };

    let (mut s, used_exp) = if exp < -4 || exp >= precision as i32 {
        // %e style: precision - 1 digits after the decimal point
        let p = precision.saturating_sub(1);
        (format_e(abs_val, p, upper), true)
    } else {
        // %f style: precision - 1 - exp digits after the decimal point
        let p = (precision as i32 - 1 - exp).max(0) as usize;
        (format!("{:.*}", p, abs_val), false)
    };

    if !hash {
        // Strip trailing zeros from the fractional part, and a trailing '.'.
        if used_exp {
            // Split mantissa and exponent
            let exp_idx = s.find(['e', 'E']).unwrap();
            let (mantissa, exp_part) = s.split_at(exp_idx);
            let trimmed = trim_trailing_zeros(mantissa);
            s = format!("{}{}", trimmed, exp_part);
        } else {
            s = trim_trailing_zeros(&s).to_string();
        }
    }

    s
}

fn trim_trailing_zeros(s: &str) -> String {
    if !s.contains('.') {
        return s.to_string();
    }
    let mut end = s.len();
    let bytes = s.as_bytes();
    while end > 0 && bytes[end - 1] == b'0' {
        end -= 1;
    }
    if end > 0 && bytes[end - 1] == b'.' {
        end -= 1;
    }
    s[..end].to_string()
}
