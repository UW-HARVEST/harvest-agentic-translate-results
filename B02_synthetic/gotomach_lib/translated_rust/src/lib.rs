use std::alloc::{Layout, alloc, dealloc};
use std::ptr;

extern "C" {
    fn write(fd: i32, buf: *const core::ffi::c_void, count: usize) -> isize;
}

type OperationFn = extern "C" fn(i32, i32, *mut core::ffi::c_void) -> i32;

struct ProcessorState {
    results: *mut i32,
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

#[unsafe(no_mangle)]
pub extern "C" fn process_value(value: i32, _unused_param: i32, _unused_context: *mut core::ffi::c_void) -> i32 {
    value + 10
}

#[unsafe(no_mangle)]
pub extern "C" fn double_value(value: i32, _unused_param: i32, _unused_context: *mut core::ffi::c_void) -> i32 {
    value * 2
}

#[unsafe(no_mangle)]
pub extern "C" fn triple_value(value: i32, _unused_param: i32, _unused_context: *mut core::ffi::c_void) -> i32 {
    value * 3
}

fn log_msg(level: &[u8], msg: &[u8]) {
    // Reproduce printf("[level] msg\n") byte-identically via write(2) to stdout
    unsafe {
        let mut buf = Vec::with_capacity(1 + level.len() + 2 + msg.len() + 1);
        buf.push(b'[');
        buf.extend_from_slice(level);
        buf.extend_from_slice(b"] ");
        buf.extend_from_slice(msg);
        buf.push(b'\n');
        write(1, buf.as_ptr() as *const core::ffi::c_void, buf.len());
    }
}

fn init_processor(capacity: usize, op: OperationFn) -> *mut ProcessorState {
    unsafe {
        let state_layout = Layout::new::<ProcessorState>();
        let state_ptr = alloc(state_layout) as *mut ProcessorState;
        if state_ptr.is_null() {
            return ptr::null_mut();
        }

        if capacity == 0 {
            // Match C malloc(0) — store a non-null dangling pointer or null.
            // C malloc(0) is implementation-defined; but the loop won't run so it doesn't matter.
            // We use a dangling pointer to avoid Layout with size 0 issues.
            let results = std::ptr::NonNull::<i32>::dangling().as_ptr();
            ptr::write(state_ptr, ProcessorState {
                results,
                capacity,
                count: 0,
                operation: op,
                status: 1,
            });
            return state_ptr;
        }

        let results_layout = Layout::array::<i32>(capacity).unwrap();
        let results_ptr = alloc(results_layout) as *mut i32;
        if results_ptr.is_null() {
            dealloc(state_ptr as *mut u8, state_layout);
            return ptr::null_mut();
        }

        ptr::write(state_ptr, ProcessorState {
            results: results_ptr,
            capacity,
            count: 0,
            operation: op,
            status: 1,
        });

        state_ptr
    }
}

fn cleanup_processor(state: *mut ProcessorState) {
    if !state.is_null() {
        unsafe {
            let s = &*state;
            if !s.results.is_null() && s.capacity > 0 {
                let results_layout = Layout::array::<i32>(s.capacity).unwrap();
                dealloc(s.results as *mut u8, results_layout);
            }
            let state_layout = Layout::new::<ProcessorState>();
            dealloc(state as *mut u8, state_layout);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn gotomach(iterations: i32, seed: i32, mode: i32, threshold: i32) -> i32 {
    let mut state: *mut ProcessorState = ptr::null_mut();
    let mut temp_buffer: *mut i32 = ptr::null_mut();
    let mut result: i32;
    let selected_op: OperationFn;

    log_msg(b"INFO", b"Starting gotomach function");

    if iterations < 0 || iterations > u16::MAX as i32 {
        log_msg(b"ERROR", b"Invalid iteration count");
        result = -1;
        // goto cleanup
        cleanup(temp_buffer, state, iterations as usize);
        return result;
    }

    if seed < 0 || seed > u16::MAX as i32 {
        log_msg(b"ERROR", b"Invalid seed value");
        result = -2;
        cleanup(temp_buffer, state, iterations as usize);
        return result;
    }

    selected_op = match mode {
        0 => process_value,
        1 => double_value,
        2 => triple_value,
        _ => {
            log_msg(b"WARNING", b"Invalid mode, using default");
            process_value
        }
    };

    let iter_usize = iterations as usize;

    state = init_processor(iter_usize, selected_op);
    if state.is_null() {
        log_msg(b"ERROR", b"Failed to initialize processor");
        result = -3;
        cleanup(temp_buffer, state, iter_usize);
        return result;
    }

    if iter_usize > 0 {
        unsafe {
            let layout = Layout::array::<i32>(iter_usize).unwrap();
            temp_buffer = alloc(layout) as *mut i32;
        }
    }
    if iter_usize > 0 && temp_buffer.is_null() {
        log_msg(b"ERROR", b"Failed to allocate temporary buffer");
        result = -4;
        cleanup(temp_buffer, state, iter_usize);
        return result;
    }

    unsafe {
        if !check_char_flag((*state).status) {
            log_msg(b"ERROR", b"Invalid state status");
            result = -5;
            cleanup(temp_buffer, state, iter_usize);
            return result;
        }

        let mut current_value = seed;
        for i in 0..iterations {
            let idx = i as usize;
            if !is_valid_state(&*state) {
                log_msg(b"ERROR", b"State became invalid during processing");
                result = -6;
                cleanup(temp_buffer, state, iter_usize);
                return result;
            }

            let val = ((*state).operation)(current_value, 0, ptr::null_mut());
            *temp_buffer.add(idx) = val;

            if *temp_buffer.add(idx) < threshold {
                *(*state).results.add((*state).count) = *temp_buffer.add(idx);
                (*state).count += 1;
            }

            current_value = *temp_buffer.add(idx) % 1000;

            if (*state).count >= u16::MAX as usize {
                log_msg(b"WARNING", b"Reached maximum count");
                break;
            }
        }

        result = 0;
        for i in 0..(*state).count {
            result += *(*state).results.add(i);
        }
    }

    log_msg(b"INFO", b"Processing completed successfully");

    cleanup(temp_buffer, state, iter_usize);
    result
}

fn cleanup(temp_buffer: *mut i32, state: *mut ProcessorState, capacity: usize) {
    if !temp_buffer.is_null() && capacity > 0 {
        unsafe {
            let layout = Layout::array::<i32>(capacity).unwrap();
            dealloc(temp_buffer as *mut u8, layout);
        }
    }
    cleanup_processor(state);
}
