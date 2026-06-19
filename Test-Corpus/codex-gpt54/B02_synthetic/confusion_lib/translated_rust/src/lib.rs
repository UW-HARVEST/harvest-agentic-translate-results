use libc::{free, malloc, memchr, printf, snprintf, strlen};
use std::ffi::{c_char, c_int, c_uint, c_void};
use std::mem::size_of;

#[repr(C)]
struct PackedFlags {
    flag1: c_uint,
    flag2: c_uint,
    flag3: c_uint,
    counter: c_uint,
    mode: c_uint,
    status: c_uint,
    reserved: c_uint,
}

#[repr(C)]
union TypeConfusion {
    int_val: c_int,
    float_val: f32,
    uint_val: c_uint,
    bytes: [c_char; 4],
}

#[repr(C)]
struct ProcessState {
    flags: PackedFlags,
    data: TypeConfusion,
    buffer: *mut c_char,
    capacity: c_int,
}

unsafe fn create_state(initial_val: c_int, capacity: c_int) -> *mut ProcessState {
    unsafe {
        let state = malloc(size_of::<ProcessState>()) as *mut ProcessState;

        if state.is_null() {
            printf(c"Error: Failed to allocate memory for state\n".as_ptr());
            return std::ptr::null_mut();
        }

        (*state).flags.flag1 = 1;
        (*state).flags.flag2 = 0;
        (*state).flags.flag3 = 1;
        (*state).flags.counter = 0;
        (*state).flags.mode = 3;
        (*state).flags.status = 15;
        (*state).flags.reserved = 0;

        (*state).data.int_val = initial_val;

        (*state).capacity = capacity;
        (*state).buffer = malloc(capacity as usize) as *mut c_char;

        if (*state).buffer.is_null() {
            printf(c"Error: Failed to allocate buffer\n".as_ptr());
            free(state.cast::<c_void>());
            return std::ptr::null_mut();
        }

        snprintf(
            (*state).buffer,
            capacity as usize,
            c"State:%d:Mode:%d".as_ptr(),
            initial_val,
            (*state).flags.mode as c_int,
        );

        state
    }
}

unsafe fn destroy_state(state: *mut ProcessState) {
    unsafe {
        if !state.is_null() {
            if !(*state).buffer.is_null() {
                free((*state).buffer.cast::<c_void>());
            }
            free(state.cast::<c_void>());
        }
    }
}

unsafe fn process_buffer(state: *mut ProcessState, target: c_char) -> c_int {
    unsafe {
        if state.is_null() || (*state).buffer.is_null() {
            printf(c"Error: Null pointer in process_buffer\n".as_ptr());
            return -1;
        }

        let mut count: c_int = 0;
        let mut ptr = (*state).buffer;
        let mut remaining = strlen((*state).buffer);

        while remaining > 0 {
            let found = memchr(ptr.cast::<c_void>(), target as c_int, remaining) as *mut c_char;

            if found.is_null() {
                break;
            }

            count += 1;
            printf(
                c"Operation: memchr_found with value %d\n".as_ptr(),
                count,
            );

            remaining -= found.offset_from(ptr) as usize + 1;
            ptr = found.add(1);
        }

        count
    }
}

unsafe fn update_flags(state: *mut ProcessState, param: c_int) {
    unsafe {
        if state.is_null() {
            return;
        }

        (*state).flags.counter = ((*state).flags.counter + 1) & 0x1f;
        (*state).flags.flag1 = (param & 1) as c_uint;
        (*state).flags.flag2 = ((param & 2) >> 1) as c_uint;
        (*state).flags.flag3 = ((param & 4) >> 2) as c_uint;
        (*state).flags.mode = ((param >> 3) & 0x7) as c_uint;

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
}

unsafe fn confuse_types(state: *mut ProcessState, operation: c_int) -> c_int {
    unsafe {
        if state.is_null() {
            return 0;
        }

        let mut result: c_int = 0;

        match operation {
            0 => {
                (*state).data.int_val = 1_078_530_011;
                printf(c"Set as int: %d\n".as_ptr(), (*state).data.int_val);
            }
            1 => {
                let float_val = (*state).data.float_val;
                printf(c"Read as float: %f\n".as_ptr(), float_val as f64);
                result = (float_val * 100.0) as c_int;
            }
            2 => {
                let uint_val = (*state).data.uint_val;
                printf(c"Read as uint: %u\n".as_ptr(), uint_val);
                result = (uint_val & 0xff) as c_int;
            }
            3 => {
                let bytes = (*state).data.bytes;
                printf(
                    c"Read as bytes: [%d, %d, %d, %d]\n".as_ptr(),
                    bytes[0] as c_int,
                    bytes[1] as c_int,
                    bytes[2] as c_int,
                    bytes[3] as c_int,
                );
                result = bytes[0] as c_int + bytes[1] as c_int;
            }
            _ => {}
        }

        result
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn confusion(param1: c_int, param2: c_int, param3: c_int, param4: c_int) -> c_int {
    unsafe {
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

        let search_char = ((b'0' as c_int) + (param3 % 10)) as c_char;
        let found_count = process_buffer(state, search_char);
        result += found_count * 10;

        let confusion_result = confuse_types(state, param4 % 4);
        result += confusion_result;

        result += ((*state).flags.counter * 5) as c_int;
        result += ((*state).flags.mode * 3) as c_int;

        printf(c"Final result: %d\n".as_ptr(), result);

        destroy_state(state);

        result
    }
}
