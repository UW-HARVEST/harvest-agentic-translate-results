// Translation of c_src/src/lib.c to Rust producing byte-identical output.
//
// We use the C runtime's printf/snprintf via FFI to guarantee that the
// formatting (especially %f, %u, etc.) matches the original C output exactly.

use std::ffi::{c_char, c_int, c_uint};

extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
    fn snprintf(s: *mut c_char, n: usize, fmt: *const c_char, ...) -> c_int;
    fn malloc(size: usize) -> *mut std::ffi::c_void;
    fn free(ptr: *mut std::ffi::c_void);
    fn strlen(s: *const c_char) -> usize;
    fn memchr(s: *const std::ffi::c_void, c: c_int, n: usize) -> *mut std::ffi::c_void;
}

// Mirrors the bit field struct from the C code.  We keep each field as a u32
// and apply masking explicitly so behaviour matches the bit-width semantics
// of the original C bit fields.
#[repr(C)]
pub struct PackedFlags {
    flag1: u32,    // 1 bit
    flag2: u32,    // 1 bit
    flag3: u32,    // 1 bit
    counter: u32,  // 5 bits
    mode: u32,     // 3 bits
    status: u32,   // 5 bits
    reserved: u32, // 16 bits
}

// The TypeConfusion union -- four bytes shared between several views.
#[repr(C)]
pub union TypeConfusion {
    int_val: i32,
    float_val: f32,
    uint_val: u32,
    bytes: [c_char; 4],
}

#[repr(C)]
pub struct ProcessState {
    flags: PackedFlags,
    data: TypeConfusion,
    buffer: *mut c_char,
    capacity: c_int,
}

#[no_mangle]
pub unsafe extern "C" fn create_state(
    initial_val: c_int,
    capacity: c_int,
) -> *mut ProcessState {
    let state = malloc(std::mem::size_of::<ProcessState>()) as *mut ProcessState;

    if state.is_null() {
        printf(b"Error: Failed to allocate memory for state\n\0".as_ptr() as *const c_char);
        return std::ptr::null_mut();
    }

    (*state).flags.flag1 = 1;
    (*state).flags.flag2 = 0;
    (*state).flags.flag3 = 1;
    (*state).flags.counter = 0;
    (*state).flags.mode = 3;
    (*state).flags.status = 15;
    (*state).flags.reserved = 0;

    (*state).data.int_val = initial_val as i32;

    (*state).capacity = capacity;
    (*state).buffer = malloc(capacity as usize) as *mut c_char;

    if (*state).buffer.is_null() {
        printf(b"Error: Failed to allocate buffer\n\0".as_ptr() as *const c_char);
        free(state as *mut std::ffi::c_void);
        return std::ptr::null_mut();
    }

    snprintf(
        (*state).buffer,
        capacity as usize,
        b"State:%d:Mode:%d\0".as_ptr() as *const c_char,
        initial_val as c_int,
        (*state).flags.mode as c_int,
    );

    state
}

#[no_mangle]
pub unsafe extern "C" fn destroy_state(state: *mut ProcessState) {
    if !state.is_null() {
        if !(*state).buffer.is_null() {
            free((*state).buffer as *mut std::ffi::c_void);
        }
        free(state as *mut std::ffi::c_void);
    }
}

#[no_mangle]
pub unsafe extern "C" fn process_buffer(state: *mut ProcessState, target: c_char) -> c_int {
    if state.is_null() || (*state).buffer.is_null() {
        printf(b"Error: Null pointer in process_buffer\n\0".as_ptr() as *const c_char);
        return -1;
    }

    let mut count: c_int = 0;
    let mut ptr: *mut c_char = (*state).buffer;
    let mut remaining: usize = strlen((*state).buffer);

    while remaining > 0 {
        let found = memchr(
            ptr as *const std::ffi::c_void,
            target as c_int,
            remaining,
        ) as *mut c_char;

        if found.is_null() {
            break;
        }

        count += 1;
        printf(
            b"Operation: memchr_found with value %d\n\0".as_ptr() as *const c_char,
            count as c_int,
        );

        // remaining -= (found - ptr + 1);
        let diff = (found as usize) - (ptr as usize);
        remaining -= diff + 1;
        ptr = found.add(1);
    }

    count
}

#[no_mangle]
pub unsafe extern "C" fn update_flags(state: *mut ProcessState, param: c_int) {
    if state.is_null() {
        return;
    }

    (*state).flags.counter = ((*state).flags.counter.wrapping_add(1)) & 0x1F;
    (*state).flags.flag1 = (param & 1) as u32;
    (*state).flags.flag2 = (((param & 2) >> 1) as u32) & 0x1;
    (*state).flags.flag3 = (((param & 4) >> 2) as u32) & 0x1;
    (*state).flags.mode = ((param >> 3) & 0x7) as u32;

    printf(
        b"Debug: state->flags.counter = %d\n\0".as_ptr() as *const c_char,
        (*state).flags.counter as c_int,
    );
    printf(
        b"Bit fields - flag1:%d flag2:%d flag3:%d mode:%d\n\0".as_ptr() as *const c_char,
        (*state).flags.flag1 as c_int,
        (*state).flags.flag2 as c_int,
        (*state).flags.flag3 as c_int,
        (*state).flags.mode as c_int,
    );
}

#[no_mangle]
pub unsafe extern "C" fn confuse_types(state: *mut ProcessState, operation: c_int) -> c_int {
    if state.is_null() {
        return 0;
    }

    let mut result: c_int = 0;

    match operation {
        0 => {
            (*state).data.int_val = 1078530011;
            printf(
                b"Set as int: %d\n\0".as_ptr() as *const c_char,
                (*state).data.int_val as c_int,
            );
        }
        1 => {
            // %f promotes to double via varargs
            printf(
                b"Read as float: %f\n\0".as_ptr() as *const c_char,
                (*state).data.float_val as f64,
            );
            // (int)(float * 100) -- truncation toward zero.
            let prod: f32 = (*state).data.float_val * 100.0f32;
            result = prod as i32 as c_int;
        }
        2 => {
            printf(
                b"Read as uint: %u\n\0".as_ptr() as *const c_char,
                (*state).data.uint_val as c_uint,
            );
            result = ((*state).data.uint_val & 0xFF) as c_int;
        }
        3 => {
            // bytes are signed char on Linux x86_64; promoted to int by varargs.
            let b0 = (*state).data.bytes[0] as c_int;
            let b1 = (*state).data.bytes[1] as c_int;
            let b2 = (*state).data.bytes[2] as c_int;
            let b3 = (*state).data.bytes[3] as c_int;
            printf(
                b"Read as bytes: [%d, %d, %d, %d]\n\0".as_ptr() as *const c_char,
                b0,
                b1,
                b2,
                b3,
            );
            // C: result = bytes[0] + bytes[1] -- both promoted to int before sum.
            result = ((*state).data.bytes[0] as i32) + ((*state).data.bytes[1] as i32);
        }
        _ => {}
    }

    result
}

#[no_mangle]
pub unsafe extern "C" fn confusion(
    param1: c_int,
    param2: c_int,
    param3: c_int,
    param4: c_int,
) -> c_int {
    printf(
        b"Debug: param1 = %d\n\0".as_ptr() as *const c_char,
        param1,
    );
    printf(
        b"Debug: param2 = %d\n\0".as_ptr() as *const c_char,
        param2,
    );
    printf(
        b"Debug: param3 = %d\n\0".as_ptr() as *const c_char,
        param3,
    );
    printf(
        b"Debug: param4 = %d\n\0".as_ptr() as *const c_char,
        param4,
    );

    let mut result: c_int = 0;

    let state = create_state(param1, 128);

    if state.is_null() {
        return -1;
    }

    update_flags(state, param2);

    let search_char: c_char = ((b'0' as c_int) + (param3 % 10)) as c_char;
    let found_count = process_buffer(state, search_char);
    result = result.wrapping_add(found_count.wrapping_mul(10));

    let confusion_result = confuse_types(state, param4 % 4);
    result = result.wrapping_add(confusion_result);

    result = result.wrapping_add(((*state).flags.counter as c_int).wrapping_mul(5));
    result = result.wrapping_add(((*state).flags.mode as c_int).wrapping_mul(3));

    printf(
        b"Final result: %d\n\0".as_ptr() as *const c_char,
        result as c_int,
    );

    destroy_state(state);

    result
}
