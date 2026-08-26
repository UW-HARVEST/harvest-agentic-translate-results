use std::ffi::{c_char, c_int, c_void};
use std::mem::size_of;
use std::ptr;

const EINVAL: c_int = 22;
const EXIT_FAILURE: c_int = 1;
const EXIT_SUCCESS: c_int = 0;

#[repr(C)]
pub struct Matrix {
    pub matrix: *mut *mut c_int,
    pub width: c_int,
    pub height: c_int,
}

unsafe extern "C" {
    static mut stderr: *mut c_void;

    fn __errno_location() -> *mut c_int;
    fn atoi(value: *const c_char) -> c_int;
    fn fclose(stream: *mut c_void) -> c_int;
    fn fopen(filename: *const c_char, mode: *const c_char) -> *mut c_void;
    fn fprintf(stream: *mut c_void, format: *const c_char, ...) -> c_int;
    fn free(pointer: *mut c_void);
    fn malloc(size: usize) -> *mut c_void;
    fn perror(message: *const c_char);
    fn snprintf(buffer: *mut c_char, size: usize, format: *const c_char, ...) -> c_int;
    fn strcat(destination: *mut c_char, source: *const c_char) -> *mut c_char;
    fn strdup(value: *const c_char) -> *mut c_char;
    fn strerror(error: c_int) -> *mut c_char;
    fn strtok_r(
        value: *mut c_char,
        delimiters: *const c_char,
        save_pointer: *mut *mut c_char,
    ) -> *mut c_char;
}

#[inline]
unsafe fn current_errno() -> c_int {
    unsafe { *__errno_location() }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn allocate_matrix(width: c_int, height: c_int) -> *mut Matrix {
    let mat = unsafe { malloc(size_of::<Matrix>()) }.cast::<Matrix>();
    if mat.is_null() {
        unsafe { perror(c"Failed to allocate memory for matrix struct".as_ptr()) };
        return ptr::null_mut();
    }

    unsafe {
        (*mat).width = width;
        (*mat).height = height;
        (*mat).matrix = malloc((height as usize).wrapping_mul(size_of::<*mut c_int>())).cast();
    }
    if unsafe { (*mat).matrix.is_null() } {
        unsafe {
            perror(c"Failed to allocate memory for matrix rows".as_ptr());
            free(mat.cast());
        }
        return ptr::null_mut();
    }

    let mut i = 0;
    while i < height {
        unsafe {
            *(*mat).matrix.add(i as usize) =
                malloc((width as usize).wrapping_mul(size_of::<c_int>())).cast();
        }
        if unsafe { (*(*mat).matrix.add(i as usize)).is_null() } {
            unsafe { perror(c"Failed to allocate memory for matrix columns".as_ptr()) };
            let mut j = 0;
            while j <= i {
                unsafe { free((*(*mat).matrix.add(j as usize)).cast()) };
                j += 1;
            }
            unsafe {
                free((*mat).matrix.cast());
                free(mat.cast());
            }
            return ptr::null_mut();
        }
        i += 1;
    }

    mat
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn free_matrix(mat: *mut Matrix) {
    if mat.is_null() {
        return;
    }

    let mut i = 0;
    while i < unsafe { (*mat).height } {
        unsafe { free((*(*mat).matrix.add(i as usize)).cast()) };
        i += 1;
    }
    unsafe {
        free((*mat).matrix.cast());
        free(mat.cast());
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn initialize_matrix_from_string(
    input: *const c_char,
    width: c_int,
    height: c_int,
) -> *mut Matrix {
    let mat = unsafe { allocate_matrix(width, height) };

    let input_copy = unsafe { strdup(input) };
    if input_copy.is_null() {
        unsafe {
            perror(c"Failed to duplicate input string".as_ptr());
            free_matrix(mat);
        }
        return ptr::null_mut();
    }

    let mut saveptr_row = ptr::null_mut();
    let mut row_token = unsafe { strtok_r(input_copy, c"\n".as_ptr(), &mut saveptr_row) };
    let mut i = 0;
    while i < height {
        if row_token.is_null() {
            unsafe {
                fprintf(stderr, c"Insufficient rows in input string.\n".as_ptr());
                free(input_copy.cast());
                free_matrix(mat);
            }
            return ptr::null_mut();
        }

        let mut saveptr_col = ptr::null_mut();
        let mut col_token = unsafe { strtok_r(row_token, c" ".as_ptr(), &mut saveptr_col) };
        let mut j = 0;
        while j < width {
            if col_token.is_null() {
                unsafe {
                    fprintf(stderr, c"Insufficient columns in row %d.\n".as_ptr(), i + 1);
                    free(input_copy.cast());
                    free_matrix(mat);
                }
                return ptr::null_mut();
            }
            unsafe {
                *(*(*mat).matrix.add(i as usize)).add(j as usize) = atoi(col_token);
                col_token = strtok_r(ptr::null_mut(), c" ".as_ptr(), &mut saveptr_col);
            }
            j += 1;
        }

        row_token = unsafe { strtok_r(ptr::null_mut(), c"\n".as_ptr(), &mut saveptr_row) };
        i += 1;
    }

    unsafe { free(input_copy.cast()) };
    mat
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn multiply_matrices(mat_a: *mut Matrix, mat_b: *mut Matrix) -> *mut Matrix {
    if unsafe { (*mat_a).width != (*mat_b).height } {
        unsafe {
            fprintf(
                stderr,
                c"Matrix dimensions do not allow multiplication.\n".as_ptr(),
            )
        };
        return ptr::null_mut();
    }

    let result = unsafe { allocate_matrix((*mat_b).width, (*mat_a).height) };
    let mut i = 0;
    while i < unsafe { (*mat_a).height } {
        let mut j = 0;
        while j < unsafe { (*mat_b).width } {
            unsafe { *(*(*result).matrix.add(i as usize)).add(j as usize) = 0 };
            let mut k = 0;
            while k < unsafe { (*mat_a).width } {
                unsafe {
                    let destination = (*(*result).matrix.add(i as usize)).add(j as usize);
                    let left = *(*(*mat_a).matrix.add(i as usize)).add(k as usize);
                    let right = *(*(*mat_b).matrix.add(k as usize)).add(j as usize);
                    *destination = (*destination).wrapping_add(left.wrapping_mul(right));
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
pub unsafe extern "C" fn matrix_to_string(mat: *mut Matrix) -> *mut c_char {
    if mat.is_null() {
        unsafe { fprintf(stderr, c"Error: Matrix is NULL.\n".as_ptr()) };
        return ptr::null_mut();
    }

    let width = unsafe { (*mat).width };
    let height = unsafe { (*mat).height };
    let buffer_size = height
        .wrapping_mul(width.wrapping_mul(10).wrapping_add(width))
        .wrapping_add(height)
        .wrapping_add(1);
    let result = unsafe { malloc(buffer_size as usize) }.cast::<c_char>();
    if result.is_null() {
        unsafe { perror(c"Failed to allocate memory for matrix string".as_ptr()) };
        return ptr::null_mut();
    }

    unsafe { *result = 0 };

    let mut i = 0;
    while i < height {
        let mut j = 0;
        while j < width {
            let mut buffer = [0 as c_char; 12];
            unsafe {
                snprintf(
                    buffer.as_mut_ptr(),
                    buffer.len(),
                    c"%d".as_ptr(),
                    *(*(*mat).matrix.add(i as usize)).add(j as usize),
                );
                strcat(result, buffer.as_ptr());
            }

            if j < width - 1 {
                unsafe { strcat(result, c" ".as_ptr()) };
            }
            j += 1;
        }
        unsafe { strcat(result, c"\n".as_ptr()) };
        i += 1;
    }

    result
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn write_to_file(filename: *const c_char, content: *const c_char) -> c_int {
    if content.is_null() {
        unsafe { fprintf(stderr, c"Error: Content is NULL.\n".as_ptr()) };
        return EINVAL;
    }

    let file = unsafe { fopen(filename, c"w".as_ptr()) };
    if file.is_null() {
        unsafe {
            fprintf(
                stderr,
                c"Error opening file '%s': %s\n".as_ptr(),
                filename,
                strerror(current_errno()),
            );
            return current_errno();
        }
    }

    if unsafe { fprintf(file, c"%s".as_ptr(), content) } < 0 {
        unsafe {
            fprintf(
                stderr,
                c"Error writing to file '%s': %s\n".as_ptr(),
                filename,
                strerror(current_errno()),
            );
            fclose(file);
            return current_errno();
        }
    }

    if unsafe { fclose(file) } != 0 {
        unsafe {
            fprintf(
                stderr,
                c"Error closing file '%s': %s\n".as_ptr(),
                filename,
                strerror(current_errno()),
            );
            return current_errno();
        }
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

    let result = unsafe { multiply_matrices(mat_a, mat_b) };
    if result.is_null() {
        unsafe {
            free_matrix(mat_a);
            free_matrix(mat_b);
        }
        return EXIT_FAILURE;
    }
    let result_string = unsafe { matrix_to_string(result) };
    if result_string.is_null() {
        unsafe {
            free_matrix(mat_a);
            free_matrix(mat_b);
            free(result.cast());
        }
        return EXIT_FAILURE;
    }

    let write_result = unsafe { write_to_file(c"matrix.txt".as_ptr(), result_string) };

    unsafe {
        free_matrix(mat_a);
        free_matrix(mat_b);
        free_matrix(result);
        free(result_string.cast());
    }

    if write_result != 0 {
        return EXIT_FAILURE;
    }

    EXIT_SUCCESS
}
