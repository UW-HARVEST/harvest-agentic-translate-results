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

//! Translation of `c_src/src/matrix.c`.

#![allow(non_camel_case_types)]

use core::ffi::{c_char, c_int, c_void};
use core::mem::size_of;
use core::ptr;

use crate::cffi::{
    atoi, fprintf, free, int_to_size, malloc, perror, snprintf, stderr_stream, strcat, strdup,
    strtok_r,
};

/// Mirror of the `matrix_t` type declared in `include/matrix.h`.
///
/// ```c
/// typedef struct {
///     int** matrix;
///     int width;
///     int height;
/// } matrix_t;
/// ```
#[repr(C)]
pub struct matrix_t {
    pub matrix: *mut *mut c_int,
    pub width: c_int,
    pub height: c_int,
}

/// `mat->matrix[i]` – the pointer to row `i`.
#[inline]
unsafe fn row_ptr(mat: *mut matrix_t, i: c_int) -> *mut c_int {
    unsafe { *(*mat).matrix.offset(i as isize) }
}

/// `&mat->matrix[i]` – the slot holding the pointer to row `i`.
#[inline]
unsafe fn row_slot(mat: *mut matrix_t, i: c_int) -> *mut *mut c_int {
    unsafe { (*mat).matrix.offset(i as isize) }
}

/// `&mat->matrix[i][j]` – the element at row `i`, column `j`.
#[inline]
unsafe fn elem_ptr(mat: *mut matrix_t, i: c_int, j: c_int) -> *mut c_int {
    unsafe { row_ptr(mat, i).offset(j as isize) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn allocate_matrix(width: c_int, height: c_int) -> *mut matrix_t {
    unsafe {
        let mat = malloc(size_of::<matrix_t>() as u64) as *mut matrix_t;
        if mat.is_null() {
            perror(c"Failed to allocate memory for matrix struct".as_ptr());
            return ptr::null_mut();
        }

        (*mat).width = width;
        (*mat).height = height;

        (*mat).matrix = malloc(
            int_to_size(height).wrapping_mul(size_of::<*mut c_int>() as u64),
        ) as *mut *mut c_int;
        if (*mat).matrix.is_null() {
            perror(c"Failed to allocate memory for matrix rows".as_ptr());
            free(mat as *mut c_void);
            return ptr::null_mut();
        }

        let mut i: c_int = 0;
        while i < height {
            *row_slot(mat, i) =
                malloc(int_to_size(width).wrapping_mul(size_of::<c_int>() as u64)) as *mut c_int;
            if row_ptr(mat, i).is_null() {
                perror(c"Failed to allocate memory for matrix columns".as_ptr());
                let mut j: c_int = 0;
                while j <= i {
                    free(row_ptr(mat, j) as *mut c_void);
                    j += 1;
                }
                free((*mat).matrix as *mut c_void);
                free(mat as *mut c_void);
                return ptr::null_mut();
            }
            i += 1;
        }

        mat
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn free_matrix(mat: *mut matrix_t) {
    unsafe {
        if mat.is_null() {
            return;
        }

        let mut i: c_int = 0;
        while i < (*mat).height {
            free(row_ptr(mat, i) as *mut c_void);
            i += 1;
        }
        free((*mat).matrix as *mut c_void);
        free(mat as *mut c_void);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn initialize_matrix_from_string(
    input: *const c_char,
    width: c_int,
    height: c_int,
) -> *mut matrix_t {
    unsafe {
        let mat = allocate_matrix(width, height);

        let input_copy = strdup(input);
        if input_copy.is_null() {
            perror(c"Failed to duplicate input string".as_ptr());
            free_matrix(mat);
            return ptr::null_mut();
        }

        let mut saveptr_row: *mut c_char = ptr::null_mut();
        let mut row_token = strtok_r(input_copy, c"\n".as_ptr(), &mut saveptr_row);
        let mut i: c_int = 0;
        while i < height {
            if row_token.is_null() {
                fprintf(
                    stderr_stream(),
                    c"Insufficient rows in input string.\n".as_ptr(),
                );
                free(input_copy as *mut c_void);
                free_matrix(mat);
                return ptr::null_mut();
            }

            let mut saveptr_col: *mut c_char = ptr::null_mut();
            let mut col_token = strtok_r(row_token, c" ".as_ptr(), &mut saveptr_col);
            let mut j: c_int = 0;
            while j < width {
                if col_token.is_null() {
                    fprintf(
                        stderr_stream(),
                        c"Insufficient columns in row %d.\n".as_ptr(),
                        i + 1,
                    );
                    free(input_copy as *mut c_void);
                    free_matrix(mat);
                    return ptr::null_mut();
                }
                *elem_ptr(mat, i, j) = atoi(col_token);
                col_token = strtok_r(ptr::null_mut(), c" ".as_ptr(), &mut saveptr_col);
                j += 1;
            }

            row_token = strtok_r(ptr::null_mut(), c"\n".as_ptr(), &mut saveptr_row);
            i += 1;
        }

        free(input_copy as *mut c_void);
        mat
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn multiply_matrices(
    mat_a: *mut matrix_t,
    mat_b: *mut matrix_t,
) -> *mut matrix_t {
    unsafe {
        if (*mat_a).width != (*mat_b).height {
            fprintf(
                stderr_stream(),
                c"Matrix dimensions do not allow multiplication.\n".as_ptr(),
            );
            return ptr::null_mut();
        }

        let result = allocate_matrix((*mat_b).width, (*mat_a).height);
        let mut i: c_int = 0;
        while i < (*mat_a).height {
            let mut j: c_int = 0;
            while j < (*mat_b).width {
                *elem_ptr(result, i, j) = 0;
                let mut k: c_int = 0;
                while k < (*mat_a).width {
                    let product =
                        (*elem_ptr(mat_a, i, k)).wrapping_mul(*elem_ptr(mat_b, k, j));
                    let slot = elem_ptr(result, i, j);
                    *slot = (*slot).wrapping_add(product);
                    k += 1;
                }
                j += 1;
            }
            i += 1;
        }

        result
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn matrix_to_string(mat: *mut matrix_t) -> *mut c_char {
    unsafe {
        if mat.is_null() {
            fprintf(stderr_stream(), c"Error: Matrix is NULL.\n".as_ptr());
            return ptr::null_mut();
        }

        let width = (*mat).width;
        let height = (*mat).height;

        // int buffer_size = mat->height * (mat->width * 10 + mat->width)
        //                   + mat->height + 1;
        let buffer_size: c_int = height
            .wrapping_mul(width.wrapping_mul(10).wrapping_add(width))
            .wrapping_add(height)
            .wrapping_add(1);

        let result = malloc(int_to_size(buffer_size)) as *mut c_char;
        if result.is_null() {
            perror(c"Failed to allocate memory for matrix string".as_ptr());
            return ptr::null_mut();
        }

        *result = 0;

        let mut i: c_int = 0;
        while i < height {
            let mut j: c_int = 0;
            while j < width {
                let mut buffer = [0 as c_char; 12];
                snprintf(
                    buffer.as_mut_ptr(),
                    buffer.len() as u64,
                    c"%d".as_ptr(),
                    *elem_ptr(mat, i, j),
                );
                strcat(result, buffer.as_ptr());

                if j < width.wrapping_sub(1) {
                    strcat(result, c" ".as_ptr());
                }
                j += 1;
            }
            strcat(result, c"\n".as_ptr());
            i += 1;
        }

        result
    }
}
