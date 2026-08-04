use std::ffi::{c_char, c_int};
use std::os::raw::c_int as RawCInt;
use std::sync::Mutex;

static ACCUMULATOR: Mutex<i32> = Mutex::new(0);
static MULTIPLIER: Mutex<i32> = Mutex::new(1);
static OPERATION_COUNT: Mutex<i32> = Mutex::new(0);

type OperationFunc = fn(i32, i32) -> i32;

fn add_to_accumulator(a: i32, b: i32) -> i32 {
    let mut acc = ACCUMULATOR.lock().unwrap();
    *acc += a + b;
    let mut count = OPERATION_COUNT.lock().unwrap();
    *count += 1;
    *acc
}

fn multiply_with_multiplier(a: i32, b: i32) -> i32 {
    let mut mul = MULTIPLIER.lock().unwrap();
    *mul *= a * b;
    let mut count = OPERATION_COUNT.lock().unwrap();
    *count += 1;
    *mul
}

fn subtract_from_accumulator(a: i32, b: i32) -> i32 {
    let mut acc = ACCUMULATOR.lock().unwrap();
    *acc -= a - b;
    let mut count = OPERATION_COUNT.lock().unwrap();
    *count += 1;
    *acc
}

fn divide_multiplier(a: i32, b: i32) -> i32 {
    let mut mul = MULTIPLIER.lock().unwrap();
    if b != 0 {
        *mul /= b;
    }
    let mut count = OPERATION_COUNT.lock().unwrap();
    *count += 1;
    *mul
}

fn process_octal_string(dest: &mut [u8], octal_val: i32) {
    let s = format!("Octal: 0{:o}, Decimal: {}", octal_val, octal_val);
    let bytes = s.as_bytes();
    let len = bytes.len().min(dest.len() - 1);
    dest[..len].copy_from_slice(&bytes[..len]);
    dest[len] = 0;
}

fn find_and_replace_char(str: &mut [u8], search_char: u8) {
    for i in 0..str.len() {
        if str[i] == 0 {
            break;
        }
        if str[i] == search_char {
            str[i] = b'X';
            break;
        }
    }
}

fn validate_and_normalize(value: i32) -> i32 {
    let is_nonzero = value != 0;
    let _is_zero = value == 0;

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

#[unsafe(no_mangle)]
pub extern "C" fn findrep(param1: RawCInt, param2: RawCInt, param3: RawCInt, param4: RawCInt) -> RawCInt {
    let param1 = param1 as i32;
    let param2 = param2 as i32;
    let param3 = param3 as i32;
    let param4 = param4 as i32;

    let mut result: i32 = 0;

    let p1_valid = (param1 != 0) as i32;
    let p2_valid = (param2 != 0) as i32;
    let p3_valid = (param3 != 0) as i32;
    let p4_valid = (param4 != 0) as i32;

    let active_params = p1_valid + p2_valid + p3_valid + p4_valid;

    let mode_add = 0o1;
    let mode_multiply = 0o2;
    let _mode_subtract = 0o3;
    let _mode_divide = 0o4;

    let normalized_p1 = validate_and_normalize(param1);
    let normalized_p2 = validate_and_normalize(param2);
    let normalized_p3 = validate_and_normalize(param3);
    let normalized_p4 = validate_and_normalize(param4);

    let mut message = [0u8; 100];
    let mut search_buffer = [0u8; 100];

    process_octal_string(&mut message, 0o123);
    let s = b"Function pointer example with static vars";
    let len = s.len().min(search_buffer.len() - 1);
    search_buffer[..len].copy_from_slice(&s[..len]);
    search_buffer[len] = 0;

    let search_str = std::str::from_utf8(&search_buffer[..len]).unwrap_or("");
    if let Some(pos) = search_str.find('p') {
        result += pos as i32;
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

    {
        let acc = ACCUMULATOR.lock().unwrap();
        if *acc > 0o150 {
            selected_op = OPERATIONS[2];
            let subtract_result = selected_op(normalized_p1, normalized_p3);
            result += subtract_result;
        }
    }

    find_and_replace_char(&mut message, b'O');

    let mut final_message = [0u8; 100];
    final_message.copy_from_slice(&message);

    let has_accumulator = {
        let acc = ACCUMULATOR.lock().unwrap();
        *acc != 0
    };
    let has_multiplier = {
        let mul = MULTIPLIER.lock().unwrap();
        *mul != 0
    };
    let both_active = has_accumulator && has_multiplier;

    if both_active {
        let acc = ACCUMULATOR.lock().unwrap();
        let mul = MULTIPLIER.lock().unwrap();
        result += *acc + *mul;
    }

    {
        let mul = MULTIPLIER.lock().unwrap();
        if *mul > 0o100 {
            selected_op = OPERATIONS[3];
            selected_op(*mul, 2);
        }
    }

    {
        let count = OPERATION_COUNT.lock().unwrap();
        result += *count * 0o10;
    }

    let result_exists = result != 0;
    if !result_exists {
        result = 0o777;
    }

    result as RawCInt
}
