// Translated from c_src/src/matrix.c
// Original Copyright 2025 MIT Lincoln Laboratory

use std::fmt::Write as _;

pub struct Matrix {
    pub matrix: Vec<Vec<i32>>,
    pub width: i32,
    pub height: i32,
}

impl Matrix {
    pub fn allocate(width: i32, height: i32) -> Option<Matrix> {
        if width < 0 || height < 0 {
            return None;
        }
        let rows = vec![vec![0i32; width as usize]; height as usize];
        Some(Matrix {
            matrix: rows,
            width,
            height,
        })
    }
}

pub fn initialize_matrix_from_string(input: &str, width: i32, height: i32) -> Option<Matrix> {
    let mut mat = Matrix::allocate(width, height)?;

    let mut row_iter = input.split('\n');
    for i in 0..height as usize {
        let row_token = match row_iter.next() {
            Some(tok) => tok,
            None => {
                eprintln!("Insufficient rows in input string.");
                return None;
            }
        };

        let mut col_iter = row_token.split(' ').filter(|s| !s.is_empty());
        for j in 0..width as usize {
            let col_token = match col_iter.next() {
                Some(tok) => tok,
                None => {
                    eprintln!("Insufficient columns in row {}.", i + 1);
                    return None;
                }
            };
            // atoi behavior: parse leading int, default 0 on failure
            let val: i32 = c_atoi(col_token);
            mat.matrix[i][j] = val;
        }
    }

    Some(mat)
}

// Mimics C's atoi: parses leading optional sign and digits, ignores leading whitespace,
// returns 0 if no valid digits found.
fn c_atoi(s: &str) -> i32 {
    let s = s.trim_start();
    let bytes = s.as_bytes();
    let mut idx = 0;
    let mut sign: i32 = 1;
    if idx < bytes.len() && (bytes[idx] == b'+' || bytes[idx] == b'-') {
        if bytes[idx] == b'-' {
            sign = -1;
        }
        idx += 1;
    }
    let mut result: i32 = 0;
    while idx < bytes.len() && bytes[idx].is_ascii_digit() {
        let digit = (bytes[idx] - b'0') as i32;
        result = result.wrapping_mul(10).wrapping_add(digit);
        idx += 1;
    }
    result.wrapping_mul(sign)
}

pub fn multiply_matrices(mat_a: &Matrix, mat_b: &Matrix) -> Option<Matrix> {
    if mat_a.width != mat_b.height {
        eprintln!("Matrix dimensions do not allow multiplication.");
        return None;
    }

    let mut result = Matrix::allocate(mat_b.width, mat_a.height)?;
    for i in 0..mat_a.height as usize {
        for j in 0..mat_b.width as usize {
            let mut sum: i32 = 0;
            for k in 0..mat_a.width as usize {
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
            // ignore Result for write! into String (cannot fail)
            let _ = write!(result, "{}", mat.matrix[i][j]);
            if (j as i32) < mat.width - 1 {
                result.push(' ');
            }
        }
        result.push('\n');
    }
    Some(result)
}
