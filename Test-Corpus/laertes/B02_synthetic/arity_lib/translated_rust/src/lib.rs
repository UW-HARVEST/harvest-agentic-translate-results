extern "C" {
    fn malloc(__size: size_t) -> *mut libc::c_void;
    fn free(__ptr: *mut libc::c_void);
    fn memmove(
        __dest: *mut libc::c_void,
        __src: *const libc::c_void,
        __n: size_t,
    ) -> *mut libc::c_void;
    fn strlen(__s: *const libc::c_char) -> size_t;
}
pub type size_t = usize;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct DataBlock {
    pub values: [libc::c_int; 4],
    pub count: libc::c_int,
    pub label: *mut libc::c_char,
}
pub const NULL: *mut libc::c_void = std::ptr::null_mut::<libc::c_void>();
#[no_mangle]
pub unsafe extern "C" fn shift_array(
    mut arr: *mut libc::c_int,
    mut size: libc::c_int,
    mut positions: libc::c_int,
) {
    if positions > 0 as libc::c_int && positions < size {
        memmove(
            arr.offset(positions as isize) as *mut libc::c_void,
            arr as *const libc::c_void,
            ((size - positions) as size_t)
                .wrapping_mul(std::mem::size_of::<libc::c_int>() as size_t),
        );
        let mut i: libc::c_int = 0 as libc::c_int;
        while i < positions {
            *arr.offset(i as isize) = 0 as libc::c_int;
            i += 1;
        }
    }
}
#[no_mangle]
pub unsafe extern "C" fn process_string(mut str: *const libc::c_char) -> libc::c_int {
    if *str != 0 {
        return strlen(str) as libc::c_int;
    }
    return 0 as libc::c_int;
}
#[no_mangle]
pub extern "C" fn apply_bitmask(
    mut value: libc::c_int,
    mut operation: libc::c_int,
) -> libc::c_int {
    let mut mask1: libc::c_int = 0o360 as libc::c_int;
    let mut mask2: libc::c_int = 0o17 as libc::c_int;
    let mut mask3: libc::c_int = 0o252 as libc::c_int;
    let mut mask4: libc::c_int = 0o125 as libc::c_int;
    match operation {
        0 => return value & mask1,
        1 => return value & mask2,
        2 => return value | mask3,
        3 => return value ^ mask4,
        _ => return value,
    };
}
#[no_mangle]
pub unsafe extern "C" fn init_matrix(mut matrix: *mut [libc::c_int; 4]) {
    let mut temp: [[libc::c_int; 4]; 3] = [
        [
            1 as libc::c_int,
            2 as libc::c_int,
            3 as libc::c_int,
            4 as libc::c_int,
        ],
        [
            5 as libc::c_int,
            6 as libc::c_int,
            7 as libc::c_int,
            8 as libc::c_int,
        ],
        [
            9 as libc::c_int,
            10 as libc::c_int,
            11 as libc::c_int,
            12 as libc::c_int,
        ],
    ];
    let mut i: libc::c_int = 0 as libc::c_int;
    while i < 3 as libc::c_int {
        let mut j: libc::c_int = 0 as libc::c_int;
        while j < 4 as libc::c_int {
            (*matrix.offset(i as isize))[j as usize] = temp[i as usize][j as usize];
            j += 1;
        }
        i += 1;
    }
}
#[no_mangle]
pub unsafe extern "C" fn compare_allocations(
    mut val1: libc::c_int,
    mut val2: libc::c_int,
) -> libc::c_int {
    let mut ptr1: *mut libc::c_int =
        malloc(std::mem::size_of::<libc::c_int>() as size_t) as *mut libc::c_int;
    let mut ptr2: *mut libc::c_int =
        malloc(std::mem::size_of::<libc::c_int>() as size_t) as *mut libc::c_int;
    let mut uninit_ptr: *mut libc::c_int = std::ptr::null_mut::<libc::c_int>();
    if ptr1.is_null() || ptr2.is_null() {
        free(ptr1 as *mut libc::c_void);
        free(ptr2 as *mut libc::c_void);
        return -(1 as libc::c_int);
    }
    *ptr1 = val1;
    *ptr2 = val2;
    let mut result: libc::c_int = 0 as libc::c_int;
    if ptr1 < ptr2 {
        result = 1 as libc::c_int;
    } else if ptr1 > ptr2 {
        result = 2 as libc::c_int;
    } else {
        result = 3 as libc::c_int;
    }
    uninit_ptr = ptr1;
    result += if *uninit_ptr > 0 as libc::c_int {
        10 as libc::c_int
    } else {
        0 as libc::c_int
    };
    free(ptr1 as *mut libc::c_void);
    free(ptr2 as *mut libc::c_void);
    return result;
}
#[no_mangle]
pub unsafe extern "C" fn arity4(
    mut param1: libc::c_int,
    mut param2: libc::c_int,
    mut param3: libc::c_int,
    mut param4: libc::c_int,
) -> libc::c_int {
    let mut result: libc::c_int = 0 as libc::c_int;
    let mut block: DataBlock = DataBlock {
        values: [param1, param2, param3, param4],
        count: 4 as libc::c_int,
        label: std::ptr::null_mut::<libc::c_char>(),
    };
    let mut test_str: [libc::c_char; 6] =
        std::mem::transmute::<[u8; 6], [libc::c_char; 6]>(*b"Hello\0");
    let mut empty_str: [libc::c_char; 1] =
        std::mem::transmute::<[u8; 1], [libc::c_char; 1]>(*b"\0");
    let mut len1: libc::c_int =
        process_string(&raw mut test_str as *mut libc::c_char);
    let mut len2: libc::c_int =
        process_string(&raw mut empty_str as *mut libc::c_char);
    result += len1 + len2;
    shift_array(
        &raw mut block.values as *mut libc::c_int,
        4 as libc::c_int,
        1 as libc::c_int,
    );
    let mut i: libc::c_int = 0 as libc::c_int;
    while i < block.count {
        result += block.values[i as usize];
        i += 1;
    }
    result = apply_bitmask(result, param1 % 4 as libc::c_int);
    let mut matrix: [[libc::c_int; 4]; 3] = [[0; 4]; 3];
    init_matrix(&raw mut matrix as *mut [libc::c_int; 4]);
    result += matrix[0 as libc::c_int as usize][0 as libc::c_int as usize]
        + matrix[2 as libc::c_int as usize][3 as libc::c_int as usize];
    let mut alloc_result: libc::c_int = compare_allocations(param1, param2);
    result += alloc_result;
    if param3 != 0 as libc::c_int {
        result = result * param3 / 100 as libc::c_int;
    }
    if param4 != 0 as libc::c_int {
        result += param4;
    }
    return result;
}
#[no_mangle]
pub unsafe extern "C" fn arity2(
    mut p1: libc::c_int,
    mut p2: libc::c_int,
) -> libc::c_int {
    return arity4(p1, p2, 0 as libc::c_int, 0 as libc::c_int);
}
#[no_mangle]
pub unsafe extern "C" fn arity3(
    mut p1: libc::c_int,
    mut p2: libc::c_int,
    mut p3: libc::c_int,
) -> libc::c_int {
    return arity4(p1, p2, p3, 0 as libc::c_int);
}
#[no_mangle]
pub unsafe extern "C" fn arity(
    mut len: libc::c_uchar,
    mut params: *mut libc::c_int,
) -> libc::c_int {
    if (len as libc::c_int) < 2 as libc::c_int {
        return -(1 as libc::c_int);
    } else if len as libc::c_int == 2 as libc::c_int {
        return arity2(
            *params.offset(0 as libc::c_int as isize),
            *params.offset(1 as libc::c_int as isize),
        );
    } else if len as libc::c_int == 3 as libc::c_int {
        return arity3(
            *params.offset(0 as libc::c_int as isize),
            *params.offset(1 as libc::c_int as isize),
            *params.offset(2 as libc::c_int as isize),
        );
    } else {
        return arity4(
            *params.offset(0 as libc::c_int as isize),
            *params.offset(1 as libc::c_int as isize),
            *params.offset(2 as libc::c_int as isize),
            *params.offset(3 as libc::c_int as isize),
        );
    };
}
