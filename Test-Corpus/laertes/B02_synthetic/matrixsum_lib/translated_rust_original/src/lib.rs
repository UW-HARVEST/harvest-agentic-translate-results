extern "C" {
    fn malloc(__size: size_t) -> *mut libc::c_void;
    fn realloc(__ptr: *mut libc::c_void, __size: size_t) -> *mut libc::c_void;
    fn free(__ptr: *mut libc::c_void);
}
pub type size_t = usize;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct DynamicArray {
    pub data: * mut libc::c_int,
    pub size: usize,
    pub capacity: usize,
}
impl std::default::Default for DynamicArray {
    fn default() -> Self {
        DynamicArray {
        data: 0 as * mut libc::c_int,
        size: usize::default(),
        capacity: usize::default()
        }
    }
}

pub const NULL: *mut libc::c_void = std::ptr::null_mut::<libc::c_void>();
#[no_mangle]
pub static mut matrix: [[libc::c_int; 4]; 3] = [
    [
        0x1 as libc::c_int,
        0x2 as libc::c_int,
        0x3 as libc::c_int,
        0x4 as libc::c_int,
    ],
    [
        0x10 as libc::c_int,
        0x20 as libc::c_int,
        0x30 as libc::c_int,
        0x40 as libc::c_int,
    ],
    [
        0xa1 as libc::c_int,
        0xb2 as libc::c_int,
        0xc3 as libc::c_int,
        0xd4 as libc::c_int,
    ],
];
pub const FLAG_READ: libc::c_int = 0o1 as libc::c_int;
pub const FLAG_WRITE: libc::c_int = 0o2 as libc::c_int;
pub const FLAG_EXECUTE: libc::c_int = 0o4 as libc::c_int;
pub const FLAG_DELETE: libc::c_int = 0o10 as libc::c_int;
#[no_mangle]
pub unsafe extern "C" fn init_array(mut initial_capacity: size_t) -> *mut DynamicArray {
    let mut arr: *mut DynamicArray =
        malloc(std::mem::size_of::<DynamicArray>() as size_t) as *mut DynamicArray;
    if arr.is_null() {
        return std::ptr::null_mut::<DynamicArray>();
    }
    (*arr).data = malloc(
        initial_capacity.wrapping_mul(std::mem::size_of::<libc::c_int>() as size_t),
    ) as *mut libc::c_int;
    if (*arr).data.is_null() {
        free(arr as *mut libc::c_void);
        return std::ptr::null_mut::<DynamicArray>();
    }
    (*arr).size = 0 as size_t;
    (*arr).capacity = initial_capacity;
    return arr;
}
#[no_mangle]
pub unsafe extern "C" fn expand_array(mut arr: *mut DynamicArray) -> libc::c_int {
    if arr.is_null() {
        return 0 as libc::c_int;
    }
    let mut new_capacity: size_t = (*arr).capacity.wrapping_mul(2 as size_t);
    let mut new_data: *mut libc::c_int = realloc(
        (*arr).data as *mut libc::c_void,
        new_capacity.wrapping_mul(std::mem::size_of::<libc::c_int>() as size_t),
    ) as *mut libc::c_int;
    if new_data.is_null() {
        return 0 as libc::c_int;
    }
    (*arr).data = new_data;
    (*arr).capacity = new_capacity;
    return 1 as libc::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn add_element(
    mut arr: *mut DynamicArray,
    mut value: libc::c_int,
) -> libc::c_int {
    if arr.is_null() {
        return 0 as libc::c_int;
    }
    if (*arr).size >= (*arr).capacity {
        if expand_array(arr) == 0 {
            return 0 as libc::c_int;
        }
    }
    let fresh0 = (*arr).size;
    (*arr).size = (*arr).size.wrapping_add(1);
    *(*arr).data.offset(fresh0 as isize) = value;
    return 1 as libc::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn free_array(mut arr: *mut DynamicArray) {
    if !arr.is_null() {
        free((*arr).data as *mut libc::c_void);
        free(arr as *mut libc::c_void);
    }
}
#[no_mangle]
pub extern "C" fn process_flags(mut flags: libc::c_int) -> libc::c_int {
    let mut count: libc::c_int = 0 as libc::c_int;
    let mut has_read: libc::c_int = flags & FLAG_READ;
    let mut read_enabled: libc::c_int = (has_read != 0) as libc::c_int;
    let mut has_write: libc::c_int = flags & FLAG_WRITE;
    let mut write_enabled: libc::c_int = (has_write != 0) as libc::c_int;
    let mut has_execute: libc::c_int = flags & FLAG_EXECUTE;
    let mut execute_enabled: libc::c_int = (has_execute != 0) as libc::c_int;
    let mut has_delete: libc::c_int = flags & FLAG_DELETE;
    let mut delete_enabled: libc::c_int = (has_delete != 0) as libc::c_int;
    count = read_enabled + write_enabled + execute_enabled + delete_enabled;
    return count;
}
#[no_mangle]
pub unsafe extern "C" fn calculate_matrix_checksum() -> libc::c_int {
    let mut sum: libc::c_int = 0 as libc::c_int;
    let mut i: libc::c_int = 0;
    let mut j: libc::c_int = 0;
    i = 0 as libc::c_int;
    while i < 3 as libc::c_int {
        j = 0 as libc::c_int;
        while j < 4 as libc::c_int {
            sum += matrix[i as usize][j as usize];
            j += 1;
        }
        i += 1;
    }
    return sum;
}
#[no_mangle]
pub unsafe extern "C" fn matrixsum(
    mut param1: libc::c_int,
    mut param2: libc::c_int,
    mut param3: libc::c_int,
    mut param4: libc::c_int,
) -> libc::c_int {
    let mut result: libc::c_int = 0 as libc::c_int;
    let mut hex_base: libc::c_int = 0xff as libc::c_int;
    let mut hex_multiplier: libc::c_int = 0x10 as libc::c_int;
    let mut permissions: libc::c_int = 0 as libc::c_int;
    let mut check1: libc::c_int = param1;
    let mut valid1: libc::c_int = (check1 != 0) as libc::c_int;
    let mut check2: libc::c_int = param2;
    let mut valid2: libc::c_int = (check2 != 0) as libc::c_int;
    let mut check3: libc::c_int = param3;
    let mut valid3: libc::c_int = (check3 != 0) as libc::c_int;
    let mut check4: libc::c_int = param4;
    let mut valid4: libc::c_int = (check4 != 0) as libc::c_int;
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
        return -(1 as libc::c_int);
    }
    add_element(arr, param1);
    add_element(arr, param2);
    add_element(arr, param3);
    add_element(arr, param4);
    let mut sum: libc::c_int = 0 as libc::c_int;
    let mut i: size_t = 0;
    i = 0 as size_t;
    while i < (*arr).size {
        sum += *(*arr).data.offset(i as isize);
        i = i.wrapping_add(1);
    }
    let mut flag_count: libc::c_int = process_flags(permissions);
    let mut matrix_sum: libc::c_int = calculate_matrix_checksum();
    result =
        sum * hex_multiplier + flag_count * hex_base + (matrix_sum & 0xfff as libc::c_int);
    free_array(arr);
    return result;
}
pub fn borrow<'a, 'b: 'a, T>(p: &'a Option<&'b mut T>) -> Option<&'a T> {
    p.as_ref().map(|x| &**x)
}

pub fn borrow_mut<'a, 'b : 'a, T>(p: &'a mut Option<&'b mut T>) -> Option<&'a mut T> {
    p.as_mut().map(|x| &mut **x)
}

pub fn owned_as_ref<'a, T>(p: &'a Option<Box<T>>) -> Option<&'a T> {
    p.as_ref().map(|x| x.as_ref())
}

pub fn owned_as_mut<'a, T>(p: &'a mut Option<Box<T>>) -> Option<&'a mut T> {
    p.as_mut().map(|x| x.as_mut())
}

pub fn option_to_raw<T>(p: Option<&T>) -> * const T {
    p.map_or(core::ptr::null(), |p| p as * const T)
}

pub fn _ref_eq<T>(p: Option<&T>, q: Option<&T>) -> bool {
    option_to_raw(p) == option_to_raw(q)
}

pub fn _ref_ne<T>(p: Option<&T>, q: Option<&T>) -> bool {
    option_to_raw(p) != option_to_raw(q)
}

