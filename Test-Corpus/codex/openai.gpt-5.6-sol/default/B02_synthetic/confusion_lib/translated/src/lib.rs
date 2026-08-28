use std::ffi::{c_char, c_int, c_uint, c_void};
use std::mem::size_of;
use std::ptr;

const FLAG1_MASK: c_uint = 1 << 0;
const FLAG2_MASK: c_uint = 1 << 1;
const FLAG3_MASK: c_uint = 1 << 2;
const COUNTER_MASK: c_uint = 0x1f << 3;
const MODE_MASK: c_uint = 0x07 << 8;

const INITIAL_FLAGS: c_uint = FLAG1_MASK | FLAG3_MASK | (3 << 8) | (15 << 11);

#[repr(C)]
pub union TypeConfusion {
    int_val: c_int,
    float_val: f32,
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
    fn snprintf(buffer: *mut c_char, size: usize, format: *const c_char, ...) -> c_int;
    fn strlen(value: *const c_char) -> usize;
    fn memchr(value: *const c_void, byte: c_int, count: usize) -> *mut c_void;
}

#[inline]
unsafe fn set_masked(flags: *mut c_uint, mask: c_uint, value: c_uint) {
    unsafe {
        let current = flags.read();
        flags.write((current & !mask) | (value & mask));
    }
}

#[inline]
unsafe fn flags_ptr(state: *mut ProcessState) -> *mut c_uint {
    unsafe { ptr::addr_of_mut!((*state).flags) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn create_state(initial_val: c_int, capacity: c_int) -> *mut ProcessState {
    let state = unsafe { malloc(size_of::<ProcessState>()) }.cast::<ProcessState>();

    if state.is_null() {
        unsafe {
            printf(c"Error: Failed to allocate memory for state\n".as_ptr());
        }
        return ptr::null_mut();
    }

    unsafe {
        ptr::addr_of_mut!((*state).flags).write(INITIAL_FLAGS);
        ptr::addr_of_mut!((*state).data).write(TypeConfusion {
            int_val: initial_val,
        });
        ptr::addr_of_mut!((*state).capacity).write(capacity);

        let buffer = malloc(capacity as usize).cast::<c_char>();
        ptr::addr_of_mut!((*state).buffer).write(buffer);

        if buffer.is_null() {
            printf(c"Error: Failed to allocate buffer\n".as_ptr());
            free(state.cast());
            return ptr::null_mut();
        }

        snprintf(
            buffer,
            capacity as usize,
            c"State:%d:Mode:%d".as_ptr(),
            initial_val,
            3 as c_int,
        );
    }

    state
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn destroy_state(state: *mut ProcessState) {
    if !state.is_null() {
        unsafe {
            let buffer = ptr::addr_of!((*state).buffer).read();
            if !buffer.is_null() {
                free(buffer.cast());
            }
            free(state.cast());
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn process_buffer(state: *mut ProcessState, target: c_char) -> c_int {
    if state.is_null() {
        unsafe {
            printf(c"Error: Null pointer in process_buffer\n".as_ptr());
        }
        return -1;
    }

    let buffer = unsafe { ptr::addr_of!((*state).buffer).read() };
    if buffer.is_null() {
        unsafe {
            printf(c"Error: Null pointer in process_buffer\n".as_ptr());
        }
        return -1;
    }

    let mut count: c_int = 0;
    let mut current = buffer;
    let mut remaining = unsafe { strlen(buffer) };

    while remaining > 0 {
        let found = unsafe { memchr(current.cast(), target as c_int, remaining) }.cast::<c_char>();

        if found.is_null() {
            break;
        }

        count = count.wrapping_add(1);
        unsafe {
            printf(c"Operation: memchr_found with value %d\n".as_ptr(), count);
        }

        let consumed = unsafe { found.offset_from(current) as usize + 1 };
        remaining -= consumed;
        current = unsafe { found.add(1) };
    }

    count
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn update_flags(state: *mut ProcessState, param: c_int) {
    if state.is_null() {
        return;
    }

    unsafe {
        let flags = flags_ptr(state);
        let counter = ((flags.read() >> 3).wrapping_add(1)) & 0x1f;
        set_masked(flags, COUNTER_MASK, counter << 3);
        set_masked(flags, FLAG1_MASK, (param & 1) as c_uint);
        set_masked(flags, FLAG2_MASK, (param & 2) as c_uint);
        set_masked(flags, FLAG3_MASK, (param & 4) as c_uint);
        set_masked(flags, MODE_MASK, ((param >> 3) as c_uint & 0x07) << 8);

        let packed = flags.read();
        let flag1 = (packed & FLAG1_MASK) as c_int;
        let flag2 = ((packed & FLAG2_MASK) >> 1) as c_int;
        let flag3 = ((packed & FLAG3_MASK) >> 2) as c_int;
        let mode = ((packed & MODE_MASK) >> 8) as c_int;

        printf(
            c"Debug: state->flags.counter = %d\n".as_ptr(),
            counter as c_int,
        );
        printf(
            c"Bit fields - flag1:%d flag2:%d flag3:%d mode:%d\n".as_ptr(),
            flag1,
            flag2,
            flag3,
            mode,
        );
    }
}

#[inline]
fn float_to_c_int(value: f32) -> c_int {
    #[cfg(target_arch = "x86_64")]
    unsafe {
        use std::arch::x86_64::{_mm_cvttss_si32, _mm_set_ss};
        _mm_cvttss_si32(_mm_set_ss(value))
    }

    #[cfg(target_arch = "x86")]
    unsafe {
        use std::arch::x86::{_mm_cvttss_si32, _mm_set_ss};
        _mm_cvttss_si32(_mm_set_ss(value))
    }

    #[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
    {
        value as c_int
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn confuse_types(state: *mut ProcessState, operation: c_int) -> c_int {
    if state.is_null() {
        return 0;
    }

    unsafe {
        let data = ptr::addr_of_mut!((*state).data);

        match operation {
            0 => {
                ptr::addr_of_mut!((*data).int_val).write(1_078_530_011);
                printf(
                    c"Set as int: %d\n".as_ptr(),
                    ptr::addr_of!((*data).int_val).read(),
                );
                0
            }
            1 => {
                let value = ptr::addr_of!((*data).float_val).read();
                printf(c"Read as float: %f\n".as_ptr(), value as f64);
                float_to_c_int(value * 100.0_f32)
            }
            2 => {
                let value = ptr::addr_of!((*data).uint_val).read();
                printf(c"Read as uint: %u\n".as_ptr(), value);
                (value & 0xff) as c_int
            }
            3 => {
                let bytes = ptr::addr_of!((*data).bytes).read();
                printf(
                    c"Read as bytes: [%d, %d, %d, %d]\n".as_ptr(),
                    bytes[0] as c_int,
                    bytes[1] as c_int,
                    bytes[2] as c_int,
                    bytes[3] as c_int,
                );
                (bytes[0] as c_int).wrapping_add(bytes[1] as c_int)
            }
            _ => 0,
        }
    }
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

        let search_char = (b'0' as c_int + param3 % 10) as c_char;
        let found_count = process_buffer(state, search_char);
        result = result.wrapping_add(found_count.wrapping_mul(10));

        let confusion_result = confuse_types(state, param4 % 4);
        result = result.wrapping_add(confusion_result);

        let flags = ptr::addr_of!((*state).flags).read();
        let counter = ((flags & COUNTER_MASK) >> 3) as c_int;
        let mode = ((flags & MODE_MASK) >> 8) as c_int;
        result = result.wrapping_add(counter.wrapping_mul(5));
        result = result.wrapping_add(mode.wrapping_mul(3));

        printf(c"Final result: %d\n".as_ptr(), result);
        destroy_state(state);
    }

    result
}
