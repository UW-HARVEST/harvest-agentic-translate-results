use std::ffi::{c_char, c_int, CStr, CString};
use std::fs::File;
use std::io::Write;
use std::os::raw::c_void;
use std::ptr;

#[repr(C)]
pub struct matrix_t {
    matrix: *mut *mut c_int,
    width: c_int,
    height: c_int,
}

unsafe fn allocate_matrix(width: c_int, height: c_int) -> *mut matrix_t {
    let mat = libc::malloc(std::mem::size_of::<matrix_t>()) as *mut matrix_t;
    if mat.is_null() {
        return ptr::null_mut();
    }

    (*mat).width = width;
    (*mat).height = height;

    (*mat).matrix = libc::malloc((height as usize) * std::mem::size_of::<*mut c_int>()) as *mut *mut c_int;
    if (*mat).matrix.is_null() {
        libc::free(mat as *mut c_void);
        return ptr::null_mut();
    }

    for i in 0..height {
        let row = libc::malloc((width as usize) * std::mem::size_of::<c_int>()) as *mut c_int;
        if row.is_null() {
            for j in 0..i {
                libc::free(*(*mat).matrix.add(j as usize) as *mut c_void);
            }
            libc::free((*mat).matrix as *mut c_void);
            libc::free(mat as *mut c_void);
            return ptr::null_mut();
        }
        *(*mat).matrix.add(i as usize) = row;
    }

    mat
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn free_matrix(mat: *mut matrix_t) {
    if mat.is_null() {
        return;
    }

    for i in 0..(*mat).height {
        libc::free(*(*mat).matrix.add(i as usize) as *mut c_void);
    }
    libc::free((*mat).matrix as *mut c_void);
    libc::free(mat as *mut c_void);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn initialize_matrix_from_string(input: *const c_char, width: c_int, height: c_int) -> *mut matrix_t {
    let mat = allocate_matrix(width, height);
    if mat.is_null() {
        return ptr::null_mut();
    }

    let input_str = CStr::from_ptr(input).to_str().unwrap_or("");
    let input_copy = CString::new(input_str).unwrap();
    let input_ptr = input_copy.into_raw();

    let mut saveptr_row: *mut c_char = ptr::null_mut();
    let row_token = libc::strtok_r(input_ptr, b"\n\0".as_ptr() as *mut c_char, &mut saveptr_row);

    let mut current_row = row_token;
    for i in 0..height {
        if current_row.is_null() {
            libc::free(input_ptr as *mut c_void);
            free_matrix(mat);
            return ptr::null_mut();
        }

        let mut saveptr_col: *mut c_char = ptr::null_mut();
        let col_token = libc::strtok_r(current_row, b" \0".as_ptr() as *mut c_char, &mut saveptr_col);
        let mut current_col = col_token;

        for j in 0..width {
            if current_col.is_null() {
                libc::free(input_ptr as *mut c_void);
                free_matrix(mat);
                return ptr::null_mut();
            }
            let val = libc::strtol(current_col, ptr::null_mut(), 10);
            *(*(*mat).matrix.add(i as usize)).add(j as usize) = val as c_int;
            current_col = libc::strtok_r(ptr::null_mut(), b" \0".as_ptr() as *mut c_char, &mut saveptr_col);
        }

        current_row = libc::strtok_r(ptr::null_mut(), b"\n\0".as_ptr() as *mut c_char, &mut saveptr_row);
    }

    libc::free(input_ptr as *mut c_void);
    mat
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn multiply_matrices(mat_a: *mut matrix_t, mat_b: *mut matrix_t) -> *mut matrix_t {
    if (*mat_a).width != (*mat_b).height {
        return ptr::null_mut();
    }

    let result = allocate_matrix((*mat_b).width, (*mat_a).height);
    if result.is_null() {
        return ptr::null_mut();
    }

    for i in 0..(*mat_a).height {
        for j in 0..(*mat_b).width {
            *(*(*result).matrix.add(i as usize)).add(j as usize) = 0;
            for k in 0..(*mat_a).width {
                let a_val = *(*(*mat_a).matrix.add(i as usize)).add(k as usize);
                let b_val = *(*(*mat_b).matrix.add(k as usize)).add(j as usize);
                let res_ptr = (*(*result).matrix.add(i as usize)).add(j as usize);
                *res_ptr += a_val * b_val;
            }
        }
    }

    result
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn matrix_to_string(mat: *mut matrix_t) -> *mut c_char {
    if mat.is_null() {
        return ptr::null_mut();
    }

    let width = (*mat).width;
    let height = (*mat).height;
    let mut result = String::new();

    for i in 0..height {
        for j in 0..width {
            let val = *(*(*mat).matrix.add(i as usize)).add(j as usize);
            result.push_str(&val.to_string());
            if j < width - 1 {
                result.push(' ');
            }
        }
        result.push('\n');
    }

    match CString::new(result) {
        Ok(cstr) => cstr.into_raw(),
        Err(_) => ptr::null_mut(),
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn write_to_file(filename: *const c_char, content: *const c_char) -> c_int {
    if content.is_null() {
        return libc::EINVAL;
    }

    let filename_str = match CStr::from_ptr(filename).to_str() {
        Ok(s) => s,
        Err(_) => return libc::EINVAL,
    };

    let content_str = match CStr::from_ptr(content).to_str() {
        Ok(s) => s,
        Err(_) => return libc::EINVAL,
    };

    let mut file = match File::create(filename_str) {
        Ok(f) => f,
        Err(_) => return libc::EIO,
    };

    if file.write_all(content_str.as_bytes()).is_err() {
        return libc::EIO;
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(width_a: c_int, height_a: c_int, matrix_a: *const c_char, width_b: c_int, height_b: c_int, matrix_b: *const c_char) -> c_int {
    const OUT_FILE: &[u8] = b"matrix.txt\0";

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
        free_matrix(res);
        return libc::EXIT_FAILURE;
    }

    let res_write = write_to_file(OUT_FILE.as_ptr() as *const c_char, res_str);

    free_matrix(mat_a);
    free_matrix(mat_b);
    free_matrix(res);
    libc::free(res_str as *mut c_void);

    if res_write != 0 {
        return libc::EXIT_FAILURE;
    }

    libc::EXIT_SUCCESS
}
