use libc::{EINVAL, EXIT_FAILURE, EXIT_SUCCESS};
use std::ffi::{CStr, CString, c_char};
use std::fs::File;
use std::io::Write;
use std::os::raw::c_int;
use std::ptr;

#[repr(C)]
pub struct matrix_t {
    pub matrix: *mut *mut c_int,
    pub width: c_int,
    pub height: c_int,
}

struct MatrixData {
    rows: Vec<*mut c_int>,
    row_buffers: Vec<Box<[c_int]>>,
}

fn allocate_matrix_internal(width: c_int, height: c_int) -> Option<(*mut matrix_t, Box<MatrixData>)> {
    if width < 0 || height < 0 {
        eprintln!("Failed to allocate memory for matrix struct");
        return None;
    }

    let width_usize = width as usize;
    let height_usize = height as usize;

    let mut row_buffers: Vec<Box<[c_int]>> = Vec::with_capacity(height_usize);
    let mut rows: Vec<*mut c_int> = Vec::with_capacity(height_usize);

    for _ in 0..height_usize {
        let mut row = vec![0 as c_int; width_usize].into_boxed_slice();
        let ptr = row.as_mut_ptr();
        rows.push(ptr);
        row_buffers.push(row);
    }

    let matrix_ptr = rows.as_mut_ptr();
    let data = Box::new(MatrixData { rows, row_buffers });
    let mat = Box::new(matrix_t {
        matrix: matrix_ptr,
        width,
        height,
    });

    Some((Box::into_raw(mat), data))
}

unsafe fn matrix_data_from_mat(mat: *mut matrix_t) -> *mut MatrixData {
    if mat.is_null() {
        return ptr::null_mut();
    }
    unsafe { (*mat).matrix.cast::<MatrixData>() }
}

#[unsafe(no_mangle)]
pub extern "C" fn free_matrix(mat: *mut matrix_t) {
    if mat.is_null() {
        return;
    }

    unsafe {
        let data_ptr = matrix_data_from_mat(mat);
        if !data_ptr.is_null() {
            let _ = Box::from_raw(data_ptr);
        }
        let _ = Box::from_raw(mat);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn initialize_matrix_from_string(
    input: *const c_char,
    width: c_int,
    height: c_int,
) -> *mut matrix_t {
    if input.is_null() {
        eprintln!("Failed to duplicate input string");
        return ptr::null_mut();
    }

    let (mat_ptr, data) = match allocate_matrix_internal(width, height) {
        Some(v) => v,
        None => return ptr::null_mut(),
    };

    let input_str = unsafe { CStr::from_ptr(input) }.to_string_lossy().into_owned();
    let rows: Vec<&str> = input_str.split('\n').collect();

    if rows.len() < height as usize {
        eprintln!("Insufficient rows in input string.");
        unsafe {
            let _ = Box::from_raw(mat_ptr);
        }
        return ptr::null_mut();
    }

    let mut data = data;
    for i in 0..height as usize {
        let cols: Vec<&str> = rows[i].split(' ').filter(|s| !s.is_empty()).collect();
        if cols.len() < width as usize {
            eprintln!("Insufficient columns in row {}.", i + 1);
            unsafe {
                let _ = Box::from_raw(mat_ptr);
            }
            return ptr::null_mut();
        }
        for j in 0..width as usize {
            let value = cols[j].parse::<c_int>().unwrap_or(0);
            data.row_buffers[i][j] = value;
        }
    }

    unsafe {
        (*mat_ptr).matrix = Box::into_raw(data).cast::<*mut c_int>();
    }

    mat_ptr
}

#[unsafe(no_mangle)]
pub extern "C" fn multiply_matrices(mat_a: *mut matrix_t, mat_b: *mut matrix_t) -> *mut matrix_t {
    if mat_a.is_null() || mat_b.is_null() {
        eprintln!("Matrix dimensions do not allow multiplication.");
        return ptr::null_mut();
    }

    let (a_width, a_height, b_width, b_height, a_data_ptr, b_data_ptr) = unsafe {
        (
            (*mat_a).width,
            (*mat_a).height,
            (*mat_b).width,
            (*mat_b).height,
            matrix_data_from_mat(mat_a),
            matrix_data_from_mat(mat_b),
        )
    };

    if a_width != b_height {
        eprintln!("Matrix dimensions do not allow multiplication.");
        return ptr::null_mut();
    }

    let (result_ptr, mut result_data) = match allocate_matrix_internal(b_width, a_height) {
        Some(v) => v,
        None => return ptr::null_mut(),
    };

    let a_data = unsafe { &*a_data_ptr };
    let b_data = unsafe { &*b_data_ptr };

    for i in 0..a_height as usize {
        for j in 0..b_width as usize {
            let mut sum: c_int = 0;
            for k in 0..a_width as usize {
                sum += a_data.row_buffers[i][k] * b_data.row_buffers[k][j];
            }
            result_data.row_buffers[i][j] = sum;
        }
    }

    unsafe {
        (*result_ptr).matrix = Box::into_raw(result_data).cast::<*mut c_int>();
    }

    result_ptr
}

#[unsafe(no_mangle)]
pub extern "C" fn matrix_to_string(mat: *mut matrix_t) -> *mut c_char {
    if mat.is_null() {
        eprintln!("Error: Matrix is NULL.");
        return ptr::null_mut();
    }

    let (width, height, data_ptr) = unsafe { ((*mat).width, (*mat).height, matrix_data_from_mat(mat)) };
    if data_ptr.is_null() {
        eprintln!("Error: Matrix is NULL.");
        return ptr::null_mut();
    }

    let data = unsafe { &*data_ptr };
    let mut result = String::new();

    for i in 0..height as usize {
        for j in 0..width as usize {
            result.push_str(&data.row_buffers[i][j].to_string());
            if j + 1 < width as usize {
                result.push(' ');
            }
        }
        result.push('\n');
    }

    match CString::new(result) {
        Ok(s) => s.into_raw(),
        Err(_) => ptr::null_mut(),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn write_to_file(filename: *const c_char, content: *const c_char) -> c_int {
    if content.is_null() {
        eprintln!("Error: Content is NULL.");
        return EINVAL;
    }
    if filename.is_null() {
        eprintln!("Error opening file '(null)': invalid argument");
        return EINVAL;
    }

    let filename_str = unsafe { CStr::from_ptr(filename) }.to_string_lossy().into_owned();
    let content_bytes = unsafe { CStr::from_ptr(content) }.to_bytes();

    let mut file = match File::create(&filename_str) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("Error opening file '{}': {}", filename_str, e);
            return e.raw_os_error().unwrap_or(EINVAL);
        }
    };

    if let Err(e) = file.write_all(content_bytes) {
        eprintln!("Error writing to file '{}': {}", filename_str, e);
        return e.raw_os_error().unwrap_or(EINVAL);
    }

    if let Err(e) = file.flush() {
        eprintln!("Error closing file '{}': {}", filename_str, e);
        return e.raw_os_error().unwrap_or(EINVAL);
    }

    0
}

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
        free_matrix(res);
        return EXIT_FAILURE;
    }

    let out_file = CString::new("matrix.txt").unwrap();
    let res_write = write_to_file(out_file.as_ptr(), res_str);

    free_matrix(mat_a);
    free_matrix(mat_b);
    free_matrix(res);
    unsafe {
        let _ = CString::from_raw(res_str);
    }

    if res_write != 0 {
        return EXIT_FAILURE;
    }

    EXIT_SUCCESS
}
