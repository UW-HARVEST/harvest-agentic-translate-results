use std::ffi::{c_char, c_int, CStr, CString};
use std::ptr;

// ── matrix_t ────────────────────────────────────────────────────────────────

#[repr(C)]
pub struct matrix_t {
    pub matrix: *mut *mut c_int,
    pub width: c_int,
    pub height: c_int,
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn allocate_matrix(width: c_int, height: c_int) -> *mut matrix_t {
    unsafe {
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
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn free_matrix(mat: *mut matrix_t) {
    if mat.is_null() {
        return;
    }
    unsafe {
        for i in 0..(*mat).height as usize {
            libc::free(*(*mat).matrix.add(i) as *mut libc::c_void);
        }
        libc::free((*mat).matrix as *mut libc::c_void);
        libc::free(mat as *mut libc::c_void);
    }
}

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

    unsafe {
        let input_cstr = CStr::from_ptr(input);
        let input_str = match input_cstr.to_str() {
            Ok(s) => s,
            Err(_) => {
                free_matrix(mat);
                return ptr::null_mut();
            }
        };

        let mut rows = input_str.split('\n');
        for i in 0..height as usize {
            let row_token = match rows.next() {
                Some(s) if !s.is_empty() => s,
                _ => {
                    eprint!("Insufficient rows in input string.\n");
                    free_matrix(mat);
                    return ptr::null_mut();
                }
            };

            let mut cols = row_token.split(' ');
            for j in 0..width as usize {
                let col_token = match cols.next() {
                    Some(s) if !s.is_empty() => s,
                    _ => {
                        eprint!("Insufficient columns in row {}.\n", i as c_int + 1);
                        free_matrix(mat);
                        return ptr::null_mut();
                    }
                };
                // atoi: parse leading digits, 0 on failure
                let val = atoi_compat(col_token);
                *(*(*mat).matrix.add(i)).add(j) = val;
            }
        }
    }
    mat
}

/// Mimics C atoi: skip leading whitespace, optional sign, parse digits, 0 on no digits.
fn atoi_compat(s: &str) -> c_int {
    let s = s.trim_start();
    let mut chars = s.chars().peekable();
    let neg = match chars.peek() {
        Some('-') => { chars.next(); true }
        Some('+') => { chars.next(); false }
        _ => false,
    };
    let mut val: c_int = 0;
    for c in chars {
        if let Some(d) = c.to_digit(10) {
            val = val.wrapping_mul(10).wrapping_add(d as c_int);
        } else {
            break;
        }
    }
    if neg { val.wrapping_neg() } else { val }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn multiply_matrices(
    mat_a: *mut matrix_t,
    mat_b: *mut matrix_t,
) -> *mut matrix_t {
    unsafe {
        if (*mat_a).width != (*mat_b).height {
            eprint!("Matrix dimensions do not allow multiplication.\n");
            return ptr::null_mut();
        }

        let result = allocate_matrix((*mat_b).width, (*mat_a).height);
        if result.is_null() {
            return ptr::null_mut();
        }
        for i in 0..(*mat_a).height as usize {
            for j in 0..(*mat_b).width as usize {
                *(*(*result).matrix.add(i)).add(j) = 0;
                for k in 0..(*mat_a).width as usize {
                    let a = *(*(*mat_a).matrix.add(i)).add(k);
                    let b = *(*(*mat_b).matrix.add(k)).add(j);
                    *(*(*result).matrix.add(i)).add(j) += a * b;
                }
            }
        }
        result
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn matrix_to_string(mat: *mut matrix_t) -> *mut c_char {
    if mat.is_null() {
        eprint!("Error: Matrix is NULL.\n");
        return ptr::null_mut();
    }

    unsafe {
        let height = (*mat).height as usize;
        let width = (*mat).width as usize;
        let buffer_size = height * (width * 10 + width) + height + 1;

        let result = libc::malloc(buffer_size) as *mut c_char;
        if result.is_null() {
            libc::perror(b"Failed to allocate memory for matrix string\0".as_ptr() as *const c_char);
            return ptr::null_mut();
        }
        *result = 0; // result[0] = '\0'

        for i in 0..height {
            for j in 0..width {
                let val = *(*(*mat).matrix.add(i)).add(j);
                let s = format!("{}", val);
                let cs = CString::new(s).unwrap();
                libc::strcat(result, cs.as_ptr());
                if j < width - 1 {
                    libc::strcat(result, b" \0".as_ptr() as *const c_char);
                }
            }
            libc::strcat(result, b"\n\0".as_ptr() as *const c_char);
        }
        result
    }
}

// ── write_to_file ───────────────────────────────────────────────────────────

#[unsafe(no_mangle)]
pub unsafe extern "C" fn write_to_file(
    filename: *const c_char,
    content: *const c_char,
) -> c_int {
    if content.is_null() {
        eprint!("Error: Content is NULL.\n");
        return libc::EINVAL;
    }

    unsafe {
        let fname = CStr::from_ptr(filename);
        let mode = b"w\0".as_ptr() as *const c_char;
        let file = libc::fopen(fname.as_ptr(), mode);
        if file.is_null() {
            let errno_val = *libc::__errno_location();
            eprint!(
                "Error opening file '{}': {}\n",
                fname.to_str().unwrap_or(""),
                std::io::Error::from_raw_os_error(errno_val)
            );
            return errno_val;
        }

        let content_cstr = CStr::from_ptr(content);
        let content_bytes = content_cstr.to_bytes();
        let written = libc::fwrite(
            content_bytes.as_ptr() as *const libc::c_void,
            1,
            content_bytes.len(),
            file,
        );
        if written < content_bytes.len() {
            let errno_val = *libc::__errno_location();
            eprint!(
                "Error writing to file '{}': {}\n",
                fname.to_str().unwrap_or(""),
                std::io::Error::from_raw_os_error(errno_val)
            );
            libc::fclose(file);
            return errno_val;
        }

        if libc::fclose(file) != 0 {
            let errno_val = *libc::__errno_location();
            eprint!(
                "Error closing file '{}': {}\n",
                fname.to_str().unwrap_or(""),
                std::io::Error::from_raw_os_error(errno_val)
            );
            return errno_val;
        }

        0
    }
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
    unsafe {
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
            // C code bug: uses free(res) instead of free_matrix(res) — reproduce exactly
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
}
