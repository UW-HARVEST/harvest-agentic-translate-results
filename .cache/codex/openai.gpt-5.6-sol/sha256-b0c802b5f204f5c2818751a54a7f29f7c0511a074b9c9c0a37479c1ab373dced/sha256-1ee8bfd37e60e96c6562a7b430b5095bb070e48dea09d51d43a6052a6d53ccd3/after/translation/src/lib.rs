use std::ffi::{c_char, c_int, c_void};
use std::mem::size_of;
use std::ptr::null_mut;

type File = c_void;

unsafe extern "C" {
    static mut stderr: *mut File;

    fn malloc(size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
    fn perror(message: *const c_char);
    fn strdup(source: *const c_char) -> *mut c_char;
    fn strtok_r(
        string: *mut c_char,
        delimiters: *const c_char,
        save_pointer: *mut *mut c_char,
    ) -> *mut c_char;
    fn atoi(string: *const c_char) -> c_int;
    fn snprintf(buffer: *mut c_char, size: usize, format: *const c_char, ...) -> c_int;
    fn strcat(destination: *mut c_char, source: *const c_char) -> *mut c_char;
    fn fprintf(stream: *mut File, format: *const c_char, ...) -> c_int;
    fn fopen(filename: *const c_char, mode: *const c_char) -> *mut File;
    fn fclose(stream: *mut File) -> c_int;
    fn strerror(error_number: c_int) -> *mut c_char;
    fn __errno_location() -> *mut c_int;
}

#[repr(C)]
pub struct matrix_t {
    pub matrix: *mut *mut c_int,
    pub width: c_int,
    pub height: c_int,
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn allocate_matrix(width: c_int, height: c_int) -> *mut matrix_t {
    unsafe {
        let mat = malloc(size_of::<matrix_t>()).cast::<matrix_t>();
        if mat.is_null() {
            perror(c"Failed to allocate memory for matrix struct".as_ptr());
            return null_mut();
        }

        (*mat).width = width;
        (*mat).height = height;

        (*mat).matrix =
            malloc((height as usize).wrapping_mul(size_of::<*mut c_int>())).cast::<*mut c_int>();
        if (*mat).matrix.is_null() {
            perror(c"Failed to allocate memory for matrix rows".as_ptr());
            free(mat.cast());
            return null_mut();
        }

        let mut i = 0;
        while i < height {
            let row = malloc((width as usize).wrapping_mul(size_of::<c_int>())).cast::<c_int>();
            *(*mat).matrix.add(i as usize) = row;
            if row.is_null() {
                perror(c"Failed to allocate memory for matrix columns".as_ptr());
                let mut j = 0;
                while j <= i {
                    free((*(*mat).matrix.add(j as usize)).cast());
                    j += 1;
                }
                free((*mat).matrix.cast());
                free(mat.cast());
                return null_mut();
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

        let mut i = 0;
        while i < (*mat).height {
            free((*(*mat).matrix.add(i as usize)).cast());
            i += 1;
        }
        free((*mat).matrix.cast());
        free(mat.cast());
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn initialize_matrix_from_string(
    input: *const c_char,
    width: c_int,
    height: c_int,
) -> *mut matrix_t {
    unsafe {
        let mat = allocate_matrix(width, height);

        let input_copy = strdup(input);
        if input_copy.is_null() {
            perror(c"Failed to duplicate input string".as_ptr());
            free_matrix(mat);
            return null_mut();
        }

        let mut saveptr_row = null_mut();
        let mut row_token = strtok_r(input_copy, c"\n".as_ptr(), &mut saveptr_row);
        let mut i = 0;
        while i < height {
            if row_token.is_null() {
                fprintf(stderr, c"Insufficient rows in input string.\n".as_ptr());
                free(input_copy.cast());
                free_matrix(mat);
                return null_mut();
            }

            let mut saveptr_col = null_mut();
            let mut col_token = strtok_r(row_token, c" ".as_ptr(), &mut saveptr_col);
            let mut j = 0;
            while j < width {
                if col_token.is_null() {
                    fprintf(stderr, c"Insufficient columns in row %d.\n".as_ptr(), i + 1);
                    free(input_copy.cast());
                    free_matrix(mat);
                    return null_mut();
                }
                *(*(*mat).matrix.add(i as usize)).add(j as usize) = atoi(col_token);
                col_token = strtok_r(null_mut(), c" ".as_ptr(), &mut saveptr_col);
                j += 1;
            }

            row_token = strtok_r(null_mut(), c"\n".as_ptr(), &mut saveptr_row);
            i += 1;
        }

        free(input_copy.cast());
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
            fprintf(
                stderr,
                c"Matrix dimensions do not allow multiplication.\n".as_ptr(),
            );
            return null_mut();
        }

        let result = allocate_matrix((*mat_b).width, (*mat_a).height);
        let mut i = 0;
        while i < (*mat_a).height {
            let mut j = 0;
            while j < (*mat_b).width {
                let result_cell = (*(*result).matrix.add(i as usize)).add(j as usize);
                *result_cell = 0;
                let mut k = 0;
                while k < (*mat_a).width {
                    let a = *(*(*mat_a).matrix.add(i as usize)).add(k as usize);
                    let b = *(*(*mat_b).matrix.add(k as usize)).add(j as usize);
                    *result_cell = (*result_cell).wrapping_add(a.wrapping_mul(b));
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
            fprintf(stderr, c"Error: Matrix is NULL.\n".as_ptr());
            return null_mut();
        }

        let row_size = (*mat).width.wrapping_mul(10).wrapping_add((*mat).width);
        let buffer_size = (*mat)
            .height
            .wrapping_mul(row_size)
            .wrapping_add((*mat).height)
            .wrapping_add(1);
        let result = malloc(buffer_size as usize).cast::<c_char>();
        if result.is_null() {
            perror(c"Failed to allocate memory for matrix string".as_ptr());
            return null_mut();
        }

        *result = 0;

        let mut i = 0;
        while i < (*mat).height {
            let mut j = 0;
            while j < (*mat).width {
                let mut buffer = [0 as c_char; 12];
                snprintf(
                    buffer.as_mut_ptr(),
                    buffer.len(),
                    c"%d".as_ptr(),
                    *(*(*mat).matrix.add(i as usize)).add(j as usize),
                );
                strcat(result, buffer.as_ptr());

                if j < (*mat).width - 1 {
                    strcat(result, c" ".as_ptr());
                }
                j += 1;
            }
            strcat(result, c"\n".as_ptr());
            i += 1;
        }

        result
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn write_to_file(filename: *const c_char, content: *const c_char) -> c_int {
    unsafe {
        if content.is_null() {
            fprintf(stderr, c"Error: Content is NULL.\n".as_ptr());
            return 22;
        }

        let file = fopen(filename, c"w".as_ptr());
        if file.is_null() {
            fprintf(
                stderr,
                c"Error opening file '%s': %s\n".as_ptr(),
                filename,
                strerror(*__errno_location()),
            );
            return *__errno_location();
        }

        if fprintf(file, c"%s".as_ptr(), content) < 0 {
            fprintf(
                stderr,
                c"Error writing to file '%s': %s\n".as_ptr(),
                filename,
                strerror(*__errno_location()),
            );
            fclose(file);
            return *__errno_location();
        }

        if fclose(file) != 0 {
            fprintf(
                stderr,
                c"Error closing file '%s': %s\n".as_ptr(),
                filename,
                strerror(*__errno_location()),
            );
            return *__errno_location();
        }

        0
    }
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
    unsafe {
        let mat_a = initialize_matrix_from_string(matrix_a, width_a, height_a);
        if mat_a.is_null() {
            return 1;
        }
        let mat_b = initialize_matrix_from_string(matrix_b, width_b, height_b);
        if mat_b.is_null() {
            free_matrix(mat_a);
            return 1;
        }

        let res = multiply_matrices(mat_a, mat_b);
        if res.is_null() {
            free_matrix(mat_a);
            free_matrix(mat_b);
            return 1;
        }
        let res_str = matrix_to_string(res);
        if res_str.is_null() {
            free_matrix(mat_a);
            free_matrix(mat_b);
            free(res.cast());
            return 1;
        }

        let res_write = write_to_file(c"matrix.txt".as_ptr(), res_str);

        free_matrix(mat_a);
        free_matrix(mat_b);
        free_matrix(res);
        free(res_str.cast());

        if res_write != 0 {
            return 1;
        }

        0
    }
}
