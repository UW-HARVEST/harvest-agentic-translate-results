// Copyright 2025 MIT Lincoln Laboratory
// Translated to Rust from c_src/src/lib.c

use std::ffi::c_int;
use std::os::raw::c_char;

extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
}

const UINT16_MAX_I32: i32 = 65535;
const UINT16_MAX_USIZE: usize = 65535;

type OperationFn = fn(value: c_int) -> c_int;

fn process_value(value: c_int) -> c_int {
    value.wrapping_add(10)
}

fn double_value(value: c_int) -> c_int {
    value.wrapping_mul(2)
}

fn triple_value(value: c_int) -> c_int {
    value.wrapping_mul(3)
}

struct ProcessorState {
    results: Vec<c_int>,
    capacity: usize,
    count: usize,
    operation: OperationFn,
    status: c_char,
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

fn init_processor(capacity: usize, op: OperationFn) -> Box<ProcessorState> {
    // Mirrors C: malloc a results buffer of `capacity` ints (uninitialized in C, zero in Rust).
    // Reads only happen at indices < count after a write, so zero-init is observationally identical.
    let results = vec![0 as c_int; capacity];
    Box::new(ProcessorState {
        results,
        capacity,
        count: 0,
        operation: op,
        status: 1,
    })
}

#[inline]
fn log_msg(line: &[u8]) {
    // `line` must be NUL-terminated. Pass through printf("%s", line) to keep stdout byte-identical
    // with the C version's printf calls (same FILE* handle, same buffering).
    unsafe {
        let fmt = b"%s\0".as_ptr() as *const c_char;
        printf(fmt, line.as_ptr() as *const c_char);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn gotomach(
    iterations: c_int,
    seed: c_int,
    mode: c_int,
    threshold: c_int,
) -> c_int {
    log_msg(b"[INFO] Starting gotomach function\n\0");

    if iterations < 0 || iterations > UINT16_MAX_I32 {
        log_msg(b"[ERROR] Invalid iteration count\n\0");
        return -1;
    }

    if seed < 0 || seed > UINT16_MAX_I32 {
        log_msg(b"[ERROR] Invalid seed value\n\0");
        return -2;
    }

    let selected_op: OperationFn = match mode {
        0 => process_value,
        1 => double_value,
        2 => triple_value,
        _ => {
            log_msg(b"[WARNING] Invalid mode, using default\n\0");
            process_value
        }
    };

    let mut state = init_processor(iterations as usize, selected_op);

    // C: malloc(iterations * sizeof(int)). Rust's vec! either succeeds or panics; we treat success
    // as the common case (matches successful malloc on systems with sufficient memory).
    let mut temp_buffer: Vec<c_int> = vec![0 as c_int; iterations as usize];

    if !check_char_flag(state.status) {
        log_msg(b"[ERROR] Invalid state status\n\0");
        return -5;
    }

    let mut current_value: c_int = seed;
    let mut state_invalid = false;

    let mut i: i32 = 0;
    while i < iterations {
        if !is_valid_state(&state) {
            log_msg(b"[ERROR] State became invalid during processing\n\0");
            state_invalid = true;
            break;
        }

        let v = (state.operation)(current_value);
        temp_buffer[i as usize] = v;

        if v < threshold {
            let idx = state.count;
            state.results[idx] = v;
            state.count += 1;
        }

        current_value = v.wrapping_rem(1000);

        if state.count >= UINT16_MAX_USIZE {
            log_msg(b"[WARNING] Reached maximum count\n\0");
            break;
        }

        i += 1;
    }

    if state_invalid {
        // C `goto cleanup` with result = -6: skip the summation and the success log.
        // Drops handle cleanup (free(temp_buffer); cleanup_processor(state)).
        return -6;
    }

    let mut result: c_int = 0;
    for j in 0..state.count {
        result = result.wrapping_add(state.results[j]);
    }

    log_msg(b"[INFO] Processing completed successfully\n\0");

    // Drops mirror C's free(temp_buffer) and cleanup_processor(state).
    drop(temp_buffer);
    drop(state);

    result
}
