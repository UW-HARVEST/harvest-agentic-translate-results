use std::ffi::{c_char, c_int};
use std::ptr;

type OperationFunc = extern "C" fn(c_int, c_int) -> c_int;

static mut ACCUMULATOR: c_int = 0;
static mut MULTIPLIER: c_int = 1;
static mut OPERATION_COUNT: c_int = 0;

#[unsafe(no_mangle)]
pub extern "C" fn add_to_accumulator(a: c_int, b: c_int) -> c_int {
    unsafe {
        ACCUMULATOR = ACCUMULATOR.wrapping_add(a.wrapping_add(b));
        OPERATION_COUNT = OPERATION_COUNT.wrapping_add(1);
        ACCUMULATOR
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn multiply_with_multiplier(a: c_int, b: c_int) -> c_int {
    unsafe {
        MULTIPLIER = MULTIPLIER.wrapping_mul(a.wrapping_mul(b));
        OPERATION_COUNT = OPERATION_COUNT.wrapping_add(1);
        MULTIPLIER
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn subtract_from_accumulator(a: c_int, b: c_int) -> c_int {
    unsafe {
        ACCUMULATOR = ACCUMULATOR.wrapping_sub(a.wrapping_sub(b));
        OPERATION_COUNT = OPERATION_COUNT.wrapping_add(1);
        ACCUMULATOR
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn divide_multiplier(_a: c_int, b: c_int) -> c_int {
    unsafe {
        if b != 0 {
            MULTIPLIER /= b;
        }
        OPERATION_COUNT = OPERATION_COUNT.wrapping_add(1);
        MULTIPLIER
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn process_octal_string(dest: *mut c_char, octal_val: c_int) {
    let buffer = format!(
        "Octal: 0{:o}, Decimal: {}",
        octal_val as u32, octal_val
    );
    copy_c_string(dest, buffer.as_bytes());
}

#[unsafe(no_mangle)]
pub extern "C" fn find_and_replace_char(str_: *mut c_char, search_char: c_int) {
    unsafe {
        let len = c_strlen(str_ as *const c_char);
        let target = search_char as u8;

        for index in 0..len {
            let current = *(str_ as *const u8).add(index);
            if current == target {
                *(str_ as *mut u8).add(index) = b'X';
                break;
            }
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn validate_and_normalize(value: c_int) -> c_int {
    let is_nonzero = bool_to_c_int(value != 0);
    let _is_zero = bool_to_c_int(value == 0);

    let lower_threshold = 0o100;
    let upper_threshold = 0o777;

    if is_nonzero != 0 && value > 0 {
        if value < lower_threshold {
            return lower_threshold;
        } else if value > upper_threshold {
            return upper_threshold;
        }
    }

    value
}

const OPERATIONS: [OperationFunc; 4] = [
    add_to_accumulator,
    multiply_with_multiplier,
    subtract_from_accumulator,
    divide_multiplier,
];

#[unsafe(no_mangle)]
pub extern "C" fn findrep(
    param1: c_int,
    param2: c_int,
    param3: c_int,
    param4: c_int,
) -> c_int {
    let mut result: c_int = 0;

    let p1_valid = bool_to_c_int(param1 != 0);
    let p2_valid = bool_to_c_int(param2 != 0);
    let p3_valid = bool_to_c_int(param3 != 0);
    let p4_valid = bool_to_c_int(param4 != 0);

    let active_params = p1_valid
        .wrapping_add(p2_valid)
        .wrapping_add(p3_valid)
        .wrapping_add(p4_valid);

    let mode_add = 0o1;
    let mode_multiply = 0o2;
    let _mode_subtract = 0o3;
    let _mode_divide = 0o4;

    let normalized_p1 = validate_and_normalize(param1);
    let normalized_p2 = validate_and_normalize(param2);
    let normalized_p3 = validate_and_normalize(param3);
    let normalized_p4 = validate_and_normalize(param4);

    let mut message = [0 as c_char; 100];
    let mut search_buffer = [0 as c_char; 100];

    process_octal_string(message.as_mut_ptr(), 0o123);
    copy_c_string(
        search_buffer.as_mut_ptr(),
        b"Function pointer example with static vars",
    );

    unsafe {
        let len = c_strlen(search_buffer.as_ptr());
        for index in 0..len {
            if *(search_buffer.as_ptr() as *const u8).add(index) == b'p' {
                result = result.wrapping_add(index as c_int);
                break;
            }
        }
    }

    if active_params >= mode_add {
        let selected_op = OPERATIONS[0];
        result = result.wrapping_add(selected_op(normalized_p1, normalized_p2));
    }

    if active_params >= mode_multiply {
        let selected_op = OPERATIONS[1];
        result = result.wrapping_add(selected_op(normalized_p3, normalized_p4));
    }

    unsafe {
        if ACCUMULATOR > 0o150 {
            let selected_op = OPERATIONS[2];
            let subtract_result = selected_op(normalized_p1, normalized_p3);
            result = result.wrapping_add(subtract_result);
        }
    }

    find_and_replace_char(message.as_mut_ptr(), b'O' as c_int);

    let mut final_message = [0 as c_char; 100];
    copy_c_string_from_ptr(final_message.as_mut_ptr(), message.as_ptr());

    unsafe {
        let has_accumulator = bool_to_c_int(ACCUMULATOR != 0);
        let has_multiplier = bool_to_c_int(MULTIPLIER != 0);
        let both_active = has_accumulator != 0 && has_multiplier != 0;

        if both_active {
            result = result
                .wrapping_add(ACCUMULATOR)
                .wrapping_add(MULTIPLIER);
        }

        if MULTIPLIER > 0o100 {
            let selected_op = OPERATIONS[3];
            selected_op(MULTIPLIER, 2);
        }

        result = result.wrapping_add(OPERATION_COUNT.wrapping_mul(0o10));
    }

    let result_exists = bool_to_c_int(result != 0);
    if result_exists == 0 {
        result = 0o777;
    }

    result
}

fn bool_to_c_int(value: bool) -> c_int {
    if value { 1 } else { 0 }
}

fn copy_c_string(dest: *mut c_char, bytes: &[u8]) {
    unsafe {
        ptr::copy_nonoverlapping(bytes.as_ptr(), dest as *mut u8, bytes.len());
        *(dest as *mut u8).add(bytes.len()) = 0;
    }
}

fn copy_c_string_from_ptr(dest: *mut c_char, src: *const c_char) {
    unsafe {
        let len = c_strlen(src);
        ptr::copy_nonoverlapping(src as *const u8, dest as *mut u8, len + 1);
    }
}

unsafe fn c_strlen(ptr_: *const c_char) -> usize {
    let mut len = 0;

    while unsafe { *(ptr_ as *const u8).add(len) } != 0 {
        len += 1;
    }

    len
}
