

extern "C" {
    fn malloc(__size: size_t) -> *mut ::core::ffi::c_void;
    fn realloc(__ptr: *mut ::core::ffi::c_void, __size: size_t) -> *mut ::core::ffi::c_void;
    fn free(__ptr: *mut ::core::ffi::c_void);
}
pub type size_t = usize;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct DynamicArray {
    pub data: *mut ::core::ffi::c_int,
    pub size: size_t,
    pub capacity: size_t,
}
pub const NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<::core::ffi::c_void>();
#[no_mangle]
pub static mut matrix: [[::core::ffi::c_int; 4]; 3] = [
    [
        0x1 as ::core::ffi::c_int,
        0x2 as ::core::ffi::c_int,
        0x3 as ::core::ffi::c_int,
        0x4 as ::core::ffi::c_int,
    ],
    [
        0x10 as ::core::ffi::c_int,
        0x20 as ::core::ffi::c_int,
        0x30 as ::core::ffi::c_int,
        0x40 as ::core::ffi::c_int,
    ],
    [
        0xa1 as ::core::ffi::c_int,
        0xb2 as ::core::ffi::c_int,
        0xc3 as ::core::ffi::c_int,
        0xd4 as ::core::ffi::c_int,
    ],
];
pub const FLAG_READ: ::core::ffi::c_int = 0o1 as ::core::ffi::c_int;
pub const FLAG_WRITE: ::core::ffi::c_int = 0o2 as ::core::ffi::c_int;
pub const FLAG_EXECUTE: ::core::ffi::c_int = 0o4 as ::core::ffi::c_int;
pub const FLAG_DELETE: ::core::ffi::c_int = 0o10 as ::core::ffi::c_int;
#[no_mangle]
pub unsafe extern "C" fn init_array(mut initial_capacity: size_t) -> *mut DynamicArray {
    let mut arr: *mut DynamicArray =
        malloc(::core::mem::size_of::<DynamicArray>() as size_t) as *mut DynamicArray;
    if arr.is_null() {
        return ::core::ptr::null_mut::<DynamicArray>();
    }
    (*arr).data = malloc(
        initial_capacity.wrapping_mul(::core::mem::size_of::<::core::ffi::c_int>() as size_t),
    ) as *mut ::core::ffi::c_int;
    if (*arr).data.is_null() {
        free(arr as *mut ::core::ffi::c_void);
        return ::core::ptr::null_mut::<DynamicArray>();
    }
    (*arr).size = 0 as size_t;
    (*arr).capacity = initial_capacity;
    return arr;
}
#[no_mangle]
pub unsafe extern "C" fn expand_array(mut arr: *mut DynamicArray) -> ::core::ffi::c_int {
    if arr.is_null() {
        return 0 as ::core::ffi::c_int;
    }
    let mut new_capacity: size_t = (*arr).capacity.wrapping_mul(2 as size_t);
    let mut new_data: *mut ::core::ffi::c_int = realloc(
        (*arr).data as *mut ::core::ffi::c_void,
        new_capacity.wrapping_mul(::core::mem::size_of::<::core::ffi::c_int>() as size_t),
    ) as *mut ::core::ffi::c_int;
    if new_data.is_null() {
        return 0 as ::core::ffi::c_int;
    }
    (*arr).data = new_data;
    (*arr).capacity = new_capacity;
    return 1 as ::core::ffi::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn add_element(
    mut arr: *mut DynamicArray,
    mut value: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    if arr.is_null() {
        return 0 as ::core::ffi::c_int;
    }
    if (*arr).size >= (*arr).capacity {
        if expand_array(arr) == 0 {
            return 0 as ::core::ffi::c_int;
        }
    }
    let fresh0 = (*arr).size;
    (*arr).size = (*arr).size.wrapping_add(1);
    *(*arr).data.offset(fresh0 as isize) = value;
    return 1 as ::core::ffi::c_int;
}
#[no_mangle]
pub fn free_array(arr: *mut DynamicArray) {
    if arr.is_null() {
        return;
    }

    unsafe {
        drop(Box::from_raw(arr));
    }
}

#[no_mangle]
pub fn process_flags(flags: i32) -> i32 {
    let read_enabled = (flags & FLAG_READ != 0) as i32;
    let write_enabled = (flags & FLAG_WRITE != 0) as i32;
    let execute_enabled = (flags & FLAG_EXECUTE != 0) as i32;
    let delete_enabled = (flags & FLAG_DELETE != 0) as i32;

    read_enabled + write_enabled + execute_enabled + delete_enabled
}

#[no_mangle]
pub unsafe extern "C" fn calculate_matrix_checksum() -> ::core::ffi::c_int {
    let mut sum: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut i: ::core::ffi::c_int = 0;
    let mut j: ::core::ffi::c_int = 0;
    i = 0 as ::core::ffi::c_int;
    while i < 3 as ::core::ffi::c_int {
        j = 0 as ::core::ffi::c_int;
        while j < 4 as ::core::ffi::c_int {
            sum += matrix[i as usize][j as usize];
            j += 1;
        }
        i += 1;
    }
    return sum;
}
#[no_mangle]
pub unsafe extern "C" fn matrixsum(
    mut param1: ::core::ffi::c_int,
    mut param2: ::core::ffi::c_int,
    mut param3: ::core::ffi::c_int,
    mut param4: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    let mut result: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut hex_base: ::core::ffi::c_int = 0xff as ::core::ffi::c_int;
    let mut hex_multiplier: ::core::ffi::c_int = 0x10 as ::core::ffi::c_int;
    let mut permissions: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut check1: ::core::ffi::c_int = param1;
    let mut valid1: ::core::ffi::c_int = (check1 != 0) as ::core::ffi::c_int;
    let mut check2: ::core::ffi::c_int = param2;
    let mut valid2: ::core::ffi::c_int = (check2 != 0) as ::core::ffi::c_int;
    let mut check3: ::core::ffi::c_int = param3;
    let mut valid3: ::core::ffi::c_int = (check3 != 0) as ::core::ffi::c_int;
    let mut check4: ::core::ffi::c_int = param4;
    let mut valid4: ::core::ffi::c_int = (check4 != 0) as ::core::ffi::c_int;
    if valid1 != 0 {
        permissions |= FLAG_READ;
    }
    if valid2 != 0 {
        permissions |= FLAG_WRITE;
    }
    if valid3 != 0 {
        permissions |= FLAG_EXECUTE;
    }
    if valid4 != 0 {
        permissions |= FLAG_DELETE;
    }
    let mut arr: *mut DynamicArray = init_array(2 as size_t);
    if arr.is_null() {
        return -(1 as ::core::ffi::c_int);
    }
    add_element(arr, param1);
    add_element(arr, param2);
    add_element(arr, param3);
    add_element(arr, param4);
    let mut sum: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut i: size_t = 0;
    i = 0 as size_t;
    while i < (*arr).size {
        sum += *(*arr).data.offset(i as isize);
        i = i.wrapping_add(1);
    }
    let mut flag_count: ::core::ffi::c_int = process_flags(permissions);
    let mut matrix_sum: ::core::ffi::c_int = calculate_matrix_checksum();
    result =
        sum * hex_multiplier + flag_count * hex_base + (matrix_sum & 0xfff as ::core::ffi::c_int);
    free_array(arr);
    return result;
}
