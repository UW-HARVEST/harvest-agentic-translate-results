use std::ffi::{c_char, c_int, c_void};
use std::mem::size_of;
use std::ptr;

type OperationFn = unsafe extern "C" fn(c_int, c_int, *mut c_void) -> c_int;

#[repr(C)]
struct ProcessorState {
    results: *mut c_int,
    capacity: usize,
    count: usize,
    operation: OperationFn,
    status: c_char,
}

unsafe extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn free(pointer: *mut c_void);
    fn puts(message: *const c_char) -> c_int;
}

fn log(message: &'static [u8]) {
    debug_assert_eq!(message.last(), Some(&0));
    unsafe {
        puts(message.as_ptr().cast());
    }
}

unsafe fn is_valid_state(state: *mut ProcessorState) -> bool {
    unsafe { (*state).status != 0 && (*state).count < (*state).capacity }
}

fn check_char_flag(flag: c_char) -> bool {
    flag != 0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn process_value(
    value: c_int,
    _unused_param: c_int,
    _unused_context: *mut c_void,
) -> c_int {
    value.wrapping_add(10)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn double_value(
    value: c_int,
    _unused_param: c_int,
    _unused_context: *mut c_void,
) -> c_int {
    value.wrapping_mul(2)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn triple_value(
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
pub unsafe extern "C" fn gotomach(
    iterations: c_int,
    seed: c_int,
    mode: c_int,
    threshold: c_int,
) -> c_int {
    let mut state = ptr::null_mut::<ProcessorState>();
    let mut temp_buffer = ptr::null_mut::<c_int>();
    let result;

    log(b"[INFO] Starting gotomach function\0");

    if !(0..=u16::MAX as c_int).contains(&iterations) {
        log(b"[ERROR] Invalid iteration count\0");
        result = -1;
    } else if !(0..=u16::MAX as c_int).contains(&seed) {
        log(b"[ERROR] Invalid seed value\0");
        result = -2;
    } else {
        let selected_op: OperationFn = match mode {
            0 => process_value,
            1 => double_value,
            2 => triple_value,
            _ => {
                log(b"[WARNING] Invalid mode, using default\0");
                process_value
            }
        };

        state = unsafe { init_processor(iterations as usize, selected_op) };
        if state.is_null() {
            log(b"[ERROR] Failed to initialize processor\0");
            result = -3;
        } else {
            temp_buffer = unsafe {
                malloc((iterations as usize).wrapping_mul(size_of::<c_int>())).cast::<c_int>()
            };
            if temp_buffer.is_null() {
                log(b"[ERROR] Failed to allocate temporary buffer\0");
                result = -4;
            } else if !check_char_flag(unsafe { (*state).status }) {
                log(b"[ERROR] Invalid state status\0");
                result = -5;
            } else {
                let mut current_value = seed;
                let mut processing_result = None;

                for i in 0..iterations as usize {
                    if !unsafe { is_valid_state(state) } {
                        log(b"[ERROR] State became invalid during processing\0");
                        processing_result = Some(-6);
                        break;
                    }

                    let operation = unsafe { (*state).operation };
                    let value = unsafe { operation(current_value, 0, ptr::null_mut()) };
                    unsafe {
                        *temp_buffer.add(i) = value;
                    }

                    if value < threshold {
                        let count = unsafe { (*state).count };
                        unsafe {
                            *(*state).results.add(count) = value;
                            (*state).count = count + 1;
                        }
                    }

                    current_value = value % 1000;

                    if unsafe { (*state).count } >= u16::MAX as usize {
                        log(b"[WARNING] Reached maximum count\0");
                        break;
                    }
                }

                if let Some(error) = processing_result {
                    result = error;
                } else {
                    let mut total: c_int = 0;
                    for i in 0..unsafe { (*state).count } {
                        total = total.wrapping_add(unsafe { *(*state).results.add(i) });
                    }
                    log(b"[INFO] Processing completed successfully\0");
                    result = total;
                }
            }
        }
    }

    if !temp_buffer.is_null() {
        unsafe {
            free(temp_buffer.cast());
        }
    }
    unsafe {
        cleanup_processor(state);
    }

    result
}
