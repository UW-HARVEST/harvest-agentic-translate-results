// Translated from c_src/src/driver.c
// Original Copyright 2025 MIT Lincoln Laboratory

use crate::matrix::{initialize_matrix_from_string, matrix_to_string, multiply_matrices};
use crate::write::write_to_file;

const OUT_FILE: &str = "matrix.txt";

pub const EXIT_SUCCESS: i32 = 0;
pub const EXIT_FAILURE: i32 = 1;

pub fn driver(
    width_a: i32,
    height_a: i32,
    matrix_a: &str,
    width_b: i32,
    height_b: i32,
    matrix_b: &str,
) -> i32 {
    let mat_a = match initialize_matrix_from_string(matrix_a, width_a, height_a) {
        Some(m) => m,
        None => return EXIT_FAILURE,
    };

    let mat_b = match initialize_matrix_from_string(matrix_b, width_b, height_b) {
        Some(m) => m,
        None => return EXIT_FAILURE,
    };

    let res = match multiply_matrices(&mat_a, &mat_b) {
        Some(m) => m,
        None => return EXIT_FAILURE,
    };

    let res_str = match matrix_to_string(&res) {
        Some(s) => s,
        None => return EXIT_FAILURE,
    };

    let res_write = write_to_file(OUT_FILE, &res_str);

    if res_write != 0 {
        return EXIT_FAILURE;
    }

    EXIT_SUCCESS
}
