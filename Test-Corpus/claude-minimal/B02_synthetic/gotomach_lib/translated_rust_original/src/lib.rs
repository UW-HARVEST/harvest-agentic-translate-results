// Copyright 2025 MIT Lincoln Laboratory
// Permission is hereby granted, free of charge,
// to any person obtaining a copy of this software
// and associated documentation files (the "Software"),
// to deal in the Software without restriction,
// including without limitation the rights to use, copy,
// modify, merge, publish, distribute, sublicense,
// and/or sell copies of the Software,
// and to permit persons to whom the Software is furnished to do so,
// subject to the following conditions:
//
// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
// EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
// THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
// IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE
// FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT,
// TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE
// OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

use std::ffi::c_void;
use std::os::raw::c_int;

type OperationFn = fn(value: c_int, unused_param: c_int, unused_context: *mut c_void) -> c_int;

struct ProcessorState {
    results: Vec<c_int>,
    capacity: usize,
    count: usize,
    operation: OperationFn,
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

pub fn process_value(value: c_int, _unused_param: c_int, _unused_context: *mut c_void) -> c_int {
    value + 10
}

pub fn double_value(value: c_int, _unused_param: c_int, _unused_context: *mut c_void) -> c_int {
    value * 2
}

pub fn triple_value(value: c_int, _unused_param: c_int, _unused_context: *mut c_void) -> c_int {
    value * 3
}

fn init_processor(capacity: usize, op: OperationFn) -> Option<Box<ProcessorState>> {
    let results = vec![0i32; capacity];
    Some(Box::new(ProcessorState {
        results,
        capacity,
        count: 0,
        operation: op,
        status: 1,
    }))
}

fn log_msg(level: &str, msg: &str) {
    println!("[{}] {}", level, msg);
}

#[no_mangle]
pub extern "C" fn gotomach(iterations: c_int, seed: c_int, mode: c_int, threshold: c_int) -> c_int {
    let mut result: c_int;

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

    let state_opt = init_processor(iterations as usize, selected_op);
    let mut state = match state_opt {
        Some(s) => s,
        None => {
            log_msg("ERROR", "Failed to initialize processor");
            return -3;
        }
    };

    let mut temp_buffer: Vec<c_int> = vec![0; iterations as usize];

    if !check_char_flag(state.status) {
        log_msg("ERROR", "Invalid state status");
        return -5;
    }

    let mut current_value: c_int = seed;
    for i in 0..(iterations as usize) {
        if !is_valid_state(&state) {
            log_msg("ERROR", "State became invalid during processing");
            return -6;
        }

        temp_buffer[i] = (state.operation)(current_value, 0, std::ptr::null_mut());

        if temp_buffer[i] < threshold {
            let count = state.count;
            state.results[count] = temp_buffer[i];
            state.count += 1;
        }

        current_value = temp_buffer[i] % 1000;

        if state.count >= u16::MAX as usize {
            log_msg("WARNING", "Reached maximum count");
            break;
        }
    }

    result = 0;
    for i in 0..state.count {
        result = result.wrapping_add(state.results[i]);
    }

    log_msg("INFO", "Processing completed successfully");

    // state and temp_buffer dropped automatically
    drop(temp_buffer);
    drop(state);

    result
}
