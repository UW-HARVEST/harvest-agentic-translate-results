use ::f128;
extern "C" {
    fn printf(__format: *const libc::c_char, ...) -> libc::c_int;
    fn memcpy(
        __dest: *mut libc::c_void,
        __src: *const libc::c_void,
        __n: size_t,
    ) -> *mut libc::c_void;
    fn strncpy(
        __dest: *mut libc::c_char,
        __src: *const libc::c_char,
        __n: size_t,
    ) -> *mut libc::c_char;
    fn sqrt(__x: libc::c_double) -> libc::c_double;
    fn __isnan(__value: libc::c_double) -> libc::c_int;
    fn __isnanf(__value: libc::c_float) -> libc::c_int;
    fn __isnanl(__value: ::f128::f128) -> libc::c_int;
}
pub type size_t = usize;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct DataBlock {
    pub id: libc::c_int,
    pub value: libc::c_double,
    pub label: [libc::c_char; 20],
}
#[no_mangle]
pub unsafe extern "C" fn safe_double_to_int(mut d: libc::c_double) -> libc::c_int {
    if d > INT_MAX as libc::c_double {
        return INT_MAX;
    } else if d < INT_MIN as libc::c_double {
        return INT_MIN;
    } else if if std::mem::size_of::<libc::c_double>() as usize
        == std::mem::size_of::<libc::c_float>() as usize
    {
        __isnanf(d as libc::c_float)
    } else if std::mem::size_of::<libc::c_double>() as usize
        == std::mem::size_of::<libc::c_double>() as usize
    {
        __isnan(d)
    } else {
        __isnanl(::f128::f128::new(d))
    } != 0
    {
        return 0 as libc::c_int;
    } else {
        return d as libc::c_int;
    };
}
#[no_mangle]
pub unsafe extern "C" fn process_with_fallthrough(
    mut code: libc::c_int,
    mut base_value: libc::c_int,
) -> libc::c_int {
    let mut result: libc::c_int = base_value;
    let mut current_block_6: u64;
    match code {
        5 => {
            result += 50 as libc::c_int;
            current_block_6 = 18292927763123642495;
        }
        4 => {
            current_block_6 = 18292927763123642495;
        }
        3 => {
            current_block_6 = 11874738012075647942;
        }
        2 => {
            current_block_6 = 16511554149692615611;
        }
        1 => {
            current_block_6 = 7028299883459863757;
        }
        0 => {
            result = 0 as libc::c_int;
            current_block_6 = 3276175668257526147;
        }
        _ => {
            result = -(1 as libc::c_int);
            current_block_6 = 3276175668257526147;
        }
    }
    match current_block_6 {
        18292927763123642495 => {
            result += 40 as libc::c_int;
            current_block_6 = 11874738012075647942;
        }
        _ => {}
    }
    match current_block_6 {
        11874738012075647942 => {
            result += 30 as libc::c_int;
            current_block_6 = 16511554149692615611;
        }
        _ => {}
    }
    match current_block_6 {
        16511554149692615611 => {
            result += 20 as libc::c_int;
            current_block_6 = 7028299883459863757;
        }
        _ => {}
    }
    match current_block_6 {
        7028299883459863757 => {
            result += 10 as libc::c_int;
        }
        _ => {}
    }
    return result;
}
#[no_mangle]
pub unsafe extern "C" fn copy_data_block(mut dest: *mut DataBlock, mut src: *const DataBlock) {
    memcpy(
        dest as *mut libc::c_void,
        src as *const libc::c_void,
        std::mem::size_of::<DataBlock>() as size_t,
    );
}
#[no_mangle]
pub unsafe extern "C" fn handle_pointer_operations(
    mut value: libc::c_int,
) -> libc::c_int {
    let mut ptr: *mut libc::c_int = std::ptr::null_mut::<libc::c_int>();
    let mut local_value: libc::c_int = value * 2 as libc::c_int;
    ptr = &raw mut local_value;
    let mut result: libc::c_int = *ptr + 100 as libc::c_int;
    return result;
}
#[no_mangle]
pub unsafe extern "C" fn overunder(
    mut a: libc::c_int,
    mut b: libc::c_int,
    mut c: libc::c_int,
    mut d: libc::c_int,
) -> libc::c_int {
    let mut total: libc::c_int = 0 as libc::c_int;
    let mut result_1: libc::c_int = a;
    let mut result_2: libc::c_int = b;
    let mut result_3: libc::c_int = c;
    let mut result_4: libc::c_int = d;
    printf(
        b"result_1 = %d\n\0" as *const u8 as *const libc::c_char,
        result_1,
    );
    printf(
        b"result_2 = %d\n\0" as *const u8 as *const libc::c_char,
        result_2,
    );
    let mut temp1: libc::c_double = a as libc::c_double * 1.5f64;
    let mut temp2: libc::c_double = b as libc::c_double * 2.7f64;
    let mut temp3: libc::c_double = c as libc::c_double / 3.3f64;
    let mut temp4: libc::c_double = sqrt((d * d + a * a) as libc::c_double);
    let mut conv1: libc::c_int = safe_double_to_int(temp1);
    let mut conv2: libc::c_int = safe_double_to_int(temp2);
    let mut conv3: libc::c_int = safe_double_to_int(temp3);
    let mut conv4: libc::c_int = safe_double_to_int(temp4);
    printf(
        b"Converted values: %d, %d, %d, %d\n\0" as *const u8 as *const libc::c_char,
        conv1,
        conv2,
        conv3,
        conv4,
    );
    let mut switch_result: libc::c_int =
        process_with_fallthrough(a % 6 as libc::c_int, b);
    printf(
        b"Switch fall-through result: %d\n\0" as *const u8 as *const libc::c_char,
        switch_result,
    );
    let mut source_block: DataBlock = DataBlock {
        id: 0,
        value: 0.,
        label: [0; 20],
    };
    source_block.id = a;
    source_block.value = temp1;
    strncpy(
        &raw mut source_block.label as *mut libc::c_char,
        b"Source\0" as *const u8 as *const libc::c_char,
        (std::mem::size_of::<[libc::c_char; 20]>() as size_t).wrapping_sub(1 as size_t),
    );
    source_block.label[(std::mem::size_of::<[libc::c_char; 20]>() as usize)
        .wrapping_sub(1 as usize) as usize] = '\0' as i32 as libc::c_char;
    let mut dest_block: DataBlock = DataBlock {
        id: 0,
        value: 0.,
        label: [0; 20],
    };
    copy_data_block(&raw mut dest_block, &raw mut source_block);
    printf(
        b"Copied block: id=%d, value=%.2f, label=%s\n\0" as *const u8 as *const libc::c_char,
        dest_block.id,
        dest_block.value,
        &raw mut dest_block.label as *mut libc::c_char,
    );
    let mut ptr_result: libc::c_int = handle_pointer_operations(c);
    printf(
        b"Pointer operation result: %d\n\0" as *const u8 as *const libc::c_char,
        ptr_result,
    );
    total = conv1 + conv2 + conv3 + conv4 + switch_result + ptr_result;
    total += dest_block.id;
    let mut overflow_test: libc::c_double = 1e15f64;
    let mut safe_conv: libc::c_int = safe_double_to_int(overflow_test);
    printf(
        b"Overflow protected conversion: %d\n\0" as *const u8 as *const libc::c_char,
        safe_conv,
    );
    let mut underflow_test: libc::c_double = -1e15f64;
    let mut safe_conv2: libc::c_int = safe_double_to_int(underflow_test);
    printf(
        b"Underflow protected conversion: %d\n\0" as *const u8 as *const libc::c_char,
        safe_conv2,
    );
    let mut array1: [libc::c_int; 5] = [a, b, c, d, a + b];
    let mut array2: [libc::c_int; 5] = [0; 5];
    memcpy(
        &raw mut array2 as *mut libc::c_int as *mut libc::c_void,
        &raw mut array1 as *mut libc::c_int as *const libc::c_void,
        std::mem::size_of::<[libc::c_int; 5]>() as size_t,
    );
    printf(b"Array copied via memcpy: \0" as *const u8 as *const libc::c_char);
    let mut i: libc::c_int = 0 as libc::c_int;
    while i < 5 as libc::c_int {
        printf(
            b"%d \0" as *const u8 as *const libc::c_char,
            array2[i as usize],
        );
        total += array2[i as usize];
        i += 1;
    }
    printf(b"\n\0" as *const u8 as *const libc::c_char);
    return total;
}
pub const INT_MAX: libc::c_int = __INT_MAX__;
pub const INT_MIN: libc::c_int = -__INT_MAX__ - 1 as libc::c_int;
pub const __INT_MAX__: libc::c_int = 2147483647 as libc::c_int;
