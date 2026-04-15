use std::os::raw::c_char;

type OperationFunc = unsafe fn(i32, i32) -> i32;

static mut ACCUMULATOR: i32 = 0;
static mut MULTIPLIER: i32 = 1;
static mut OPERATION_COUNT: i32 = 0;

unsafe fn add_to_accumulator(a: i32, b: i32) -> i32 {
    ACCUMULATOR += a + b;
    OPERATION_COUNT += 1;
    ACCUMULATOR
}

unsafe fn multiply_with_multiplier(a: i32, b: i32) -> i32 {
    MULTIPLIER *= a * b;
    OPERATION_COUNT += 1;
    MULTIPLIER
}

unsafe fn subtract_from_accumulator(a: i32, b: i32) -> i32 {
    ACCUMULATOR -= a - b;
    OPERATION_COUNT += 1;
    ACCUMULATOR
}

unsafe fn divide_multiplier(a: i32, b: i32) -> i32 {
    if b != 0 {
        MULTIPLIER /= b;
    }
    OPERATION_COUNT += 1;
    MULTIPLIER
}

unsafe fn process_octal_string(dest: *mut c_char, octal_val: i32) {
    let s = format!("Octal: 0{:o}, Decimal: {}\0", octal_val, octal_val);
    std::ptr::copy_nonoverlapping(s.as_ptr() as *const c_char, dest, s.len());
}

unsafe fn find_and_replace_char(str: *mut c_char, search_char: i32) {
    let mut current = str;
    while *current != 0 {
        if *current == search_char as c_char {
            *current = b'X' as c_char;
            break;
        }
        current = current.add(1);
    }
}

fn validate_and_normalize(value: i32) -> i32 {
    let is_nonzero = value != 0;
    let _is_zero = !is_nonzero;

    let lower_threshold = 0o100;
    let upper_threshold = 0o777;

    if is_nonzero && value > 0 {
        if value < lower_threshold {
            return lower_threshold;
        } else if value > upper_threshold {
            return upper_threshold;
        }
    }

    value
}

static OPERATIONS: [OperationFunc; 4] = [
    add_to_accumulator,
    multiply_with_multiplier,
    subtract_from_accumulator,
    divide_multiplier,
];

#[no_mangle]
pub extern "C" fn findrep(param1: i32, param2: i32, param3: i32, param4: i32) -> i32 {
    unsafe {
        let mut result = 0;

        let p1_valid = if param1 != 0 { 1 } else { 0 };
        let p2_valid = if param2 != 0 { 1 } else { 0 };
        let p3_valid = if param3 != 0 { 1 } else { 0 };
        let p4_valid = if param4 != 0 { 1 } else { 0 };

        let active_params = p1_valid + p2_valid + p3_valid + p4_valid;

        let mode_add = 0o1;
        let mode_multiply = 0o2;
        let _mode_subtract = 0o3;
        let _mode_divide = 0o4;

        let normalized_p1 = validate_and_normalize(param1);
        let normalized_p2 = validate_and_normalize(param2);
        let normalized_p3 = validate_and_normalize(param3);
        let normalized_p4 = validate_and_normalize(param4);

        let mut message: [c_char; 100] = [0; 100];
        let mut search_buffer: [c_char; 100] = [0; 100];

        process_octal_string(message.as_mut_ptr(), 0o123);

        let search_str = b"Function pointer example with static vars\0";
        std::ptr::copy_nonoverlapping(
            search_str.as_ptr() as *const c_char,
            search_buffer.as_mut_ptr(),
            search_str.len(),
        );

        let mut found_idx = -1;
        for i in 0..100 {
            if search_buffer[i] == 0 {
                break;
            }
            if search_buffer[i] == b'p' as c_char {
                found_idx = i as i32;
                break;
            }
        }
        if found_idx != -1 {
            result += found_idx;
        }

        if active_params >= mode_add {
            let selected_op = OPERATIONS[0];
            result += selected_op(normalized_p1, normalized_p2);
        }

        if active_params >= mode_multiply {
            let selected_op = OPERATIONS[1];
            result += selected_op(normalized_p3, normalized_p4);
        }

        if ACCUMULATOR > 0o150 {
            let selected_op = OPERATIONS[2];
            let subtract_result = selected_op(normalized_p1, normalized_p3);
            result += subtract_result;
        }

        find_and_replace_char(message.as_mut_ptr(), b'O' as i32);

        let mut final_message: [c_char; 100] = [0; 100];
        let mut i = 0;
        while message[i] != 0 && i < 99 {
            final_message[i] = message[i];
            i += 1;
        }
        final_message[i] = 0;

        let has_accumulator = ACCUMULATOR != 0;
        let has_multiplier = MULTIPLIER != 0;
        let both_active = has_accumulator && has_multiplier;

        if both_active {
            result += ACCUMULATOR + MULTIPLIER;
        }

        if MULTIPLIER > 0o100 {
            let selected_op = OPERATIONS[3];
            selected_op(MULTIPLIER, 2);
        }

        result += OPERATION_COUNT * 0o10;

        let result_exists = result != 0;
        if !result_exists {
            result = 0o777;
        }

        result
    }
}
