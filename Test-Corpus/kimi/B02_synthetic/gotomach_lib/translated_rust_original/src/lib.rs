use std::ffi::{c_char, c_int, c_void};
use std::os::raw::c_int as RawCInt;

pub type OperationFn = unsafe extern "C" fn(c_int, c_int, *mut c_void) -> c_int;

#[repr(C)]
pub struct ProcessorState {
    pub results: *mut c_int,
    pub capacity: usize,
    pub count: usize,
    pub operation: Option<OperationFn>,
    pub status: c_char,
}

unsafe fn is_valid_state(state: *const ProcessorState) -> bool {
    if (*state).status != 0 {
        return (*state).count < (*state).capacity;
    }
    false
}

unsafe fn check_char_flag(flag: c_char) -> bool {
    flag != 0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn process_value(value: c_int, _unused_param: c_int, _unused_context: *mut c_void) -> c_int {
    value + 10
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn double_value(value: c_int, _unused_param: c_int, _unused_context: *mut c_void) -> c_int {
    value * 2
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn triple_value(value: c_int, _unused_param: c_int, _unused_context: *mut c_void) -> c_int {
    value * 3
}

unsafe fn init_processor(capacity: usize, op: OperationFn) -> *mut ProcessorState {
    let state = libc::malloc(std::mem::size_of::<ProcessorState>()) as *mut ProcessorState;
    if state.is_null() {
        return std::ptr::null_mut();
    }

    let results = libc::malloc(capacity * std::mem::size_of::<c_int>()) as *mut c_int;
    if results.is_null() {
        libc::free(state as *mut libc::c_void);
        return std::ptr::null_mut();
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
            libc::free((*state).results as *mut libc::c_void);
        }
        libc::free(state as *mut libc::c_void);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn gotomach(iterations: c_int, seed: c_int, mode: c_int, threshold: c_int) -> c_int {
    let mut state: *mut ProcessorState = std::ptr::null_mut();
    let mut temp_buffer: *mut c_int = std::ptr::null_mut();
    let mut result: c_int = 0;
    let mut selected_op: Option<OperationFn> = None;

    libc::printf("[INFO] Starting gotomach function\n".as_ptr() as *const i8);

    if iterations < 0 || iterations > u16::MAX as c_int {
        libc::printf("[ERROR] Invalid iteration count\n".as_ptr() as *const i8);
        result = -1;
        goto_cleanup(&mut state, &mut temp_buffer, result)
    }

    if seed < 0 || seed > u16::MAX as c_int {
        libc::printf("[ERROR] Invalid seed value\n".as_ptr() as *const i8);
        result = -2;
        goto_cleanup(&mut state, &mut temp_buffer, result)
    }

    selected_op = match mode {
        0 => Some(process_value),
        1 => Some(double_value),
        2 => Some(triple_value),
        _ => {
            libc::printf("[WARNING] Invalid mode, using default\n".as_ptr() as *const i8);
            Some(process_value)
        }
    };

    state = init_processor(iterations as usize, selected_op.unwrap());
    if state.is_null() {
        libc::printf("[ERROR] Failed to initialize processor\n".as_ptr() as *const i8);
        result = -3;
        goto_cleanup(&mut state, &mut temp_buffer, result)
    }

    temp_buffer = libc::malloc((iterations as usize) * std::mem::size_of::<c_int>()) as *mut c_int;
    if temp_buffer.is_null() {
        libc::printf("[ERROR] Failed to allocate temporary buffer\n".as_ptr() as *const i8);
        result = -4;
        goto_cleanup(&mut state, &mut temp_buffer, result)
    }

    if !check_char_flag((*state).status) {
        libc::printf("[ERROR] Invalid state status\n".as_ptr() as *const i8);
        result = -5;
        goto_cleanup(&mut state, &mut temp_buffer, result)
    }

    let mut current_value = seed;
    for i in 0..iterations {
        if !is_valid_state(state) {
            libc::printf("[ERROR] State became invalid during processing\n".as_ptr() as *const i8);
            result = -6;
            goto_cleanup(&mut state, &mut temp_buffer, result)
        }

        let op = (*state).operation.unwrap();
        *temp_buffer.add(i as usize) = op(current_value, 0, std::ptr::null_mut());

        if *temp_buffer.add(i as usize) < threshold {
            *(*state).results.add((*state).count) = *temp_buffer.add(i as usize);
            (*state).count += 1;
        }

        current_value = *temp_buffer.add(i as usize) % 1000;

        if (*state).count >= u16::MAX as usize {
            libc::printf("[WARNING] Reached maximum count\n".as_ptr() as *const i8);
            break;
        }
    }

    result = 0;
    for i in 0..(*state).count {
        result += *(*state).results.add(i);
    }

    libc::printf("[INFO] Processing completed successfully\n".as_ptr() as *const i8);

goto_cleanup(&mut state, &mut temp_buffer, result)
}

unsafe fn goto_cleanup(state: &mut *mut ProcessorState, temp_buffer: &mut *mut c_int, result: c_int) -> c_int {
    if !(*temp_buffer).is_null() {
        libc::free(*temp_buffer as *mut libc::c_void);
    }
    cleanup_processor(*state);
    result
}
