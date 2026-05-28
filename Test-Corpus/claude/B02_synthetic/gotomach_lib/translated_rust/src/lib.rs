// Copyright 2025 MIT Lincoln Laboratory
// Translated from C to Rust preserving exact output behavior.

use std::ffi::c_void;
use std::os::raw::{c_char, c_int};

const UINT16_MAX: c_int = 65535;

type OperationFn = unsafe extern "C" fn(value: c_int, unused_param: c_int, unused_context: *mut c_void) -> c_int;

#[repr(C)]
struct ProcessorState {
    results: *mut c_int,
    capacity: usize,
    count: usize,
    operation: Option<OperationFn>,
    status: c_char,
}

unsafe fn is_valid_state(state: *mut ProcessorState) -> bool {
    if (*state).status != 0 {
        return (*state).count < (*state).capacity;
    }
    false
}

fn check_char_flag(flag: c_char) -> bool {
    flag != 0
}

#[unsafe(no_mangle)]
pub extern "C" fn process_value(value: c_int, _unused_param: c_int, _unused_context: *mut c_void) -> c_int {
    value + 10
}

#[unsafe(no_mangle)]
pub extern "C" fn double_value(value: c_int, _unused_param: c_int, _unused_context: *mut c_void) -> c_int {
    value * 2
}

#[unsafe(no_mangle)]
pub extern "C" fn triple_value(value: c_int, _unused_param: c_int, _unused_context: *mut c_void) -> c_int {
    value * 3
}

unsafe fn init_processor(capacity: usize, op: Option<OperationFn>) -> *mut ProcessorState {
    let state = libc::malloc(std::mem::size_of::<ProcessorState>()) as *mut ProcessorState;
    if state.is_null() {
        return std::ptr::null_mut();
    }

    let results = libc::malloc(capacity.wrapping_mul(std::mem::size_of::<c_int>())) as *mut c_int;
    if results.is_null() {
        libc::free(state as *mut c_void);
        return std::ptr::null_mut();
    }

    (*state).results = results;
    (*state).capacity = capacity;
    (*state).count = 0;
    (*state).operation = op;
    (*state).status = 1;

    state
}

unsafe fn cleanup_processor(state: *mut ProcessorState) {
    if !state.is_null() {
        if !(*state).results.is_null() {
            libc::free((*state).results as *mut c_void);
        }
        libc::free(state as *mut c_void);
    }
}

// Use libc::printf to match C's stdio buffering exactly.
unsafe fn log_msg(msg: &[u8]) {
    // msg must include the trailing \n and null terminator
    libc::printf(msg.as_ptr() as *const c_char);
}

#[unsafe(no_mangle)]
pub extern "C" fn gotomach(iterations: c_int, seed: c_int, mode: c_int, threshold: c_int) -> c_int {
    unsafe {
        let mut state: *mut ProcessorState = std::ptr::null_mut();
        let mut temp_buffer: *mut c_int = std::ptr::null_mut();
        let mut result: c_int = 0;
        let selected_op: Option<OperationFn>;

        log_msg(b"[INFO] Starting gotomach function\n\0");

        'cleanup: loop {
            if iterations < 0 || iterations > UINT16_MAX {
                log_msg(b"[ERROR] Invalid iteration count\n\0");
                result = -1;
                break 'cleanup;
            }

            if seed < 0 || seed > UINT16_MAX {
                log_msg(b"[ERROR] Invalid seed value\n\0");
                result = -2;
                break 'cleanup;
            }

            selected_op = match mode {
                0 => Some(process_value as OperationFn),
                1 => Some(double_value as OperationFn),
                2 => Some(triple_value as OperationFn),
                _ => {
                    log_msg(b"[WARNING] Invalid mode, using default\n\0");
                    Some(process_value as OperationFn)
                }
            };

            state = init_processor(iterations as usize, selected_op);
            if state.is_null() {
                log_msg(b"[ERROR] Failed to initialize processor\n\0");
                result = -3;
                break 'cleanup;
            }

            temp_buffer = libc::malloc((iterations as usize).wrapping_mul(std::mem::size_of::<c_int>())) as *mut c_int;
            if temp_buffer.is_null() {
                log_msg(b"[ERROR] Failed to allocate temporary buffer\n\0");
                result = -4;
                break 'cleanup;
            }

            if !check_char_flag((*state).status) {
                log_msg(b"[ERROR] Invalid state status\n\0");
                result = -5;
                break 'cleanup;
            }

            let mut current_value: c_int = seed;
            let mut early_break = false;
            let mut i: c_int = 0;
            while i < iterations {
                if !is_valid_state(state) {
                    log_msg(b"[ERROR] State became invalid during processing\n\0");
                    result = -6;
                    break 'cleanup;
                }

                let op = (*state).operation.unwrap();
                let computed = op(current_value, 0, std::ptr::null_mut());
                *temp_buffer.add(i as usize) = computed;

                if computed < threshold {
                    let count = (*state).count;
                    *(*state).results.add(count) = computed;
                    (*state).count = count + 1;
                }

                current_value = computed % 1000;

                if (*state).count >= UINT16_MAX as usize {
                    log_msg(b"[WARNING] Reached maximum count\n\0");
                    early_break = true;
                    break;
                }

                i += 1;
            }
            let _ = early_break;

            result = 0;
            let count = (*state).count;
            let mut i: usize = 0;
            while i < count {
                result = result.wrapping_add(*(*state).results.add(i));
                i += 1;
            }

            log_msg(b"[INFO] Processing completed successfully\n\0");
            break 'cleanup;
        }

        if !temp_buffer.is_null() {
            libc::free(temp_buffer as *mut c_void);
        }
        cleanup_processor(state);

        result
    }
}
