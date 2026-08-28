//! Translation of `c_src/src/matrix.c`.

use std::ffi::c_char;
use std::ffi::c_int;

use crate::cutil::{
    atoi, free, int_to_size_t, malloc, perror, stderr_write, strdup, strtok_r,
};

/// `matrix_t` from `include/matrix.h`.
#[repr(C)]
pub struct matrix_t {
    pub matrix: *mut *mut c_int,
    pub width: c_int,
    pub height: c_int,
}

const SIZEOF_INT_PTR: usize = std::mem::size_of::<*mut c_int>();
const SIZEOF_INT: usize = std::mem::size_of::<c_int>();

#[unsafe(no_mangle)]
pub extern "C" fn allocate_matrix(width: c_int, height: c_int) -> *mut matrix_t {
    unsafe {
        let mat = malloc(std::mem::size_of::<matrix_t>()) as *mut matrix_t;
        if mat.is_null() {
            perror("Failed to allocate memory for matrix struct");
            return std::ptr::null_mut();
        }

        (*mat).width = width;
        (*mat).height = height;

        // `height * sizeof(int*)`: `height` is converted to size_t first, so a
        // negative height becomes a huge allocation request (and thus fails).
        (*mat).matrix =
            malloc(int_to_size_t(height).wrapping_mul(SIZEOF_INT_PTR)) as *mut *mut c_int;
        if (*mat).matrix.is_null() {
            perror("Failed to allocate memory for matrix rows");
            free(mat as *mut u8);
            return std::ptr::null_mut();
        }

        let mut i: c_int = 0;
        while i < height {
            let row = malloc(int_to_size_t(width).wrapping_mul(SIZEOF_INT)) as *mut c_int;
            *(*mat).matrix.offset(i as isize) = row;
            if row.is_null() {
                perror("Failed to allocate memory for matrix columns");
                // Matches the original loop bound `j <= i` (the last entry is
                // the NULL row that just failed; free(NULL) is a no-op).
                let mut j: c_int = 0;
                while j <= i {
                    free(*(*mat).matrix.offset(j as isize) as *mut u8);
                    j += 1;
                }
                free((*mat).matrix as *mut u8);
                free(mat as *mut u8);
                return std::ptr::null_mut();
            }
            i += 1;
        }

        mat
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn free_matrix(mat: *mut matrix_t) {
    unsafe {
        if mat.is_null() {
            return;
        }

        let mut i: c_int = 0;
        while i < (*mat).height {
            free(*(*mat).matrix.offset(i as isize) as *mut u8);
            i += 1;
        }
        free((*mat).matrix as *mut u8);
        free(mat as *mut u8);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn initialize_matrix_from_string(
    input: *const c_char,
    width: c_int,
    height: c_int,
) -> *mut matrix_t {
    unsafe {
        // NOTE: the original does not check `allocate_matrix` for NULL here.
        let mat = allocate_matrix(width, height);

        let input_copy = strdup(input);
        if input_copy.is_null() {
            perror("Failed to duplicate input string");
            free_matrix(mat);
            return std::ptr::null_mut();
        }

        let mut saveptr_row: *mut c_char = std::ptr::null_mut();
        let mut row_token = strtok_r(input_copy, b'\n', &mut saveptr_row);

        let mut i: c_int = 0;
        while i < height {
            if row_token.is_null() {
                stderr_write(b"Insufficient rows in input string.\n");
                free(input_copy as *mut u8);
                free_matrix(mat);
                return std::ptr::null_mut();
            }

            let mut saveptr_col: *mut c_char = std::ptr::null_mut();
            let mut col_token = strtok_r(row_token, b' ', &mut saveptr_col);

            let mut j: c_int = 0;
            while j < width {
                if col_token.is_null() {
                    stderr_write(
                        format!("Insufficient columns in row {}.\n", i.wrapping_add(1))
                            .as_bytes(),
                    );
                    free(input_copy as *mut u8);
                    free_matrix(mat);
                    return std::ptr::null_mut();
                }
                *(*(*mat).matrix.offset(i as isize)).offset(j as isize) = atoi(col_token);
                col_token = strtok_r(std::ptr::null_mut(), b' ', &mut saveptr_col);
                j += 1;
            }

            row_token = strtok_r(std::ptr::null_mut(), b'\n', &mut saveptr_row);
            i += 1;
        }

        free(input_copy as *mut u8);
        mat
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn multiply_matrices(mat_a: *mut matrix_t, mat_b: *mut matrix_t) -> *mut matrix_t {
    unsafe {
        if (*mat_a).width != (*mat_b).height {
            stderr_write(b"Matrix dimensions do not allow multiplication.\n");
            return std::ptr::null_mut();
        }

        // NOTE: as in the original, the result of `allocate_matrix` is not
        // checked for NULL.
        let result = allocate_matrix((*mat_b).width, (*mat_a).height);

        let mut i: c_int = 0;
        while i < (*mat_a).height {
            let mut j: c_int = 0;
            while j < (*mat_b).width {
                let cell = (*(*result).matrix.offset(i as isize)).offset(j as isize);
                *cell = 0;
                let mut k: c_int = 0;
                while k < (*mat_a).width {
                    let a = *(*(*mat_a).matrix.offset(i as isize)).offset(k as isize);
                    let b = *(*(*mat_b).matrix.offset(k as isize)).offset(j as isize);
                    // Signed overflow is UB in C; on the usual targets it wraps.
                    *cell = (*cell).wrapping_add(a.wrapping_mul(b));
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
pub extern "C" fn matrix_to_string(mat: *mut matrix_t) -> *mut c_char {
    unsafe {
        if mat.is_null() {
            stderr_write(b"Error: Matrix is NULL.\n");
            return std::ptr::null_mut();
        }

        let height = (*mat).height;
        let width = (*mat).width;

        // Same (int) arithmetic as the original, including its wrap-around.
        let buffer_size: c_int = height
            .wrapping_mul(width.wrapping_mul(10).wrapping_add(width))
            .wrapping_add(height)
            .wrapping_add(1);

        let result = malloc(int_to_size_t(buffer_size)) as *mut c_char;
        if result.is_null() {
            perror("Failed to allocate memory for matrix string");
            return std::ptr::null_mut();
        }

        *result = 0;

        // Reproduces the original chain of `snprintf` + `strcat` calls byte for
        // byte, including the fact that the size estimate above only budgets 11
        // bytes per column and so can be overrun (undefined behaviour in the
        // original, faithfully preserved here).
        let mut len: usize = 0;
        let mut i: c_int = 0;
        while i < height {
            let mut j: c_int = 0;
            while j < width {
                let value = *(*(*mat).matrix.offset(i as isize)).offset(j as isize);
                // snprintf into char[12]: an int is at most 11 chars plus NUL,
                // so nothing is ever truncated.
                let text = value.to_string();
                let bytes = text.as_bytes();
                std::ptr::copy_nonoverlapping(bytes.as_ptr(), result.add(len) as *mut u8, bytes.len());
                len += bytes.len();
                *result.add(len) = 0;

                if j < width.wrapping_sub(1) {
                    *result.add(len) = b' ' as c_char;
                    len += 1;
                    *result.add(len) = 0;
                }
                j += 1;
            }
            *result.add(len) = b'\n' as c_char;
            len += 1;
            *result.add(len) = 0;
            i += 1;
        }

        result
    }
}
