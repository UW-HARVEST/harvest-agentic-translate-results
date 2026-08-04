








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
pub fn shift_array(arr: &mut [::core::ffi::c_int], positions: ::core::ffi::c_int) {
    let size = arr.len() as ::core::ffi::c_int;
    if positions > 0 && positions < size {
        let positions = positions as usize;
        arr.copy_within(0..(size as usize - positions), positions);
        arr[..positions].fill(0);
    }
}

#[no_mangle]
pub fn process_string(s: &[::core::ffi::c_char]) -> ::core::ffi::c_int {
    if s.first().copied().unwrap_or(0) != 0 {
        s.iter().take_while(|&&ch| ch != 0).count() as ::core::ffi::c_int
    } else {
        0
    }
}

#[no_mangle]
pub fn apply_bitmask(value: i32, operation: i32) -> i32 {
    let mask1 = 0o360;
    let mask2 = 0o17;
    let mask3 = 0o252;
    let mask4 = 0o125;

    match operation {
        0 => value & mask1,
        1 => value & mask2,
        2 => value | mask3,
        3 => value ^ mask4,
        _ => value,
    }
}

#[no_mangle]
pub fn init_matrix(matrix: &mut [[i32; 4]; 3]) {
    *matrix = [
        [1, 2, 3, 4],
        [5, 6, 7, 8],
        [9, 10, 11, 12],
    ];
}

#[no_mangle]
pub fn compare_allocations(
    val1: ::core::ffi::c_int,
    val2: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    let ptr1 = Box::new(val1);
    let ptr2 = Box::new(val2);

    let addr1 = (&*ptr1 as *const ::core::ffi::c_int) as usize;
    let addr2 = (&*ptr2 as *const ::core::ffi::c_int) as usize;

    let mut result: ::core::ffi::c_int = if addr1 < addr2 {
        1
    } else if addr1 > addr2 {
        2
    } else {
        3
    };

    let uninit_ptr = &*ptr1;
    if *uninit_ptr > 0 {
        result += 10;
    }

    result
}

#[no_mangle]
pub fn arity4(
    param1: i32,
    param2: i32,
    param3: i32,
    param4: i32,
) -> i32 {
    let mut result = 0;

    let mut block = DataBlock {
        values: [param1, param2, param3, param4],
        count: 4,
        label: ::core::ptr::null_mut(),
    };

    let test_str: [::core::ffi::c_char; 6] = [72, 101, 108, 108, 111, 0];
    let empty_str: [::core::ffi::c_char; 1] = [0];

    let len1 = process_string(&test_str);
    let len2 = process_string(&empty_str);
    result += len1 + len2;

    shift_array(&mut block.values, 1);

    for &value in block.values.iter().take(block.count as usize) {
        result += value;
    }

    result = apply_bitmask(result, param1 % 4);

    let mut matrix = [[0; 4]; 3];
    init_matrix(&mut matrix);
    result += matrix[0][0] + matrix[2][3];

    let alloc_result = compare_allocations(param1, param2);
    result += alloc_result;

    if param3 != 0 {
        result = result * param3 / 100;
    }

    if param4 != 0 {
        result += param4;
    }

    result
}

#[no_mangle]
pub fn arity2(p1: i32, p2: i32) -> i32 {
    arity4(p1, p2, 0, 0)
}

#[no_mangle]
pub fn arity3(p1: i32, p2: i32, p3: i32) -> i32 {
    arity4(p1, p2, p3, 0)
}

#[no_mangle]
pub fn arity(len: u8, params: &[i32]) -> i32 {
    if len < 2 {
        -1
    } else if len == 2 {
        arity2(params[0], params[1])
    } else if len == 3 {
        arity3(params[0], params[1], params[2])
    } else {
        arity4(params[0], params[1], params[2], params[3])
    }
}

