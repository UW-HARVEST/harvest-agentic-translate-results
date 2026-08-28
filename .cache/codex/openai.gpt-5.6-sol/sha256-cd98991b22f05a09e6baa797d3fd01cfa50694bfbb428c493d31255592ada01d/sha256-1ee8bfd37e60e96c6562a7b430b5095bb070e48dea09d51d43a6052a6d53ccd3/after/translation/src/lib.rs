use std::ffi::{c_double, c_int, c_void};

const OCTAL_MASK_1: c_int = 0o777;
const OCTAL_MASK_2: c_int = 0o100;
const OCTAL_FLAG: c_int = 0o200;
const OCTAL_BASE: c_int = 0o10;

#[repr(C)]
struct DataPoint {
    value: c_int,
    coefficient: c_double,
}

unsafe extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
}

#[used]
static MALLOC: unsafe extern "C" fn(usize) -> *mut c_void = malloc;

#[inline(never)]
unsafe fn call_malloc(size: usize) -> *mut c_void {
    let allocator = unsafe { std::ptr::read_volatile(&raw const MALLOC) };
    unsafe { allocator(size) }
}

#[unsafe(no_mangle)]
pub extern "C" fn safe_double_to_int(d: c_double) -> c_int {
    if d.is_nan() {
        return 0;
    }

    if d.is_infinite() {
        return if d > 0.0 { c_int::MAX } else { c_int::MIN };
    }

    if d >= c_int::MAX as c_double {
        return c_int::MAX;
    }
    if d <= c_int::MIN as c_double {
        return c_int::MIN;
    }

    d as c_int
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn process_array_reverse(end: *mut c_int, count: c_int) -> c_int {
    let mut sum: c_int = 0;
    let mut ptr = end;

    for _ in 0..count {
        // SAFETY: As in the C API, the caller must provide `count` readable
        // elements ending at `end`.
        sum = sum.wrapping_add(unsafe { *ptr });
        ptr = ptr.wrapping_sub(1);
    }

    sum
}

#[unsafe(no_mangle)]
pub extern "C" fn switch_fallthrough_calculator(value: c_int, operation: c_int) -> c_int {
    match operation {
        0 => value.wrapping_mul(OCTAL_BASE).wrapping_add(OCTAL_FLAG) & OCTAL_MASK_1,
        1 => value.wrapping_add(OCTAL_FLAG) & OCTAL_MASK_1,
        2 => value & OCTAL_MASK_1,
        3 => value.wrapping_mul(3).wrapping_add(OCTAL_MASK_2),
        4 => value.wrapping_add(OCTAL_MASK_2),
        _ => 0,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn allocate_and_compute(size: c_int, multiplier: c_double) -> c_int {
    let allocation_size = (size as usize).wrapping_mul(size_of::<DataPoint>());
    // SAFETY: Calling the C allocator preserves the source function's handling
    // of zero, negative, and oversized allocation requests.
    let points = unsafe { malloc(allocation_size) }.cast::<DataPoint>();

    if points.is_null() {
        return -1;
    }

    for i in 0..size {
        // SAFETY: A successful allocation has room for `size` entries whenever
        // the source C program has defined behavior.
        let point = unsafe { points.add(i as usize) };
        unsafe {
            (*point).value = i.wrapping_mul(OCTAL_BASE);
            (*point).coefficient = i as c_double * multiplier;
        }
    }

    let mut sum = 0.0;
    for i in 0..size {
        let point = unsafe { points.add(i as usize) };
        unsafe {
            sum += (*point).value as c_double * (*point).coefficient;
        }
    }

    let result = safe_double_to_int(sum);
    unsafe { free(points.cast::<c_void>()) };
    result
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn foreach_sum(array: *mut c_int, count: c_int) -> c_int {
    let mut total: c_int = 0;

    for i in 0..count {
        // SAFETY: As in the C API, the caller must provide `count` readable
        // elements at `array`.
        total = total.wrapping_add(unsafe { *array.add(i as usize) });
    }

    total
}

#[unsafe(no_mangle)]
pub extern "C" fn fallcalc(param1: c_int, param2: c_int, param3: c_int, param4: c_int) -> c_int {
    let base_value = param1.wrapping_mul(OCTAL_MASK_2).wrapping_add(param2);

    let array_size: c_int = 5;
    let allocation_size = (array_size as usize).wrapping_mul(size_of::<c_int>());
    let data_array = unsafe { call_malloc(allocation_size) }.cast::<c_int>();

    if data_array.is_null() {
        return -1;
    }

    for i in 0..array_size {
        unsafe {
            *data_array.add(i as usize) = i
                .wrapping_add(1)
                .wrapping_mul(OCTAL_BASE)
                .wrapping_add(param1);
        }
    }

    let foreach_result = unsafe { foreach_sum(data_array, array_size) };
    let last_element = unsafe { data_array.add(array_size as usize - 1) };
    let reverse_sum = unsafe { process_array_reverse(last_element, array_size) };
    let switch_result = switch_fallthrough_calculator(param2, param3 % 5);

    let floating_calc =
        param1 as c_double * 3.7 + param2 as c_double * 2.3 - param3 as c_double * 0.5;
    let converted = safe_double_to_int(floating_calc);
    let alloc_result = unsafe { allocate_and_compute(param4 % 10 + 1, 1.5) };

    let mut result = base_value
        .wrapping_add(foreach_result)
        .wrapping_add(reverse_sum)
        .wrapping_add(switch_result)
        .wrapping_add(converted)
        .wrapping_add(alloc_result);

    if param3 > OCTAL_FLAG {
        result |= OCTAL_FLAG;
    }

    unsafe { free(data_array.cast::<c_void>()) };
    result & OCTAL_MASK_1
}
