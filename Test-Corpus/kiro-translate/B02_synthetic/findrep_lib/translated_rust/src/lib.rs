use std::ffi::c_int;

static mut ACCUMULATOR: c_int = 0;
static mut MULTIPLIER: c_int = 1;
static mut OPERATION_COUNT: c_int = 0;

type OperationFunc = fn(c_int, c_int) -> c_int;

fn add_to_accumulator(a: c_int, b: c_int) -> c_int {
    unsafe {
        ACCUMULATOR += a + b;
        OPERATION_COUNT += 1;
        ACCUMULATOR
    }
}

fn multiply_with_multiplier(a: c_int, b: c_int) -> c_int {
    unsafe {
        MULTIPLIER *= a * b;
        OPERATION_COUNT += 1;
        MULTIPLIER
    }
}

fn subtract_from_accumulator(a: c_int, b: c_int) -> c_int {
    unsafe {
        ACCUMULATOR -= a - b;
        OPERATION_COUNT += 1;
        ACCUMULATOR
    }
}

fn divide_multiplier(_a: c_int, b: c_int) -> c_int {
    unsafe {
        if b != 0 {
            MULTIPLIER /= b;
        }
        OPERATION_COUNT += 1;
        MULTIPLIER
    }
}

fn process_octal_string(dest: &mut [u8], octal_val: c_int) {
    let s = format!("Octal: 0{:o}, Decimal: {}", octal_val, octal_val);
    let bytes = s.as_bytes();
    dest[..bytes.len()].copy_from_slice(bytes);
    dest[bytes.len()] = 0;
}

fn find_and_replace_char(str_buf: &mut [u8], search_char: u8) {
    if let Some(pos) = str_buf.iter().position(|&b| b == 0) {
        if let Some(found) = str_buf[..pos].iter().position(|&b| b == search_char) {
            str_buf[found] = b'X';
        }
    }
}

fn validate_and_normalize(value: c_int) -> c_int {
    let is_nonzero = if value != 0 { 1 } else { 0 };
    let lower_threshold: c_int = 0o100; // 64
    let upper_threshold: c_int = 0o777; // 511

    if is_nonzero != 0 && value > 0 {
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

#[unsafe(no_mangle)]
pub extern "C" fn findrep(param1: c_int, param2: c_int, param3: c_int, param4: c_int) -> c_int {
    let mut result: c_int = 0;

    let p1_valid = if param1 != 0 { 1 } else { 0 };
    let p2_valid = if param2 != 0 { 1 } else { 0 };
    let p3_valid = if param3 != 0 { 1 } else { 0 };
    let p4_valid = if param4 != 0 { 1 } else { 0 };

    let active_params = p1_valid + p2_valid + p3_valid + p4_valid;

    let mode_add: c_int = 0o1;
    let mode_multiply: c_int = 0o2;

    let normalized_p1 = validate_and_normalize(param1);
    let normalized_p2 = validate_and_normalize(param2);
    let normalized_p3 = validate_and_normalize(param3);
    let normalized_p4 = validate_and_normalize(param4);

    let mut message = [0u8; 100];
    let search_buffer = b"Function pointer example with static vars\0";

    process_octal_string(&mut message, 0o123);

    // memchr(search_buffer, 'p', strlen(search_buffer))
    let search_len = search_buffer.iter().position(|&b| b == 0).unwrap_or(search_buffer.len());
    if let Some(pos) = search_buffer[..search_len].iter().position(|&b| b == b'p') {
        result += pos as c_int;
    }

    let mut selected_op: OperationFunc;

    if active_params >= mode_add {
        selected_op = OPERATIONS[0];
        result += selected_op(normalized_p1, normalized_p2);
    }

    if active_params >= mode_multiply {
        selected_op = OPERATIONS[1];
        result += selected_op(normalized_p3, normalized_p4);
    }

    unsafe {
        if ACCUMULATOR > 0o150 {
            selected_op = OPERATIONS[2];
            let subtract_result = selected_op(normalized_p1, normalized_p3);
            result += subtract_result;
        }
    }

    find_and_replace_char(&mut message, b'O');

    unsafe {
        let has_accumulator = if ACCUMULATOR != 0 { 1 } else { 0 };
        let has_multiplier = if MULTIPLIER != 0 { 1 } else { 0 };
        let both_active = has_accumulator != 0 && has_multiplier != 0;

        if both_active {
            result += ACCUMULATOR + MULTIPLIER;
        }

        if MULTIPLIER > 0o100 {
            selected_op = OPERATIONS[3];
            selected_op(MULTIPLIER, 2);
        }

        result += OPERATION_COUNT * 0o10;
    }

    let result_exists = result != 0;
    if !result_exists {
        result = 0o777;
    }

    result
}
