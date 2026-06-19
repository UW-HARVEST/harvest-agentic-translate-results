use std::ffi::{c_char, c_int, c_void};
use std::mem;
use std::ptr;

unsafe extern "C" {
    static mut stderr: *mut libc::FILE;
}

#[repr(C)]
pub struct matrix_t {
    pub matrix: *mut *mut c_int,
    pub width: c_int,
    pub height: c_int,
}

const EXIT_SUCCESS: c_int = 0;
const EXIT_FAILURE: c_int = 1;
const EINVAL_VALUE: c_int = 22;

const FAILED_MATRIX_STRUCT: &[u8] = b"Failed to allocate memory for matrix struct\0";
const FAILED_MATRIX_ROWS: &[u8] = b"Failed to allocate memory for matrix rows\0";
const FAILED_MATRIX_COLUMNS: &[u8] = b"Failed to allocate memory for matrix columns\0";
const FAILED_DUP_INPUT: &[u8] = b"Failed to duplicate input string\0";
const FAILED_MATRIX_STRING: &[u8] = b"Failed to allocate memory for matrix string\0";

const INSUFFICIENT_ROWS: &[u8] = b"Insufficient rows in input string.\n\0";
const INSUFFICIENT_COLUMNS: &[u8] = b"Insufficient columns in row %d.\n\0";
const DIMENSIONS_ERROR: &[u8] = b"Matrix dimensions do not allow multiplication.\n\0";
const NULL_MATRIX_ERROR: &[u8] = b"Error: Matrix is NULL.\n\0";
const NULL_CONTENT_ERROR: &[u8] = b"Error: Content is NULL.\n\0";
const OPEN_ERROR: &[u8] = b"Error opening file '%s': %s\n\0";
const WRITE_ERROR: &[u8] = b"Error writing to file '%s': %s\n\0";
const CLOSE_ERROR: &[u8] = b"Error closing file '%s': %s\n\0";

const NEWLINE_DELIM: &[u8] = b"\n\0";
const SPACE_DELIM: &[u8] = b" \0";
const WRITE_MODE: &[u8] = b"w\0";
const STRING_FORMAT: &[u8] = b"%s\0";
const INT_FORMAT: &[u8] = b"%d\0";
const SPACE_STRING: &[u8] = b" \0";
const NEWLINE_STRING: &[u8] = b"\n\0";
const OUT_FILE: &[u8] = b"matrix.txt\0";

unsafe fn errno_location() -> *mut c_int {
    #[cfg(any(target_os = "linux", target_os = "android"))]
    {
        unsafe { libc::__errno_location() }
    }
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    {
        unsafe { libc::__error() }
    }
}

unsafe fn current_errno() -> c_int {
    unsafe { *errno_location() }
}

unsafe fn c_stderr() -> *mut libc::FILE {
    unsafe { stderr }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn allocate_matrix(width: c_int, height: c_int) -> *mut matrix_t {
    let mat = unsafe { libc::malloc(mem::size_of::<matrix_t>()) as *mut matrix_t };
    if mat.is_null() {
        unsafe { libc::perror(FAILED_MATRIX_STRUCT.as_ptr() as *const c_char) };
        return ptr::null_mut();
    }

    unsafe {
        (*mat).width = width;
        (*mat).height = height;
    }

    let row_bytes = (height as usize).wrapping_mul(mem::size_of::<*mut c_int>());
    let rows = unsafe { libc::malloc(row_bytes) as *mut *mut c_int };
    unsafe {
        (*mat).matrix = rows;
    }
    if rows.is_null() {
        unsafe {
            libc::perror(FAILED_MATRIX_ROWS.as_ptr() as *const c_char);
            libc::free(mat as *mut c_void);
        }
        return ptr::null_mut();
    }

    for i in 0..height {
        let col_bytes = (width as usize).wrapping_mul(mem::size_of::<c_int>());
        let row = unsafe { libc::malloc(col_bytes) as *mut c_int };
        unsafe {
            *rows.add(i as usize) = row;
        }
        if row.is_null() {
            unsafe {
                libc::perror(FAILED_MATRIX_COLUMNS.as_ptr() as *const c_char);
                for j in 0..=i {
                    libc::free(*rows.add(j as usize) as *mut c_void);
                }
                libc::free(rows as *mut c_void);
                libc::free(mat as *mut c_void);
            }
            return ptr::null_mut();
        }
    }

    mat
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn free_matrix(mat: *mut matrix_t) {
    if mat.is_null() {
        return;
    }

    unsafe {
        for i in 0..(*mat).height {
            libc::free(*(*mat).matrix.add(i as usize) as *mut c_void);
        }
        libc::free((*mat).matrix as *mut c_void);
        libc::free(mat as *mut c_void);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn initialize_matrix_from_string(
    input: *const c_char,
    width: c_int,
    height: c_int,
) -> *mut matrix_t {
    let mat = unsafe { allocate_matrix(width, height) };

    let input_copy = unsafe { libc::strdup(input) };
    if input_copy.is_null() {
        unsafe {
            libc::perror(FAILED_DUP_INPUT.as_ptr() as *const c_char);
            free_matrix(mat);
        }
        return ptr::null_mut();
    }

    let mut saveptr_row: *mut c_char = ptr::null_mut();
    let mut row_token = unsafe {
        libc::strtok_r(
            input_copy,
            NEWLINE_DELIM.as_ptr() as *const c_char,
            &mut saveptr_row,
        )
    };

    for i in 0..height {
        if row_token.is_null() {
            unsafe {
                libc::fprintf(c_stderr(), INSUFFICIENT_ROWS.as_ptr() as *const c_char);
                libc::free(input_copy as *mut c_void);
                free_matrix(mat);
            }
            return ptr::null_mut();
        }

        let mut saveptr_col: *mut c_char = ptr::null_mut();
        let mut col_token = unsafe {
            libc::strtok_r(
                row_token,
                SPACE_DELIM.as_ptr() as *const c_char,
                &mut saveptr_col,
            )
        };

        for j in 0..width {
            if col_token.is_null() {
                unsafe {
                    libc::fprintf(
                        c_stderr(),
                        INSUFFICIENT_COLUMNS.as_ptr() as *const c_char,
                        i + 1,
                    );
                    libc::free(input_copy as *mut c_void);
                    free_matrix(mat);
                }
                return ptr::null_mut();
            }
            unsafe {
                *(*(*mat).matrix.add(i as usize)).add(j as usize) = libc::atoi(col_token);
                col_token = libc::strtok_r(
                    ptr::null_mut(),
                    SPACE_DELIM.as_ptr() as *const c_char,
                    &mut saveptr_col,
                );
            }
        }

        row_token = unsafe {
            libc::strtok_r(
                ptr::null_mut(),
                NEWLINE_DELIM.as_ptr() as *const c_char,
                &mut saveptr_row,
            )
        };
    }

    unsafe { libc::free(input_copy as *mut c_void) };
    mat
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn multiply_matrices(
    mat_a: *mut matrix_t,
    mat_b: *mut matrix_t,
) -> *mut matrix_t {
    unsafe {
        if (*mat_a).width != (*mat_b).height {
            libc::fprintf(c_stderr(), DIMENSIONS_ERROR.as_ptr() as *const c_char);
            return ptr::null_mut();
        }

        let result = allocate_matrix((*mat_b).width, (*mat_a).height);
        for i in 0..(*mat_a).height {
            for j in 0..(*mat_b).width {
                *(*(*result).matrix.add(i as usize)).add(j as usize) = 0;
                for k in 0..(*mat_a).width {
                    let a = *(*(*mat_a).matrix.add(i as usize)).add(k as usize);
                    let b = *(*(*mat_b).matrix.add(k as usize)).add(j as usize);
                    let cell = (*(*(*result).matrix.add(i as usize)).add(j as usize))
                        .wrapping_add(a.wrapping_mul(b));
                    *(*(*result).matrix.add(i as usize)).add(j as usize) = cell;
                }
            }
        }

        result
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn matrix_to_string(mat: *mut matrix_t) -> *mut c_char {
    if mat.is_null() {
        unsafe {
            libc::fprintf(c_stderr(), NULL_MATRIX_ERROR.as_ptr() as *const c_char);
        }
        return ptr::null_mut();
    }

    let buffer_size = unsafe {
        (*mat)
            .height
            .wrapping_mul((*mat).width.wrapping_mul(10).wrapping_add((*mat).width))
            .wrapping_add((*mat).height)
            .wrapping_add(1)
    };
    let result = unsafe { libc::malloc(buffer_size as usize) as *mut c_char };
    if result.is_null() {
        unsafe { libc::perror(FAILED_MATRIX_STRING.as_ptr() as *const c_char) };
        return ptr::null_mut();
    }

    unsafe {
        *result = 0;

        for i in 0..(*mat).height {
            for j in 0..(*mat).width {
                let mut buffer = [0 as c_char; 12];
                libc::snprintf(
                    buffer.as_mut_ptr(),
                    buffer.len(),
                    INT_FORMAT.as_ptr() as *const c_char,
                    *(*(*mat).matrix.add(i as usize)).add(j as usize),
                );
                libc::strcat(result, buffer.as_ptr());

                if j < (*mat).width - 1 {
                    libc::strcat(result, SPACE_STRING.as_ptr() as *const c_char);
                }
            }
            libc::strcat(result, NEWLINE_STRING.as_ptr() as *const c_char);
        }
    }

    result
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn write_to_file(filename: *const c_char, content: *const c_char) -> c_int {
    if content.is_null() {
        unsafe {
            libc::fprintf(c_stderr(), NULL_CONTENT_ERROR.as_ptr() as *const c_char);
        }
        return EINVAL_VALUE;
    }

    let file = unsafe { libc::fopen(filename, WRITE_MODE.as_ptr() as *const c_char) };
    if file.is_null() {
        let err = unsafe { current_errno() };
        unsafe {
            libc::fprintf(
                c_stderr(),
                OPEN_ERROR.as_ptr() as *const c_char,
                filename,
                libc::strerror(err),
            );
        }
        return err;
    }

    let wrote = unsafe { libc::fprintf(file, STRING_FORMAT.as_ptr() as *const c_char, content) };
    if wrote < 0 {
        let err = unsafe { current_errno() };
        unsafe {
            libc::fprintf(
                c_stderr(),
                WRITE_ERROR.as_ptr() as *const c_char,
                filename,
                libc::strerror(err),
            );
            libc::fclose(file);
        }
        return err;
    }

    if unsafe { libc::fclose(file) } != 0 {
        let err = unsafe { current_errno() };
        unsafe {
            libc::fprintf(
                c_stderr(),
                CLOSE_ERROR.as_ptr() as *const c_char,
                filename,
                libc::strerror(err),
            );
        }
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
    let mat_a = unsafe { initialize_matrix_from_string(matrix_a, width_a, height_a) };
    if mat_a.is_null() {
        return EXIT_FAILURE;
    }

    let mat_b = unsafe { initialize_matrix_from_string(matrix_b, width_b, height_b) };
    if mat_b.is_null() {
        unsafe { free_matrix(mat_a) };
        return EXIT_FAILURE;
    }

    let res = unsafe { multiply_matrices(mat_a, mat_b) };
    if res.is_null() {
        unsafe {
            free_matrix(mat_a);
            free_matrix(mat_b);
        }
        return EXIT_FAILURE;
    }

    let res_str = unsafe { matrix_to_string(res) };
    if res_str.is_null() {
        unsafe {
            free_matrix(mat_a);
            free_matrix(mat_b);
            libc::free(res as *mut c_void);
        }
        return EXIT_FAILURE;
    }

    let res_write = unsafe { write_to_file(OUT_FILE.as_ptr() as *const c_char, res_str) };

    unsafe {
        free_matrix(mat_a);
        free_matrix(mat_b);
        free_matrix(res);
        libc::free(res_str as *mut c_void);
    }

    if res_write != 0 {
        return EXIT_FAILURE;
    }

    EXIT_SUCCESS
}
