// Translation of c_src/ to Rust producing byte-identical output.

use std::ffi::{c_char, c_int, c_void};
use std::mem;

#[repr(C)]
pub struct matrix_t {
    pub matrix: *mut *mut c_int,
    pub width: c_int,
    pub height: c_int,
}

extern "C" {
    static stderr: *mut libc::FILE;
}

// EXIT_SUCCESS / EXIT_FAILURE on Linux glibc.
const EXIT_SUCCESS: c_int = 0;
const EXIT_FAILURE: c_int = 1;

#[inline]
unsafe fn cell_ptr(mat: *mut matrix_t, i: c_int, j: c_int) -> *mut c_int {
    let row_ptr = *((*mat).matrix.offset(i as isize));
    row_ptr.offset(j as isize)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn allocate_matrix(width: c_int, height: c_int) -> *mut matrix_t {
    let mat = libc::malloc(mem::size_of::<matrix_t>()) as *mut matrix_t;
    if mat.is_null() {
        libc::perror(b"Failed to allocate memory for matrix struct\0".as_ptr() as *const c_char);
        return std::ptr::null_mut();
    }

    (*mat).width = width;
    (*mat).height = height;

    (*mat).matrix =
        libc::malloc((height as usize) * mem::size_of::<*mut c_int>()) as *mut *mut c_int;
    if (*mat).matrix.is_null() {
        libc::perror(b"Failed to allocate memory for matrix rows\0".as_ptr() as *const c_char);
        libc::free(mat as *mut c_void);
        return std::ptr::null_mut();
    }

    for i in 0..height {
        let row =
            libc::malloc((width as usize) * mem::size_of::<c_int>()) as *mut c_int;
        *((*mat).matrix.offset(i as isize)) = row;
        if row.is_null() {
            libc::perror(
                b"Failed to allocate memory for matrix columns\0".as_ptr() as *const c_char,
            );
            // Preserve original loop bounds (j <= i) including the just-failed slot.
            // free(NULL) is a no-op so this is harmless.
            for j in 0..=i {
                libc::free(*((*mat).matrix.offset(j as isize)) as *mut c_void);
            }
            libc::free((*mat).matrix as *mut c_void);
            libc::free(mat as *mut c_void);
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
        libc::free(*((*mat).matrix.offset(i as isize)) as *mut c_void);
    }
    libc::free((*mat).matrix as *mut c_void);
    libc::free(mat as *mut c_void);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn initialize_matrix_from_string(
    input: *const c_char,
    width: c_int,
    height: c_int,
) -> *mut matrix_t {
    // NOTE: original C does not check the result of allocate_matrix; preserve.
    let mat = allocate_matrix(width, height);

    let input_copy = libc::strdup(input);
    if input_copy.is_null() {
        libc::perror(b"Failed to duplicate input string\0".as_ptr() as *const c_char);
        free_matrix(mat);
        return std::ptr::null_mut();
    }

    let row_delim = b"\n\0".as_ptr() as *const c_char;
    let col_delim = b" \0".as_ptr() as *const c_char;

    let mut saveptr_row: *mut c_char = std::ptr::null_mut();
    let mut row_token = libc::strtok_r(input_copy, row_delim, &mut saveptr_row);

    for i in 0..height {
        if row_token.is_null() {
            libc::fprintf(
                stderr,
                b"Insufficient rows in input string.\n\0".as_ptr() as *const c_char,
            );
            libc::free(input_copy as *mut c_void);
            free_matrix(mat);
            return std::ptr::null_mut();
        }

        let mut saveptr_col: *mut c_char = std::ptr::null_mut();
        let mut col_token = libc::strtok_r(row_token, col_delim, &mut saveptr_col);
        for j in 0..width {
            if col_token.is_null() {
                libc::fprintf(
                    stderr,
                    b"Insufficient columns in row %d.\n\0".as_ptr() as *const c_char,
                    i + 1,
                );
                libc::free(input_copy as *mut c_void);
                free_matrix(mat);
                return std::ptr::null_mut();
            }
            *cell_ptr(mat, i, j) = libc::atoi(col_token);
            col_token = libc::strtok_r(std::ptr::null_mut(), col_delim, &mut saveptr_col);
        }

        row_token = libc::strtok_r(std::ptr::null_mut(), row_delim, &mut saveptr_row);
    }

    libc::free(input_copy as *mut c_void);
    mat
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn multiply_matrices(
    mat_a: *mut matrix_t,
    mat_b: *mut matrix_t,
) -> *mut matrix_t {
    if (*mat_a).width != (*mat_b).height {
        libc::fprintf(
            stderr,
            b"Matrix dimensions do not allow multiplication.\n\0".as_ptr() as *const c_char,
        );
        return std::ptr::null_mut();
    }

    let result = allocate_matrix((*mat_b).width, (*mat_a).height);
    for i in 0..(*mat_a).height {
        for j in 0..(*mat_b).width {
            *cell_ptr(result, i, j) = 0;
            for k in 0..(*mat_a).width {
                let a_val = *cell_ptr(mat_a, i, k);
                let b_val = *cell_ptr(mat_b, k, j);
                // Match C int (signed two's complement) overflow wrap.
                let cur = *cell_ptr(result, i, j);
                *cell_ptr(result, i, j) = cur.wrapping_add(a_val.wrapping_mul(b_val));
            }
        }
    }

    result
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn matrix_to_string(mat: *mut matrix_t) -> *mut c_char {
    if mat.is_null() {
        libc::fprintf(
            stderr,
            b"Error: Matrix is NULL.\n\0".as_ptr() as *const c_char,
        );
        return std::ptr::null_mut();
    }

    let height = (*mat).height as usize;
    let width = (*mat).width as usize;

    // Match the original buffer-size formula exactly.
    let buffer_size = height * (width * 10 + width) + height + 1;
    let result = libc::malloc(buffer_size) as *mut c_char;
    if result.is_null() {
        libc::perror(b"Failed to allocate memory for matrix string\0".as_ptr() as *const c_char);
        return std::ptr::null_mut();
    }

    *result = 0;

    let space = b" \0".as_ptr() as *const c_char;
    let newline = b"\n\0".as_ptr() as *const c_char;

    for i in 0..(*mat).height {
        for j in 0..(*mat).width {
            let val = *cell_ptr(mat, i, j);
            // Equivalent to snprintf(buffer, 12, "%d", val) + strcat.
            let s = format!("{}\0", val);
            libc::strcat(result, s.as_ptr() as *const c_char);

            if j < (*mat).width - 1 {
                libc::strcat(result, space);
            }
        }
        libc::strcat(result, newline);
    }

    result
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn write_to_file(
    filename: *const c_char,
    content: *const c_char,
) -> c_int {
    if content.is_null() {
        libc::fprintf(
            stderr,
            b"Error: Content is NULL.\n\0".as_ptr() as *const c_char,
        );
        return libc::EINVAL;
    }

    let mode = b"w\0".as_ptr() as *const c_char;
    let file = libc::fopen(filename, mode);
    if file.is_null() {
        let err = *libc::__errno_location();
        libc::fprintf(
            stderr,
            b"Error opening file '%s': %s\n\0".as_ptr() as *const c_char,
            filename,
            libc::strerror(err),
        );
        return err;
    }

    if libc::fprintf(file, b"%s\0".as_ptr() as *const c_char, content) < 0 {
        let err = *libc::__errno_location();
        libc::fprintf(
            stderr,
            b"Error writing to file '%s': %s\n\0".as_ptr() as *const c_char,
            filename,
            libc::strerror(err),
        );
        libc::fclose(file);
        return err;
    }

    if libc::fclose(file) != 0 {
        let err = *libc::__errno_location();
        libc::fprintf(
            stderr,
            b"Error closing file '%s': %s\n\0".as_ptr() as *const c_char,
            filename,
            libc::strerror(err),
        );
        return err;
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
        // Preserve original C bug: uses free(res) instead of free_matrix(res).
        libc::free(res as *mut c_void);
        return EXIT_FAILURE;
    }

    let out_file = b"matrix.txt\0".as_ptr() as *const c_char;
    let res_write = write_to_file(out_file, res_str);

    free_matrix(mat_a);
    free_matrix(mat_b);
    free_matrix(res);
    libc::free(res_str as *mut c_void);

    if res_write != 0 {
        return EXIT_FAILURE;
    }

    EXIT_SUCCESS
}
