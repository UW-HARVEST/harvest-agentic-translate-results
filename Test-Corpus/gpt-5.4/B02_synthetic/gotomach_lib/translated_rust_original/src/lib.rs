use std::os::raw::c_int;

type OperationFn = fn(c_int, c_int, *mut core::ffi::c_void) -> c_int;

fn process_value(value: c_int, _unused_param: c_int, _unused_context: *mut core::ffi::c_void) -> c_int {
    value + 10
}

fn double_value(value: c_int, _unused_param: c_int, _unused_context: *mut core::ffi::c_void) -> c_int {
    value * 2
}

fn triple_value(value: c_int, _unused_param: c_int, _unused_context: *mut core::ffi::c_void) -> c_int {
    value * 3
}

struct ProcessorState {
    results: Vec<c_int>,
    capacity: usize,
    operation: OperationFn,
    status: i8,
}

fn is_valid_state(state: &ProcessorState) -> bool {
    if state.status != 0 {
        state.results.len() < state.capacity
    } else {
        false
    }
}

fn check_char_flag(flag: i8) -> bool {
    flag != 0
}

fn init_processor(capacity: usize, op: OperationFn) -> Option<ProcessorState> {
    let mut results = Vec::new();
    if results.try_reserve_exact(capacity).is_err() {
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
    let mut result: c_int = 0;

    println!("[INFO] Starting gotomach function");

    if iterations < 0 || iterations > u16::MAX as c_int {
        println!("[ERROR] Invalid iteration count");
        return -1;
    }

    if seed < 0 || seed > u16::MAX as c_int {
        println!("[ERROR] Invalid seed value");
        return -2;
    }

    let selected_op: OperationFn = match mode {
        0 => process_value,
        1 => double_value,
        2 => triple_value,
        _ => {
            println!("[WARNING] Invalid mode, using default");
            process_value
        }
    };

    let iterations_usize = iterations as usize;

    let mut state = match init_processor(iterations_usize, selected_op) {
        Some(state) => state,
        None => {
            println!("[ERROR] Failed to initialize processor");
            return -3;
        }
    };

    let mut temp_buffer: Vec<c_int> = Vec::new();
    if temp_buffer.try_reserve_exact(iterations_usize).is_err() {
        println!("[ERROR] Failed to allocate temporary buffer");
        return -4;
    }
    temp_buffer.resize(iterations_usize, 0);

    if !check_char_flag(state.status) {
        println!("[ERROR] Invalid state status");
        return -5;
    }

    let mut current_value = seed;
    for i in 0..iterations_usize {
        if !is_valid_state(&state) {
            println!("[ERROR] State became invalid during processing");
            return -6;
        }

        temp_buffer[i] = (state.operation)(current_value, 0, core::ptr::null_mut());

        if temp_buffer[i] < threshold {
            state.results.push(temp_buffer[i]);
        }

        current_value = temp_buffer[i] % 1000;

        if state.results.len() >= u16::MAX as usize {
            println!("[WARNING] Reached maximum count");
            break;
        }
    }

    for value in &state.results {
        result += *value;
    }

    println!("[INFO] Processing completed successfully");
    result
}
