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
        // Read exactly key_size bytes; if fewer are available, break (matches fread behavior).
        let mut total_read = 0usize;
        let mut read_failed = false;
        while total_read < key_size {
            match ins.read(&mut key[total_read..]) {
                Ok(0) => {
                    read_failed = true;
                    break;
                }
                Ok(n) => total_read += n,
                Err(_) => {
                    read_failed = true;
                    break;
                }
            }
        }
        if read_failed || total_read < key_size {
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
                let v = self.matrix_get(r, c);
                let s = format_f64_printf(format, v);
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

/// A minimal printf-style formatter for `f64` values. Supports the format
/// specifiers used by this project: `%[flags][width][.precision]<conv>` where
/// `conv` is one of `f`, `e`, `E`, `g`, `G`. Flags handled: `-`, `+`, ` `, `0`, `#`.
fn format_f64_printf(format: &str, value: f64) -> String {
    let bytes = format.as_bytes();
    let mut out = String::new();
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        if b != b'%' {
            out.push(b as char);
            i += 1;
            continue;
        }
        // we have a '%'
        i += 1;
        if i < bytes.len() && bytes[i] == b'%' {
            out.push('%');
            i += 1;
            continue;
        }
        // parse flags
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
        // parse width
        let mut width: Option<usize> = None;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            let d = (bytes[i] - b'0') as usize;
            width = Some(width.unwrap_or(0) * 10 + d);
            i += 1;
        }
        // parse precision
        let mut precision: Option<usize> = None;
        if i < bytes.len() && bytes[i] == b'.' {
            i += 1;
            let mut p: usize = 0;
            let mut has_digits = false;
            while i < bytes.len() && bytes[i].is_ascii_digit() {
                p = p * 10 + (bytes[i] - b'0') as usize;
                has_digits = true;
                i += 1;
            }
            precision = Some(if has_digits { p } else { 0 });
        }
        // skip length modifiers (l, h, L, etc.) — not relevant for f64
        while i < bytes.len()
            && (bytes[i] == b'l' || bytes[i] == b'h' || bytes[i] == b'L')
        {
            i += 1;
        }
        if i >= bytes.len() {
            break;
        }
        let conv = bytes[i];
        i += 1;
        let body = match conv {
            b'f' | b'F' => format_fixed(value, precision.unwrap_or(6), flag_plus, flag_space, flag_hash),
            b'e' => format_exp(value, precision.unwrap_or(6), flag_plus, flag_space, flag_hash, false),
            b'E' => format_exp(value, precision.unwrap_or(6), flag_plus, flag_space, flag_hash, true),
            b'g' => format_general(value, precision.unwrap_or(6), flag_plus, flag_space, flag_hash, false),
            b'G' => format_general(value, precision.unwrap_or(6), flag_plus, flag_space, flag_hash, true),
            b'd' | b'i' => {
                let v = value as i64;
                let mut s = if v < 0 {
                    format!("-{}", -v)
                } else if flag_plus {
                    format!("+{}", v)
                } else if flag_space {
                    format!(" {}", v)
                } else {
                    format!("{}", v)
                };
                if let Some(p) = precision {
                    let digits_only: String = s.trim_start_matches(|c: char| c == '+' || c == '-' || c == ' ').to_string();
                    let sign: String = s[..s.len() - digits_only.len()].to_string();
                    if digits_only.len() < p {
                        let pad = "0".repeat(p - digits_only.len());
                        s = format!("{}{}{}", sign, pad, digits_only);
                    }
                }
                s
            }
            _ => {
                // Unsupported conversion — emit raw.
                let mut s = String::from("%");
                s.push(conv as char);
                s
            }
        };
        // apply width
        let result = apply_width(&body, width, flag_minus, flag_zero, conv);
        out.push_str(&result);
    }
    out
}

fn apply_width(body: &str, width: Option<usize>, left: bool, zero: bool, conv: u8) -> String {
    let w = match width {
        Some(w) => w,
        None => return body.to_string(),
    };
    if body.len() >= w {
        return body.to_string();
    }
    let pad = w - body.len();
    if left {
        let mut s = String::with_capacity(w);
        s.push_str(body);
        s.push_str(&" ".repeat(pad));
        s
    } else if zero
        && (conv == b'f'
            || conv == b'F'
            || conv == b'e'
            || conv == b'E'
            || conv == b'g'
            || conv == b'G'
            || conv == b'd'
            || conv == b'i')
    {
        // zero-pad after the sign character if there is one
        let bytes = body.as_bytes();
        if !bytes.is_empty() && (bytes[0] == b'+' || bytes[0] == b'-' || bytes[0] == b' ') {
            let mut s = String::with_capacity(w);
            s.push(bytes[0] as char);
            s.push_str(&"0".repeat(pad));
            s.push_str(&body[1..]);
            s
        } else {
            let mut s = String::with_capacity(w);
            s.push_str(&"0".repeat(pad));
            s.push_str(body);
            s
        }
    } else {
        let mut s = String::with_capacity(w);
        s.push_str(&" ".repeat(pad));
        s.push_str(body);
        s
    }
}

fn sign_prefix(value: f64, flag_plus: bool, flag_space: bool) -> &'static str {
    if value.is_sign_negative() && !value.is_nan() {
        "-"
    } else if flag_plus {
        "+"
    } else if flag_space {
        " "
    } else {
        ""
    }
}

fn format_fixed(
    value: f64,
    precision: usize,
    flag_plus: bool,
    flag_space: bool,
    flag_hash: bool,
) -> String {
    if value.is_nan() {
        return "nan".to_string();
    }
    if value.is_infinite() {
        let mut s = String::new();
        s.push_str(sign_prefix(value, flag_plus, flag_space));
        s.push_str("inf");
        return s;
    }
    let absv = value.abs();
    let body = format!("{:.*}", precision, absv);
    let body = if flag_hash && precision == 0 && !body.contains('.') {
        format!("{}.", body)
    } else {
        body
    };
    let mut s = String::new();
    s.push_str(sign_prefix(value, flag_plus, flag_space));
    s.push_str(&body);
    s
}

fn format_exp(
    value: f64,
    precision: usize,
    flag_plus: bool,
    flag_space: bool,
    flag_hash: bool,
    upper: bool,
) -> String {
    if value.is_nan() {
        return if upper { "NAN".into() } else { "nan".into() };
    }
    if value.is_infinite() {
        let mut s = String::new();
        s.push_str(sign_prefix(value, flag_plus, flag_space));
        s.push_str(if upper { "INF" } else { "inf" });
        return s;
    }
    let absv = value.abs();
    // Determine mantissa and exponent in base 10.
    let (mantissa, exp) = if absv == 0.0 {
        (0.0_f64, 0_i32)
    } else {
        let exp = absv.log10().floor() as i32;
        let mant = absv / 10f64.powi(exp);
        // round mantissa to required precision
        let factor = 10f64.powi(precision as i32);
        let rounded = (mant * factor).round() / factor;
        // adjust if rounding pushed mantissa to 10
        if rounded >= 10.0 {
            (rounded / 10.0, exp + 1)
        } else if rounded < 1.0 && rounded > 0.0 {
            (rounded * 10.0, exp - 1)
        } else {
            (rounded, exp)
        }
    };
    let mantissa_str = format!("{:.*}", precision, mantissa);
    let mantissa_str = if flag_hash && precision == 0 && !mantissa_str.contains('.') {
        format!("{}.", mantissa_str)
    } else {
        mantissa_str
    };
    let e_char = if upper { 'E' } else { 'e' };
    let exp_sign = if exp < 0 { '-' } else { '+' };
    let exp_abs = exp.abs();
    let exp_str = format!("{}{}{:02}", e_char, exp_sign, exp_abs);
    let mut s = String::new();
    s.push_str(sign_prefix(value, flag_plus, flag_space));
    s.push_str(&mantissa_str);
    s.push_str(&exp_str);
    s
}

fn format_general(
    value: f64,
    precision: usize,
    flag_plus: bool,
    flag_space: bool,
    flag_hash: bool,
    upper: bool,
) -> String {
    if value.is_nan() {
        return if upper { "NAN".into() } else { "nan".into() };
    }
    if value.is_infinite() {
        let mut s = String::new();
        s.push_str(sign_prefix(value, flag_plus, flag_space));
        s.push_str(if upper { "INF" } else { "inf" });
        return s;
    }
    // Per C99: if precision is 0, treat as 1.
    let p = if precision == 0 { 1 } else { precision };
    let absv = value.abs();
    // compute decimal exponent X
    let x = if absv == 0.0 {
        0_i32
    } else {
        absv.log10().floor() as i32
    };
    // Decide which style.
    let use_exp = !(x >= -4 && x < p as i32);
    let s = if use_exp {
        format_exp(value, p - 1, flag_plus, flag_space, flag_hash, upper)
    } else {
        // %f with precision = p - 1 - x
        let prec = (p as i32 - 1 - x).max(0) as usize;
        format_fixed(value, prec, flag_plus, flag_space, flag_hash)
    };
    if flag_hash {
        // # flag: keep trailing zeros and decimal point — already handled by passing flag_hash through.
        s
    } else {
        // Strip trailing zeros after decimal point and trailing decimal point.
        strip_trailing_zeros_general(&s, use_exp, upper)
    }
}

fn strip_trailing_zeros_general(s: &str, use_exp: bool, upper: bool) -> String {
    let e_marker = if upper { 'E' } else { 'e' };
    if use_exp {
        // split on e/E
        let (mant, exp_part) = match s.find(e_marker) {
            Some(idx) => (&s[..idx], &s[idx..]),
            None => (s, ""),
        };
        if mant.contains('.') {
            let mant_trimmed = mant.trim_end_matches('0');
            let mant_trimmed = mant_trimmed.trim_end_matches('.');
            format!("{}{}", mant_trimmed, exp_part)
        } else {
            s.to_string()
        }
    } else if s.contains('.') {
        let trimmed = s.trim_end_matches('0');
        let trimmed = trimmed.trim_end_matches('.');
        trimmed.to_string()
    } else {
        s.to_string()
    }
}
