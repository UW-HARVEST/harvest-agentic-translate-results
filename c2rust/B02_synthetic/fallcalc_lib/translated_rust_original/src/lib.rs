use ::f128;
extern "C" {
    fn malloc(__size: size_t) -> *mut ::core::ffi::c_void;
    fn free(__ptr: *mut ::core::ffi::c_void);
    fn __isinf(__value: ::core::ffi::c_double) -> ::core::ffi::c_int;
    fn __isnan(__value: ::core::ffi::c_double) -> ::core::ffi::c_int;
    fn __isinff(__value: ::core::ffi::c_float) -> ::core::ffi::c_int;
    fn __isnanf(__value: ::core::ffi::c_float) -> ::core::ffi::c_int;
    fn __isinfl(__value: ::f128::f128) -> ::core::ffi::c_int;
    fn __isnanl(__value: ::f128::f128) -> ::core::ffi::c_int;
}
pub type size_t = usize;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct DataPoint {
    pub value: ::core::ffi::c_int,
    pub coefficient: ::core::ffi::c_double,
}
pub const NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<::core::ffi::c_void>();
pub const OCTAL_MASK_1: ::core::ffi::c_int = 0o777 as ::core::ffi::c_int;
pub const OCTAL_MASK_2: ::core::ffi::c_int = 0o100 as ::core::ffi::c_int;
pub const OCTAL_FLAG: ::core::ffi::c_int = 0o200 as ::core::ffi::c_int;
pub const OCTAL_BASE: ::core::ffi::c_int = 0o10 as ::core::ffi::c_int;
#[no_mangle]
pub unsafe extern "C" fn safe_double_to_int(mut d: ::core::ffi::c_double) -> ::core::ffi::c_int {
    if if ::core::mem::size_of::<::core::ffi::c_double>() as usize
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
    }
    if if ::core::mem::size_of::<::core::ffi::c_double>() as usize
        == ::core::mem::size_of::<::core::ffi::c_float>() as usize
    {
        __isinff(d as ::core::ffi::c_float)
    } else if ::core::mem::size_of::<::core::ffi::c_double>() as usize
        == ::core::mem::size_of::<::core::ffi::c_double>() as usize
    {
        __isinf(d)
    } else {
        __isinfl(::f128::f128::new(d))
    } != 0
    {
        return if d > 0 as ::core::ffi::c_int as ::core::ffi::c_double {
            INT_MAX
        } else {
            INT_MIN
        };
    }
    if d >= INT_MAX as ::core::ffi::c_double {
        return INT_MAX;
    }
    if d <= INT_MIN as ::core::ffi::c_double {
        return INT_MIN;
    }
    return d as ::core::ffi::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn process_array_reverse(
    mut end: *mut ::core::ffi::c_int,
    mut count: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    let mut sum: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut ptr: *mut ::core::ffi::c_int = end;
    let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while i < count {
        sum += *ptr;
        ptr = ptr.offset(-1);
        i += 1;
    }
    return sum;
}
#[no_mangle]
pub unsafe extern "C" fn switch_fallthrough_calculator(
    mut value: ::core::ffi::c_int,
    mut operation: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    let mut result: ::core::ffi::c_int = value;
    let mut current_block_5: u64;
    match operation {
        0 => {
            result *= OCTAL_BASE;
            current_block_5 = 5023088878038355716;
        }
        1 => {
            current_block_5 = 5023088878038355716;
        }
        2 => {
            current_block_5 = 16023805999610312643;
        }
        3 => {
            result *= 3 as ::core::ffi::c_int;
            current_block_5 = 8515868523999336537;
        }
        4 => {
            current_block_5 = 8515868523999336537;
        }
        _ => {
            result = 0 as ::core::ffi::c_int;
            current_block_5 = 14523784380283086299;
        }
    }
    match current_block_5 {
        5023088878038355716 => {
            result += OCTAL_FLAG;
            current_block_5 = 16023805999610312643;
        }
        8515868523999336537 => {
            result += OCTAL_MASK_2;
            current_block_5 = 14523784380283086299;
        }
        _ => {}
    }
    match current_block_5 {
        16023805999610312643 => {
            result &= OCTAL_MASK_1;
        }
        _ => {}
    }
    return result;
}
#[no_mangle]
pub unsafe extern "C" fn allocate_and_compute(
    mut size: ::core::ffi::c_int,
    mut multiplier: ::core::ffi::c_double,
) -> ::core::ffi::c_int {
    let mut points: *mut DataPoint =
        malloc((size as size_t).wrapping_mul(::core::mem::size_of::<DataPoint>() as size_t))
            as *mut DataPoint;
    if points.is_null() {
        return -(1 as ::core::ffi::c_int);
    }
    let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while i < size {
        (*points.offset(i as isize)).value = i * OCTAL_BASE;
        (*points.offset(i as isize)).coefficient = i as ::core::ffi::c_double * multiplier;
        i += 1;
    }
    let mut sum: ::core::ffi::c_double = 0.0f64;
    let mut i_0: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while i_0 < size {
        sum += (*points.offset(i_0 as isize)).value as ::core::ffi::c_double
            * (*points.offset(i_0 as isize)).coefficient;
        i_0 += 1;
    }
    let mut result: ::core::ffi::c_int = safe_double_to_int(sum);
    free(points as *mut ::core::ffi::c_void);
    return result;
}
#[no_mangle]
pub unsafe extern "C" fn foreach_sum(
    mut array: *mut ::core::ffi::c_int,
    mut count: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    let mut total: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut element: ::core::ffi::c_int = 0;
    let mut keep: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
    let mut idx: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut size: ::core::ffi::c_int = count;
    while keep != 0 && idx < size {
        element = *array.offset(idx as isize);
        while keep != 0 {
            total += element;
            keep = (keep == 0) as ::core::ffi::c_int;
        }
        keep = (keep == 0) as ::core::ffi::c_int;
        idx += 1;
    }
    return total;
}
#[no_mangle]
pub unsafe extern "C" fn fallcalc(
    mut param1: ::core::ffi::c_int,
    mut param2: ::core::ffi::c_int,
    mut param3: ::core::ffi::c_int,
    mut param4: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    let mut result: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut base_value: ::core::ffi::c_int = param1 * OCTAL_MASK_2 + param2;
    let mut array_size: ::core::ffi::c_int = 5 as ::core::ffi::c_int;
    let mut data_array: *mut ::core::ffi::c_int = malloc(
        (array_size as size_t).wrapping_mul(::core::mem::size_of::<::core::ffi::c_int>() as size_t),
    ) as *mut ::core::ffi::c_int;
    if data_array.is_null() {
        return -(1 as ::core::ffi::c_int);
    }
    let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while i < array_size {
        *data_array.offset(i as isize) = (i + 1 as ::core::ffi::c_int) * OCTAL_BASE + param1;
        i += 1;
    }
    let mut foreach_result: ::core::ffi::c_int = foreach_sum(data_array, array_size);
    let mut last_element: *mut ::core::ffi::c_int = data_array
        .offset(array_size as isize)
        .offset(-(1 as ::core::ffi::c_int as isize));
    let mut reverse_sum: ::core::ffi::c_int = process_array_reverse(last_element, array_size);
    let mut switch_result: ::core::ffi::c_int =
        switch_fallthrough_calculator(param2, param3 % 5 as ::core::ffi::c_int);
    let mut floating_calc: ::core::ffi::c_double = param1 as ::core::ffi::c_double * 3.7f64
        + param2 as ::core::ffi::c_double * 2.3f64
        - param3 as ::core::ffi::c_double * 0.5f64;
    let mut converted: ::core::ffi::c_int = safe_double_to_int(floating_calc);
    let mut alloc_result: ::core::ffi::c_int = allocate_and_compute(
        param4 % 10 as ::core::ffi::c_int + 1 as ::core::ffi::c_int,
        1.5f64,
    );
    result = base_value + foreach_result + reverse_sum + switch_result + converted + alloc_result;
    if param3 > OCTAL_FLAG {
        result |= OCTAL_FLAG;
    }
    free(data_array as *mut ::core::ffi::c_void);
    result &= OCTAL_MASK_1;
    return result;
}
pub const INT_MAX: ::core::ffi::c_int = __INT_MAX__;
pub const INT_MIN: ::core::ffi::c_int = -__INT_MAX__ - 1 as ::core::ffi::c_int;
pub const __INT_MAX__: ::core::ffi::c_int = 2147483647 as ::core::ffi::c_int;
