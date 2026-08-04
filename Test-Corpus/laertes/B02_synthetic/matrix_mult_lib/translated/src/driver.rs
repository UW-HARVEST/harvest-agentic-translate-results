extern "C" {
    fn free(__ptr: *mut libc::c_void);
    
    
    
    
    
}
pub use crate::src::matrix::free_matrix;
pub use crate::src::matrix::initialize_matrix_from_string;
pub use crate::src::matrix::matrix_to_string;
pub use crate::src::matrix::multiply_matrices;
pub use crate::src::write::write_to_file;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct matrix_t {
    pub matrix: *mut *mut libc::c_int,
    pub width: libc::c_int,
    pub height: libc::c_int,
}
pub const NULL: *mut libc::c_void = std::ptr::null_mut::<libc::c_void>();
pub const EXIT_FAILURE: libc::c_int = 1 as libc::c_int;
pub const EXIT_SUCCESS: libc::c_int = 0 as libc::c_int;
pub const OUT_FILE: [libc::c_char; 11] =
    unsafe { std::mem::transmute::<[u8; 11], [libc::c_char; 11]>(*b"matrix.txt\0") };
#[no_mangle]
pub unsafe extern "C" fn driver(
    mut width_a: libc::c_int,
    mut height_a: libc::c_int,
    mut matrix_a: *const libc::c_char,
    mut width_b: libc::c_int,
    mut height_b: libc::c_int,
    mut matrix_b: *const libc::c_char,
) -> libc::c_int {
    let mut mat_a: *mut matrix_t = initialize_matrix_from_string(matrix_a, width_a, height_a);
    if mat_a.is_null() {
        return EXIT_FAILURE;
    }
    let mut mat_b: *mut matrix_t = initialize_matrix_from_string(matrix_b, width_b, height_b);
    if mat_b.is_null() {
        free_matrix(mat_a);
        return EXIT_FAILURE;
    }
    let mut res: *mut matrix_t = multiply_matrices(mat_a, mat_b);
    if res.is_null() {
        free_matrix(mat_a);
        free_matrix(mat_b);
        return EXIT_FAILURE;
    }
    let mut res_str: *mut libc::c_char = matrix_to_string(res);
    if res_str.is_null() {
        free_matrix(mat_a);
        free_matrix(mat_b);
        free(res as *mut libc::c_void);
        return EXIT_FAILURE;
    }
    let mut res_write: libc::c_int = write_to_file(OUT_FILE.as_ptr(), res_str);
    free_matrix(mat_a);
    free_matrix(mat_b);
    free_matrix(res);
    free(res_str as *mut libc::c_void);
    if res_write != 0 as libc::c_int {
        return EXIT_FAILURE;
    }
    return EXIT_SUCCESS;
}
