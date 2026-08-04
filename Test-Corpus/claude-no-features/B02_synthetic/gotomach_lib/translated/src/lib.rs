// Copyright 2025 MIT Lincoln Laboratory
// Translated from C to Rust.

use std::ffi::c_int;
use std::os::raw::{c_char, c_void};

// Match LOG_MSG macro behavior exactly: printf("[<LEVEL>] <msg>\n")
// We use libc::printf to ensure byte-identical stdout output (including
// stdio buffering semantics).
fn log_msg(level: &str, msg: &str) {
    // Build the format string: "[<level>] <msg>\n\0"
    let formatted = format!("[{}] {}\n\0", level, msg);
    unsafe {
        libc::printf(
            b"%s\0".as_ptr() as *const c_char,
            formatted.as_ptr() as *const c_char,
        );
    }
}

type OperationFn = fn(c_int, c_int, *mut c_void) -> c_int;

#[repr(C)]
struct ProcessorState {
    results: *mut c_int,
    capacity: usize,
    count: usize,
    operation: Option<OperationFn>,
    status: u8, // C 'char' - using u8 to be safe regardless of signedness
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

fn init_processor(capacity: usize, op: OperationFn) -> *mut ProcessorState {
    unsafe {
        let state_layout = std::alloc::Layout::new::<ProcessorState>();
        // Use libc::malloc to mirror C semantics where allocation could conceivably fail.
        let state_ptr = libc::malloc(std::mem::size_of::<ProcessorState>()) as *mut ProcessorState;
        if state_ptr.is_null() {
            let _ = state_layout; // suppress unused warning if any
            return std::ptr::null_mut();
        }

        let results_ptr =
            libc::malloc(capacity.wrapping_mul(std::mem::size_of::<c_int>())) as *mut c_int;
        if results_ptr.is_null() {
            libc::free(state_ptr as *mut c_void);
            return std::ptr::null_mut();
        }

        (*state_ptr).results = results_ptr;
        (*state_ptr).capacity = capacity;
        (*state_ptr).count = 0;
        (*state_ptr).operation = Some(op);
        (*state_ptr).status = 1;

        state_ptr
    }
}

fn cleanup_processor(state: *mut ProcessorState) {
    unsafe {
        if !state.is_null() {
            if !(*state).results.is_null() {
                libc::free((*state).results as *mut c_void);
            }
            libc::free(state as *mut c_void);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn gotomach(
    iterations: c_int,
    seed: c_int,
    mode: c_int,
    threshold: c_int,
) -> c_int {
    let mut state: *mut ProcessorState = std::ptr::null_mut();
    let mut temp_buffer: *mut c_int = std::ptr::null_mut();
    let mut result: c_int = 0;
    let selected_op: OperationFn;

    log_msg("INFO", "Starting gotomach function");

    // UINT16_MAX = 65535
    const UINT16_MAX: c_int = 65535;

    'cleanup: {
        if iterations < 0 || iterations > UINT16_MAX {
            log_msg("ERROR", "Invalid iteration count");
            result = -1;
            break 'cleanup;
        }

        if seed < 0 || seed > UINT16_MAX {
            log_msg("ERROR", "Invalid seed value");
            result = -2;
            break 'cleanup;
        }

        match mode {
            0 => {
                selected_op = process_value;
            }
            1 => {
                selected_op = double_value;
            }
            2 => {
                selected_op = triple_value;
            }
            _ => {
                log_msg("WARNING", "Invalid mode, using default");
                selected_op = process_value;
            }
        }

        state = init_processor(iterations as usize, selected_op);
        if state.is_null() {
            log_msg("ERROR", "Failed to initialize processor");
            result = -3;
            break 'cleanup;
        }

        unsafe {
            temp_buffer = libc::malloc(
                (iterations as usize).wrapping_mul(std::mem::size_of::<c_int>()),
            ) as *mut c_int;
        }
        if temp_buffer.is_null() {
            log_msg("ERROR", "Failed to allocate temporary buffer");
            result = -4;
            break 'cleanup;
        }

        unsafe {
            if !check_char_flag((*state).status) {
                log_msg("ERROR", "Invalid state status");
                result = -5;
                break 'cleanup;
            }
        }

        let mut current_value: c_int = seed;
        let mut early_break = false;
        let mut i: c_int = 0;
        while i < iterations {
            unsafe {
                if !is_valid_state(&*state) {
                    log_msg("ERROR", "State became invalid during processing");
                    result = -6;
                    early_break = true;
                    break;
                }

                let op = (*state).operation.unwrap();
                let val = op(current_value, 0, std::ptr::null_mut());
                *temp_buffer.add(i as usize) = val;

                if val < threshold {
                    let count = (*state).count;
                    *(*state).results.add(count) = val;
                    (*state).count = count + 1;
                }

                current_value = val % 1000;

                if (*state).count >= UINT16_MAX as usize {
                    log_msg("WARNING", "Reached maximum count");
                    break;
                }
            }
            i += 1;
        }

        if early_break {
            break 'cleanup;
        }

        result = 0;
        unsafe {
            let count = (*state).count;
            for j in 0..count {
                result = result.wrapping_add(*(*state).results.add(j));
            }
        }

        log_msg("INFO", "Processing completed successfully");
    }

    // cleanup:
    unsafe {
        if !temp_buffer.is_null() {
            libc::free(temp_buffer as *mut c_void);
        }
    }
    cleanup_processor(state);

    result
}
