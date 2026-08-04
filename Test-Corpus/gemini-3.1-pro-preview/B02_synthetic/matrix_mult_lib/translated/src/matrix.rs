use std::ffi::CStr;
use std::os::raw::{c_char, c_int};
use std::ptr;

#[repr(C)]
pub struct matrix_t {
    pub matrix: *mut *mut c_int,
    pub width: c_int,
    pub height: c_int,
}

#[unsafe(no_mangle)]
pub extern "C" fn initialize_matrix_from_string(
    input: *const c_char,
    width: c_int,
    height: c_int,
) -> *mut matrix_t {
    if input.is_null() || width <= 0 || height <= 0 {
        return ptr::null_mut();
    }

    let input_str = unsafe { CStr::from_ptr(input) }.to_string_lossy();
    let mut values = Vec::new();

    for line in input_str.lines() {
        let row: Vec<c_int> = line
            .split_whitespace()
            .filter_map(|s| s.parse::<c_int>().ok())
            .collect();
        if !row.is_empty() {
            values.push(row);
        }
    }

    if values.len() < height as usize {
        return ptr::null_mut();
    }

    for row in values.iter().take(height as usize) {
        if row.len() < width as usize {
            return ptr::null_mut();
        }
    }

    let mut row_ptrs = Vec::with_capacity(height as usize);
    for i in 0..(height as usize) {
        let mut row = values[i][0..(width as usize)].to_vec();
        row.shrink_to_fit();
        let ptr = row.as_mut_ptr();
        std::mem::forget(row);
        row_ptrs.push(ptr);
    }
    row_ptrs.shrink_to_fit();
    let matrix_ptr = row_ptrs.as_mut_ptr();
    std::mem::forget(row_ptrs);

    let mat = Box::new(matrix_t {
        matrix: matrix_ptr,
        width,
        height,
    });

    Box::into_raw(mat)
}

#[unsafe(no_mangle)]
pub extern "C" fn free_matrix(mat: *mut matrix_t) {
    if mat.is_null() {
        return;
    }
    unsafe {
        let mat_box = Box::from_raw(mat);
        let height = mat_box.height as usize;
        let width = mat_box.width as usize;
        let row_ptrs = Vec::from_raw_parts(mat_box.matrix, height, height);
        for ptr in row_ptrs {
            let _row = Vec::from_raw_parts(ptr, width, width);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn multiply_matrices(mat_a: *mut matrix_t, mat_b: *mut matrix_t) -> *mut matrix_t {
    if mat_a.is_null() || mat_b.is_null() {
        return ptr::null_mut();
    }

    let (a_width, a_height, a_matrix) = unsafe {
        ((*mat_a).width as usize, (*mat_a).height as usize, (*mat_a).matrix)
    };
    let (b_width, b_height, b_matrix) = unsafe {
        ((*mat_b).width as usize, (*mat_b).height as usize, (*mat_b).matrix)
    };

    if a_width != b_height {
        return ptr::null_mut();
    }

    let mut result_rows = Vec::with_capacity(a_height);
    for i in 0..a_height {
        let mut row = vec![0; b_width];
        for j in 0..b_width {
            let mut sum = 0;
            for k in 0..a_width {
                let a_val = unsafe { *(*a_matrix.add(i)).add(k) };
                let b_val = unsafe { *(*b_matrix.add(k)).add(j) };
                sum += a_val * b_val;
            }
            row[j] = sum;
        }
        row.shrink_to_fit();
        let ptr = row.as_mut_ptr();
        std::mem::forget(row);
        result_rows.push(ptr);
    }
    result_rows.shrink_to_fit();
    let matrix_ptr = result_rows.as_mut_ptr();
    std::mem::forget(result_rows);

    let mat = Box::new(matrix_t {
        matrix: matrix_ptr,
        width: b_width as c_int,
        height: a_height as c_int,
    });

    Box::into_raw(mat)
}

#[unsafe(no_mangle)]
pub extern "C" fn matrix_to_string(mat: *mut matrix_t) -> *mut c_char {
    if mat.is_null() {
        return ptr::null_mut();
    }

    let (width, height, matrix) = unsafe {
        ((*mat).width as usize, (*mat).height as usize, (*mat).matrix)
    };

    let mut result = String::new();
    for i in 0..height {
        for j in 0..width {
            let val = unsafe { *(*matrix.add(i)).add(j) };
            result.push_str(&val.to_string());
            if j < width - 1 {
                result.push(' ');
            }
        }
        result.push('\n');
    }

    let bytes = result.as_bytes();
    unsafe {
        let ptr = libc::malloc(bytes.len() + 1) as *mut c_char;
        if ptr.is_null() {
            return ptr::null_mut();
        }
        std::ptr::copy_nonoverlapping(bytes.as_ptr() as *const c_char, ptr, bytes.len());
        *ptr.add(bytes.len()) = 0;
        ptr
    }
}
