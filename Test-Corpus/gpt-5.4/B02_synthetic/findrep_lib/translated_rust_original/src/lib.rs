use std::os::raw::c_int;
use std::sync::Mutex;

type OperationFunc = fn(c_int, c_int) -> c_int;

#[derive(Clone, Copy)]
struct State {
    accumulator: c_int,
    multiplier: c_int,
    operation_count: c_int,
}

static STATE: Mutex<State> = Mutex::new(State {
    accumulator: 0,
    multiplier: 1,
    operation_count: 0,
});

fn add_to_accumulator(a: c_int, b: c_int) -> c_int {
    let mut state = STATE.lock().unwrap();
    state.accumulator += a + b;
    state.operation_count += 1;
    state.accumulator
}

fn multiply_with_multiplier(a: c_int, b: c_int) -> c_int {
    let mut state = STATE.lock().unwrap();
    state.multiplier *= a * b;
    state.operation_count += 1;
    state.multiplier
}

fn subtract_from_accumulator(a: c_int, b: c_int) -> c_int {
    let mut state = STATE.lock().unwrap();
    state.accumulator -= a - b;
    state.operation_count += 1;
    state.accumulator
}

fn divide_multiplier(_a: c_int, b: c_int) -> c_int {
    let mut state = STATE.lock().unwrap();
    if b != 0 {
        state.multiplier /= b;
    }
    state.operation_count += 1;
    state.multiplier
}

fn process_octal_string(octal_val: c_int) -> String {
    format!("Octal: 0{:o}, Decimal: {}", octal_val, octal_val)
}

fn find_and_replace_char(s: &mut String, search_char: u8) {
    if let Some(pos) = s.as_bytes().iter().position(|&b| b == search_char) {
        s.replace_range(pos..pos + 1, "X");
    }
}

fn validate_and_normalize(value: c_int) -> c_int {
    let is_nonzero = value != 0;
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
pub extern "C" fn findrep(param1: c_int, param2: c_int, param3: c_int, param4: c_int) -> c_int {
    let mut result: c_int = 0;

    let p1_valid = (param1 != 0) as c_int;
    let p2_valid = (param2 != 0) as c_int;
    let p3_valid = (param3 != 0) as c_int;
    let p4_valid = (param4 != 0) as c_int;

    let active_params = p1_valid + p2_valid + p3_valid + p4_valid;

    let mode_add = 0o1;
    let mode_multiply = 0o2;

    let normalized_p1 = validate_and_normalize(param1);
    let normalized_p2 = validate_and_normalize(param2);
    let normalized_p3 = validate_and_normalize(param3);
    let normalized_p4 = validate_and_normalize(param4);

    let mut message = process_octal_string(0o123);
    let search_buffer = "Function pointer example with static vars";

    if let Some(pos) = search_buffer.as_bytes().iter().position(|&b| b == b'p') {
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

    let accumulator_now = {
        let state = STATE.lock().unwrap();
        state.accumulator
    };

    if accumulator_now > 0o150 {
        selected_op = OPERATIONS[2];
        let subtract_result = selected_op(normalized_p1, normalized_p3);
        result += subtract_result;
    }

    find_and_replace_char(&mut message, b'O');

    let _final_message = message.clone();

    let (accumulator, multiplier, operation_count) = {
        let state = STATE.lock().unwrap();
        (state.accumulator, state.multiplier, state.operation_count)
    };

    let has_accumulator = accumulator != 0;
    let has_multiplier = multiplier != 0;
    let both_active = has_accumulator && has_multiplier;

    if both_active {
        result += accumulator + multiplier;
    }

    if multiplier > 0o100 {
        selected_op = OPERATIONS[3];
        let _ = selected_op(multiplier, 2);
    }

    let operation_count_now = {
        let state = STATE.lock().unwrap();
        state.operation_count
    };
    result += operation_count_now * 0o10;

    let result_exists = result != 0;
    if !result_exists {
        result = 0o777;
    }

    result
}
