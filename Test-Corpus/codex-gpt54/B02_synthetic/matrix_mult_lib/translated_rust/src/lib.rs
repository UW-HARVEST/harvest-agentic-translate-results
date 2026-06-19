use libc::{self, EXIT_FAILURE, EXIT_SUCCESS};
use std::ffi::{c_char, c_int, c_void};
use std::ptr;

unsafe extern "C" {
    static mut stderr: *mut libc::FILE;
}

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct matrix_t {
    pub matrix: *mut *mut c_int,
    pub width: c_int,
    pub height: c_int,
}

#[inline]
unsafe fn current_errno() -> c_int {
    // Linux/glibc target in this environment exposes errno through __errno_location.
    unsafe { *libc::__errno_location() }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn allocate_matrix(width: c_int, height: c_int) -> *mut matrix_t {
    let mat = unsafe { libc::malloc(std::mem::size_of::<matrix_t>()) as *mut matrix_t };
    if mat.is_null() {
        unsafe {
            libc::perror(b"Failed to allocate memory for matrix struct\0".as_ptr().cast());
        }
        return ptr::null_mut();
    }

    unsafe {
        (*mat).width = width;
        (*mat).height = height;
    }

    let row_bytes = (height as usize).wrapping_mul(std::mem::size_of::<*mut c_int>());
    unsafe {
        (*mat).matrix = libc::malloc(row_bytes) as *mut *mut c_int;
    }
    if unsafe { (*mat).matrix }.is_null() {
        unsafe {
            libc::perror(b"Failed to allocate memory for matrix rows\0".as_ptr().cast());
            libc::free(mat.cast::<c_void>());
        }
        return ptr::null_mut();
    }

    let mut i = 0;
    while i < height {
        let col_bytes = (width as usize).wrapping_mul(std::mem::size_of::<c_int>());
        let row = unsafe { libc::malloc(col_bytes) as *mut c_int };
        unsafe {
            *(*mat).matrix.add(i as usize) = row;
        }
        if row.is_null() {
            unsafe {
                libc::perror(b"Failed to allocate memory for matrix columns\0".as_ptr().cast());
            }
            let mut j = 0;
            while j <= i {
                unsafe {
                    libc::free((*(*mat).matrix.add(j as usize)).cast::<c_void>());
                }
                j += 1;
            }
            unsafe {
                libc::free((*mat).matrix.cast::<c_void>());
                libc::free(mat.cast::<c_void>());
            }
            return ptr::null_mut();
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

    let mut i = 0;
    while i < unsafe { (*mat).height } {
        unsafe {
            libc::free((*(*mat).matrix.add(i as usize)).cast::<c_void>());
        }
        i += 1;
    }
    unsafe {
        libc::free((*mat).matrix.cast::<c_void>());
        libc::free(mat.cast::<c_void>());
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
            libc::perror(b"Failed to duplicate input string\0".as_ptr().cast());
            free_matrix(mat);
        }
        return ptr::null_mut();
    }

    let mut saveptr_row: *mut c_char = ptr::null_mut();
    let mut row_token =
        unsafe { libc::strtok_r(input_copy, b"\n\0".as_ptr().cast(), &mut saveptr_row) };

    let mut i = 0;
    while i < height {
        if row_token.is_null() {
            unsafe {
                    libc::fprintf(
                    stderr,
                    b"Insufficient rows in input string.\n\0".as_ptr().cast(),
                );
                libc::free(input_copy.cast::<c_void>());
                free_matrix(mat);
            }
            return ptr::null_mut();
        }

        let mut saveptr_col: *mut c_char = ptr::null_mut();
        let mut col_token =
            unsafe { libc::strtok_r(row_token, b" \0".as_ptr().cast(), &mut saveptr_col) };

        let mut j = 0;
        while j < width {
            if col_token.is_null() {
                unsafe {
                    libc::fprintf(
                        stderr,
                        b"Insufficient columns in row %d.\n\0".as_ptr().cast(),
                        i + 1,
                    );
                    libc::free(input_copy.cast::<c_void>());
                    free_matrix(mat);
                }
                return ptr::null_mut();
            }

            unsafe {
                *(*(*mat).matrix.add(i as usize)).add(j as usize) = libc::atoi(col_token);
            }
            col_token = unsafe { libc::strtok_r(ptr::null_mut(), b" \0".as_ptr().cast(), &mut saveptr_col) };
            j += 1;
        }

        row_token = unsafe { libc::strtok_r(ptr::null_mut(), b"\n\0".as_ptr().cast(), &mut saveptr_row) };
        i += 1;
    }

    unsafe {
        libc::free(input_copy.cast::<c_void>());
    }
    mat
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn multiply_matrices(
    mat_a: *mut matrix_t,
    mat_b: *mut matrix_t,
) -> *mut matrix_t {
    if unsafe { (*mat_a).width != (*mat_b).height } {
        unsafe {
            libc::fprintf(
                stderr,
                b"Matrix dimensions do not allow multiplication.\n\0"
                    .as_ptr()
                    .cast(),
            );
        }
        return ptr::null_mut();
    }

    let result = unsafe { allocate_matrix((*mat_b).width, (*mat_a).height) };

    let mut i = 0;
    while i < unsafe { (*mat_a).height } {
        let mut j = 0;
        while j < unsafe { (*mat_b).width } {
            unsafe {
                *(*(*result).matrix.add(i as usize)).add(j as usize) = 0;
            }
            let mut k = 0;
            while k < unsafe { (*mat_a).width } {
                unsafe {
                    *(*(*result).matrix.add(i as usize)).add(j as usize) +=
                        *(*(*mat_a).matrix.add(i as usize)).add(k as usize)
                            * *(*(*mat_b).matrix.add(k as usize)).add(j as usize);
                }
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
        unsafe {
            libc::fprintf(stderr, b"Error: Matrix is NULL.\n\0".as_ptr().cast());
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
        unsafe {
            libc::perror(b"Failed to allocate memory for matrix string\0".as_ptr().cast());
        }
        return ptr::null_mut();
    }

    unsafe {
        *result = 0;
    }

    let mut i = 0;
    while i < unsafe { (*mat).height } {
        let mut j = 0;
        while j < unsafe { (*mat).width } {
            let mut buffer = [0 as c_char; 12];
            unsafe {
                libc::snprintf(
                    buffer.as_mut_ptr(),
                    buffer.len(),
                    b"%d\0".as_ptr().cast(),
                    *(*(*mat).matrix.add(i as usize)).add(j as usize),
                );
                libc::strcat(result, buffer.as_ptr());
            }

            if j < unsafe { (*mat).width - 1 } {
                unsafe {
                    libc::strcat(result, b" \0".as_ptr().cast());
                }
            }

            j += 1;
        }
        unsafe {
            libc::strcat(result, b"\n\0".as_ptr().cast());
        }
        i += 1;
    }

    result
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn write_to_file(filename: *const c_char, content: *const c_char) -> c_int {
    if content.is_null() {
        unsafe {
            libc::fprintf(stderr, b"Error: Content is NULL.\n\0".as_ptr().cast());
        }
        return libc::EINVAL;
    }

    let file = unsafe { libc::fopen(filename, b"w\0".as_ptr().cast()) };
    if file.is_null() {
        unsafe {
            libc::fprintf(
                stderr,
                b"Error opening file '%s': %s\n\0".as_ptr().cast(),
                filename,
                libc::strerror(current_errno()),
            );
        }
        return unsafe { current_errno() };
    }

    if unsafe { libc::fprintf(file, b"%s\0".as_ptr().cast(), content) } < 0 {
        unsafe {
            libc::fprintf(
                stderr,
                b"Error writing to file '%s': %s\n\0".as_ptr().cast(),
                filename,
                libc::strerror(current_errno()),
            );
            libc::fclose(file);
        }
        return unsafe { current_errno() };
    }

    if unsafe { libc::fclose(file) } != 0 {
        unsafe {
            libc::fprintf(
                stderr,
                b"Error closing file '%s': %s\n\0".as_ptr().cast(),
                filename,
                libc::strerror(current_errno()),
            );
        }
        return unsafe { current_errno() };
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
        unsafe {
            free_matrix(mat_a);
        }
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
            libc::free(res.cast::<c_void>());
        }
        return EXIT_FAILURE;
    }

    let res_write = unsafe { write_to_file(b"matrix.txt\0".as_ptr().cast(), res_str) };

    unsafe {
        free_matrix(mat_a);
        free_matrix(mat_b);
        free_matrix(res);
        libc::free(res_str.cast::<c_void>());
    }

    if res_write != 0 {
        return EXIT_FAILURE;
    }

    EXIT_SUCCESS
}
