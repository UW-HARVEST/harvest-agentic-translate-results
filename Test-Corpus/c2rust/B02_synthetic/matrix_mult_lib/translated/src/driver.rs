extern "C" {
    fn free(__ptr: *mut ::core::ffi::c_void);
    fn initialize_matrix_from_string(
        input: *const ::core::ffi::c_char,
        width: ::core::ffi::c_int,
        height: ::core::ffi::c_int,
    ) -> *mut matrix_t;
    fn free_matrix(mat: *mut matrix_t);
    fn multiply_matrices(mat_a: *mut matrix_t, mat_b: *mut matrix_t) -> *mut matrix_t;
    fn matrix_to_string(mat: *mut matrix_t) -> *mut ::core::ffi::c_char;
    fn write_to_file(
        filename: *const ::core::ffi::c_char,
        contents: *const ::core::ffi::c_char,
    ) -> ::core::ffi::c_int;
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct matrix_t {
    pub matrix: *mut *mut ::core::ffi::c_int,
    pub width: ::core::ffi::c_int,
    pub height: ::core::ffi::c_int,
}
pub const NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<::core::ffi::c_void>();
pub const EXIT_FAILURE: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const EXIT_SUCCESS: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const OUT_FILE: [::core::ffi::c_char; 11] =
    unsafe { ::core::mem::transmute::<[u8; 11], [::core::ffi::c_char; 11]>(*b"matrix.txt\0") };
#[no_mangle]
pub unsafe extern "C" fn driver(
    mut width_a: ::core::ffi::c_int,
    mut height_a: ::core::ffi::c_int,
    mut matrix_a: *const ::core::ffi::c_char,
    mut width_b: ::core::ffi::c_int,
    mut height_b: ::core::ffi::c_int,
    mut matrix_b: *const ::core::ffi::c_char,
) -> ::core::ffi::c_int {
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
    let mut res_str: *mut ::core::ffi::c_char = matrix_to_string(res);
    if res_str.is_null() {
        free_matrix(mat_a);
        free_matrix(mat_b);
        free(res as *mut ::core::ffi::c_void);
        return EXIT_FAILURE;
    }
    let mut res_write: ::core::ffi::c_int = write_to_file(OUT_FILE.as_ptr(), res_str);
    free_matrix(mat_a);
    free_matrix(mat_b);
    free_matrix(res);
    free(res_str as *mut ::core::ffi::c_void);
    if res_write != 0 as ::core::ffi::c_int {
        return EXIT_FAILURE;
    }
    return EXIT_SUCCESS;
}
