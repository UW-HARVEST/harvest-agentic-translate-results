use std::os::raw::{c_char, c_int};

macro_rules! log_msg {
    ($level:ident, $msg:expr) => {
        println!("[{}] {}", stringify!($level), $msg);
    };
}

fn process_value(value: i32, _unused_param: i32, _unused_context: *mut std::ffi::c_void) -> i32 {
    value + 10
}

fn double_value(value: i32, _unused_param: i32, _unused_context: *mut std::ffi::c_void) -> i32 {
    value * 2
}

fn triple_value(value: i32, _unused_param: i32, _unused_context: *mut std::ffi::c_void) -> i32 {
    value * 3
}

struct ProcessorState {
    results: Vec<i32>,
    capacity: usize,
    operation: fn(i32, i32, *mut std::ffi::c_void) -> i32,
    status: c_char,
}

impl ProcessorState {
    fn is_valid_state(&self) -> bool {
        if self.status != 0 {
            return self.results.len() < self.capacity;
        }
        false
    }
}

fn check_char_flag(flag: c_char) -> bool {
    flag != 0
}

fn init_processor(
    capacity: usize,
    op: fn(i32, i32, *mut std::ffi::c_void) -> i32,
) -> Option<ProcessorState> {
    let mut results = Vec::new();
    if results.try_reserve(capacity).is_err() {
        return None;
    }
    Some(ProcessorState {
        results,
        capacity,
        operation: op,
        status: 1,
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn gotomach(iterations: c_int, seed: c_int, mode: c_int, threshold: c_int) -> c_int {
    log_msg!(INFO, "Starting gotomach function");

    if iterations < 0 || iterations > u16::MAX as c_int {
        log_msg!(ERROR, "Invalid iteration count");
        return -1;
    }

    if seed < 0 || seed > u16::MAX as c_int {
        log_msg!(ERROR, "Invalid seed value");
        return -2;
    }

    let selected_op: fn(i32, i32, *mut std::ffi::c_void) -> i32 = match mode {
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

    let mut temp_buffer = Vec::new();
    if temp_buffer.try_reserve(iterations as usize).is_err() {
        log_msg!(ERROR, "Failed to allocate temporary buffer");
        return -4;
    }
    temp_buffer.resize(iterations as usize, 0);

    if !check_char_flag(state.status) {
        log_msg!(ERROR, "Invalid state status");
        return -5;
    }

    let mut current_value = seed;
    for i in 0..(iterations as usize) {
        if !state.is_valid_state() {
            log_msg!(ERROR, "State became invalid during processing");
            return -6;
        }

        temp_buffer[i] = (state.operation)(current_value, 0, std::ptr::null_mut());

        if temp_buffer[i] < threshold {
            state.results.push(temp_buffer[i]);
        }

        current_value = temp_buffer[i] % 1000;

        if state.results.len() >= u16::MAX as usize {
            log_msg!(WARNING, "Reached maximum count");
            break;
        }
    }

    let result: i32 = state.results.iter().sum();

    log_msg!(INFO, "Processing completed successfully");

    result
}
