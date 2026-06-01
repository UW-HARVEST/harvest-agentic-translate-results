/*
 * Rust translation of MIT Lincoln Laboratory matrix driver C code.
 * Behavior is intended to be byte-identical to the original C implementation.
 */

use std::ffi::c_char;
use std::ffi::c_int;
use std::ptr;

// Matches the C `matrix_t` layout exactly:
//   typedef struct {
//       int** matrix;
//       int width;
//       int height;
//   } matrix_t;
#[repr(C)]
pub struct MatrixT {
    pub matrix: *mut *mut c_int,
    pub width: c_int,
    pub height: c_int,
}

// EINVAL — matches glibc constant value (22).
const EINVAL: c_int = 22;

// EXIT_FAILURE / EXIT_SUCCESS as defined by glibc <stdlib.h>.
const EXIT_SUCCESS: c_int = 0;
const EXIT_FAILURE: c_int = 1;

extern "C" {
    fn perror(s: *const c_char);
    fn fprintf(stream: *mut libc::FILE, fmt: *const c_char, ...) -> c_int;
    fn fopen(path: *const c_char, mode: *const c_char) -> *mut libc::FILE;
    fn fclose(stream: *mut libc::FILE) -> c_int;
    fn strerror(errnum: c_int) -> *mut c_char;
    fn strdup(s: *const c_char) -> *mut c_char;
    fn strtok_r(
        str: *mut c_char,
        delim: *const c_char,
        saveptr: *mut *mut c_char,
    ) -> *mut c_char;
    fn atoi(s: *const c_char) -> c_int;
    fn snprintf(s: *mut c_char, n: usize, fmt: *const c_char, ...) -> c_int;
    fn strcat(dest: *mut c_char, src: *const c_char) -> *mut c_char;
    fn malloc(size: usize) -> *mut libc::c_void;
    fn free(ptr: *mut libc::c_void);

    static stderr: *mut libc::FILE;
}

// errno via __errno_location
extern "C" {
    fn __errno_location() -> *mut c_int;
}

unsafe fn errno() -> c_int {
    unsafe { *__errno_location() }
}

// Helper: returns a pointer to a static C string from a byte literal that
// already ends with a NUL byte.
fn cstr(s: &[u8]) -> *const c_char {
    debug_assert!(s.last() == Some(&0));
    s.as_ptr() as *const c_char
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn allocate_matrix(width: c_int, height: c_int) -> *mut MatrixT {
    unsafe {
        let mat = malloc(std::mem::size_of::<MatrixT>()) as *mut MatrixT;
        if mat.is_null() {
            perror(cstr(b"Failed to allocate memory for matrix struct\0"));
            return ptr::null_mut();
        }

        (*mat).width = width;
        (*mat).height = height;

        let rows_size = (height as usize) * std::mem::size_of::<*mut c_int>();
        (*mat).matrix = malloc(rows_size) as *mut *mut c_int;
        if (*mat).matrix.is_null() {
            perror(cstr(b"Failed to allocate memory for matrix rows\0"));
            free(mat as *mut libc::c_void);
            return ptr::null_mut();
        }

        for i in 0..height {
            let col_size = (width as usize) * std::mem::size_of::<c_int>();
            let row = malloc(col_size) as *mut c_int;
            *(*mat).matrix.offset(i as isize) = row;
            if row.is_null() {
                perror(cstr(b"Failed to allocate memory for matrix columns\0"));
                let mut j = 0;
                while j <= i {
                    free(*(*mat).matrix.offset(j as isize) as *mut libc::c_void);
                    j += 1;
                }
                free((*mat).matrix as *mut libc::c_void);
                free(mat as *mut libc::c_void);
                return ptr::null_mut();
            }
        }

        mat
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn free_matrix(mat: *mut MatrixT) {
    unsafe {
        if mat.is_null() {
            return;
        }

        for i in 0..(*mat).height {
            free(*(*mat).matrix.offset(i as isize) as *mut libc::c_void);
        }
        free((*mat).matrix as *mut libc::c_void);
        free(mat as *mut libc::c_void);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn initialize_matrix_from_string(
    input: *const c_char,
    width: c_int,
    height: c_int,
) -> *mut MatrixT {
    unsafe {
        let mat = allocate_matrix(width, height);

        let input_copy = strdup(input);
        if input_copy.is_null() {
            perror(cstr(b"Failed to duplicate input string\0"));
            free_matrix(mat);
            return ptr::null_mut();
        }

        let mut saveptr_row: *mut c_char = ptr::null_mut();
        let mut row_token = strtok_r(input_copy, cstr(b"\n\0"), &mut saveptr_row);
        for i in 0..height {
            if row_token.is_null() {
                fprintf(stderr, cstr(b"Insufficient rows in input string.\n\0"));
                free(input_copy as *mut libc::c_void);
                free_matrix(mat);
                return ptr::null_mut();
            }

            let mut saveptr_col: *mut c_char = ptr::null_mut();
            let mut col_token = strtok_r(row_token, cstr(b" \0"), &mut saveptr_col);
            for j in 0..width {
                if col_token.is_null() {
                    fprintf(
                        stderr,
                        cstr(b"Insufficient columns in row %d.\n\0"),
                        (i + 1) as c_int,
                    );
                    free(input_copy as *mut libc::c_void);
                    free_matrix(mat);
                    return ptr::null_mut();
                }
                *(*(*mat).matrix.offset(i as isize)).offset(j as isize) = atoi(col_token);
                col_token = strtok_r(ptr::null_mut(), cstr(b" \0"), &mut saveptr_col);
            }

            row_token = strtok_r(ptr::null_mut(), cstr(b"\n\0"), &mut saveptr_row);
        }

        free(input_copy as *mut libc::c_void);
        mat
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn multiply_matrices(
    mat_a: *mut MatrixT,
    mat_b: *mut MatrixT,
) -> *mut MatrixT {
    unsafe {
        if (*mat_a).width != (*mat_b).height {
            fprintf(
                stderr,
                cstr(b"Matrix dimensions do not allow multiplication.\n\0"),
            );
            return ptr::null_mut();
        }

        let result = allocate_matrix((*mat_b).width, (*mat_a).height);
        for i in 0..(*mat_a).height {
            for j in 0..(*mat_b).width {
                let result_row = *(*result).matrix.offset(i as isize);
                *result_row.offset(j as isize) = 0;
                for k in 0..(*mat_a).width {
                    let a_row = *(*mat_a).matrix.offset(i as isize);
                    let b_row = *(*mat_b).matrix.offset(k as isize);
                    let a = *a_row.offset(k as isize);
                    let b = *b_row.offset(j as isize);
                    // Use wrapping arithmetic to match C signed-integer behavior
                    // (C's int multiplication overflow is UB, but on most
                    // implementations wraps; we mirror that here without
                    // panicking).
                    let cur = *result_row.offset(j as isize);
                    *result_row.offset(j as isize) = cur.wrapping_add(a.wrapping_mul(b));
                }
            }
        }

        result
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn matrix_to_string(mat: *mut MatrixT) -> *mut c_char {
    unsafe {
        if mat.is_null() {
            fprintf(stderr, cstr(b"Error: Matrix is NULL.\n\0"));
            return ptr::null_mut();
        }

        let height = (*mat).height;
        let width = (*mat).width;
        let buffer_size = (height as usize) * ((width as usize) * 10 + (width as usize))
            + (height as usize)
            + 1;

        let result = malloc(buffer_size) as *mut c_char;
        if result.is_null() {
            perror(cstr(b"Failed to allocate memory for matrix string\0"));
            return ptr::null_mut();
        }

        *result = 0;

        let mut buffer: [c_char; 12] = [0; 12];
        for i in 0..height {
            for j in 0..width {
                let val = *(*(*mat).matrix.offset(i as isize)).offset(j as isize);
                snprintf(
                    buffer.as_mut_ptr(),
                    buffer.len(),
                    cstr(b"%d\0"),
                    val,
                );
                strcat(result, buffer.as_ptr());

                if j < width - 1 {
                    strcat(result, cstr(b" \0"));
                }
            }
            strcat(result, cstr(b"\n\0"));
        }

        result
    }
}

// ----- write.c -----

#[unsafe(no_mangle)]
pub unsafe extern "C" fn write_to_file(
    filename: *const c_char,
    content: *const c_char,
) -> c_int {
    unsafe {
        if content.is_null() {
            fprintf(stderr, cstr(b"Error: Content is NULL.\n\0"));
            return EINVAL;
        }

        let file = fopen(filename, cstr(b"w\0"));
        if file.is_null() {
            fprintf(
                stderr,
                cstr(b"Error opening file '%s': %s\n\0"),
                filename,
                strerror(errno()),
            );
            return errno();
        }

        if fprintf(file, cstr(b"%s\0"), content) < 0 {
            fprintf(
                stderr,
                cstr(b"Error writing to file '%s': %s\n\0"),
                filename,
                strerror(errno()),
            );
            fclose(file);
            return errno();
        }

        if fclose(file) != 0 {
            fprintf(
                stderr,
                cstr(b"Error closing file '%s': %s\n\0"),
                filename,
                strerror(errno()),
            );
            return errno();
        }

        0
    }
}

// ----- driver.c -----

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
            free(res as *mut libc::c_void);
            return EXIT_FAILURE;
        }

        let out_file = cstr(b"matrix.txt\0");
        let res_write = write_to_file(out_file, res_str);

        free_matrix(mat_a);
        free_matrix(mat_b);
        free_matrix(res);
        free(res_str as *mut libc::c_void);

        if res_write != 0 {
            return EXIT_FAILURE;
        }

        EXIT_SUCCESS
    }
}
