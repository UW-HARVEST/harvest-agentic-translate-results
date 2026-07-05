










pub type operation_func = Option<
    unsafe extern "C" fn(
        ::core::ffi::c_int,
        ::core::ffi::c_int,
        ::core::ffi::c_int,
        ::core::ffi::c_int,
    ) -> ::core::ffi::c_int,
>;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct Result_0 {
    pub value: ::core::ffi::c_int,
    pub scaled: ::core::ffi::c_double,
    pub rank: ::core::ffi::c_int,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct ResultArray {
    pub data: [Result_0; 10],
    pub count: ::core::ffi::c_int,
}
pub const INT32_MIN: ::core::ffi::c_int =
    -(2147483647 as ::core::ffi::c_int) - 1 as ::core::ffi::c_int;
pub const INT32_MAX: ::core::ffi::c_int = 2147483647 as ::core::ffi::c_int;
#[no_mangle]
pub unsafe extern "C" fn add_operation(
    a: ::core::ffi::c_int,
    b: ::core::ffi::c_int,
    _unused1: ::core::ffi::c_int,
    _unused2: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    a + b
}

#[no_mangle]
pub extern "C" fn multiply_operation(a: i32, b: i32, _unused1: i32, _unused2: i32) -> i32 {
    a * b
}

#[no_mangle]
pub unsafe extern "C" fn subtract_operation(
    a: ::core::ffi::c_int,
    b: ::core::ffi::c_int,
    _unused1: ::core::ffi::c_int,
    _unused2: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    a - b
}

#[no_mangle]
pub extern "C" fn modulo_operation(a: i32, b: i32, _unused1: i32, _unused2: i32) -> i32 {
    if b == 0 {
        0
    } else {
        a % b
    }
}

#[no_mangle]
pub fn safe_double_to_int(d: f64) -> i32 {
    if d.is_nan() {
        0
    } else if d >= i32::MAX as f64 {
        i32::MAX
    } else if d <= i32::MIN as f64 {
        i32::MIN
    } else {
        d as i32
    }
}

#[no_mangle]
pub fn compute_scaled_value(base: i32, scale_factor: f64) -> i32 {
    let scaled = base as f64 * scale_factor;
    safe_double_to_int(scaled)
}

#[no_mangle]
pub fn compare_results_in_array(
    arr: &ResultArray,
    idx1: i32,
    idx2: i32,
) -> i32 {
    let idx1 = match usize::try_from(idx1) {
        Ok(i) => i,
        Err(_) => return 0,
    };
    let idx2 = match usize::try_from(idx2) {
        Ok(i) => i,
        Err(_) => return 0,
    };

    let count = match usize::try_from(arr.count) {
        Ok(c) => c,
        Err(_) => return 0,
    };

    if idx1 >= count || idx2 >= count {
        return 0;
    }

    if idx1 < idx2 {
        -1
    } else if idx1 > idx2 {
        1
    } else {
        0
    }
}

#[no_mangle]
pub fn init_result_array(arr: &mut ResultArray, values: &[i32], count: i32) {
    let limited_count = count.min(10).max(0) as usize;
    let actual_count = limited_count.min(values.len());

    arr.count = actual_count as i32;

    for (i, value) in values.iter().copied().take(actual_count).enumerate() {
        arr.data[i] = Result_0 {
            value,
            scaled: value as f64 * 1.5,
            rank: i as i32,
        };
    }
}

#[no_mangle]
pub fn process_with_foreach(arr: &mut ResultArray, op: operation_func) -> i32 {
    let mut total = 0;
    let mut keep = 1;
    let mut count_iter = 0;
    let size = arr.count;

    while keep != 0 && count_iter != size {
        let item = &mut arr.data[count_iter as usize];

        while keep != 0 {
            let result = unsafe {
                op.expect("non-null function pointer")(item.value, item.rank, 0, 0)
            };
            total += result;

            let temp = result as f64 * 0.75;
            item.scaled = temp;
            item.value = safe_double_to_int(temp);

            keep = 0;
        }

        keep = 1;
        count_iter += 1;
    }

    total
}

#[no_mangle]
pub fn compute_weighted_sum(arr: &ResultArray) -> i32 {
    let mut sum: i32 = 0;
    let count = arr.count as usize;
    let data = &arr.data[..count];

    for (i, current) in data.iter().enumerate() {
        let weight = if i > 0 { i as i32 } else { 1 };
        let weighted = current.value as f64 * weight as f64 * 0.8f64;
        sum += safe_double_to_int(weighted);
    }

    sum
}

#[no_mangle]
pub fn arrayfunc(
    param1: i32,
    param2: i32,
    param3: i32,
    param4: i32,
) -> i32 {
    let operations = [
        Some(add_operation as unsafe extern "C" fn(i32, i32, i32, i32) -> i32),
        Some(multiply_operation as unsafe extern "C" fn(i32, i32, i32, i32) -> i32),
        Some(subtract_operation as unsafe extern "C" fn(i32, i32, i32, i32) -> i32),
        Some(modulo_operation as unsafe extern "C" fn(i32, i32, i32, i32) -> i32),
    ];

    let values = [
        param1,
        param2,
        param3,
        param4,
        param1 + param2,
        param2 - param3,
        param3 * 2,
        param4 / 2 + 1,
    ];

    let mut arr = ResultArray {
        data: [Result_0 {
            value: 0,
            scaled: 0.0,
            rank: 0,
        }; 10],
        count: 0,
    };

    init_result_array(&mut arr, &values, 8);

    let mut result = 0;

    for op in operations {
        result += unsafe { process_with_foreach(&mut arr, op) };
    }

    result += compute_weighted_sum(&mut arr);

    for i in 0..(arr.count - 1) {
        result += unsafe { compare_results_in_array(&mut arr, i, i + 1) };
    }

    let final_scale = result as f64 * 0.333f64;
    safe_double_to_int(final_scale)
}

