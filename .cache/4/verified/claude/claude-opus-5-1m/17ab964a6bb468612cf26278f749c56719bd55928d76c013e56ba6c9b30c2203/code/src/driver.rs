/*
 * Copyright 2025 MIT Lincoln Laboratory
 * Permission is hereby granted, free of charge,
 * to any person obtaining a copy of this software
 * and associated documentation files (the "Software"),
 * to deal in the Software without restriction,
 * including without limitation the rights to use, copy,
 * modify, merge, publish, distribute, sublicense,
 * and/or sell copies of the Software,
 * and to permit persons to whom the Software is furnished to do so,
 * subject to the following conditions:
 *
 * The above copyright notice and this permission notice
 * shall be included in all copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
 * EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
 * THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
 * IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE
 * FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT,
 * TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE
 * OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
 */

//! Translation of `c_src/src/driver.c`.

use core::ffi::{c_char, c_int, c_void};

use crate::cffi::{free, EXIT_FAILURE, EXIT_SUCCESS};
use crate::matrix::{free_matrix, initialize_matrix_from_string, matrix_to_string, multiply_matrices};
use crate::write::write_to_file;

/// `#define OUT_FILE "matrix.txt"`
const OUT_FILE: &core::ffi::CStr = c"matrix.txt";

#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(
    width_a: c_int,
    height_a: c_int,
    matrix_a: *const c_char,
    width_b: c_int,
    height_b: c_int,
    matrix_b: *const c_char,
) -> c_int {
    unsafe {
        let mat_a = initialize_matrix_from_string(matrix_a, width_a, height_a);
        if mat_a.is_null() {
            return EXIT_FAILURE;
        }
        let mat_b = initialize_matrix_from_string(matrix_b, width_b, height_b);
        if mat_b.is_null() {
            free_matrix(mat_a);
            return EXIT_FAILURE;
        }

        let res = multiply_matrices(mat_a, mat_b);
        if res.is_null() {
            free_matrix(mat_a);
            free_matrix(mat_b);
            return EXIT_FAILURE;
        }
        let res_str = matrix_to_string(res);
        if res_str.is_null() {
            free_matrix(mat_a);
            free_matrix(mat_b);
            // NOTE: the original C code releases only the `matrix_t` struct
            // here (`free(res)`), leaking the row allocations.  Reproduced
            // verbatim.
            free(res as *mut c_void);
            return EXIT_FAILURE;
        }

        let res_write = write_to_file(OUT_FILE.as_ptr(), res_str);

        free_matrix(mat_a);
        free_matrix(mat_b);
        free_matrix(res);
        free(res_str as *mut c_void);

        if res_write != 0 {
            return EXIT_FAILURE;
        }

        EXIT_SUCCESS
    }
}
