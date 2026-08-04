// Translation of c_src/src/matrix.c

use std::io::Write;

pub struct Matrix {
    pub matrix: Vec<Vec<i32>>,
    pub width: i32,
    pub height: i32,
}

pub fn allocate_matrix(width: i32, height: i32) -> Option<Matrix> {
    // Behaves like the C version: allocates the rows/columns. Allocation
    // failures in safe Rust effectively panic, so the explicit NULL paths
    // are unreachable here and we always succeed on valid sizes.
    let mut rows: Vec<Vec<i32>> = Vec::with_capacity(height.max(0) as usize);
    for _ in 0..height.max(0) {
        rows.push(vec![0i32; width.max(0) as usize]);
    }
    Some(Matrix {
        matrix: rows,
        width,
        height,
    })
}

pub fn initialize_matrix_from_string(input: &str, width: i32, height: i32) -> Option<Matrix> {
    let mut mat = allocate_matrix(width, height)?;

    // Mimic strtok_r behavior: split on '\n', then split each row on ' '.
    // strtok_r treats consecutive delimiters as a single delimiter and
    // skips leading delimiters; replicate that with split + filter empty.
    let row_tokens: Vec<&str> = input
        .split('\n')
        .filter(|s| !s.is_empty())
        .collect();

    for i in 0..height as usize {
        let row_token = match row_tokens.get(i) {
            Some(t) => *t,
            None => {
                let _ = writeln!(std::io::stderr(), "Insufficient rows in input string.");
                return None;
            }
        };

        let col_tokens: Vec<&str> = row_token
            .split(' ')
            .filter(|s| !s.is_empty())
            .collect();

        for j in 0..width as usize {
            let col_token = match col_tokens.get(j) {
                Some(t) => *t,
                None => {
                    let _ = writeln!(
                        std::io::stderr(),
                        "Insufficient columns in row {}.",
                        i as i32 + 1
                    );
                    return None;
                }
            };
            // Mimic atoi: parses a leading optional sign and digits, returning 0
            // if no conversion possible. Stops at first non-digit.
            mat.matrix[i][j] = c_atoi(col_token);
        }
    }

    Some(mat)
}

fn c_atoi(s: &str) -> i32 {
    let bytes = s.as_bytes();
    let mut i = 0usize;
    // Skip leading whitespace like atoi does
    while i < bytes.len() && (bytes[i] as char).is_whitespace() {
        i += 1;
    }
    let mut sign: i32 = 1;
    if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
        if bytes[i] == b'-' {
            sign = -1;
        }
        i += 1;
    }
    let mut result: i32 = 0;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        let digit = (bytes[i] - b'0') as i32;
        // Mimic C atoi which has UB on overflow; use wrapping arithmetic
        result = result.wrapping_mul(10).wrapping_add(digit);
        i += 1;
    }
    result.wrapping_mul(sign)
}

pub fn multiply_matrices(mat_a: &Matrix, mat_b: &Matrix) -> Option<Matrix> {
    if mat_a.width != mat_b.height {
        let _ = writeln!(
            std::io::stderr(),
            "Matrix dimensions do not allow multiplication."
        );
        return None;
    }

    let mut result = allocate_matrix(mat_b.width, mat_a.height)?;
    for i in 0..mat_a.height as usize {
        for j in 0..mat_b.width as usize {
            let mut sum: i32 = 0;
            for k in 0..mat_a.width as usize {
                // Match C int arithmetic semantics with wrapping ops.
                sum = sum.wrapping_add(mat_a.matrix[i][k].wrapping_mul(mat_b.matrix[k][j]));
            }
            result.matrix[i][j] = sum;
        }
    }
    Some(result)
}

pub fn matrix_to_string(mat: &Matrix) -> Option<String> {
    let mut result = String::new();
    for i in 0..mat.height as usize {
        for j in 0..mat.width as usize {
            result.push_str(&format!("{}", mat.matrix[i][j]));
            if (j as i32) < mat.width - 1 {
                result.push(' ');
            }
        }
        result.push('\n');
    }
    Some(result)
}
