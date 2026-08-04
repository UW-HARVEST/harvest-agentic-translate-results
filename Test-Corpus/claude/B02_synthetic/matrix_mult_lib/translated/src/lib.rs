// Library translation of c_src to Rust producing byte-identical outputs.

use std::ffi::{c_char, c_int, CStr};
use std::io::Write;
use std::ptr;

// EXIT_SUCCESS / EXIT_FAILURE
const EXIT_SUCCESS: c_int = 0;
const EXIT_FAILURE: c_int = 1;

// EINVAL
const EINVAL: c_int = 22;

// matrix_t struct, layout-compatible with the C definition.
#[repr(C)]
pub struct matrix_t {
    pub matrix: *mut *mut c_int,
    pub width: c_int,
    pub height: c_int,
}

// ---------- helpers ----------

fn eprint_bytes(bytes: &[u8]) {
    let stderr = std::io::stderr();
    let mut handle = stderr.lock();
    let _ = handle.write_all(bytes);
}

// errno helpers using libc
fn get_errno() -> c_int {
    unsafe { *libc::__errno_location() as c_int }
}

fn strerror_string(err: c_int) -> String {
    unsafe {
        let p = libc::strerror(err);
        if p.is_null() {
            String::new()
        } else {
            CStr::from_ptr(p).to_string_lossy().into_owned()
        }
    }
}

// Replicates C's atoi behavior (skip whitespace, optional sign, parse digits, stop on non-digit, return 0 on no digits).
fn c_atoi(bytes: &[u8]) -> c_int {
    let mut i = 0usize;
    // Skip whitespace per isspace
    while i < bytes.len() {
        match bytes[i] {
            b' ' | b'\t' | b'\n' | b'\r' | 0x0b | 0x0c => i += 1,
            _ => break,
        }
    }
    let mut neg = false;
    if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
        neg = bytes[i] == b'-';
        i += 1;
    }
    let mut val: i64 = 0;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        val = val.wrapping_mul(10).wrapping_add((bytes[i] - b'0') as i64);
        i += 1;
    }
    if neg {
        val = -val;
    }
    val as c_int
}

// ---------- matrix.c ----------

fn allocate_matrix(width: c_int, height: c_int) -> *mut matrix_t {
    unsafe {
        let mat = libc::malloc(std::mem::size_of::<matrix_t>()) as *mut matrix_t;
        if mat.is_null() {
            // perror("Failed to allocate memory for matrix struct")
            let msg = format!("Failed to allocate memory for matrix struct: {}\n", strerror_string(get_errno()));
            eprint_bytes(msg.as_bytes());
            return ptr::null_mut();
        }

        (*mat).width = width;
        (*mat).height = height;

        let row_ptrs_size = (height as usize).wrapping_mul(std::mem::size_of::<*mut c_int>());
        (*mat).matrix = libc::malloc(row_ptrs_size) as *mut *mut c_int;
        if (*mat).matrix.is_null() {
            let msg = format!("Failed to allocate memory for matrix rows: {}\n", strerror_string(get_errno()));
            eprint_bytes(msg.as_bytes());
            libc::free(mat as *mut libc::c_void);
            return ptr::null_mut();
        }

        for i in 0..height as isize {
            let col_size = (width as usize).wrapping_mul(std::mem::size_of::<c_int>());
            let row = libc::malloc(col_size) as *mut c_int;
            *((*mat).matrix.offset(i)) = row;
            if row.is_null() {
                let msg = format!("Failed to allocate memory for matrix columns: {}\n", strerror_string(get_errno()));
                eprint_bytes(msg.as_bytes());
                // C frees j = 0..=i (inclusive)
                let mut j: isize = 0;
                while j <= i {
                    libc::free(*((*mat).matrix.offset(j)) as *mut libc::c_void);
                    j += 1;
                }
                libc::free((*mat).matrix as *mut libc::c_void);
                libc::free(mat as *mut libc::c_void);
                return ptr::null_mut();
            }
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
        for i in 0..(*mat).height as isize {
            libc::free(*((*mat).matrix.offset(i)) as *mut libc::c_void);
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

    // strdup the input
    let input_copy = unsafe { libc::strdup(input) };
    if input_copy.is_null() {
        let msg = format!("Failed to duplicate input string: {}\n", strerror_string(get_errno()));
        eprint_bytes(msg.as_bytes());
        unsafe { free_matrix(mat) };
        return ptr::null_mut();
    }

    unsafe {
        let mut saveptr_row: *mut c_char = ptr::null_mut();
        let nl = b"\n\0".as_ptr() as *const c_char;
        let sp = b" \0".as_ptr() as *const c_char;

        let mut row_token = libc::strtok_r(input_copy, nl, &mut saveptr_row);
        for i in 0..height {
            if row_token.is_null() {
                eprint_bytes(b"Insufficient rows in input string.\n");
                libc::free(input_copy as *mut libc::c_void);
                free_matrix(mat);
                return ptr::null_mut();
            }

            let mut saveptr_col: *mut c_char = ptr::null_mut();
            let mut col_token = libc::strtok_r(row_token, sp, &mut saveptr_col);
            for j in 0..width {
                if col_token.is_null() {
                    let msg = format!("Insufficient columns in row {}.\n", i + 1);
                    eprint_bytes(msg.as_bytes());
                    libc::free(input_copy as *mut libc::c_void);
                    free_matrix(mat);
                    return ptr::null_mut();
                }
                // atoi
                let bytes = CStr::from_ptr(col_token).to_bytes();
                let val = c_atoi(bytes);
                *(*(*mat).matrix.offset(i as isize)).offset(j as isize) = val;
                col_token = libc::strtok_r(ptr::null_mut(), sp, &mut saveptr_col);
            }

            row_token = libc::strtok_r(ptr::null_mut(), nl, &mut saveptr_row);
        }

        libc::free(input_copy as *mut libc::c_void);
    }
    mat
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn multiply_matrices(
    mat_a: *mut matrix_t,
    mat_b: *mut matrix_t,
) -> *mut matrix_t {
    unsafe {
        if (*mat_a).width != (*mat_b).height {
            eprint_bytes(b"Matrix dimensions do not allow multiplication.\n");
            return ptr::null_mut();
        }

        let result = allocate_matrix((*mat_b).width, (*mat_a).height);
        for i in 0..(*mat_a).height as isize {
            for j in 0..(*mat_b).width as isize {
                let cell = (*(*result).matrix.offset(i)).offset(j);
                *cell = 0;
                for k in 0..(*mat_a).width as isize {
                    let a = *(*(*mat_a).matrix.offset(i)).offset(k);
                    let b = *(*(*mat_b).matrix.offset(k)).offset(j);
                    // Use wrapping_add and wrapping_mul to mirror typical C overflow behavior
                    *cell = (*cell).wrapping_add(a.wrapping_mul(b));
                }
            }
        }
        result
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn matrix_to_string(mat: *mut matrix_t) -> *mut c_char {
    if mat.is_null() {
        eprint_bytes(b"Error: Matrix is NULL.\n");
        return ptr::null_mut();
    }
    unsafe {
        let height = (*mat).height as usize;
        let width = (*mat).width as usize;
        let buffer_size = height * (width * 10 + width) + height + 1;
        let result = libc::malloc(buffer_size) as *mut c_char;
        if result.is_null() {
            let msg = format!("Failed to allocate memory for matrix string: {}\n", strerror_string(get_errno()));
            eprint_bytes(msg.as_bytes());
            return ptr::null_mut();
        }
        // result[0] = '\0'
        *result = 0;

        for i in 0..(*mat).height as isize {
            for j in 0..(*mat).width as isize {
                let val = *(*(*mat).matrix.offset(i)).offset(j);
                let s = format!("{}", val);
                let s_bytes = s.as_bytes();
                // strcat
                let cur_len = libc::strlen(result);
                let dst = result.add(cur_len) as *mut u8;
                ptr::copy_nonoverlapping(s_bytes.as_ptr(), dst, s_bytes.len());
                *dst.add(s_bytes.len()) = 0;

                if j < (*mat).width as isize - 1 {
                    let cur_len = libc::strlen(result);
                    let dst = result.add(cur_len) as *mut u8;
                    *dst = b' ';
                    *dst.add(1) = 0;
                }
            }
            let cur_len = libc::strlen(result);
            let dst = result.add(cur_len) as *mut u8;
            *dst = b'\n';
            *dst.add(1) = 0;
        }
        result
    }
}

// ---------- write.c ----------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn write_to_file(filename: *const c_char, content: *const c_char) -> c_int {
    if content.is_null() {
        eprint_bytes(b"Error: Content is NULL.\n");
        return EINVAL;
    }

    unsafe {
        // fopen
        let mode = b"w\0".as_ptr() as *const c_char;
        let file = libc::fopen(filename, mode);
        if file.is_null() {
            let err = get_errno();
            let fname = CStr::from_ptr(filename).to_string_lossy();
            let msg = format!("Error opening file '{}': {}\n", fname, strerror_string(err));
            eprint_bytes(msg.as_bytes());
            return err;
        }

        // fprintf(file, "%s", content) — write the content
        let len = libc::strlen(content);
        let written = libc::fwrite(content as *const libc::c_void, 1, len, file);
        if written < len {
            let err = get_errno();
            let fname = CStr::from_ptr(filename).to_string_lossy();
            let msg = format!("Error writing to file '{}': {}\n", fname, strerror_string(err));
            eprint_bytes(msg.as_bytes());
            libc::fclose(file);
            return err;
        }

        if libc::fclose(file) != 0 {
            let err = get_errno();
            let fname = CStr::from_ptr(filename).to_string_lossy();
            let msg = format!("Error closing file '{}': {}\n", fname, strerror_string(err));
            eprint_bytes(msg.as_bytes());
            return err;
        }
    }

    0
}

// ---------- driver.c ----------

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
            // Note: original C frees `res` (not free_matrix(res)) here — replicate that bug.
            libc::free(res as *mut libc::c_void);
            return EXIT_FAILURE;
        }

        let out_file = b"matrix.txt\0".as_ptr() as *const c_char;
        let res_write = write_to_file(out_file, res_str);

        free_matrix(mat_a);
        free_matrix(mat_b);
        free_matrix(res);
        libc::free(res_str as *mut libc::c_void);

        if res_write != 0 {
            return EXIT_FAILURE;
        }

        EXIT_SUCCESS
    }
}
