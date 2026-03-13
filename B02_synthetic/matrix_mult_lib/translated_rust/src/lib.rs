use std::ffi::{c_char, c_int, CStr, CString};
use std::io::Write;
use std::ptr;

/// Mirrors the C `matrix_t` struct layout.
#[repr(C)]
pub struct matrix_t {
    pub matrix: *mut *mut c_int,
    pub width: c_int,
    pub height: c_int,
}

/// Allocate a matrix with row pointers (mirrors C's int** layout).
/// Returns null on allocation failure (prints to stderr like the C version).
fn allocate_matrix(width: c_int, height: c_int) -> *mut matrix_t {
    let mat = Box::into_raw(Box::new(matrix_t {
        matrix: ptr::null_mut(),
        width,
        height,
    }));

    let mut rows: Vec<*mut c_int> = Vec::with_capacity(height as usize);
    for _i in 0..height {
        let row = vec![0 as c_int; width as usize];
        rows.push(Box::into_raw(row.into_boxed_slice()) as *mut c_int);
    }

    let row_ptrs = rows.into_boxed_slice();
    unsafe {
        (*mat).matrix = Box::into_raw(row_ptrs) as *mut *mut c_int;
    }

    mat
}

#[unsafe(no_mangle)]
pub extern "C" fn free_matrix(mat: *mut matrix_t) {
    if mat.is_null() {
        return;
    }
    unsafe {
        let height = (*mat).height;
        let matrix_ptr = (*mat).matrix;
        if !matrix_ptr.is_null() {
            for i in 0..height as usize {
                let row = *matrix_ptr.add(i);
                if !row.is_null() {
                    drop(Box::from_raw(std::slice::from_raw_parts_mut(
                        row,
                        (*mat).width as usize,
                    )));
                }
            }
            drop(Box::from_raw(std::slice::from_raw_parts_mut(
                matrix_ptr,
                height as usize,
            )));
        }
        drop(Box::from_raw(mat));
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn initialize_matrix_from_string(
    input: *const c_char,
    width: c_int,
    height: c_int,
) -> *mut matrix_t {
    let mat = allocate_matrix(width, height);
    if mat.is_null() {
        return ptr::null_mut();
    }

    let input_str = unsafe { CStr::from_ptr(input) };
    let input_str = match input_str.to_str() {
        Ok(s) => s,
        Err(_) => {
            eprintln!("Failed to duplicate input string");
            free_matrix(mat);
            return ptr::null_mut();
        }
    };

    // strtok_r skips consecutive delimiters, so filter empties
    let mut rows = input_str.split('\n').filter(|s| !s.is_empty());
    for i in 0..height as usize {
        let row_token = match rows.next() {
            Some(s) => s,
            None => {
                eprintln!("Insufficient rows in input string.");
                free_matrix(mat);
                return ptr::null_mut();
            }
        };

        let mut cols = row_token.split(' ').filter(|s| !s.is_empty());
        for j in 0..width as usize {
            let col_token = match cols.next() {
                Some(s) => s,
                None => {
                    eprintln!("Insufficient columns in row {}.", i + 1);
                    free_matrix(mat);
                    return ptr::null_mut();
                }
            };
            // Reproduce C atoi behavior: skip whitespace, optional sign, digits
            let trimmed = col_token.trim_start();
            let val: c_int = if trimmed.is_empty() {
                0
            } else {
                let (neg, rest) = if trimmed.starts_with('-') {
                    (true, &trimmed[1..])
                } else if trimmed.starts_with('+') {
                    (false, &trimmed[1..])
                } else {
                    (false, trimmed)
                };
                let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
                let abs: c_int = digits.parse().unwrap_or(0);
                if neg { abs.wrapping_neg() } else { abs }
            };
            unsafe {
                let row_ptr = *(*mat).matrix.add(i);
                *row_ptr.add(j) = val;
            }
        }
    }

    mat
}

#[unsafe(no_mangle)]
pub extern "C" fn multiply_matrices(
    mat_a: *mut matrix_t,
    mat_b: *mut matrix_t,
) -> *mut matrix_t {
    unsafe {
        if (*mat_a).width != (*mat_b).height {
            eprintln!("Matrix dimensions do not allow multiplication.");
            return ptr::null_mut();
        }

        let result = allocate_matrix((*mat_b).width, (*mat_a).height);
        for i in 0..(*mat_a).height as usize {
            for j in 0..(*mat_b).width as usize {
                let mut sum: c_int = 0;
                for k in 0..(*mat_a).width as usize {
                    let a_val = *(*(*mat_a).matrix.add(i)).add(k);
                    let b_val = *(*(*mat_b).matrix.add(k)).add(j);
                    sum = sum.wrapping_add(a_val.wrapping_mul(b_val));
                }
                *(*(*result).matrix.add(i)).add(j) = sum;
            }
        }

        result
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn matrix_to_string(mat: *mut matrix_t) -> *mut c_char {
    if mat.is_null() {
        eprintln!("Error: Matrix is NULL.");
        return ptr::null_mut();
    }

    unsafe {
        let mut result = String::new();
        for i in 0..(*mat).height as usize {
            for j in 0..(*mat).width as usize {
                let val = *(*(*mat).matrix.add(i)).add(j);
                if j > 0 {
                    result.push(' ');
                }
                result.push_str(&val.to_string());
            }
            result.push('\n');
        }

        match CString::new(result) {
            Ok(cs) => cs.into_raw(),
            Err(_) => ptr::null_mut(),
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn write_to_file(
    filename: *const c_char,
    content: *const c_char,
) -> c_int {
    if content.is_null() {
        eprintln!("Error: Content is NULL.");
        return libc_einval();
    }

    let filename_str = unsafe { CStr::from_ptr(filename) };
    let content_str = unsafe { CStr::from_ptr(content) };

    let filename_str = match filename_str.to_str() {
        Ok(s) => s,
        Err(_) => return libc_einval(),
    };

    let file = std::fs::File::create(filename_str);
    let mut file = match file {
        Ok(f) => f,
        Err(e) => {
            eprintln!(
                "Error opening file '{}': {}",
                filename_str,
                errno_str_for(&e)
            );
            return raw_os_error(&e);
        }
    };

    if let Err(e) = file.write_all(content_str.to_bytes()) {
        eprintln!(
            "Error writing to file '{}': {}",
            filename_str,
            errno_str_for(&e)
        );
        return raw_os_error(&e);
    }

    if let Err(e) = file.flush() {
        eprintln!(
            "Error closing file '{}': {}",
            filename_str,
            errno_str_for(&e)
        );
        return raw_os_error(&e);
    }

    0
}

const OUT_FILE: &[u8] = b"matrix.txt\0";

#[unsafe(no_mangle)]
pub extern "C" fn driver(
    width_a: c_int,
    height_a: c_int,
    matrix_a: *const c_char,
    width_b: c_int,
    height_b: c_int,
    matrix_b: *const c_char,
) -> c_int {
    let mat_a = initialize_matrix_from_string(matrix_a, width_a, height_a);
    if mat_a.is_null() {
        return 1; // EXIT_FAILURE
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
        free_matrix(res);
        return 1;
    }

    let res_write = write_to_file(OUT_FILE.as_ptr() as *const c_char, res_str);

    free_matrix(mat_a);
    free_matrix(mat_b);
    free_matrix(res);
    unsafe {
        drop(CString::from_raw(res_str));
    }

    if res_write != 0 {
        return 1;
    }

    0 // EXIT_SUCCESS
}

fn libc_einval() -> c_int {
    22 // EINVAL on Linux
}

fn raw_os_error(e: &std::io::Error) -> c_int {
    e.raw_os_error().unwrap_or(libc_einval())
}

fn errno_str_for(e: &std::io::Error) -> String {
    format!("{}", e)
}
