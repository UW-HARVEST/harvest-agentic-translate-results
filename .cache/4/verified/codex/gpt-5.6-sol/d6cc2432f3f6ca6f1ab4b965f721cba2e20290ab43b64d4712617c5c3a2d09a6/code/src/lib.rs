use std::ffi::{c_char, c_int, c_uint, c_void};
use std::ptr;

unsafe extern "C" {
    fn free(ptr: *mut c_void);
    fn malloc(size: usize) -> *mut c_void;
    fn memchr(ptr: *const c_void, value: c_int, size: usize) -> *mut c_void;
    fn printf(format: *const c_char, ...) -> c_int;
    fn snprintf(buffer: *mut c_char, size: usize, format: *const c_char, ...) -> c_int;
    fn strlen(value: *const c_char) -> usize;
}

const FLAG1_SHIFT: c_uint = 0;
const FLAG2_SHIFT: c_uint = 1;
const FLAG3_SHIFT: c_uint = 2;
const COUNTER_SHIFT: c_uint = 3;
const MODE_SHIFT: c_uint = 8;
const INITIAL_FLAGS: c_uint = 1 | (1 << FLAG3_SHIFT) | (3 << MODE_SHIFT) | (15 << 11);

const DEBUG_PARAM1: &[u8] = b"Debug: param1 = %d\n\0";
const DEBUG_PARAM2: &[u8] = b"Debug: param2 = %d\n\0";
const DEBUG_PARAM3: &[u8] = b"Debug: param3 = %d\n\0";
const DEBUG_PARAM4: &[u8] = b"Debug: param4 = %d\n\0";
const DEBUG_COUNTER: &[u8] = b"Debug: state->flags.counter = %d\n\0";
const ERROR_BUFFER: &[u8] = b"Error: Failed to allocate buffer\n\0";
const ERROR_PROCESS: &[u8] = b"Error: Null pointer in process_buffer\n\0";
const ERROR_STATE: &[u8] = b"Error: Failed to allocate memory for state\n\0";
const FINAL_RESULT: &[u8] = b"Final result: %d\n\0";
const FLAGS: &[u8] = b"Bit fields - flag1:%d flag2:%d flag3:%d mode:%d\n\0";
const OPERATION: &[u8] = b"Operation: memchr_found with value %d\n\0";
const READ_BYTES: &[u8] = b"Read as bytes: [%d, %d, %d, %d]\n\0";
const READ_FLOAT: &[u8] = b"Read as float: %f\n\0";
const READ_UINT: &[u8] = b"Read as uint: %u\n\0";
const SET_INT: &[u8] = b"Set as int: %d\n\0";
const STATE_FORMAT: &[u8] = b"State:%d:Mode:%d\0";

#[repr(C)]
pub struct PackedFlags {
    bits: c_uint,
}

impl PackedFlags {
    fn field(&self, shift: c_uint, mask: c_uint) -> c_uint {
        (self.bits >> shift) & mask
    }

    fn set_field(&mut self, shift: c_uint, mask: c_uint, value: c_uint) {
        self.bits = (self.bits & !(mask << shift)) | ((value & mask) << shift);
    }

    fn counter(&self) -> c_uint {
        self.field(COUNTER_SHIFT, 0x1f)
    }

    fn mode(&self) -> c_uint {
        self.field(MODE_SHIFT, 0x07)
    }
}

#[repr(C)]
pub union TypeConfusion {
    int_val: c_int,
    float_val: f32,
    uint_val: c_uint,
    bytes: [c_char; 4],
}

#[repr(C)]
pub struct ProcessState {
    flags: PackedFlags,
    data: TypeConfusion,
    buffer: *mut c_char,
    capacity: c_int,
}

fn format_ptr(value: &[u8]) -> *const c_char {
    value.as_ptr().cast()
}

#[cfg(target_arch = "x86_64")]
fn c_float_to_int(value: f32) -> c_int {
    // GCC uses CVTTSS2SI here, including its INT_MIN result for invalid conversions.
    unsafe { std::arch::x86_64::_mm_cvttss_si32(std::arch::x86_64::_mm_set_ss(value)) }
}

#[cfg(not(target_arch = "x86_64"))]
fn c_float_to_int(value: f32) -> c_int {
    value as c_int
}

#[unsafe(no_mangle)]
pub extern "C" fn create_state(initial_val: c_int, capacity: c_int) -> *mut ProcessState {
    let state = unsafe { malloc(size_of::<ProcessState>()) }.cast::<ProcessState>();

    if state.is_null() {
        unsafe {
            printf(format_ptr(ERROR_STATE));
        }
        return ptr::null_mut();
    }

    unsafe {
        ptr::addr_of_mut!((*state).flags).write(PackedFlags {
            bits: INITIAL_FLAGS,
        });
        ptr::addr_of_mut!((*state).data).write(TypeConfusion {
            int_val: initial_val,
        });
        ptr::addr_of_mut!((*state).capacity).write(capacity);

        let buffer = malloc(capacity as usize).cast::<c_char>();
        ptr::addr_of_mut!((*state).buffer).write(buffer);

        if buffer.is_null() {
            printf(format_ptr(ERROR_BUFFER));
            free(state.cast());
            return ptr::null_mut();
        }

        snprintf(
            buffer,
            capacity as usize,
            format_ptr(STATE_FORMAT),
            initial_val,
            3 as c_int,
        );
    }

    state
}

#[unsafe(no_mangle)]
pub extern "C" fn destroy_state(state: *mut ProcessState) {
    if state.is_null() {
        return;
    }

    unsafe {
        if !(*state).buffer.is_null() {
            free((*state).buffer.cast());
        }
        free(state.cast());
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn process_buffer(state: *mut ProcessState, target: c_char) -> c_int {
    if state.is_null() || unsafe { (*state).buffer.is_null() } {
        unsafe {
            printf(format_ptr(ERROR_PROCESS));
        }
        return -1;
    }

    let mut count: c_int = 0;
    let mut current = unsafe { (*state).buffer };
    let mut remaining = unsafe { strlen(current) };

    while remaining > 0 {
        let found = unsafe { memchr(current.cast(), target as c_int, remaining) }.cast::<c_char>();

        if found.is_null() {
            break;
        }

        count = count.wrapping_add(1);
        unsafe {
            printf(format_ptr(OPERATION), count);
        }

        let consumed = unsafe { found.offset_from(current) as usize + 1 };
        remaining -= consumed;
        current = unsafe { found.add(1) };
    }

    count
}

#[unsafe(no_mangle)]
pub extern "C" fn update_flags(state: *mut ProcessState, param: c_int) {
    if state.is_null() {
        return;
    }

    unsafe {
        let flags = &mut (*state).flags;
        let counter = flags.counter().wrapping_add(1) & 0x1f;
        flags.set_field(COUNTER_SHIFT, 0x1f, counter);
        flags.set_field(FLAG1_SHIFT, 1, (param & 1) as c_uint);
        flags.set_field(FLAG2_SHIFT, 1, ((param & 2) >> 1) as c_uint);
        flags.set_field(FLAG3_SHIFT, 1, ((param & 4) >> 2) as c_uint);
        flags.set_field(MODE_SHIFT, 7, ((param >> 3) & 7) as c_uint);

        printf(format_ptr(DEBUG_COUNTER), flags.counter() as c_int);
        printf(
            format_ptr(FLAGS),
            flags.field(FLAG1_SHIFT, 1) as c_int,
            flags.field(FLAG2_SHIFT, 1) as c_int,
            flags.field(FLAG3_SHIFT, 1) as c_int,
            flags.mode() as c_int,
        );
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn confuse_types(state: *mut ProcessState, operation: c_int) -> c_int {
    if state.is_null() {
        return 0;
    }

    let mut result: c_int = 0;

    unsafe {
        match operation {
            0 => {
                (*state).data.int_val = 1_078_530_011;
                printf(format_ptr(SET_INT), (*state).data.int_val);
            }
            1 => {
                let value = (*state).data.float_val;
                printf(format_ptr(READ_FLOAT), value as f64);
                result = c_float_to_int(value * 100.0_f32);
            }
            2 => {
                let value = (*state).data.uint_val;
                printf(format_ptr(READ_UINT), value);
                result = (value & 0xff) as c_int;
            }
            3 => {
                let bytes = (*state).data.bytes;
                printf(
                    format_ptr(READ_BYTES),
                    bytes[0] as c_int,
                    bytes[1] as c_int,
                    bytes[2] as c_int,
                    bytes[3] as c_int,
                );
                result = (bytes[0] as c_int).wrapping_add(bytes[1] as c_int);
            }
            _ => {}
        }
    }

    result
}

#[unsafe(no_mangle)]
pub extern "C" fn confusion(param1: c_int, param2: c_int, param3: c_int, param4: c_int) -> c_int {
    unsafe {
        printf(format_ptr(DEBUG_PARAM1), param1);
        printf(format_ptr(DEBUG_PARAM2), param2);
        printf(format_ptr(DEBUG_PARAM3), param3);
        printf(format_ptr(DEBUG_PARAM4), param4);
    }

    let state = create_state(param1, 128);
    if state.is_null() {
        return -1;
    }

    update_flags(state, param2);

    let search_char = (b'0' as c_int).wrapping_add(param3 % 10) as c_char;
    let found_count = process_buffer(state, search_char);
    let mut result = found_count.wrapping_mul(10);

    let confusion_result = confuse_types(state, param4 % 4);
    result = result.wrapping_add(confusion_result);

    unsafe {
        result = result.wrapping_add(((*state).flags.counter() as c_int).wrapping_mul(5));
        result = result.wrapping_add(((*state).flags.mode() as c_int).wrapping_mul(3));
        printf(format_ptr(FINAL_RESULT), result);
    }

    destroy_state(state);
    result
}
