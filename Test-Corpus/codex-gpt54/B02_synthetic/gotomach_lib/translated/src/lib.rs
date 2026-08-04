use std::ffi::{c_char, c_int, c_void};
use std::mem::size_of;
use std::ptr::{self, NonNull};

type OperationFn = extern "C" fn(value: c_int, unused_param: c_int, unused_context: *mut c_void) -> c_int;

#[repr(C)]
struct ProcessorState {
    results: *mut c_int,
    capacity: usize,
    count: usize,
    operation: OperationFn,
    status: c_char,
}

fn log_info_start() {
    unsafe {
        libc::printf(b"[INFO] Starting gotomach function\n\0".as_ptr().cast());
    }
}

fn log_error_invalid_iterations() {
    unsafe {
        libc::printf(b"[ERROR] Invalid iteration count\n\0".as_ptr().cast());
    }
}

fn log_error_invalid_seed() {
    unsafe {
        libc::printf(b"[ERROR] Invalid seed value\n\0".as_ptr().cast());
    }
}

fn log_warning_invalid_mode() {
    unsafe {
        libc::printf(b"[WARNING] Invalid mode, using default\n\0".as_ptr().cast());
    }
}

fn log_error_init_failed() {
    unsafe {
        libc::printf(b"[ERROR] Failed to initialize processor\n\0".as_ptr().cast());
    }
}

fn log_error_temp_alloc_failed() {
    unsafe {
        libc::printf(b"[ERROR] Failed to allocate temporary buffer\n\0".as_ptr().cast());
    }
}

fn log_error_invalid_status() {
    unsafe {
        libc::printf(b"[ERROR] Invalid state status\n\0".as_ptr().cast());
    }
}

fn log_error_invalid_during_processing() {
    unsafe {
        libc::printf(b"[ERROR] State became invalid during processing\n\0".as_ptr().cast());
    }
}

fn log_warning_max_count() {
    unsafe {
        libc::printf(b"[WARNING] Reached maximum count\n\0".as_ptr().cast());
    }
}

fn log_info_completed() {
    unsafe {
        libc::printf(b"[INFO] Processing completed successfully\n\0".as_ptr().cast());
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

unsafe fn init_processor(capacity: usize, op: OperationFn) -> *mut ProcessorState {
    let state = unsafe { libc::malloc(size_of::<ProcessorState>()) }.cast::<ProcessorState>();
    if state.is_null() {
        return ptr::null_mut();
    }

    let results = unsafe { libc::malloc(capacity.wrapping_mul(size_of::<c_int>())) }.cast::<c_int>();
    if results.is_null() {
        unsafe { libc::free(state.cast()) };
        return ptr::null_mut();
    }

    unsafe {
        ptr::write(
            state,
            ProcessorState {
                results,
                capacity,
                count: 0,
                operation: op,
                status: 1,
            },
        );
    }

    state
}

unsafe fn cleanup_processor(state: *mut ProcessorState) {
    if let Some(state) = NonNull::new(state) {
        let state_ref = unsafe { state.as_ref() };
        if !state_ref.results.is_null() {
            unsafe { libc::free(state_ref.results.cast()) };
        }
        unsafe { libc::free(state.as_ptr().cast()) };
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn gotomach(iterations: c_int, seed: c_int, mode: c_int, threshold: c_int) -> c_int {
    let mut state: *mut ProcessorState = ptr::null_mut();
    let mut temp_buffer: *mut c_int = ptr::null_mut();
    let mut result: c_int;
    let selected_op: OperationFn;

    log_info_start();

    if iterations < 0 || iterations > u16::MAX as c_int {
        log_error_invalid_iterations();
        result = -1;
        goto_cleanup(temp_buffer, state);
        return result;
    }

    if seed < 0 || seed > u16::MAX as c_int {
        log_error_invalid_seed();
        result = -2;
        goto_cleanup(temp_buffer, state);
        return result;
    }

    match mode {
        0 => selected_op = process_value,
        1 => selected_op = double_value,
        2 => selected_op = triple_value,
        _ => {
            log_warning_invalid_mode();
            selected_op = process_value;
        }
    }

    unsafe {
        state = init_processor(iterations as usize, selected_op);
    }
    if state.is_null() {
        log_error_init_failed();
        result = -3;
        goto_cleanup(temp_buffer, state);
        return result;
    }

    unsafe {
        temp_buffer = libc::malloc((iterations as usize).wrapping_mul(size_of::<c_int>())).cast::<c_int>();
    }
    if temp_buffer.is_null() {
        log_error_temp_alloc_failed();
        result = -4;
        goto_cleanup(temp_buffer, state);
        return result;
    }

    if unsafe { !check_char_flag((*state).status) } {
        log_error_invalid_status();
        result = -5;
        goto_cleanup(temp_buffer, state);
        return result;
    }

    let mut current_value = seed;
    let mut i = 0;
    while i < iterations {
        if unsafe { !is_valid_state(&*state) } {
            log_error_invalid_during_processing();
            result = -6;
            goto_cleanup(temp_buffer, state);
            return result;
        }

        unsafe {
            let op = (*state).operation;
            *temp_buffer.add(i as usize) = op(current_value, 0, ptr::null_mut());

            let temp_value = *temp_buffer.add(i as usize);
            if temp_value < threshold {
                *(*state).results.add((*state).count) = temp_value;
                (*state).count += 1;
            }

            current_value = temp_value % 1000;

            if (*state).count >= u16::MAX as usize {
                log_warning_max_count();
                break;
            }
        }

        i += 1;
    }

    result = 0;
    unsafe {
        let mut idx = 0usize;
        while idx < (*state).count {
            result = result.wrapping_add(*(*state).results.add(idx));
            idx += 1;
        }
    }

    log_info_completed();

    goto_cleanup(temp_buffer, state);
    result
}

fn goto_cleanup(temp_buffer: *mut c_int, state: *mut ProcessorState) {
    unsafe {
        if !temp_buffer.is_null() {
            libc::free(temp_buffer.cast());
        }
        cleanup_processor(state);
    }
}
