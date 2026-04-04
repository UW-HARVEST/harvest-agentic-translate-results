extern "C" {
    fn malloc(__size: size_t) -> *mut ::core::ffi::c_void;
    fn free(__ptr: *mut ::core::ffi::c_void);
    fn memmove(
        __dest: *mut ::core::ffi::c_void,
        __src: *const ::core::ffi::c_void,
        __n: size_t,
    ) -> *mut ::core::ffi::c_void;
    fn strlen(__s: *const ::core::ffi::c_char) -> size_t;
}
pub type size_t = usize;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct DataBlock {
    pub values: [::core::ffi::c_int; 4],
    pub count: ::core::ffi::c_int,
    pub label: *mut ::core::ffi::c_char,
}
pub const NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<::core::ffi::c_void>();
#[no_mangle]
pub unsafe extern "C" fn shift_array(
    mut arr: *mut ::core::ffi::c_int,
    mut size: ::core::ffi::c_int,
    mut positions: ::core::ffi::c_int,
) {
    if positions > 0 as ::core::ffi::c_int && positions < size {
        memmove(
            arr.offset(positions as isize) as *mut ::core::ffi::c_void,
            arr as *const ::core::ffi::c_void,
            ((size - positions) as size_t)
                .wrapping_mul(::core::mem::size_of::<::core::ffi::c_int>() as size_t),
        );
        let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
        while i < positions {
            *arr.offset(i as isize) = 0 as ::core::ffi::c_int;
            i += 1;
        }
    }
}
#[no_mangle]
pub unsafe extern "C" fn process_string(mut str: *const ::core::ffi::c_char) -> ::core::ffi::c_int {
    if *str != 0 {
        return strlen(str) as ::core::ffi::c_int;
    }
    return 0 as ::core::ffi::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn apply_bitmask(
    mut value: ::core::ffi::c_int,
    mut operation: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    let mut mask1: ::core::ffi::c_int = 0o360 as ::core::ffi::c_int;
    let mut mask2: ::core::ffi::c_int = 0o17 as ::core::ffi::c_int;
    let mut mask3: ::core::ffi::c_int = 0o252 as ::core::ffi::c_int;
    let mut mask4: ::core::ffi::c_int = 0o125 as ::core::ffi::c_int;
    match operation {
        0 => return value & mask1,
        1 => return value & mask2,
        2 => return value | mask3,
        3 => return value ^ mask4,
        _ => return value,
    };
}
#[no_mangle]
pub unsafe extern "C" fn init_matrix(mut matrix: *mut [::core::ffi::c_int; 4]) {
    let mut temp: [[::core::ffi::c_int; 4]; 3] = [
        [
            1 as ::core::ffi::c_int,
            2 as ::core::ffi::c_int,
            3 as ::core::ffi::c_int,
            4 as ::core::ffi::c_int,
        ],
        [
            5 as ::core::ffi::c_int,
            6 as ::core::ffi::c_int,
            7 as ::core::ffi::c_int,
            8 as ::core::ffi::c_int,
        ],
        [
            9 as ::core::ffi::c_int,
            10 as ::core::ffi::c_int,
            11 as ::core::ffi::c_int,
            12 as ::core::ffi::c_int,
        ],
    ];
    let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while i < 3 as ::core::ffi::c_int {
        let mut j: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
        while j < 4 as ::core::ffi::c_int {
            (*matrix.offset(i as isize))[j as usize] = temp[i as usize][j as usize];
            j += 1;
        }
        i += 1;
    }
}
#[no_mangle]
pub unsafe extern "C" fn compare_allocations(
    mut val1: ::core::ffi::c_int,
    mut val2: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    let mut ptr1: *mut ::core::ffi::c_int =
        malloc(::core::mem::size_of::<::core::ffi::c_int>() as size_t) as *mut ::core::ffi::c_int;
    let mut ptr2: *mut ::core::ffi::c_int =
        malloc(::core::mem::size_of::<::core::ffi::c_int>() as size_t) as *mut ::core::ffi::c_int;
    let mut uninit_ptr: *mut ::core::ffi::c_int = ::core::ptr::null_mut::<::core::ffi::c_int>();
    if ptr1.is_null() || ptr2.is_null() {
        free(ptr1 as *mut ::core::ffi::c_void);
        free(ptr2 as *mut ::core::ffi::c_void);
        return -(1 as ::core::ffi::c_int);
    }
    *ptr1 = val1;
    *ptr2 = val2;
    let mut result: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    if ptr1 < ptr2 {
        result = 1 as ::core::ffi::c_int;
    } else if ptr1 > ptr2 {
        result = 2 as ::core::ffi::c_int;
    } else {
        result = 3 as ::core::ffi::c_int;
    }
    uninit_ptr = ptr1;
    result += if *uninit_ptr > 0 as ::core::ffi::c_int {
        10 as ::core::ffi::c_int
    } else {
        0 as ::core::ffi::c_int
    };
    free(ptr1 as *mut ::core::ffi::c_void);
    free(ptr2 as *mut ::core::ffi::c_void);
    return result;
}
#[no_mangle]
pub unsafe extern "C" fn arity4(
    mut param1: ::core::ffi::c_int,
    mut param2: ::core::ffi::c_int,
    mut param3: ::core::ffi::c_int,
    mut param4: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    let mut result: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut block: DataBlock = DataBlock {
        values: [param1, param2, param3, param4],
        count: 4 as ::core::ffi::c_int,
        label: ::core::ptr::null_mut::<::core::ffi::c_char>(),
    };
    let mut test_str: [::core::ffi::c_char; 6] =
        ::core::mem::transmute::<[u8; 6], [::core::ffi::c_char; 6]>(*b"Hello\0");
    let mut empty_str: [::core::ffi::c_char; 1] =
        ::core::mem::transmute::<[u8; 1], [::core::ffi::c_char; 1]>(*b"\0");
    let mut len1: ::core::ffi::c_int =
        process_string(&raw mut test_str as *mut ::core::ffi::c_char);
    let mut len2: ::core::ffi::c_int =
        process_string(&raw mut empty_str as *mut ::core::ffi::c_char);
    result += len1 + len2;
    shift_array(
        &raw mut block.values as *mut ::core::ffi::c_int,
        4 as ::core::ffi::c_int,
        1 as ::core::ffi::c_int,
    );
    let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while i < block.count {
        result += block.values[i as usize];
        i += 1;
    }
    result = apply_bitmask(result, param1 % 4 as ::core::ffi::c_int);
    let mut matrix: [[::core::ffi::c_int; 4]; 3] = [[0; 4]; 3];
    init_matrix(&raw mut matrix as *mut [::core::ffi::c_int; 4]);
    result += matrix[0 as ::core::ffi::c_int as usize][0 as ::core::ffi::c_int as usize]
        + matrix[2 as ::core::ffi::c_int as usize][3 as ::core::ffi::c_int as usize];
    let mut alloc_result: ::core::ffi::c_int = compare_allocations(param1, param2);
    result += alloc_result;
    if param3 != 0 as ::core::ffi::c_int {
        result = result * param3 / 100 as ::core::ffi::c_int;
    }
    if param4 != 0 as ::core::ffi::c_int {
        result += param4;
    }
    return result;
}
#[no_mangle]
pub unsafe extern "C" fn arity2(
    mut p1: ::core::ffi::c_int,
    mut p2: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    return arity4(p1, p2, 0 as ::core::ffi::c_int, 0 as ::core::ffi::c_int);
}
#[no_mangle]
pub unsafe extern "C" fn arity3(
    mut p1: ::core::ffi::c_int,
    mut p2: ::core::ffi::c_int,
    mut p3: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    return arity4(p1, p2, p3, 0 as ::core::ffi::c_int);
}
#[no_mangle]
pub unsafe extern "C" fn arity(
    mut len: ::core::ffi::c_uchar,
    mut params: *mut ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    if (len as ::core::ffi::c_int) < 2 as ::core::ffi::c_int {
        return -(1 as ::core::ffi::c_int);
    } else if len as ::core::ffi::c_int == 2 as ::core::ffi::c_int {
        return arity2(
            *params.offset(0 as ::core::ffi::c_int as isize),
            *params.offset(1 as ::core::ffi::c_int as isize),
        );
    } else if len as ::core::ffi::c_int == 3 as ::core::ffi::c_int {
        return arity3(
            *params.offset(0 as ::core::ffi::c_int as isize),
            *params.offset(1 as ::core::ffi::c_int as isize),
            *params.offset(2 as ::core::ffi::c_int as isize),
        );
    } else {
        return arity4(
            *params.offset(0 as ::core::ffi::c_int as isize),
            *params.offset(1 as ::core::ffi::c_int as isize),
            *params.offset(2 as ::core::ffi::c_int as isize),
            *params.offset(3 as ::core::ffi::c_int as isize),
        );
    };
}
