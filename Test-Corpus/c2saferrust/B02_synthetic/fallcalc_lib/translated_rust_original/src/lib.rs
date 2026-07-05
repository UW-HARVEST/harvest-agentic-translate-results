





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
pub fn safe_double_to_int(d: f64) -> i32 {
    if d.is_nan() {
        return 0;
    }
    if d.is_infinite() {
        return if d.is_sign_positive() { INT_MAX } else { INT_MIN };
    }
    if d >= INT_MAX as f64 {
        return INT_MAX;
    }
    if d <= INT_MIN as f64 {
        return INT_MIN;
    }
    d as i32
}

#[no_mangle]
pub fn process_array_reverse(values: &[::core::ffi::c_int]) -> ::core::ffi::c_int {
    values.iter().rev().copied().sum()
}

#[no_mangle]
pub fn switch_fallthrough_calculator(mut value: i32, operation: i32) -> i32 {
    match operation {
        0 => {
            value *= OCTAL_BASE;
            value += OCTAL_FLAG;
            value &= OCTAL_MASK_1;
            value
        }
        1 => {
            value += OCTAL_FLAG;
            value &= OCTAL_MASK_1;
            value
        }
        2 => {
            value &= OCTAL_MASK_1;
            value
        }
        3 => {
            value *= 3;
            value += OCTAL_MASK_2;
            value
        }
        4 => {
            value += OCTAL_MASK_2;
            value
        }
        _ => 0,
    }
}

#[no_mangle]
pub fn allocate_and_compute(
    size: ::core::ffi::c_int,
    multiplier: ::core::ffi::c_double,
) -> ::core::ffi::c_int {
    if size <= 0 {
        return safe_double_to_int(0.0);
    }

    let size_usize = size as usize;
    let mut points = vec![
        DataPoint {
            value: 0,
            coefficient: 0.0,
        };
        size_usize
    ];

    for (i, point) in points.iter_mut().enumerate() {
        let i_c_int = i as ::core::ffi::c_int;
        point.value = i_c_int * OCTAL_BASE;
        point.coefficient = i_c_int as ::core::ffi::c_double * multiplier;
    }

    let sum: ::core::ffi::c_double = points
        .iter()
        .map(|point| point.value as ::core::ffi::c_double * point.coefficient)
        .sum();

    safe_double_to_int(sum)
}

#[no_mangle]
pub fn foreach_sum(array: *mut ::core::ffi::c_int, count: ::core::ffi::c_int) -> ::core::ffi::c_int {
    if array.is_null() || count <= 0 {
        return 0;
    }

    let size = count as usize;
    let slice = unsafe { ::core::slice::from_raw_parts(array as *const ::core::ffi::c_int, size) };
    slice.iter().copied().sum()
}

#[no_mangle]
pub fn fallcalc(
    param1: ::core::ffi::c_int,
    param2: ::core::ffi::c_int,
    param3: ::core::ffi::c_int,
    param4: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    let base_value: ::core::ffi::c_int = param1 * OCTAL_MASK_2 + param2;

    let array_size: ::core::ffi::c_int = 5;
    let mut data_array: Vec<::core::ffi::c_int> = (0..array_size)
        .map(|i| (i + 1) * OCTAL_BASE + param1)
        .collect();

    let foreach_result: ::core::ffi::c_int = foreach_sum(data_array.as_mut_ptr(), array_size);

    let reverse_sum: ::core::ffi::c_int = process_array_reverse(&data_array);

    let switch_result: ::core::ffi::c_int =
        switch_fallthrough_calculator(param2, param3 % 5);

    let floating_calc: ::core::ffi::c_double =
        param1 as ::core::ffi::c_double * 3.7
            + param2 as ::core::ffi::c_double * 2.3
            - param3 as ::core::ffi::c_double * 0.5;

    let converted: ::core::ffi::c_int = safe_double_to_int(floating_calc);

    let alloc_result: ::core::ffi::c_int =
        allocate_and_compute(param4 % 10 + 1, 1.5);

    let mut result: ::core::ffi::c_int =
        base_value + foreach_result + reverse_sum + switch_result + converted + alloc_result;

    if param3 > OCTAL_FLAG {
        result |= OCTAL_FLAG;
    }

    result &= OCTAL_MASK_1;
    result
}

pub const INT_MAX: ::core::ffi::c_int = __INT_MAX__;
pub const INT_MIN: ::core::ffi::c_int = -__INT_MAX__ - 1 as ::core::ffi::c_int;
pub const __INT_MAX__: ::core::ffi::c_int = 2147483647 as ::core::ffi::c_int;
