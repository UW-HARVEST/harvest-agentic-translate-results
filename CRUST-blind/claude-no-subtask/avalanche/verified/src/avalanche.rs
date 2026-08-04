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
        // Read exactly `key_size` bytes; if EOF reached early, break.
        let n_in = read_exact_or_short(ins, &mut key);
        if n_in < key_size {
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
                        if ((hvalue[j_word] ^ htemp[j_word]) & j_mask) != 0 {
                            let curr = results.matrix_get(row, col);
                            results.matrix_set(row, col, curr + 1.0);
                        }
                    }
                }
            }
        }
    }

    if key_count > 0 {
        // convert matrix entries to the mean values
        let kc = key_count as f64;
        for v in results.vals.iter_mut() {
            *v /= kc;
        }
    }
}

/// Read up to `buf.len()` bytes from `r` into `buf`, returning the number of
/// bytes actually read. This mirrors the behavior of C's `fread` where a
/// short read indicates EOF.
fn read_exact_or_short(r: &mut dyn Read, buf: &mut [u8]) -> usize {
    let mut total = 0usize;
    while total < buf.len() {
        match r.read(&mut buf[total..]) {
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

/// Apply a single C-style printf format specifier (for floating-point types)
/// to the supplied f64 and emit the formatted text. Supports flags
/// (`-`, `+`, ` `, `0`, `#`), width, precision, and specifiers `f`, `F`,
/// `e`, `E`, `g`, `G`. Any other characters are passed through verbatim.
fn format_double(fmt: &str, val: f64) -> String {
    let bytes = fmt.as_bytes();
    let mut out = String::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 1 < bytes.len() {
            // Handle "%%" -> literal '%'
            if bytes[i + 1] == b'%' {
                out.push('%');
                i += 2;
                continue;
            }
            i += 1;
            // Parse flags
            let mut left_align = false;
            let mut plus = false;
            let mut space = false;
            let mut zero = false;
            let mut hash_flag = false;
            while i < bytes.len() {
                match bytes[i] {
                    b'-' => left_align = true,
                    b'+' => plus = true,
                    b' ' => space = true,
                    b'0' => zero = true,
                    b'#' => hash_flag = true,
                    _ => break,
                }
                i += 1;
            }
            // Parse width
            let mut width: usize = 0;
            let mut had_width = false;
            while i < bytes.len() && bytes[i].is_ascii_digit() {
                width = width * 10 + (bytes[i] - b'0') as usize;
                had_width = true;
                i += 1;
            }
            let _ = had_width;
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
            if i >= bytes.len() {
                break;
            }
            let spec = bytes[i];
            i += 1;

            let formatted = format_one_double(
                val, spec, precision, width, left_align, plus, space, zero, hash_flag,
            );
            out.push_str(&formatted);
        } else {
            out.push(bytes[i] as char);
            i += 1;
        }
    }
    out
}

#[allow(clippy::too_many_arguments)]
fn format_one_double(
    val: f64,
    spec: u8,
    precision: Option<usize>,
    width: usize,
    left_align: bool,
    plus: bool,
    space: bool,
    zero: bool,
    hash_flag: bool,
) -> String {
    // Sign handling
    let is_neg = val.is_sign_negative() && !val.is_nan();
    let sign_str: &str = if is_neg {
        "-"
    } else if plus {
        "+"
    } else if space {
        " "
    } else {
        ""
    };
    let abs_val = if is_neg { -val } else { val };

    let prec = precision.unwrap_or(6);
    let core: String = match spec {
        b'f' | b'F' => format!("{:.*}", prec, abs_val),
        b'e' => format_e(abs_val, prec, false),
        b'E' => format_e(abs_val, prec, true),
        b'g' => format_g(abs_val, prec, false, hash_flag),
        b'G' => format_g(abs_val, prec, true, hash_flag),
        _ => {
            // Unknown specifier - fall back to default float printing
            format!("{}", abs_val)
        }
    };

    let total_len = sign_str.len() + core.len();
    if total_len >= width {
        format!("{}{}", sign_str, core)
    } else {
        let pad = width - total_len;
        if left_align {
            let mut s = String::with_capacity(width);
            s.push_str(sign_str);
            s.push_str(&core);
            for _ in 0..pad {
                s.push(' ');
            }
            s
        } else if zero {
            // Zero-pad after sign
            let mut s = String::with_capacity(width);
            s.push_str(sign_str);
            for _ in 0..pad {
                s.push('0');
            }
            s.push_str(&core);
            s
        } else {
            let mut s = String::with_capacity(width);
            for _ in 0..pad {
                s.push(' ');
            }
            s.push_str(sign_str);
            s.push_str(&core);
            s
        }
    }
}

/// Format a non-negative f64 in scientific (`%e`) form with the given precision.
fn format_e(val: f64, precision: usize, upper: bool) -> String {
    if !val.is_finite() {
        return format_special(val, upper);
    }
    if val == 0.0 {
        let mut s = String::new();
        s.push('0');
        if precision > 0 {
            s.push('.');
            for _ in 0..precision {
                s.push('0');
            }
        }
        s.push_str(if upper { "E+00" } else { "e+00" });
        return s;
    }

    // Compute exponent and mantissa.
    let exp_floor = val.log10().floor() as i32;
    let mut mantissa = val / 10f64.powi(exp_floor);
    // Round mantissa to specified precision.
    let factor = 10f64.powi(precision as i32);
    let mut rounded = (mantissa * factor).round() / factor;
    let mut exp_final = exp_floor;
    // Carry: if rounding pushes mantissa to >= 10, shift exponent.
    if rounded >= 10.0 {
        rounded /= 10.0;
        exp_final += 1;
    }
    mantissa = rounded;

    let mantissa_str = format!("{:.*}", precision, mantissa);
    let exp_sign = if exp_final < 0 { '-' } else { '+' };
    let exp_abs = exp_final.unsigned_abs();
    let exp_letter = if upper { 'E' } else { 'e' };
    // C's %e uses at least 2 digits in the exponent.
    if exp_abs < 100 {
        format!("{}{}{}{:02}", mantissa_str, exp_letter, exp_sign, exp_abs)
    } else {
        format!("{}{}{}{}", mantissa_str, exp_letter, exp_sign, exp_abs)
    }
}

/// Format a non-negative f64 in `%g` style with the given precision.
fn format_g(val: f64, precision: usize, upper: bool, hash_flag: bool) -> String {
    if !val.is_finite() {
        return format_special(val, upper);
    }
    // %g precision is the number of significant digits; default is 6, 0 -> 1.
    let p = if precision == 0 { 1 } else { precision };

    if val == 0.0 {
        if hash_flag {
            // Keep trailing zeros and decimal point.
            let mut s = String::from("0");
            if p > 1 {
                s.push('.');
                for _ in 0..(p - 1) {
                    s.push('0');
                }
            } else {
                s.push('.');
            }
            return s;
        }
        return "0".to_string();
    }

    // Compute the exponent X used by %g to choose the format style.
    let exp_x = val.log10().floor() as i32;
    // If P > X >= -4, use fixed format; else scientific.
    if (exp_x < -4) || (exp_x >= p as i32) {
        // Scientific style with precision = p - 1.
        let s = format_e(val, p - 1, upper);
        if hash_flag {
            s
        } else {
            strip_trailing_zeros_e(&s, upper)
        }
    } else {
        // Fixed style with decimals = p - 1 - X.
        let decimals = (p as i32 - 1 - exp_x).max(0) as usize;
        let s = format!("{:.*}", decimals, val);
        if hash_flag {
            s
        } else {
            strip_trailing_zeros_f(&s)
        }
    }
}

fn format_special(val: f64, upper: bool) -> String {
    if val.is_nan() {
        if upper { "NAN".to_string() } else { "nan".to_string() }
    } else if val.is_infinite() {
        if upper { "INF".to_string() } else { "inf".to_string() }
    } else {
        // Should not happen.
        format!("{}", val)
    }
}

/// Strip trailing zeros (and a trailing decimal point) from a fixed-point
/// number representation produced by Rust's default `{:.N}` formatting.
fn strip_trailing_zeros_f(s: &str) -> String {
    if !s.contains('.') {
        return s.to_string();
    }
    let trimmed = s.trim_end_matches('0');
    let trimmed = trimmed.trim_end_matches('.');
    trimmed.to_string()
}

/// Strip trailing zeros from the mantissa portion of a scientific form string
/// produced by `format_e`. Removes a trailing decimal point if no fractional
/// digits remain. The exponent portion is preserved.
fn strip_trailing_zeros_e(s: &str, upper: bool) -> String {
    let exp_letter = if upper { 'E' } else { 'e' };
    if let Some(pos) = s.find(exp_letter) {
        let (mantissa, exp_part) = s.split_at(pos);
        let mantissa_trimmed = if mantissa.contains('.') {
            let t = mantissa.trim_end_matches('0');
            let t = t.trim_end_matches('.');
            t.to_string()
        } else {
            mantissa.to_string()
        };
        format!("{}{}", mantissa_trimmed, exp_part)
    } else {
        s.to_string()
    }
}
