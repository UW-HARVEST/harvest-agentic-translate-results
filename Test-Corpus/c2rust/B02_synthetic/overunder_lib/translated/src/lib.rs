use ::f128;
extern "C" {
    fn printf(__format: *const ::core::ffi::c_char, ...) -> ::core::ffi::c_int;
    fn memcpy(
        __dest: *mut ::core::ffi::c_void,
        __src: *const ::core::ffi::c_void,
        __n: size_t,
    ) -> *mut ::core::ffi::c_void;
    fn strncpy(
        __dest: *mut ::core::ffi::c_char,
        __src: *const ::core::ffi::c_char,
        __n: size_t,
    ) -> *mut ::core::ffi::c_char;
    fn sqrt(__x: ::core::ffi::c_double) -> ::core::ffi::c_double;
    fn __isnan(__value: ::core::ffi::c_double) -> ::core::ffi::c_int;
    fn __isnanf(__value: ::core::ffi::c_float) -> ::core::ffi::c_int;
    fn __isnanl(__value: ::f128::f128) -> ::core::ffi::c_int;
}
pub type size_t = usize;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct DataBlock {
    pub id: ::core::ffi::c_int,
    pub value: ::core::ffi::c_double,
    pub label: [::core::ffi::c_char; 20],
}
#[no_mangle]
pub unsafe extern "C" fn safe_double_to_int(mut d: ::core::ffi::c_double) -> ::core::ffi::c_int {
    if d > INT_MAX as ::core::ffi::c_double {
        return INT_MAX;
    } else if d < INT_MIN as ::core::ffi::c_double {
        return INT_MIN;
    } else if if ::core::mem::size_of::<::core::ffi::c_double>() as usize
        == ::core::mem::size_of::<::core::ffi::c_float>() as usize
    {
        __isnanf(d as ::core::ffi::c_float)
    } else if ::core::mem::size_of::<::core::ffi::c_double>() as usize
        == ::core::mem::size_of::<::core::ffi::c_double>() as usize
    {
        __isnan(d)
    } else {
        __isnanl(::f128::f128::new(d))
    } != 0
    {
        return 0 as ::core::ffi::c_int;
    } else {
        return d as ::core::ffi::c_int;
    };
}
#[no_mangle]
pub unsafe extern "C" fn process_with_fallthrough(
    mut code: ::core::ffi::c_int,
    mut base_value: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    let mut result: ::core::ffi::c_int = base_value;
    let mut current_block_6: u64;
    match code {
        5 => {
            result += 50 as ::core::ffi::c_int;
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
            result = 0 as ::core::ffi::c_int;
            current_block_6 = 3276175668257526147;
        }
        _ => {
            result = -(1 as ::core::ffi::c_int);
            current_block_6 = 3276175668257526147;
        }
    }
    match current_block_6 {
        18292927763123642495 => {
            result += 40 as ::core::ffi::c_int;
            current_block_6 = 11874738012075647942;
        }
        _ => {}
    }
    match current_block_6 {
        11874738012075647942 => {
            result += 30 as ::core::ffi::c_int;
            current_block_6 = 16511554149692615611;
        }
        _ => {}
    }
    match current_block_6 {
        16511554149692615611 => {
            result += 20 as ::core::ffi::c_int;
            current_block_6 = 7028299883459863757;
        }
        _ => {}
    }
    match current_block_6 {
        7028299883459863757 => {
            result += 10 as ::core::ffi::c_int;
        }
        _ => {}
    }
    return result;
}
#[no_mangle]
pub unsafe extern "C" fn copy_data_block(mut dest: *mut DataBlock, mut src: *const DataBlock) {
    memcpy(
        dest as *mut ::core::ffi::c_void,
        src as *const ::core::ffi::c_void,
        ::core::mem::size_of::<DataBlock>() as size_t,
    );
}
#[no_mangle]
pub unsafe extern "C" fn handle_pointer_operations(
    mut value: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    let mut ptr: *mut ::core::ffi::c_int = ::core::ptr::null_mut::<::core::ffi::c_int>();
    let mut local_value: ::core::ffi::c_int = value * 2 as ::core::ffi::c_int;
    ptr = &raw mut local_value;
    let mut result: ::core::ffi::c_int = *ptr + 100 as ::core::ffi::c_int;
    return result;
}
#[no_mangle]
pub unsafe extern "C" fn overunder(
    mut a: ::core::ffi::c_int,
    mut b: ::core::ffi::c_int,
    mut c: ::core::ffi::c_int,
    mut d: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    let mut total: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut result_1: ::core::ffi::c_int = a;
    let mut result_2: ::core::ffi::c_int = b;
    let mut result_3: ::core::ffi::c_int = c;
    let mut result_4: ::core::ffi::c_int = d;
    printf(
        b"result_1 = %d\n\0" as *const u8 as *const ::core::ffi::c_char,
        result_1,
    );
    printf(
        b"result_2 = %d\n\0" as *const u8 as *const ::core::ffi::c_char,
        result_2,
    );
    let mut temp1: ::core::ffi::c_double = a as ::core::ffi::c_double * 1.5f64;
    let mut temp2: ::core::ffi::c_double = b as ::core::ffi::c_double * 2.7f64;
    let mut temp3: ::core::ffi::c_double = c as ::core::ffi::c_double / 3.3f64;
    let mut temp4: ::core::ffi::c_double = sqrt((d * d + a * a) as ::core::ffi::c_double);
    let mut conv1: ::core::ffi::c_int = safe_double_to_int(temp1);
    let mut conv2: ::core::ffi::c_int = safe_double_to_int(temp2);
    let mut conv3: ::core::ffi::c_int = safe_double_to_int(temp3);
    let mut conv4: ::core::ffi::c_int = safe_double_to_int(temp4);
    printf(
        b"Converted values: %d, %d, %d, %d\n\0" as *const u8 as *const ::core::ffi::c_char,
        conv1,
        conv2,
        conv3,
        conv4,
    );
    let mut switch_result: ::core::ffi::c_int =
        process_with_fallthrough(a % 6 as ::core::ffi::c_int, b);
    printf(
        b"Switch fall-through result: %d\n\0" as *const u8 as *const ::core::ffi::c_char,
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
        &raw mut source_block.label as *mut ::core::ffi::c_char,
        b"Source\0" as *const u8 as *const ::core::ffi::c_char,
        (::core::mem::size_of::<[::core::ffi::c_char; 20]>() as size_t).wrapping_sub(1 as size_t),
    );
    source_block.label[(::core::mem::size_of::<[::core::ffi::c_char; 20]>() as usize)
        .wrapping_sub(1 as usize) as usize] = '\0' as i32 as ::core::ffi::c_char;
    let mut dest_block: DataBlock = DataBlock {
        id: 0,
        value: 0.,
        label: [0; 20],
    };
    copy_data_block(&raw mut dest_block, &raw mut source_block);
    printf(
        b"Copied block: id=%d, value=%.2f, label=%s\n\0" as *const u8 as *const ::core::ffi::c_char,
        dest_block.id,
        dest_block.value,
        &raw mut dest_block.label as *mut ::core::ffi::c_char,
    );
    let mut ptr_result: ::core::ffi::c_int = handle_pointer_operations(c);
    printf(
        b"Pointer operation result: %d\n\0" as *const u8 as *const ::core::ffi::c_char,
        ptr_result,
    );
    total = conv1 + conv2 + conv3 + conv4 + switch_result + ptr_result;
    total += dest_block.id;
    let mut overflow_test: ::core::ffi::c_double = 1e15f64;
    let mut safe_conv: ::core::ffi::c_int = safe_double_to_int(overflow_test);
    printf(
        b"Overflow protected conversion: %d\n\0" as *const u8 as *const ::core::ffi::c_char,
        safe_conv,
    );
    let mut underflow_test: ::core::ffi::c_double = -1e15f64;
    let mut safe_conv2: ::core::ffi::c_int = safe_double_to_int(underflow_test);
    printf(
        b"Underflow protected conversion: %d\n\0" as *const u8 as *const ::core::ffi::c_char,
        safe_conv2,
    );
    let mut array1: [::core::ffi::c_int; 5] = [a, b, c, d, a + b];
    let mut array2: [::core::ffi::c_int; 5] = [0; 5];
    memcpy(
        &raw mut array2 as *mut ::core::ffi::c_int as *mut ::core::ffi::c_void,
        &raw mut array1 as *mut ::core::ffi::c_int as *const ::core::ffi::c_void,
        ::core::mem::size_of::<[::core::ffi::c_int; 5]>() as size_t,
    );
    printf(b"Array copied via memcpy: \0" as *const u8 as *const ::core::ffi::c_char);
    let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while i < 5 as ::core::ffi::c_int {
        printf(
            b"%d \0" as *const u8 as *const ::core::ffi::c_char,
            array2[i as usize],
        );
        total += array2[i as usize];
        i += 1;
    }
    printf(b"\n\0" as *const u8 as *const ::core::ffi::c_char);
    return total;
}
pub const INT_MAX: ::core::ffi::c_int = __INT_MAX__;
pub const INT_MIN: ::core::ffi::c_int = -__INT_MAX__ - 1 as ::core::ffi::c_int;
pub const __INT_MAX__: ::core::ffi::c_int = 2147483647 as ::core::ffi::c_int;
