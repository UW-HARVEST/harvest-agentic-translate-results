use std::os::raw::c_int;

type OperationFn = fn(c_int, c_int, *mut ()) -> c_int;

struct ProcessorState {
    results: Vec<c_int>,
    capacity: usize,
    count: usize,
    operation: OperationFn,
    status: i8,
}

fn is_valid_state(state: &ProcessorState) -> bool {
    if state.status != 0 {
        state.count < state.capacity
    } else {
        false
    }
}

fn check_char_flag(flag: i8) -> bool {
    flag != 0
}

fn process_value(value: c_int, _unused_param: c_int, _unused_context: *mut ()) -> c_int {
    value + 10
}

fn double_value(value: c_int, _unused_param: c_int, _unused_context: *mut ()) -> c_int {
    value * 2
}

fn triple_value(value: c_int, _unused_param: c_int, _unused_context: *mut ()) -> c_int {
    value * 3
}

fn init_processor(capacity: usize, op: OperationFn) -> Option<ProcessorState> {
    Some(ProcessorState {
        results: vec![0; capacity],
        capacity,
        count: 0,
        operation: op,
        status: 1,
    })
}

macro_rules! log_msg {
    ($level:ident, $msg:expr) => {
        print!(concat!("[", stringify!($level), "] ", $msg, "\n"));
    };
}

#[unsafe(no_mangle)]
pub extern "C" fn gotomach(iterations: c_int, seed: c_int, mode: c_int, threshold: c_int) -> c_int {
    let mut result: c_int;

    log_msg!(INFO, "Starting gotomach function");

    if iterations < 0 || iterations > 65535 {
        log_msg!(ERROR, "Invalid iteration count");
        return -1;
    }

    if seed < 0 || seed > 65535 {
        log_msg!(ERROR, "Invalid seed value");
        return -2;
    }

    let selected_op: OperationFn = match mode {
        0 => process_value,
        1 => double_value,
        2 => triple_value,
        _ => {
            log_msg!(WARNING, "Invalid mode, using default");
            process_value
        }
    };

    let mut state = match init_processor(iterations as usize, selected_op) {
        Some(s) => s,
        None => {
            log_msg!(ERROR, "Failed to initialize processor");
            return -3;
        }
    };

    let mut temp_buffer = vec![0i32; iterations as usize];

    if !check_char_flag(state.status) {
        log_msg!(ERROR, "Invalid state status");
        return -5;
    }

    let mut current_value = seed;
    for i in 0..iterations as usize {
        if !is_valid_state(&state) {
            log_msg!(ERROR, "State became invalid during processing");
            return -6;
        }

        temp_buffer[i] = (state.operation)(current_value, 0, std::ptr::null_mut());

        if temp_buffer[i] < threshold {
            state.results[state.count] = temp_buffer[i];
            state.count += 1;
        }

        current_value = temp_buffer[i] % 1000;

        if state.count >= 65535 {
            log_msg!(WARNING, "Reached maximum count");
            break;
        }
    }

    result = 0;
    for i in 0..state.count {
        result += state.results[i];
    }

    log_msg!(INFO, "Processing completed successfully");

    result
}
