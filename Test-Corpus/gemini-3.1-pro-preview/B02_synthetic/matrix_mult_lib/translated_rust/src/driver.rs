use std::os::raw::{c_char, c_int};

use crate::matrix::{initialize_matrix_from_string, multiply_matrices, matrix_to_string, free_matrix};
use crate::write::write_to_file;

const OUT_FILE: &[u8] = b"matrix.txt\0";

#[unsafe(no_mangle)]
pub extern "C" fn driver(
    width_a: c_int,
    height_a: c_int,
    matrix_a: *const c_char,
    width_b: c_int,
    height_b: c_int,
    matrix_b: *const c_char,
) -> c_int {
    let mat_a = initialize_matrix_from_string(matrix_a, width_a, height_a);
    if mat_a.is_null() {
        return 1;
    }

    let mat_b = initialize_matrix_from_string(matrix_b, width_b, height_b);
    if mat_b.is_null() {
        free_matrix(mat_a);
        return 1;
    }

    let res = multiply_matrices(mat_a, mat_b);
    if res.is_null() {
        free_matrix(mat_a);
        free_matrix(mat_b);
        return 1;
    }

    let res_str = matrix_to_string(res);
    if res_str.is_null() {
        free_matrix(mat_a);
        free_matrix(mat_b);
        free_matrix(res);
        return 1;
    }

    let res_write = write_to_file(OUT_FILE.as_ptr() as *const c_char, res_str);

    free_matrix(mat_a);
    free_matrix(mat_b);
    free_matrix(res);
    unsafe {
        libc::free(res_str as *mut libc::c_void);
    }

    if res_write != 0 {
        return 1;
    }

    0
}
