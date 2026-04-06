use std::ffi::{c_char, c_int};
use std::ptr;

// ── matrix_t ────────────────────────────────────────────────────────────────

#[repr(C)]
pub struct matrix_t {
    pub matrix: *mut *mut c_int,
    pub width: c_int,
    pub height: c_int,
}

// ── allocate_matrix ─────────────────────────────────────────────────────────

#[unsafe(no_mangle)]
pub unsafe extern "C" fn allocate_matrix(width: c_int, height: c_int) -> *mut matrix_t {
    let mat = libc::malloc(std::mem::size_of::<matrix_t>()) as *mut matrix_t;
    if mat.is_null() {
        libc::perror(b"Failed to allocate memory for matrix struct\0".as_ptr() as *const c_char);
        return ptr::null_mut();
    }

    (*mat).width = width;
    (*mat).height = height;

    (*mat).matrix = libc::malloc((height as usize) * std::mem::size_of::<*mut c_int>()) as *mut *mut c_int;
    if (*mat).matrix.is_null() {
        libc::perror(b"Failed to allocate memory for matrix rows\0".as_ptr() as *const c_char);
        libc::free(mat as *mut libc::c_void);
        return ptr::null_mut();
    }

    for i in 0..height as usize {
        let row = libc::malloc((width as usize) * std::mem::size_of::<c_int>()) as *mut c_int;
        if row.is_null() {
            libc::perror(b"Failed to allocate memory for matrix columns\0".as_ptr() as *const c_char);
            // C bug: frees 0..=i (inclusive), which frees the NULL row too.
            // We reproduce it exactly.
            for j in 0..=i {
                libc::free(*(*mat).matrix.add(j) as *mut libc::c_void);
            }
            libc::free((*mat).matrix as *mut libc::c_void);
            libc::free(mat as *mut libc::c_void);
            return ptr::null_mut();
        }
        *(*mat).matrix.add(i) = row;
    }

    mat
}

// ── free_matrix ─────────────────────────────────────────────────────────────

#[unsafe(no_mangle)]
pub unsafe extern "C" fn free_matrix(mat: *mut matrix_t) {
    if mat.is_null() {
        return;
    }
    for i in 0..(*mat).height as usize {
        libc::free(*(*mat).matrix.add(i) as *mut libc::c_void);
    }
    libc::free((*mat).matrix as *mut libc::c_void);
    libc::free(mat as *mut libc::c_void);
}

// ── initialize_matrix_from_string ───────────────────────────────────────────

#[unsafe(no_mangle)]
pub unsafe extern "C" fn initialize_matrix_from_string(
    input: *const c_char,
    width: c_int,
    height: c_int,
) -> *mut matrix_t {
    let mat = allocate_matrix(width, height);
    if mat.is_null() {
        return ptr::null_mut();
    }

    let input_copy = libc::strdup(input);
    if input_copy.is_null() {
        libc::perror(b"Failed to duplicate input string\0".as_ptr() as *const c_char);
        free_matrix(mat);
        return ptr::null_mut();
    }

    let mut saveptr_row: *mut c_char = ptr::null_mut();
    let newline = b"\n\0".as_ptr() as *const c_char;
    let space = b" \0".as_ptr() as *const c_char;

    let mut row_token = libc::strtok_r(input_copy, newline, &mut saveptr_row);

    for i in 0..height {
        if row_token.is_null() {
            libc::fprintf(
                libc_stderr(),
                b"Insufficient rows in input string.\n\0".as_ptr() as *const c_char,
            );
            libc::free(input_copy as *mut libc::c_void);
            free_matrix(mat);
            return ptr::null_mut();
        }

        let mut saveptr_col: *mut c_char = ptr::null_mut();
        let mut col_token = libc::strtok_r(row_token, space, &mut saveptr_col);

        for j in 0..width {
            if col_token.is_null() {
                libc::fprintf(
                    libc_stderr(),
                    b"Insufficient columns in row %d.\n\0".as_ptr() as *const c_char,
                    i + 1,
                );
                libc::free(input_copy as *mut libc::c_void);
                free_matrix(mat);
                return ptr::null_mut();
            }
            *(*(*mat).matrix.add(i as usize)).add(j as usize) = libc::atoi(col_token);
            col_token = libc::strtok_r(ptr::null_mut(), space, &mut saveptr_col);
        }

        row_token = libc::strtok_r(ptr::null_mut(), newline, &mut saveptr_row);
    }

    libc::free(input_copy as *mut libc::c_void);
    mat
}

// ── multiply_matrices ───────────────────────────────────────────────────────

#[unsafe(no_mangle)]
pub unsafe extern "C" fn multiply_matrices(
    mat_a: *mut matrix_t,
    mat_b: *mut matrix_t,
) -> *mut matrix_t {
    if (*mat_a).width != (*mat_b).height {
        libc::fprintf(
            libc_stderr(),
            b"Matrix dimensions do not allow multiplication.\n\0".as_ptr() as *const c_char,
        );
        return ptr::null_mut();
    }

    let result = allocate_matrix((*mat_b).width, (*mat_a).height);
    for i in 0..(*mat_a).height as usize {
        for j in 0..(*mat_b).width as usize {
            *(*(*result).matrix.add(i)).add(j) = 0;
            for k in 0..(*mat_a).width as usize {
                *(*(*result).matrix.add(i)).add(j) +=
                    *(*(*mat_a).matrix.add(i)).add(k) * *(*(*mat_b).matrix.add(k)).add(j);
            }
        }
    }

    result
}

// ── matrix_to_string ────────────────────────────────────────────────────────

#[unsafe(no_mangle)]
pub unsafe extern "C" fn matrix_to_string(mat: *mut matrix_t) -> *mut c_char {
    if mat.is_null() {
        libc::fprintf(
            libc_stderr(),
            b"Error: Matrix is NULL.\n\0".as_ptr() as *const c_char,
        );
        return ptr::null_mut();
    }

    let h = (*mat).height;
    let w = (*mat).width;
    let buffer_size = (h * (w * 10 + w) + h + 1) as usize;
    let result = libc::malloc(buffer_size) as *mut c_char;
    if result.is_null() {
        libc::perror(b"Failed to allocate memory for matrix string\0".as_ptr() as *const c_char);
        return ptr::null_mut();
    }
    *result = 0;

    for i in 0..h as usize {
        for j in 0..w as usize {
            let mut buffer = [0u8; 12];
            libc::snprintf(
                buffer.as_mut_ptr() as *mut c_char,
                buffer.len(),
                b"%d\0".as_ptr() as *const c_char,
                *(*(*mat).matrix.add(i)).add(j),
            );
            libc::strcat(result, buffer.as_ptr() as *const c_char);

            if (j as c_int) < w - 1 {
                libc::strcat(result, b" \0".as_ptr() as *const c_char);
            }
        }
        libc::strcat(result, b"\n\0".as_ptr() as *const c_char);
    }

    result
}

// ── write_to_file ───────────────────────────────────────────────────────────

#[unsafe(no_mangle)]
pub unsafe extern "C" fn write_to_file(
    filename: *const c_char,
    content: *const c_char,
) -> c_int {
    if content.is_null() {
        libc::fprintf(
            libc_stderr(),
            b"Error: Content is NULL.\n\0".as_ptr() as *const c_char,
        );
        return libc::EINVAL;
    }

    let file = libc::fopen(filename, b"w\0".as_ptr() as *const c_char);
    if file.is_null() {
        libc::fprintf(
            libc_stderr(),
            b"Error opening file '%s': %s\n\0".as_ptr() as *const c_char,
            filename,
            libc::strerror(*libc::__errno_location()),
        );
        return *libc::__errno_location();
    }

    if libc::fprintf(file, b"%s\0".as_ptr() as *const c_char, content) < 0 {
        libc::fprintf(
            libc_stderr(),
            b"Error writing to file '%s': %s\n\0".as_ptr() as *const c_char,
            filename,
            libc::strerror(*libc::__errno_location()),
        );
        libc::fclose(file);
        return *libc::__errno_location();
    }

    if libc::fclose(file) != 0 {
        libc::fprintf(
            libc_stderr(),
            b"Error closing file '%s': %s\n\0".as_ptr() as *const c_char,
            filename,
            libc::strerror(*libc::__errno_location()),
        );
        return *libc::__errno_location();
    }

    0
}

// ── driver ──────────────────────────────────────────────────────────────────

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
        return libc::EXIT_FAILURE;
    }
    let mat_b = initialize_matrix_from_string(matrix_b, width_b, height_b);
    if mat_b.is_null() {
        free_matrix(mat_a);
        return libc::EXIT_FAILURE;
    }

    let res = multiply_matrices(mat_a, mat_b);
    if res.is_null() {
        free_matrix(mat_a);
        free_matrix(mat_b);
        return libc::EXIT_FAILURE;
    }
    let res_str = matrix_to_string(res);
    if res_str.is_null() {
        free_matrix(mat_a);
        free_matrix(mat_b);
        // C bug: uses free(res) instead of free_matrix(res). Reproduce exactly.
        libc::free(res as *mut libc::c_void);
        return libc::EXIT_FAILURE;
    }

    let out_file = b"matrix.txt\0".as_ptr() as *const c_char;
    let res_write = write_to_file(out_file, res_str);

    free_matrix(mat_a);
    free_matrix(mat_b);
    free_matrix(res);
    libc::free(res_str as *mut libc::c_void);

    if res_write != 0 {
        return libc::EXIT_FAILURE;
    }

    libc::EXIT_SUCCESS
}

// ── helper: get stderr FILE* ────────────────────────────────────────────────

unsafe fn libc_stderr() -> *mut libc::FILE {
    // On Linux glibc, stderr is a global symbol
    extern "C" {
        static mut stderr: *mut libc::FILE;
    }
    stderr
}
