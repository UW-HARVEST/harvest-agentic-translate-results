// Copyright 2025 MIT Lincoln Laboratory
// Translated from C to Rust, preserving exact behavior and output.

use std::ffi::c_int;
use std::os::raw::c_void;

const UINT16_MAX: c_int = 65535;

type OperationFn = fn(c_int, c_int, *mut c_void) -> c_int;

struct ProcessorState {
    results: Vec<c_int>,
    capacity: usize,
    count: usize,
    operation: Option<OperationFn>,
    status: u8,
}

fn is_valid_state(state: &ProcessorState) -> bool {
    if state.status != 0 {
        return state.count < state.capacity;
    }
    false
}

fn check_char_flag(flag: u8) -> bool {
    flag != 0
}

fn process_value(value: c_int, _unused_param: c_int, _unused_context: *mut c_void) -> c_int {
    value + 10
}

fn double_value(value: c_int, _unused_param: c_int, _unused_context: *mut c_void) -> c_int {
    value * 2
}

fn triple_value(value: c_int, _unused_param: c_int, _unused_context: *mut c_void) -> c_int {
    value * 3
}

fn init_processor(capacity: usize, op: OperationFn) -> Option<Box<ProcessorState>> {
    // Mirror the C code's allocation: a vector with `capacity` ints.
    let results = vec![0 as c_int; capacity];
    Some(Box::new(ProcessorState {
        results,
        capacity,
        count: 0,
        operation: Some(op),
        status: 1,
    }))
}

/// Print a log line using libc::printf to ensure byte-identical output to
/// stdout (matching the C implementation's buffering and ordering).
fn log_msg(level: &str, msg: &str) {
    // Construct the formatted line "[LEVEL] MSG\n" and emit via printf("%s", ...)
    // to preserve the C stdout stream behavior.
    let line = format!("[{}] {}\n", level, msg);
    let cstr = std::ffi::CString::new(line).unwrap();
    unsafe {
        libc::printf(b"%s\0".as_ptr() as *const libc::c_char, cstr.as_ptr());
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn gotomach(iterations: c_int, seed: c_int, mode: c_int, threshold: c_int) -> c_int {
    let mut state: Option<Box<ProcessorState>> = None;
    let mut temp_buffer: Option<Vec<c_int>> = None;
    #[allow(unused_assignments)]
    let mut result: c_int = 0;
    let selected_op: OperationFn;

    log_msg("INFO", "Starting gotomach function");

    // Validate iteration count.
    if iterations < 0 || iterations > UINT16_MAX {
        log_msg("ERROR", "Invalid iteration count");
        result = -1;
        return cleanup(temp_buffer, state, result);
    }

    // Validate seed value.
    if seed < 0 || seed > UINT16_MAX {
        log_msg("ERROR", "Invalid seed value");
        result = -2;
        return cleanup(temp_buffer, state, result);
    }

    // Select operation based on mode.
    selected_op = match mode {
        0 => process_value,
        1 => double_value,
        2 => triple_value,
        _ => {
            log_msg("WARNING", "Invalid mode, using default");
            process_value
        }
    };

    // Initialize processor state.
    state = init_processor(iterations as usize, selected_op);
    if state.is_none() {
        log_msg("ERROR", "Failed to initialize processor");
        result = -3;
        return cleanup(temp_buffer, state, result);
    }

    // Allocate temporary buffer.
    temp_buffer = Some(vec![0 as c_int; iterations as usize]);
    if temp_buffer.is_none() {
        log_msg("ERROR", "Failed to allocate temporary buffer");
        result = -4;
        return cleanup(temp_buffer, state, result);
    }

    // Validate state status flag.
    {
        let s = state.as_ref().unwrap();
        if !check_char_flag(s.status) {
            log_msg("ERROR", "Invalid state status");
            result = -5;
            return cleanup(temp_buffer, state, result);
        }
    }

    let mut current_value: c_int = seed;
    {
        let s = state.as_mut().unwrap();
        let buf = temp_buffer.as_mut().unwrap();
        let mut early_break = false;
        let mut early_return: Option<c_int> = None;

        for i in 0..iterations {
            if !is_valid_state(s) {
                log_msg("ERROR", "State became invalid during processing");
                early_return = Some(-6);
                break;
            }

            let op = s.operation.unwrap();
            let val = op(current_value, 0, std::ptr::null_mut());
            buf[i as usize] = val;

            if val < threshold {
                let idx = s.count;
                s.results[idx] = val;
                s.count += 1;
            }

            current_value = val % 1000;

            if s.count >= UINT16_MAX as usize {
                log_msg("WARNING", "Reached maximum count");
                early_break = true;
                break;
            }
        }

        let _ = early_break;

        if let Some(r) = early_return {
            result = r;
            return cleanup(temp_buffer, state, result);
        }
    }

    result = 0;
    {
        let s = state.as_ref().unwrap();
        for i in 0..s.count {
            result = result.wrapping_add(s.results[i]);
        }
    }

    log_msg("INFO", "Processing completed successfully");

    cleanup(temp_buffer, state, result)
}

fn cleanup(
    temp_buffer: Option<Vec<c_int>>,
    state: Option<Box<ProcessorState>>,
    result: c_int,
) -> c_int {
    drop(temp_buffer);
    drop(state);
    result
}
