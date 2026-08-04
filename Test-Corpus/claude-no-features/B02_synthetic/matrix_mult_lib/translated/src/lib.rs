// Translated from MIT Lincoln Laboratory C code (matrix/write/driver).
// Preserves byte-identical output to the original C implementation.

#![allow(non_camel_case_types)]
#![allow(clippy::missing_safety_doc)]

use core::ffi::{c_char, c_int, c_void};
use libc::{
    EINVAL, EXIT_FAILURE, EXIT_SUCCESS, FILE, atoi, fclose, fopen, fprintf, free, malloc, perror,
    strerror, strtok_r,
};

// Match the C struct layout exactly.
#[repr(C)]
pub struct matrix_t {
    pub matrix: *mut *mut c_int,
    pub width: c_int,
    pub height: c_int,
}

// libc constants/symbols for stderr access.
extern "C" {
    static stderr: *mut FILE;
    fn strdup(s: *const c_char) -> *mut c_char;
    fn strcat(dest: *mut c_char, src: *const c_char) -> *mut c_char;
    fn snprintf(s: *mut c_char, n: usize, format: *const c_char, ...) -> c_int;
    fn __errno_location() -> *mut c_int;
}

#[inline]
unsafe fn errno() -> c_int {
    *__errno_location()
}

/// Allocate a matrix_t with rows*cols of c_int, using libc malloc so callers can free() it
/// the same way the C code does. Mirrors the original allocate_matrix exactly.
unsafe fn allocate_matrix(width: c_int, height: c_int) -> *mut matrix_t {
    let mat = malloc(core::mem::size_of::<matrix_t>()) as *mut matrix_t;
    if mat.is_null() {
        perror(c"Failed to allocate memory for matrix struct".as_ptr());
        return core::ptr::null_mut();
    }

    (*mat).width = width;
    (*mat).height = height;

    (*mat).matrix =
        malloc((height as usize).wrapping_mul(core::mem::size_of::<*mut c_int>())) as *mut *mut c_int;
    if (*mat).matrix.is_null() {
        perror(c"Failed to allocate memory for matrix rows".as_ptr());
        free(mat as *mut c_void);
        return core::ptr::null_mut();
    }

    let mut i: c_int = 0;
    while i < height {
        let row_ptr =
            malloc((width as usize).wrapping_mul(core::mem::size_of::<c_int>())) as *mut c_int;
        *(*mat).matrix.offset(i as isize) = row_ptr;
        if row_ptr.is_null() {
            perror(c"Failed to allocate memory for matrix columns".as_ptr());
            // Bug-for-bug: original frees j = 0 ..= i (which double-frees the just-failed slot);
            // however because we just stored the failing row pointer, the j == i element points
            // to NULL. free(NULL) is a no-op in C, so semantics are preserved.
            let mut j: c_int = 0;
            while j <= i {
                free(*(*mat).matrix.offset(j as isize) as *mut c_void);
                j += 1;
            }
            free((*mat).matrix as *mut c_void);
            free(mat as *mut c_void);
            return core::ptr::null_mut();
        }
        i += 1;
    }

    mat
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn free_matrix(mat: *mut matrix_t) {
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn initialize_matrix_from_string(
    input: *const c_char,
    width: c_int,
    height: c_int,
) -> *mut matrix_t {
    let mat = allocate_matrix(width, height);

    let input_copy = strdup(input);
    if input_copy.is_null() {
        perror(c"Failed to duplicate input string".as_ptr());
        free_matrix(mat);
        return core::ptr::null_mut();
    }

    let mut saveptr_row: *mut c_char = core::ptr::null_mut();
    let mut row_token = strtok_r(input_copy, c"\n".as_ptr(), &mut saveptr_row);

    let mut i: c_int = 0;
    while i < height {
        if row_token.is_null() {
            fprintf(stderr, c"Insufficient rows in input string.\n".as_ptr());
            free(input_copy as *mut c_void);
            free_matrix(mat);
            return core::ptr::null_mut();
        }

        let mut saveptr_col: *mut c_char = core::ptr::null_mut();
        let mut col_token = strtok_r(row_token, c" ".as_ptr(), &mut saveptr_col);

        let mut j: c_int = 0;
        while j < width {
            if col_token.is_null() {
                fprintf(
                    stderr,
                    c"Insufficient columns in row %d.\n".as_ptr(),
                    i + 1,
                );
                free(input_copy as *mut c_void);
                free_matrix(mat);
                return core::ptr::null_mut();
            }
            *(*(*mat).matrix.offset(i as isize)).offset(j as isize) = atoi(col_token);
            col_token = strtok_r(core::ptr::null_mut(), c" ".as_ptr(), &mut saveptr_col);
            j += 1;
        }

        row_token = strtok_r(core::ptr::null_mut(), c"\n".as_ptr(), &mut saveptr_row);
        i += 1;
    }

    free(input_copy as *mut c_void);
    mat
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn multiply_matrices(
    mat_a: *mut matrix_t,
    mat_b: *mut matrix_t,
) -> *mut matrix_t {
    if (*mat_a).width != (*mat_b).height {
        fprintf(
            stderr,
            c"Matrix dimensions do not allow multiplication.\n".as_ptr(),
        );
        return core::ptr::null_mut();
    }

    let result = allocate_matrix((*mat_b).width, (*mat_a).height);
    let mut i: c_int = 0;
    while i < (*mat_a).height {
        let mut j: c_int = 0;
        while j < (*mat_b).width {
            *(*(*result).matrix.offset(i as isize)).offset(j as isize) = 0;
            let mut k: c_int = 0;
            while k < (*mat_a).width {
                let a = *(*(*mat_a).matrix.offset(i as isize)).offset(k as isize);
                let b = *(*(*mat_b).matrix.offset(k as isize)).offset(j as isize);
                let dst = (*(*result).matrix.offset(i as isize)).offset(j as isize);
                // Match C semantics: signed int multiply/add with two's-complement wrap
                // (technically UB in C on overflow, but the typical compiler behavior is wrapping).
                *dst = (*dst).wrapping_add(a.wrapping_mul(b));
                k += 1;
            }
            j += 1;
        }
        i += 1;
    }

    result
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn matrix_to_string(mat: *mut matrix_t) -> *mut c_char {
    if mat.is_null() {
        fprintf(stderr, c"Error: Matrix is NULL.\n".as_ptr());
        return core::ptr::null_mut();
    }

    let height = (*mat).height;
    let width = (*mat).width;
    let buffer_size = (height as isize)
        .wrapping_mul((width as isize).wrapping_mul(10).wrapping_add(width as isize))
        .wrapping_add(height as isize)
        .wrapping_add(1);

    let result = malloc(buffer_size as usize) as *mut c_char;
    if result.is_null() {
        perror(c"Failed to allocate memory for matrix string".as_ptr());
        return core::ptr::null_mut();
    }

    *result = 0;

    let mut i: c_int = 0;
    while i < height {
        let mut j: c_int = 0;
        while j < width {
            let mut buffer: [c_char; 12] = [0; 12];
            snprintf(
                buffer.as_mut_ptr(),
                buffer.len(),
                c"%d".as_ptr(),
                *(*(*mat).matrix.offset(i as isize)).offset(j as isize),
            );
            strcat(result, buffer.as_ptr());

            if j < width - 1 {
                strcat(result, c" ".as_ptr());
            }
            j += 1;
        }
        strcat(result, c"\n".as_ptr());
        i += 1;
    }

    result
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn write_to_file(filename: *const c_char, content: *const c_char) -> c_int {
    if content.is_null() {
        fprintf(stderr, c"Error: Content is NULL.\n".as_ptr());
        return EINVAL;
    }

    let file = fopen(filename, c"w".as_ptr());
    if file.is_null() {
        fprintf(
            stderr,
            c"Error opening file '%s': %s\n".as_ptr(),
            filename,
            strerror(errno()),
        );
        return errno();
    }

    if fprintf(file, c"%s".as_ptr(), content) < 0 {
        fprintf(
            stderr,
            c"Error writing to file '%s': %s\n".as_ptr(),
            filename,
            strerror(errno()),
        );
        fclose(file);
        return errno();
    }

    if fclose(file) != 0 {
        fprintf(
            stderr,
            c"Error closing file '%s': %s\n".as_ptr(),
            filename,
            strerror(errno()),
        );
        return errno();
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(
    width_a: c_int,
    height_a: c_int,
    matrix_a: *const c_char,
    width_b: c_int,
    height_b: c_int,
    matrix_b: *const c_char,
) -> c_int {
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
        free(res as *mut c_void);
        return EXIT_FAILURE;
    }

    let res_write = write_to_file(c"matrix.txt".as_ptr(), res_str);

    free_matrix(mat_a);
    free_matrix(mat_b);
    free_matrix(res);
    free(res_str as *mut c_void);

    if res_write != 0 {
        return EXIT_FAILURE;
    }

    EXIT_SUCCESS
}
