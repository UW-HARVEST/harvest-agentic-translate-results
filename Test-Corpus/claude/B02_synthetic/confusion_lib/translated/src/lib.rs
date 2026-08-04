// Translation of c_src/src/lib.c to Rust producing byte-identical output.
//
// We use the C runtime's printf/snprintf via FFI to guarantee that the
// formatting (especially %f, %u, etc.) matches the original C output exactly.

use std::ffi::{c_char, c_int, c_uint};

extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
    fn snprintf(s: *mut c_char, n: usize, fmt: *const c_char, ...) -> c_int;
}

// Mirrors the bit field struct from the C code.  We keep each field as a u32
// and apply masking explicitly so behaviour matches the bit-width semantics
// of the original C bit fields.
struct PackedFlags {
    flag1: u32,    // 1 bit
    flag2: u32,    // 1 bit
    flag3: u32,    // 1 bit
    counter: u32,  // 5 bits
    mode: u32,     // 3 bits
    #[allow(dead_code)]
    status: u32,   // 5 bits
    #[allow(dead_code)]
    reserved: u32, // 16 bits
}

// The TypeConfusion union -- four bytes shared between several views.
#[repr(C)]
union TypeConfusion {
    int_val: i32,
    float_val: f32,
    uint_val: u32,
    bytes: [c_char; 4],
}

struct ProcessState {
    flags: PackedFlags,
    data: TypeConfusion,
    buffer: Vec<u8>, // C-style null-terminated buffer of size `capacity`.
    #[allow(dead_code)]
    capacity: i32,
}

fn create_state(initial_val: i32, capacity: i32) -> Option<Box<ProcessState>> {
    if capacity <= 0 {
        // Allocation of a zero/negative-sized buffer would fail in C; mimic
        // by reporting the buffer-allocation error.  In practice this is
        // never exercised by the public entry point.
        unsafe {
            printf(b"Error: Failed to allocate buffer\n\0".as_ptr() as *const c_char);
        }
        return None;
    }

    let mut state = Box::new(ProcessState {
        flags: PackedFlags {
            flag1: 1,
            flag2: 0,
            flag3: 1,
            counter: 0,
            mode: 3,
            status: 15,
            reserved: 0,
        },
        data: TypeConfusion {
            int_val: initial_val,
        },
        buffer: vec![0u8; capacity as usize],
        capacity,
    });

    // snprintf(state->buffer, capacity, "State:%d:Mode:%d", initial_val, state->flags.mode);
    unsafe {
        snprintf(
            state.buffer.as_mut_ptr() as *mut c_char,
            capacity as usize,
            b"State:%d:Mode:%d\0".as_ptr() as *const c_char,
            initial_val as c_int,
            state.flags.mode as c_int,
        );
    }

    Some(state)
}

fn process_buffer(state: &ProcessState, target: u8) -> i32 {
    // strlen on the buffer
    let len = state
        .buffer
        .iter()
        .position(|&b| b == 0)
        .unwrap_or(state.buffer.len());

    let mut count: i32 = 0;
    let mut ptr_idx: usize = 0;
    let mut remaining: usize = len;

    while remaining > 0 {
        // memchr equivalent
        let slice_end = ptr_idx + remaining;
        let slice = &state.buffer[ptr_idx..slice_end];
        let pos = slice.iter().position(|&b| b == target);
        match pos {
            None => break,
            Some(p) => {
                count += 1;
                // LOG_OPERATION(memchr_found, count)
                unsafe {
                    printf(
                        b"Operation: memchr_found with value %d\n\0".as_ptr() as *const c_char,
                        count as c_int,
                    );
                }
                let found_idx = ptr_idx + p;
                remaining -= found_idx - ptr_idx + 1;
                ptr_idx = found_idx + 1;
            }
        }
    }

    count
}

fn update_flags(state: &mut ProcessState, param: i32) {
    state.flags.counter = (state.flags.counter.wrapping_add(1)) & 0x1F;
    state.flags.flag1 = (param & 1) as u32 & 0x1;
    state.flags.flag2 = (((param & 2) >> 1) as u32) & 0x1;
    state.flags.flag3 = (((param & 4) >> 2) as u32) & 0x1;
    state.flags.mode = ((param >> 3) & 0x7) as u32;

    unsafe {
        // DEBUG_VAR(state->flags.counter)
        printf(
            b"Debug: state->flags.counter = %d\n\0".as_ptr() as *const c_char,
            state.flags.counter as c_int,
        );
        printf(
            b"Bit fields - flag1:%d flag2:%d flag3:%d mode:%d\n\0".as_ptr() as *const c_char,
            state.flags.flag1 as c_int,
            state.flags.flag2 as c_int,
            state.flags.flag3 as c_int,
            state.flags.mode as c_int,
        );
    }
}

fn confuse_types(state: &mut ProcessState, operation: i32) -> i32 {
    let mut result: i32 = 0;

    unsafe {
        match operation {
            0 => {
                state.data.int_val = 1078530011;
                printf(
                    b"Set as int: %d\n\0".as_ptr() as *const c_char,
                    state.data.int_val as c_int,
                );
            }
            1 => {
                // %f promotes to double via varargs
                printf(
                    b"Read as float: %f\n\0".as_ptr() as *const c_char,
                    state.data.float_val as f64,
                );
                // (int)(float * 100) -- truncation toward zero.
                let prod: f32 = state.data.float_val * 100.0f32;
                result = prod as i32;
            }
            2 => {
                printf(
                    b"Read as uint: %u\n\0".as_ptr() as *const c_char,
                    state.data.uint_val as c_uint,
                );
                result = (state.data.uint_val & 0xFF) as i32;
            }
            3 => {
                // bytes are signed char on Linux x86_64; promoted to int by varargs.
                let b0 = state.data.bytes[0] as c_int;
                let b1 = state.data.bytes[1] as c_int;
                let b2 = state.data.bytes[2] as c_int;
                let b3 = state.data.bytes[3] as c_int;
                printf(
                    b"Read as bytes: [%d, %d, %d, %d]\n\0".as_ptr() as *const c_char,
                    b0,
                    b1,
                    b2,
                    b3,
                );
                // C: result = bytes[0] + bytes[1] -- both promoted to int before sum.
                result = (state.data.bytes[0] as i32) + (state.data.bytes[1] as i32);
            }
            _ => {}
        }
    }

    result
}

#[unsafe(no_mangle)]
pub extern "C" fn confusion(param1: c_int, param2: c_int, param3: c_int, param4: c_int) -> c_int {
    unsafe {
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
    }

    let mut result: i32 = 0;

    let mut state = match create_state(param1 as i32, 128) {
        Some(s) => s,
        None => return -1,
    };

    update_flags(&mut state, param2 as i32);

    // search_char = '0' + (param3 % 10);  -- C truncated remainder, char param.
    let search_int: i32 = (b'0' as i32) + ((param3 as i32) % 10);
    // Conversion to char (then to unsigned char in memchr) is just a byte
    // truncation: take the low 8 bits.
    let target_byte: u8 = search_int as u8;

    let found_count = process_buffer(&state, target_byte);
    result = result.wrapping_add(found_count.wrapping_mul(10));

    let confusion_result = confuse_types(&mut state, (param4 as i32) % 4);
    result = result.wrapping_add(confusion_result);

    result = result.wrapping_add((state.flags.counter as i32).wrapping_mul(5));
    result = result.wrapping_add((state.flags.mode as i32).wrapping_mul(3));

    unsafe {
        printf(
            b"Final result: %d\n\0".as_ptr() as *const c_char,
            result as c_int,
        );
    }

    // destroy_state happens automatically when `state` Box drops here.
    drop(state);

    result as c_int
}
