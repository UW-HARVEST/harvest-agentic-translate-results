// Copyright 2025 MIT Lincoln Laboratory
// Translated from C to Rust. Behavior preserved exactly.

use std::ffi::c_char;
use std::os::raw::{c_float, c_int, c_uint, c_void};

extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
    fn snprintf(buf: *mut c_char, size: usize, format: *const c_char, ...) -> c_int;
    fn malloc(size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
    fn memchr(s: *const c_void, c: c_int, n: usize) -> *mut c_void;
    fn strlen(s: *const c_char) -> usize;
}

// Mirrors the C bit-field struct semantically. Fields are stored as full u32
// values masked to the appropriate widths during assignment, matching the
// behavior of the C bitfields when printed via %d/%u.
#[repr(C)]
pub struct PackedFlags {
    flag1: u32,
    flag2: u32,
    flag3: u32,
    counter: u32,
    mode: u32,
    status: u32,
    reserved: u32,
}

#[repr(C)]
pub union TypeConfusion {
    int_val: i32,
    float_val: c_float,
    uint_val: u32,
    bytes: [i8; 4],
}

#[repr(C)]
pub struct ProcessState {
    flags: PackedFlags,
    data: TypeConfusion,
    buffer: *mut c_char,
    capacity: c_int,
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn create_state(initial_val: c_int, capacity: c_int) -> *mut ProcessState {
    let state = malloc(std::mem::size_of::<ProcessState>()) as *mut ProcessState;

    if state.is_null() {
        printf(c"Error: Failed to allocate memory for state\n".as_ptr());
        return std::ptr::null_mut();
    }

    (*state).flags.flag1 = 1 & 0x1;
    (*state).flags.flag2 = 0 & 0x1;
    (*state).flags.flag3 = 1 & 0x1;
    (*state).flags.counter = 0 & 0x1F;
    (*state).flags.mode = 3 & 0x7;
    (*state).flags.status = 15 & 0x1F;
    (*state).flags.reserved = 0 & 0xFFFF;

    (*state).data.int_val = initial_val;

    (*state).capacity = capacity;
    (*state).buffer = malloc(capacity as usize) as *mut c_char;

    if (*state).buffer.is_null() {
        printf(c"Error: Failed to allocate buffer\n".as_ptr());
        free(state as *mut c_void);
        return std::ptr::null_mut();
    }

    snprintf(
        (*state).buffer,
        capacity as usize,
        c"State:%d:Mode:%d".as_ptr(),
        initial_val,
        (*state).flags.mode as c_uint,
    );

    state
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn destroy_state(state: *mut ProcessState) {
    if !state.is_null() {
        if !(*state).buffer.is_null() {
            free((*state).buffer as *mut c_void);
        }
        free(state as *mut c_void);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn process_buffer(state: *mut ProcessState, target: c_char) -> c_int {
    if state.is_null() || (*state).buffer.is_null() {
        printf(c"Error: Null pointer in process_buffer\n".as_ptr());
        return -1;
    }

    let mut count: c_int = 0;
    let mut ptr: *mut c_char = (*state).buffer;
    let mut remaining: usize = strlen((*state).buffer);

    while remaining > 0 {
        let found = memchr(ptr as *const c_void, target as c_int, remaining) as *mut c_char;

        if found.is_null() {
            break;
        }

        count += 1;
        // LOG_OPERATION(memchr_found, count)
        printf(
            c"Operation: memchr_found with value %d\n".as_ptr(),
            count,
        );

        // remaining -= (found - ptr + 1);
        let diff = (found as isize) - (ptr as isize);
        remaining -= (diff + 1) as usize;
        ptr = found.offset(1);
    }

    count
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn update_flags(state: *mut ProcessState, param: c_int) {
    if state.is_null() {
        return;
    }

    (*state).flags.counter = ((*state).flags.counter.wrapping_add(1)) & 0x1F;
    (*state).flags.flag1 = (param & 1) as u32 & 0x1;
    (*state).flags.flag2 = (((param & 2) >> 1) as u32) & 0x1;
    (*state).flags.flag3 = (((param & 4) >> 2) as u32) & 0x1;
    (*state).flags.mode = (((param >> 3) & 0x7) as u32) & 0x7;

    // DEBUG_VAR(state->flags.counter)
    printf(
        c"Debug: state->flags.counter = %d\n".as_ptr(),
        (*state).flags.counter as c_int,
    );
    printf(
        c"Bit fields - flag1:%d flag2:%d flag3:%d mode:%d\n".as_ptr(),
        (*state).flags.flag1 as c_int,
        (*state).flags.flag2 as c_int,
        (*state).flags.flag3 as c_int,
        (*state).flags.mode as c_int,
    );
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn confuse_types(state: *mut ProcessState, operation: c_int) -> c_int {
    if state.is_null() {
        return 0;
    }

    let mut result: c_int = 0;

    match operation {
        0 => {
            (*state).data.int_val = 1078530011;
            printf(
                c"Set as int: %d\n".as_ptr(),
                (*state).data.int_val,
            );
        }
        1 => {
            // printf %f promotes float to double
            printf(
                c"Read as float: %f\n".as_ptr(),
                (*state).data.float_val as f64,
            );
            result = ((*state).data.float_val * 100.0) as c_int;
        }
        2 => {
            printf(
                c"Read as uint: %u\n".as_ptr(),
                (*state).data.uint_val as c_uint,
            );
            result = ((*state).data.uint_val & 0xFF) as c_int;
        }
        3 => {
            printf(
                c"Read as bytes: [%d, %d, %d, %d]\n".as_ptr(),
                (*state).data.bytes[0] as c_int,
                (*state).data.bytes[1] as c_int,
                (*state).data.bytes[2] as c_int,
                (*state).data.bytes[3] as c_int,
            );
            result =
                (*state).data.bytes[0] as c_int + (*state).data.bytes[1] as c_int;
        }
        _ => {}
    }

    result
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn confusion(
    param1: c_int,
    param2: c_int,
    param3: c_int,
    param4: c_int,
) -> c_int {
    // DEBUG_VAR for each param (different stringified names)
    printf(c"Debug: param1 = %d\n".as_ptr(), param1);
    printf(c"Debug: param2 = %d\n".as_ptr(), param2);
    printf(c"Debug: param3 = %d\n".as_ptr(), param3);
    printf(c"Debug: param4 = %d\n".as_ptr(), param4);

    let mut result: c_int = 0;

    let state = create_state(param1, 128);

    if state.is_null() {
        return -1;
    }

    update_flags(state, param2);

    // char search_char = '0' + (param3 % 10);
    let search_char: c_char =
        ((b'0' as c_int).wrapping_add(param3 % 10)) as c_char;
    let found_count = process_buffer(state, search_char);
    result += found_count * 10;

    let confusion_result = confuse_types(state, param4 % 4);
    result += confusion_result;

    result += ((*state).flags.counter as c_int) * 5;
    result += ((*state).flags.mode as c_int) * 3;

    printf(c"Final result: %d\n".as_ptr(), result);

    destroy_state(state);

    result
}
