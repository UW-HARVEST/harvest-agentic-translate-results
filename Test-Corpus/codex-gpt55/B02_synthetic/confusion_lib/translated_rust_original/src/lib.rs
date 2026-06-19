use std::ffi::{c_char, c_float, c_int, c_uint, c_void};
use std::mem::size_of;
use std::ptr;

#[repr(C)]
union TypeConfusion {
    int_val: c_int,
    float_val: c_float,
    uint_val: c_uint,
    bytes: [c_char; 4],
}

#[repr(C)]
pub struct ProcessState {
    flags: c_uint,
    data: TypeConfusion,
    buffer: *mut c_char,
    capacity: c_int,
}

unsafe extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
    fn printf(format: *const c_char, ...) -> c_int;
    fn snprintf(s: *mut c_char, n: usize, format: *const c_char, ...) -> c_int;
    fn strlen(s: *const c_char) -> usize;
    fn memchr(s: *const c_void, c: c_int, n: usize) -> *mut c_void;
}

const FLAG1_SHIFT: u32 = 0;
const FLAG2_SHIFT: u32 = 1;
const FLAG3_SHIFT: u32 = 2;
const COUNTER_SHIFT: u32 = 3;
const MODE_SHIFT: u32 = 8;
const STATUS_SHIFT: u32 = 11;

unsafe fn set_bits(flags: *mut c_uint, shift: u32, width: u32, value: c_uint) {
    let mask = ((1u32 << width) - 1) << shift;
    unsafe {
        *flags = (*flags & !mask) | ((value << shift) & mask);
    }
}

unsafe fn get_bits(flags: c_uint, shift: u32, width: u32) -> c_uint {
    (flags >> shift) & ((1u32 << width) - 1)
}

unsafe fn set_flag1(state: *mut ProcessState, value: c_uint) {
    unsafe { set_bits(ptr::addr_of_mut!((*state).flags), FLAG1_SHIFT, 1, value) };
}

unsafe fn set_flag2(state: *mut ProcessState, value: c_uint) {
    unsafe { set_bits(ptr::addr_of_mut!((*state).flags), FLAG2_SHIFT, 1, value) };
}

unsafe fn set_flag3(state: *mut ProcessState, value: c_uint) {
    unsafe { set_bits(ptr::addr_of_mut!((*state).flags), FLAG3_SHIFT, 1, value) };
}

unsafe fn set_counter(state: *mut ProcessState, value: c_uint) {
    unsafe { set_bits(ptr::addr_of_mut!((*state).flags), COUNTER_SHIFT, 5, value) };
}

unsafe fn set_mode(state: *mut ProcessState, value: c_uint) {
    unsafe { set_bits(ptr::addr_of_mut!((*state).flags), MODE_SHIFT, 3, value) };
}

unsafe fn set_status(state: *mut ProcessState, value: c_uint) {
    unsafe { set_bits(ptr::addr_of_mut!((*state).flags), STATUS_SHIFT, 5, value) };
}

unsafe fn set_reserved(state: *mut ProcessState, value: c_uint) {
    unsafe { set_bits(ptr::addr_of_mut!((*state).flags), 16, 16, value) };
}

unsafe fn flag1(state: *const ProcessState) -> c_uint {
    unsafe { get_bits((*state).flags, FLAG1_SHIFT, 1) }
}

unsafe fn flag2(state: *const ProcessState) -> c_uint {
    unsafe { get_bits((*state).flags, FLAG2_SHIFT, 1) }
}

unsafe fn flag3(state: *const ProcessState) -> c_uint {
    unsafe { get_bits((*state).flags, FLAG3_SHIFT, 1) }
}

unsafe fn counter(state: *const ProcessState) -> c_uint {
    unsafe { get_bits((*state).flags, COUNTER_SHIFT, 5) }
}

unsafe fn mode(state: *const ProcessState) -> c_uint {
    unsafe { get_bits((*state).flags, MODE_SHIFT, 3) }
}

fn c_float_to_int(value: c_float) -> c_int {
    if value.is_nan() || value >= 2_147_483_648.0 || value < -2_147_483_648.0 {
        c_int::MIN
    } else {
        value as c_int
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn create_state(initial_val: c_int, capacity: c_int) -> *mut ProcessState {
    let state = unsafe { malloc(size_of::<ProcessState>()) as *mut ProcessState };

    if state.is_null() {
        unsafe {
            printf(c"Error: Failed to allocate memory for state\n".as_ptr());
        }
        return ptr::null_mut();
    }

    unsafe {
        (*state).flags = 0;
        set_flag1(state, 1);
        set_flag2(state, 0);
        set_flag3(state, 1);
        set_counter(state, 0);
        set_mode(state, 3);
        set_status(state, 15);
        set_reserved(state, 0);

        (*state).data.int_val = initial_val;

        (*state).capacity = capacity;
        (*state).buffer = malloc(capacity as usize) as *mut c_char;

        if (*state).buffer.is_null() {
            printf(c"Error: Failed to allocate buffer\n".as_ptr());
            free(state as *mut c_void);
            return ptr::null_mut();
        }

        snprintf(
            (*state).buffer,
            capacity as usize,
            c"State:%d:Mode:%d".as_ptr(),
            initial_val,
            mode(state) as c_int,
        );
    }

    state
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn destroy_state(state: *mut ProcessState) {
    if !state.is_null() {
        unsafe {
            if !(*state).buffer.is_null() {
                free((*state).buffer as *mut c_void);
            }
            free(state as *mut c_void);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn process_buffer(state: *mut ProcessState, target: c_char) -> c_int {
    if state.is_null() || unsafe { (*state).buffer.is_null() } {
        unsafe {
            printf(c"Error: Null pointer in process_buffer\n".as_ptr());
        }
        return -1;
    }

    let mut count: c_int = 0;
    let mut ptr = unsafe { (*state).buffer };
    let mut remaining = unsafe { strlen((*state).buffer) };

    while remaining > 0 {
        let found = unsafe { memchr(ptr as *const c_void, target as c_int, remaining) as *mut c_char };

        if found.is_null() {
            break;
        }

        count += 1;
        unsafe {
            printf(c"Operation: memchr_found with value %d\n".as_ptr(), count);
        }

        let consumed = unsafe { found.offset_from(ptr) as usize + 1 };
        remaining -= consumed;
        ptr = unsafe { found.add(1) };
    }

    count
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn update_flags(state: *mut ProcessState, param: c_int) {
    if state.is_null() {
        return;
    }

    unsafe {
        set_counter(state, (counter(state) + 1) & 0x1F);
        set_flag1(state, (param & 1) as c_uint);
        set_flag2(state, ((param & 2) >> 1) as c_uint);
        set_flag3(state, ((param & 4) >> 2) as c_uint);
        set_mode(state, ((param >> 3) & 0x7) as c_uint);

        printf(
            c"Debug: state->flags.counter = %d\n".as_ptr(),
            counter(state) as c_int,
        );
        printf(
            c"Bit fields - flag1:%d flag2:%d flag3:%d mode:%d\n".as_ptr(),
            flag1(state) as c_int,
            flag2(state) as c_int,
            flag3(state) as c_int,
            mode(state) as c_int,
        );
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn confuse_types(state: *mut ProcessState, operation: c_int) -> c_int {
    if state.is_null() {
        return 0;
    }

    let mut result: c_int = 0;

    unsafe {
        match operation {
            0 => {
                (*state).data.int_val = 1078530011;
                printf(c"Set as int: %d\n".as_ptr(), (*state).data.int_val);
            }
            1 => {
                printf(
                    c"Read as float: %f\n".as_ptr(),
                    (*state).data.float_val as f64,
                );
                result = c_float_to_int((*state).data.float_val * 100.0);
            }
            2 => {
                printf(c"Read as uint: %u\n".as_ptr(), (*state).data.uint_val);
                result = ((*state).data.uint_val & 0xFF) as c_int;
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
    unsafe {
        printf(c"Debug: param1 = %d\n".as_ptr(), param1);
        printf(c"Debug: param2 = %d\n".as_ptr(), param2);
        printf(c"Debug: param3 = %d\n".as_ptr(), param3);
        printf(c"Debug: param4 = %d\n".as_ptr(), param4);
    }

    let mut result: c_int = 0;

    let state = unsafe { create_state(param1, 128) };

    if state.is_null() {
        return -1;
    }

    unsafe {
        update_flags(state, param2);

        let search_char = b'0' as c_int + (param3 % 10);
        let found_count = process_buffer(state, search_char as c_char);
        result += found_count * 10;

        let confusion_result = confuse_types(state, param4 % 4);
        result += confusion_result;

        result += counter(state) as c_int * 5;
        result += mode(state) as c_int * 3;

        printf(c"Final result: %d\n".as_ptr(), result);

        destroy_state(state);
    }

    result
}
