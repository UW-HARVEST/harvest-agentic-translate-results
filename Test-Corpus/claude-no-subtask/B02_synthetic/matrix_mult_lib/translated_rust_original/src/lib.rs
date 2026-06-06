// Translated from C to Rust. Preserves byte-identical output for the same inputs.

#![allow(non_camel_case_types)]

use std::ffi::{c_char, c_int, c_void};

#[repr(C)]
pub struct matrix_t {
    pub matrix: *mut *mut c_int,
    pub width: c_int,
    pub height: c_int,
}

// EXIT_SUCCESS = 0, EXIT_FAILURE = 1 on POSIX/glibc
const EXIT_SUCCESS: c_int = 0;
const EXIT_FAILURE: c_int = 1;
const EINVAL: c_int = 22;

extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
    fn perror(s: *const c_char);
    fn strerror(errnum: c_int) -> *mut c_char;
    fn __errno_location() -> *mut c_int;
    fn fopen(filename: *const c_char, mode: *const c_char) -> *mut c_void;
    fn fclose(stream: *mut c_void) -> c_int;
    fn fwrite(ptr: *const c_void, size: usize, nmemb: usize, stream: *mut c_void) -> usize;
    fn fprintf(stream: *mut c_void, format: *const c_char, ...) -> c_int;
    static stderr: *mut c_void;
}

unsafe fn errno() -> c_int {
    *__errno_location()
}

// ----------------------- matrix.c -------------------------------------------

unsafe fn allocate_matrix(width: c_int, height: c_int) -> *mut matrix_t {
    let mat = malloc(std::mem::size_of::<matrix_t>()) as *mut matrix_t;
    if mat.is_null() {
        perror(b"Failed to allocate memory for matrix struct\0".as_ptr() as *const c_char);
        return std::ptr::null_mut();
    }

    (*mat).width = width;
    (*mat).height = height;

    let row_array_size = (height as usize).wrapping_mul(std::mem::size_of::<*mut c_int>());
    (*mat).matrix = malloc(row_array_size) as *mut *mut c_int;
    if (*mat).matrix.is_null() {
        perror(b"Failed to allocate memory for matrix rows\0".as_ptr() as *const c_char);
        free(mat as *mut c_void);
        return std::ptr::null_mut();
    }

    for i in 0..height {
        let col_size = (width as usize).wrapping_mul(std::mem::size_of::<c_int>());
        let row_ptr = malloc(col_size) as *mut c_int;
        *(*mat).matrix.offset(i as isize) = row_ptr;
        if row_ptr.is_null() {
            perror(b"Failed to allocate memory for matrix columns\0".as_ptr() as *const c_char);
            // Mirror the original C bug exactly: `for (j = 0; j <= i; j++)`
            // frees the just-failed (NULL) pointer too. free(NULL) is safe.
            for j in 0..=i {
                free(*(*mat).matrix.offset(j as isize) as *mut c_void);
            }
            free((*mat).matrix as *mut c_void);
            free(mat as *mut c_void);
            return std::ptr::null_mut();
        }
    }

    mat
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn free_matrix(mat: *mut matrix_t) {
    if mat.is_null() {
        return;
    }

    for i in 0..(*mat).height {
        free(*(*mat).matrix.offset(i as isize) as *mut c_void);
    }
    free((*mat).matrix as *mut c_void);
    free(mat as *mut c_void);
}

// strtok_r-equivalent for parsing.
// Splits a byte slice on any of the delimiter bytes; consecutive delimiters
// are treated as a single delimiter (matching strtok_r semantics).
fn strtok_r_iter<'a>(data: &'a [u8], delims: &[u8]) -> Vec<&'a [u8]> {
    let mut result = Vec::new();
    let mut i = 0;
    while i < data.len() {
        // Skip leading delimiters.
        while i < data.len() && delims.contains(&data[i]) {
            i += 1;
        }
        if i >= data.len() {
            break;
        }
        let start = i;
        while i < data.len() && !delims.contains(&data[i]) {
            i += 1;
        }
        result.push(&data[start..i]);
    }
    result
}

// C atoi: skip leading whitespace, optional sign, then digits.
fn c_atoi(s: &[u8]) -> c_int {
    let mut idx = 0;
    while idx < s.len() && matches!(s[idx], b' ' | b'\t' | b'\n' | b'\r' | b'\x0b' | b'\x0c') {
        idx += 1;
    }
    let mut neg = false;
    if idx < s.len() {
        if s[idx] == b'-' {
            neg = true;
            idx += 1;
        } else if s[idx] == b'+' {
            idx += 1;
        }
    }
    let mut val: i64 = 0;
    while idx < s.len() && s[idx].is_ascii_digit() {
        val = val.wrapping_mul(10).wrapping_add((s[idx] - b'0') as i64);
        idx += 1;
    }
    if neg {
        val = val.wrapping_neg();
    }
    val as c_int
}

unsafe fn cstr_len(s: *const c_char) -> usize {
    let mut len = 0usize;
    while *s.add(len) != 0 {
        len += 1;
    }
    len
}

unsafe fn cstr_to_slice<'a>(s: *const c_char) -> &'a [u8] {
    let len = cstr_len(s);
    std::slice::from_raw_parts(s as *const u8, len)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn initialize_matrix_from_string(
    input: *const c_char,
    width: c_int,
    height: c_int,
) -> *mut matrix_t {
    let mat = allocate_matrix(width, height);

    // strdup the input. If allocation failed, return NULL after free_matrix.
    // (Note: real strdup in C would only fail under OOM.)
    let input_slice = cstr_to_slice(input);
    let input_copy_size = input_slice.len() + 1;
    let input_copy = malloc(input_copy_size) as *mut u8;
    if input_copy.is_null() {
        perror(b"Failed to duplicate input string\0".as_ptr() as *const c_char);
        free_matrix(mat);
        return std::ptr::null_mut();
    }
    std::ptr::copy_nonoverlapping(input_slice.as_ptr(), input_copy, input_slice.len());
    *input_copy.add(input_slice.len()) = 0;

    // Build a working slice (we won't actually mutate; we'll just iterate over rows).
    let working_slice = std::slice::from_raw_parts(input_copy, input_slice.len());
    let rows = strtok_r_iter(working_slice, b"\n");

    for i in 0..height as usize {
        if i >= rows.len() {
            fprintf(
                stderr,
                b"Insufficient rows in input string.\n\0".as_ptr() as *const c_char,
            );
            free(input_copy as *mut c_void);
            free_matrix(mat);
            return std::ptr::null_mut();
        }

        let cols = strtok_r_iter(rows[i], b" ");
        for j in 0..width as usize {
            if j >= cols.len() {
                fprintf(
                    stderr,
                    b"Insufficient columns in row %d.\n\0".as_ptr() as *const c_char,
                    (i as c_int) + 1,
                );
                free(input_copy as *mut c_void);
                free_matrix(mat);
                return std::ptr::null_mut();
            }
            let val = c_atoi(cols[j]);
            *(*(*mat).matrix.offset(i as isize)).offset(j as isize) = val;
        }
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
            b"Matrix dimensions do not allow multiplication.\n\0".as_ptr() as *const c_char,
        );
        return std::ptr::null_mut();
    }

    let result = allocate_matrix((*mat_b).width, (*mat_a).height);
    for i in 0..(*mat_a).height {
        for j in 0..(*mat_b).width {
            *(*(*result).matrix.offset(i as isize)).offset(j as isize) = 0;
            for k in 0..(*mat_a).width {
                let a_val = *(*(*mat_a).matrix.offset(i as isize)).offset(k as isize);
                let b_val = *(*(*mat_b).matrix.offset(k as isize)).offset(j as isize);
                let cur = *(*(*result).matrix.offset(i as isize)).offset(j as isize);
                // Match C `int` overflow as wrapping (same as -fwrapv / typical compilers).
                let new_val = cur.wrapping_add(a_val.wrapping_mul(b_val));
                *(*(*result).matrix.offset(i as isize)).offset(j as isize) = new_val;
            }
        }
    }

    result
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn matrix_to_string(mat: *mut matrix_t) -> *mut c_char {
    if mat.is_null() {
        fprintf(
            stderr,
            b"Error: Matrix is NULL.\n\0".as_ptr() as *const c_char,
        );
        return std::ptr::null_mut();
    }

    let height = (*mat).height as usize;
    let width = (*mat).width as usize;
    let buffer_size = height * (width * 10 + width) + height + 1;

    let result = malloc(buffer_size) as *mut u8;
    if result.is_null() {
        perror(b"Failed to allocate memory for matrix string\0".as_ptr() as *const c_char);
        return std::ptr::null_mut();
    }

    // Build the string content as bytes.
    let mut content = Vec::<u8>::new();
    for i in 0..(*mat).height {
        for j in 0..(*mat).width {
            let val = *(*(*mat).matrix.offset(i as isize)).offset(j as isize);
            // C uses snprintf with "%d" -> standard decimal representation.
            let s = format!("{}", val);
            content.extend_from_slice(s.as_bytes());
            if j < (*mat).width - 1 {
                content.push(b' ');
            }
        }
        content.push(b'\n');
    }

    // Copy into the malloc'd buffer. The C version uses strcat repeatedly into
    // a buffer initially containing only '\0'; the final NUL terminator is
    // present after the last strcat. We mirror that here by copying the bytes
    // and writing a trailing NUL.
    if content.len() + 1 > buffer_size {
        // Shouldn't happen given the buffer size formula, but match C: it would
        // overflow the buffer in C. To stay safe, truncate. (Original C has a
        // potential overflow if int repr exceeds 10 chars; but for typical
        // inputs this branch is unreachable.)
        std::ptr::copy_nonoverlapping(content.as_ptr(), result, buffer_size - 1);
        *result.add(buffer_size - 1) = 0;
    } else {
        std::ptr::copy_nonoverlapping(content.as_ptr(), result, content.len());
        *result.add(content.len()) = 0;
    }

    result as *mut c_char
}

// ----------------------- write.c --------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn write_to_file(filename: *const c_char, content: *const c_char) -> c_int {
    if content.is_null() {
        fprintf(stderr, b"Error: Content is NULL.\n\0".as_ptr() as *const c_char);
        return EINVAL;
    }

    let mode = b"w\0".as_ptr() as *const c_char;
    let file = fopen(filename, mode);
    if file.is_null() {
        fprintf(
            stderr,
            b"Error opening file '%s': %s\n\0".as_ptr() as *const c_char,
            filename,
            strerror(errno()),
        );
        return errno();
    }

    // Equivalent to fprintf(file, "%s", content): write content bytes (no NUL).
    let len = cstr_len(content);
    let written = fwrite(content as *const c_void, 1, len, file);
    if written < len {
        fprintf(
            stderr,
            b"Error writing to file '%s': %s\n\0".as_ptr() as *const c_char,
            filename,
            strerror(errno()),
        );
        fclose(file);
        return errno();
    }

    if fclose(file) != 0 {
        fprintf(
            stderr,
            b"Error closing file '%s': %s\n\0".as_ptr() as *const c_char,
            filename,
            strerror(errno()),
        );
        return errno();
    }

    0
}

// ----------------------- driver.c -------------------------------------------

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

    let res_write = write_to_file(b"matrix.txt\0".as_ptr() as *const c_char, res_str);

    free_matrix(mat_a);
    free_matrix(mat_b);
    free_matrix(res);
    free(res_str as *mut c_void);

    if res_write != 0 {
        return EXIT_FAILURE;
    }

    EXIT_SUCCESS
}
