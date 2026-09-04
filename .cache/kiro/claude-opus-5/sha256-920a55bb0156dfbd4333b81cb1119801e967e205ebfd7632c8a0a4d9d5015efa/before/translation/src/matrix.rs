//! Translation of `c_src/src/matrix.c`.

use core::ffi::{c_char, c_int, c_void};
use core::ptr;

use crate::cstd::{
    self, SIZEOF_INT, SIZEOF_INT_PTR, atoi, c_size_mul, free, malloc, perror, snprintf, stderr,
    strdup, strtok_r,
};

/// `typedef struct { int** matrix; int width; int height; } matrix_t;`
#[repr(C)]
pub struct matrix_t {
    pub matrix: *mut *mut c_int,
    pub width: c_int,
    pub height: c_int,
}

/// Non-static in the C source, therefore part of the exported ABI even though
/// it is absent from `matrix.h`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn allocate_matrix(width: c_int, height: c_int) -> *mut matrix_t {
    unsafe {
        let mat = malloc(core::mem::size_of::<matrix_t>()) as *mut matrix_t;
        if mat.is_null() {
            perror(c"Failed to allocate memory for matrix struct".as_ptr());
            return ptr::null_mut();
        }

        (*mat).width = width;
        (*mat).height = height;

        (*mat).matrix = malloc(c_size_mul(height, SIZEOF_INT_PTR)) as *mut *mut c_int;
        if (*mat).matrix.is_null() {
            perror(c"Failed to allocate memory for matrix rows".as_ptr());
            free(mat as *mut c_void);
            return ptr::null_mut();
        }

        let mut i: c_int = 0;
        while i < height {
            let row = malloc(c_size_mul(width, SIZEOF_INT)) as *mut c_int;
            *(*mat).matrix.offset(i as isize) = row;
            if row.is_null() {
                perror(c"Failed to allocate memory for matrix columns".as_ptr());
                // Note: the C code frees indices 0..=i inclusive; slot `i`
                // holds NULL, and free(NULL) is a no-op. Reproduced as-is.
                let mut j: c_int = 0;
                while j <= i {
                    free(*(*mat).matrix.offset(j as isize) as *mut c_void);
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
            free(*(*mat).matrix.offset(i as isize) as *mut c_void);
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
        // The C code deliberately does not check `mat` for NULL here.
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
                cstd::fprintf(stderr, c"Insufficient rows in input string.\n".as_ptr());
                free(input_copy as *mut c_void);
                free_matrix(mat);
                return ptr::null_mut();
            }

            let mut saveptr_col: *mut c_char = ptr::null_mut();
            let mut col_token = strtok_r(row_token, c" ".as_ptr(), &mut saveptr_col);

            let mut j: c_int = 0;
            while j < width {
                if col_token.is_null() {
                    cstd::fprintf(
                        stderr,
                        c"Insufficient columns in row %d.\n".as_ptr(),
                        i + 1,
                    );
                    free(input_copy as *mut c_void);
                    free_matrix(mat);
                    return ptr::null_mut();
                }
                let row = *(*mat).matrix.offset(i as isize);
                *row.offset(j as isize) = atoi(col_token);
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
            cstd::fprintf(
                stderr,
                c"Matrix dimensions do not allow multiplication.\n".as_ptr(),
            );
            return ptr::null_mut();
        }

        // The C code does not check `result` for NULL.
        let result = allocate_matrix((*mat_b).width, (*mat_a).height);

        let mut i: c_int = 0;
        while i < (*mat_a).height {
            let mut j: c_int = 0;
            while j < (*mat_b).width {
                let res_row = *(*result).matrix.offset(i as isize);
                *res_row.offset(j as isize) = 0;
                let mut k: c_int = 0;
                while k < (*mat_a).width {
                    let a = *(*(*mat_a).matrix.offset(i as isize)).offset(k as isize);
                    let b = *(*(*mat_b).matrix.offset(k as isize)).offset(j as isize);
                    let cur = *res_row.offset(j as isize);
                    *res_row.offset(j as isize) = cur.wrapping_add(a.wrapping_mul(b));
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
            cstd::fprintf(stderr, c"Error: Matrix is NULL.\n".as_ptr());
            return ptr::null_mut();
        }

        let width = (*mat).width;
        let height = (*mat).height;

        // int arithmetic, wrapping like gcc: height * (width * 10 + width) + height + 1
        let buffer_size = height
            .wrapping_mul(width.wrapping_mul(10).wrapping_add(width))
            .wrapping_add(height)
            .wrapping_add(1);

        let result = malloc(c_size_mul(buffer_size, 1)) as *mut c_char;
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
                let value = *(*(*mat).matrix.offset(i as isize)).offset(j as isize);
                snprintf(buffer.as_mut_ptr(), buffer.len(), c"%d".as_ptr(), value);
                cstd::strcat(result, buffer.as_ptr());

                if j < width.wrapping_sub(1) {
                    cstd::strcat(result, c" ".as_ptr());
                }
                j += 1;
            }
            cstd::strcat(result, c"\n".as_ptr());
            i += 1;
        }

        result
    }
}
