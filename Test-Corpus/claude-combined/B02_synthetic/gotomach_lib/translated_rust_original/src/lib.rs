// Copyright 2025 MIT Lincoln Laboratory
// Translated to Rust. Reproduces C behavior byte-for-byte.

use std::ffi::c_int;
use std::os::raw::c_void;
use std::ptr;

// Use libc printf to match buffering / output behavior of the C version exactly.
extern "C" {
    fn printf(fmt: *const u8, ...) -> c_int;
    fn malloc(size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
}

const UINT16_MAX: c_int = 65535;

type OperationFn =
    unsafe extern "C" fn(value: c_int, unused_param: c_int, unused_context: *mut c_void) -> c_int;

#[repr(C)]
struct ProcessorState {
    results: *mut c_int,
    capacity: usize,
    count: usize,
    operation: Option<OperationFn>,
    status: u8, // `char` in C
}

unsafe fn is_valid_state(state: *mut ProcessorState) -> bool {
    if (*state).status != 0 {
        return (*state).count < (*state).capacity;
    }
    false
}

fn check_char_flag(flag: u8) -> bool {
    flag != 0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn process_value(
    value: c_int,
    _unused_param: c_int,
    _unused_context: *mut c_void,
) -> c_int {
    value + 10
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn double_value(
    value: c_int,
    _unused_param: c_int,
    _unused_context: *mut c_void,
) -> c_int {
    value * 2
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn triple_value(
    value: c_int,
    _unused_param: c_int,
    _unused_context: *mut c_void,
) -> c_int {
    value * 3
}

unsafe fn init_processor(capacity: usize, op: OperationFn) -> *mut ProcessorState {
    let state = malloc(std::mem::size_of::<ProcessorState>()) as *mut ProcessorState;
    if state.is_null() {
        return ptr::null_mut();
    }

    let results = malloc(capacity.wrapping_mul(std::mem::size_of::<c_int>())) as *mut c_int;
    if results.is_null() {
        free(state as *mut c_void);
        return ptr::null_mut();
    }

    (*state).results = results;
    (*state).capacity = capacity;
    (*state).count = 0;
    (*state).operation = Some(op);
    (*state).status = 1;

    state
}

unsafe fn cleanup_processor(state: *mut ProcessorState) {
    if !state.is_null() {
        if !(*state).results.is_null() {
            free((*state).results as *mut c_void);
        }
        free(state as *mut c_void);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn gotomach(
    iterations: c_int,
    seed: c_int,
    mode: c_int,
    threshold: c_int,
) -> c_int {
    let mut state: *mut ProcessorState = ptr::null_mut();
    let mut temp_buffer: *mut c_int = ptr::null_mut();
    let mut result: c_int;
    let selected_op: OperationFn;

    printf(b"[INFO] Starting gotomach function\n\0".as_ptr());

    'cleanup: {
        if iterations < 0 || iterations > UINT16_MAX {
            printf(b"[ERROR] Invalid iteration count\n\0".as_ptr());
            result = -1;
            break 'cleanup;
        }

        if seed < 0 || seed > UINT16_MAX {
            printf(b"[ERROR] Invalid seed value\n\0".as_ptr());
            result = -2;
            break 'cleanup;
        }

        selected_op = match mode {
            0 => process_value,
            1 => double_value,
            2 => triple_value,
            _ => {
                printf(b"[WARNING] Invalid mode, using default\n\0".as_ptr());
                process_value
            }
        };

        state = init_processor(iterations as usize, selected_op);
        if state.is_null() {
            printf(b"[ERROR] Failed to initialize processor\n\0".as_ptr());
            result = -3;
            break 'cleanup;
        }

        temp_buffer =
            malloc((iterations as usize).wrapping_mul(std::mem::size_of::<c_int>())) as *mut c_int;
        if temp_buffer.is_null() {
            printf(b"[ERROR] Failed to allocate temporary buffer\n\0".as_ptr());
            result = -4;
            break 'cleanup;
        }

        if !check_char_flag((*state).status) {
            printf(b"[ERROR] Invalid state status\n\0".as_ptr());
            result = -5;
            break 'cleanup;
        }

        let mut current_value: c_int = seed;
        let mut early_break = false;
        for i in 0..iterations {
            if !is_valid_state(state) {
                printf(b"[ERROR] State became invalid during processing\n\0".as_ptr());
                result = -6;
                break 'cleanup;
            }

            let op = (*state).operation.unwrap();
            let val = op(current_value, 0, ptr::null_mut());
            *temp_buffer.offset(i as isize) = val;

            if val < threshold {
                let count = (*state).count;
                *(*state).results.add(count) = val;
                (*state).count = count + 1;
            }

            current_value = val % 1000;

            if (*state).count >= UINT16_MAX as usize {
                printf(b"[WARNING] Reached maximum count\n\0".as_ptr());
                early_break = true;
                break;
            }
        }
        let _ = early_break;

        result = 0;
        let count = (*state).count;
        for i in 0..count {
            result = result.wrapping_add(*(*state).results.add(i));
        }

        printf(b"[INFO] Processing completed successfully\n\0".as_ptr());
    }

    // cleanup:
    if !temp_buffer.is_null() {
        free(temp_buffer as *mut c_void);
    }
    cleanup_processor(state);

    result
}
