use std::ffi::{c_char, c_int, c_void};

type OperationFn = extern "C" fn(c_int, c_int, *mut c_void) -> c_int;

unsafe extern "C" {
    fn puts(s: *const c_char) -> c_int;
}

struct ProcessorState {
    results: Vec<c_int>,
    capacity: usize,
    count: usize,
    operation: OperationFn,
    status: c_char,
}

fn log_msg(level: &str, msg: &str) {
    let line: &'static [u8] = match (level, msg) {
        ("INFO", "Starting gotomach function") => b"[INFO] Starting gotomach function\0",
        ("ERROR", "Invalid iteration count") => b"[ERROR] Invalid iteration count\0",
        ("ERROR", "Invalid seed value") => b"[ERROR] Invalid seed value\0",
        ("WARNING", "Invalid mode, using default") => b"[WARNING] Invalid mode, using default\0",
        ("ERROR", "Failed to initialize processor") => {
            b"[ERROR] Failed to initialize processor\0"
        }
        ("ERROR", "Failed to allocate temporary buffer") => {
            b"[ERROR] Failed to allocate temporary buffer\0"
        }
        ("ERROR", "Invalid state status") => b"[ERROR] Invalid state status\0",
        ("ERROR", "State became invalid during processing") => {
            b"[ERROR] State became invalid during processing\0"
        }
        ("WARNING", "Reached maximum count") => b"[WARNING] Reached maximum count\0",
        ("INFO", "Processing completed successfully") => {
            b"[INFO] Processing completed successfully\0"
        }
        _ => b"\0",
    };

    unsafe {
        puts(line.as_ptr().cast());
    }
}

fn is_valid_state(state: &ProcessorState) -> bool {
    if state.status != 0 {
        return state.count < state.capacity;
    }
    false
}

fn check_char_flag(flag: c_char) -> bool {
    flag != 0
}

#[unsafe(no_mangle)]
pub extern "C" fn process_value(
    value: c_int,
    _unused_param: c_int,
    _unused_context: *mut c_void,
) -> c_int {
    value.wrapping_add(10)
}

#[unsafe(no_mangle)]
pub extern "C" fn double_value(
    value: c_int,
    _unused_param: c_int,
    _unused_context: *mut c_void,
) -> c_int {
    value.wrapping_mul(2)
}

#[unsafe(no_mangle)]
pub extern "C" fn triple_value(
    value: c_int,
    _unused_param: c_int,
    _unused_context: *mut c_void,
) -> c_int {
    value.wrapping_mul(3)
}

fn init_processor(capacity: usize, op: OperationFn) -> Option<ProcessorState> {
    let mut results = Vec::new();
    if results.try_reserve_exact(capacity).is_err() {
        return None;
    }

    Some(ProcessorState {
        results,
        capacity,
        count: 0,
        operation: op,
        status: 1,
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn gotomach(
    iterations: c_int,
    seed: c_int,
    mode: c_int,
    threshold: c_int,
) -> c_int {
    log_msg("INFO", "Starting gotomach function");

    if iterations < 0 || iterations > u16::MAX as c_int {
        log_msg("ERROR", "Invalid iteration count");
        return -1;
    }

    if seed < 0 || seed > u16::MAX as c_int {
        log_msg("ERROR", "Invalid seed value");
        return -2;
    }

    let selected_op: OperationFn = match mode {
        0 => process_value,
        1 => double_value,
        2 => triple_value,
        _ => {
            log_msg("WARNING", "Invalid mode, using default");
            process_value
        }
    };

    let iterations_usize = iterations as usize;
    let mut state = match init_processor(iterations_usize, selected_op) {
        Some(state) => state,
        None => {
            log_msg("ERROR", "Failed to initialize processor");
            return -3;
        }
    };

    let mut temp_buffer = Vec::new();
    if temp_buffer.try_reserve_exact(iterations_usize).is_err() {
        log_msg("ERROR", "Failed to allocate temporary buffer");
        return -4;
    }
    temp_buffer.resize(iterations_usize, 0);

    if !check_char_flag(state.status) {
        log_msg("ERROR", "Invalid state status");
        return -5;
    }

    let mut current_value = seed;
    for i in 0..iterations_usize {
        if !is_valid_state(&state) {
            log_msg("ERROR", "State became invalid during processing");
            return -6;
        }

        temp_buffer[i] = (state.operation)(current_value, 0, std::ptr::null_mut());

        if temp_buffer[i] < threshold {
            state.results.push(temp_buffer[i]);
            state.count += 1;
        }

        current_value = temp_buffer[i] % 1000;

        if state.count >= u16::MAX as usize {
            log_msg("WARNING", "Reached maximum count");
            break;
        }
    }

    let mut result: c_int = 0;
    for value in state.results.iter().take(state.count) {
        result = result.wrapping_add(*value);
    }

    log_msg("INFO", "Processing completed successfully");

    result
}
