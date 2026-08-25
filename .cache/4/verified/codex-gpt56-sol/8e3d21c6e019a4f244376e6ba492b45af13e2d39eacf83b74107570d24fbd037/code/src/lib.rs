use std::ffi::{c_int, c_void};
use std::ptr;

type OperationFn = extern "C" fn(c_int, c_int, *mut c_void) -> c_int;

#[repr(C)]
struct ProcessorState {
    results: *mut c_int,
    capacity: usize,
    count: usize,
    operation: OperationFn,
    status: i8,
}

unsafe extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
    fn puts(string: *const i8) -> c_int;
}

#[inline]
fn log(message: &'static [u8]) {
    unsafe {
        puts(message.as_ptr().cast());
    }
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

unsafe fn init_processor(capacity: usize, operation: OperationFn) -> *mut ProcessorState {
    let state = unsafe { malloc(size_of::<ProcessorState>()).cast::<ProcessorState>() };
    if state.is_null() {
        return ptr::null_mut();
    }

    let results = unsafe { malloc(capacity.wrapping_mul(size_of::<c_int>())).cast::<c_int>() };
    if results.is_null() {
        unsafe {
            free(state.cast());
        }
        return ptr::null_mut();
    }

    unsafe {
        ptr::write(
            state,
            ProcessorState {
                results,
                capacity,
                count: 0,
                operation,
                status: 1,
            },
        );
    }
    state
}

unsafe fn cleanup_processor(state: *mut ProcessorState) {
    if !state.is_null() {
        let results = unsafe { (*state).results };
        if !results.is_null() {
            unsafe {
                free(results.cast());
            }
        }
        unsafe {
            free(state.cast());
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn gotomach(iterations: c_int, seed: c_int, mode: c_int, threshold: c_int) -> c_int {
    log(b"[INFO] Starting gotomach function\0");

    if !(0..=u16::MAX as c_int).contains(&iterations) {
        log(b"[ERROR] Invalid iteration count\0");
        return -1;
    }

    if !(0..=u16::MAX as c_int).contains(&seed) {
        log(b"[ERROR] Invalid seed value\0");
        return -2;
    }

    let operation = match mode {
        0 => process_value,
        1 => double_value,
        2 => triple_value,
        _ => {
            log(b"[WARNING] Invalid mode, using default\0");
            process_value
        }
    };

    let state = unsafe { init_processor(iterations as usize, operation) };
    if state.is_null() {
        log(b"[ERROR] Failed to initialize processor\0");
        return -3;
    }

    let temp_buffer =
        unsafe { malloc((iterations as usize).wrapping_mul(size_of::<c_int>())).cast::<c_int>() };
    if temp_buffer.is_null() {
        log(b"[ERROR] Failed to allocate temporary buffer\0");
        unsafe {
            cleanup_processor(state);
        }
        return -4;
    }

    if unsafe { (*state).status == 0 } {
        log(b"[ERROR] Invalid state status\0");
        unsafe {
            free(temp_buffer.cast());
            cleanup_processor(state);
        }
        return -5;
    }

    let mut result = 0_i32;
    let mut current_value = seed;

    for i in 0..iterations as usize {
        let is_valid = unsafe { (*state).status != 0 && (*state).count < (*state).capacity };
        if !is_valid {
            log(b"[ERROR] State became invalid during processing\0");
            result = -6;
            break;
        }

        let value = unsafe { ((*state).operation)(current_value, 0, ptr::null_mut()) };
        unsafe {
            *temp_buffer.add(i) = value;
        }

        if value < threshold {
            unsafe {
                *(*state).results.add((*state).count) = value;
                (*state).count += 1;
            }
        }

        current_value = value % 1000;

        if unsafe { (*state).count >= u16::MAX as usize } {
            log(b"[WARNING] Reached maximum count\0");
            break;
        }
    }

    if result != -6 {
        result = 0;
        for i in 0..unsafe { (*state).count } {
            result = result.wrapping_add(unsafe { *(*state).results.add(i) });
        }
        log(b"[INFO] Processing completed successfully\0");
    }

    unsafe {
        free(temp_buffer.cast());
        cleanup_processor(state);
    }
    result
}
