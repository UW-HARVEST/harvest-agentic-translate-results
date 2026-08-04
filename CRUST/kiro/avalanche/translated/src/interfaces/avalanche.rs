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
                    let diff = hvalue[j_word] ^ htemp[j_word];
                    for j_bit in 0..32u32 {
                        let col = j_word * 32 + j_bit as usize;
                        let j_mask: u32 = 0x80000000 >> j_bit;
                        if diff & j_mask != 0 {
                            results.vals[row * results.n_cols + col] += 1.0;
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
                let val = self.vals[r * self.n_cols + c];
                let s = c_format(format, val);
                let _ = write!(fout, "{}", s);
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

/// Minimal C-style printf formatter for a single f64 value.
/// Supports: %f, %g, %e with optional width, precision, space/+ flags.
fn c_format(fmt: &str, val: f64) -> String {
    let bytes = fmt.as_bytes();
    let mut i = 0;
    // find '%'
    while i < bytes.len() && bytes[i] != b'%' {
        i += 1;
    }
    if i >= bytes.len() {
        return fmt.to_string();
    }
    i += 1; // skip '%'

    // flags
    let mut flag_space = false;
    let mut flag_plus = false;
    let mut flag_minus = false;
    let mut flag_zero = false;
    while i < bytes.len() {
        match bytes[i] {
            b' ' => flag_space = true,
            b'+' => flag_plus = true,
            b'-' => flag_minus = true,
            b'0' => flag_zero = true,
            _ => break,
        }
        i += 1;
    }

    // width
    let mut width: usize = 0;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        width = width * 10 + (bytes[i] - b'0') as usize;
        i += 1;
    }

    // precision
    let mut precision: Option<usize> = None;
    if i < bytes.len() && bytes[i] == b'.' {
        i += 1;
        let mut p = 0usize;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            p = p * 10 + (bytes[i] - b'0') as usize;
            i += 1;
        }
        precision = Some(p);
    }

    if i >= bytes.len() {
        return fmt.to_string();
    }
    let spec = bytes[i] as char;

    let prec = precision.unwrap_or(if spec == 'g' { 6 } else { 6 });

    let mut num_str = match spec {
        'f' => format!("{:.prec$}", val, prec = prec),
        'e' => format!("{:.prec$e}", val, prec = prec),
        'g' => format_g(val, prec),
        _ => format!("{}", val),
    };

    // prefix for sign/space
    if !num_str.starts_with('-') {
        if flag_plus {
            num_str.insert(0, '+');
        } else if flag_space {
            num_str.insert(0, ' ');
        }
    }

    // pad to width
    if num_str.len() < width {
        let pad = width - num_str.len();
        if flag_minus {
            num_str.push_str(&" ".repeat(pad));
        } else if flag_zero && !flag_minus {
            // insert zeros after sign
            let insert_pos = if num_str.starts_with('-') || num_str.starts_with('+') || num_str.starts_with(' ') { 1 } else { 0 };
            num_str.insert_str(insert_pos, &"0".repeat(pad));
        } else {
            num_str = " ".repeat(pad) + &num_str;
        }
    }

    num_str
}

/// Format like C's %g: use shortest of %f and %e with given significant digits,
/// trailing zeros removed.
fn format_g(val: f64, prec: usize) -> String {
    if val == 0.0 {
        return if val.is_sign_negative() { "-0".to_string() } else { "0".to_string() };
    }
    let p = if prec == 0 { 1 } else { prec };
    let exp = val.abs().log10().floor() as i32;
    if exp >= -(1) && exp < p as i32 {
        // use fixed notation with (p - 1 - exp) decimal places
        let dec = (p as i32 - 1 - exp).max(0) as usize;
        let s = format!("{:.dec$}", val, dec = dec);
        trim_trailing_zeros(&s)
    } else {
        // use scientific notation with p-1 decimal places
        let dec = p - 1;
        let s = format!("{:.dec$e}", val, dec = dec);
        // Rust uses 'e' lowercase; C uses 'e' with at least 2-digit exponent
        fix_exponent(&trim_trailing_zeros_exp(&s))
    }
}

fn trim_trailing_zeros(s: &str) -> String {
    if !s.contains('.') {
        return s.to_string();
    }
    let s = s.trim_end_matches('0');
    let s = s.trim_end_matches('.');
    s.to_string()
}

fn trim_trailing_zeros_exp(s: &str) -> String {
    if let Some(epos) = s.find('e') {
        let mantissa = &s[..epos];
        let exp_part = &s[epos..];
        let trimmed = trim_trailing_zeros(mantissa);
        format!("{}{}", trimmed, exp_part)
    } else {
        s.to_string()
    }
}

fn fix_exponent(s: &str) -> String {
    // Convert Rust's "1.5e2" to C-style "1.5e+02", "1.5e-2" to "1.5e-02"
    if let Some(epos) = s.find('e') {
        let mantissa = &s[..epos];
        let exp_str = &s[epos + 1..];
        let (sign, digits) = if exp_str.starts_with('-') {
            ("-", &exp_str[1..])
        } else if exp_str.starts_with('+') {
            ("+", &exp_str[1..])
        } else {
            ("+", exp_str)
        };
        let exp_num: i32 = digits.parse().unwrap_or(0);
        if exp_num.abs() < 10 {
            format!("{}e{}{:02}", mantissa, sign, exp_num.abs())
        } else if exp_num.abs() < 100 {
            format!("{}e{}{:02}", mantissa, sign, exp_num.abs())
        } else {
            format!("{}e{}{:03}", mantissa, sign, exp_num.abs())
        }
    } else {
        s.to_string()
    }
}
