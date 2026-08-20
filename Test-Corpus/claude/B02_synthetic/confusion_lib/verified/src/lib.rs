// Rust translation of c_src/src/lib.c
//
// Copyright 2025 MIT Lincoln Laboratory
// Permission is hereby granted, free of charge,
// to any person obtaining a copy of this software
// and associated documentation files (the "Software"),
// to deal in the Software without restriction,
// including without limitation the rights to use, copy,
// modify, merge, publish, distribute, sublicense,
// and/or sell copies of the Software,
// and to permit persons to whom the Software is furnished to do so,
// subject to the following conditions:
//
// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
// EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
// THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
// IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE
// FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT,
// TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE
// OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
//
// The public ABI reproduced here (as exported by the C shared library) is:
//
//     create_state, destroy_state, process_buffer,
//     update_flags, confuse_types, confusion
//
// All formatted output is emitted through the C library's `printf` /
// `snprintf` so the bytes written to stdout (and the stdio buffering
// behaviour) are identical to the original.

mod ffi;
mod types;

use core::ffi::{c_char, c_int, c_void};
use core::mem::size_of;
use core::ptr;

use ffi::cvttss2si;
use types::{PackedFlags, ProcessState, TypeConfusion};

// ---------------------------------------------------------------------------
// Format strings, byte-for-byte as produced by the C preprocessor.
//
//   #define STRINGIFY(x) #x
//   #define DEBUG_VAR(var) printf("Debug: " STRINGIFY(var) " = %d\n", var)
//   #define LOG_OPERATION(op, val)
//       printf("Operation: " STRINGIFY(op) " with value %d\n", val)
// ---------------------------------------------------------------------------

const FMT_ERR_STATE_ALLOC: &[u8] = b"Error: Failed to allocate memory for state\n\0";
const FMT_ERR_BUFFER_ALLOC: &[u8] = b"Error: Failed to allocate buffer\n\0";
const FMT_STATE_MODE: &[u8] = b"State:%d:Mode:%d\0";
const FMT_ERR_NULL_PROCESS_BUFFER: &[u8] = b"Error: Null pointer in process_buffer\n\0";
/// `LOG_OPERATION(memchr_found, count)`
const FMT_LOG_MEMCHR_FOUND: &[u8] = b"Operation: memchr_found with value %d\n\0";
/// `DEBUG_VAR(state->flags.counter)`
const FMT_DEBUG_COUNTER: &[u8] = b"Debug: state->flags.counter = %d\n\0";
const FMT_BIT_FIELDS: &[u8] = b"Bit fields - flag1:%d flag2:%d flag3:%d mode:%d\n\0";
const FMT_SET_AS_INT: &[u8] = b"Set as int: %d\n\0";
const FMT_READ_AS_FLOAT: &[u8] = b"Read as float: %f\n\0";
const FMT_READ_AS_UINT: &[u8] = b"Read as uint: %u\n\0";
const FMT_READ_AS_BYTES: &[u8] = b"Read as bytes: [%d, %d, %d, %d]\n\0";
/// `DEBUG_VAR(param1)` .. `DEBUG_VAR(param4)`
const FMT_DEBUG_PARAM1: &[u8] = b"Debug: param1 = %d\n\0";
const FMT_DEBUG_PARAM2: &[u8] = b"Debug: param2 = %d\n\0";
const FMT_DEBUG_PARAM3: &[u8] = b"Debug: param3 = %d\n\0";
const FMT_DEBUG_PARAM4: &[u8] = b"Debug: param4 = %d\n\0";
const FMT_FINAL_RESULT: &[u8] = b"Final result: %d\n\0";

#[inline]
const fn cstr(bytes: &[u8]) -> *const c_char {
    bytes.as_ptr() as *const c_char
}

// ---------------------------------------------------------------------------
// ProcessState* create_state(int initial_val, int capacity)
// ---------------------------------------------------------------------------

/// ```c
/// ProcessState* create_state(int initial_val, int capacity) { ... }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn create_state(initial_val: c_int, capacity: c_int) -> *mut ProcessState {
    let state = ffi::malloc(size_of::<ProcessState>()) as *mut ProcessState;

    if state.is_null() {
        ffi::printf(cstr(FMT_ERR_STATE_ALLOC));
        return ptr::null_mut();
    }

    // Every one of the 32 bits of `PackedFlags` is assigned below, so building
    // the storage unit up from zero is equivalent to gcc's read-modify-write
    // sequence over the freshly malloc'd (indeterminate) bytes.
    let mut flags = PackedFlags { bits: 0 };
    flags.set_flag1(1);
    flags.set_flag2(0);
    flags.set_flag3(1);
    flags.set_counter(0);
    flags.set_mode(3);
    flags.set_status(15);
    flags.set_reserved(0);
    (*state).flags = flags;

    (*state).data = TypeConfusion {
        int_val: initial_val,
    };

    (*state).capacity = capacity;
    // C converts the `int` argument to `size_t`, i.e. sign-extends it; a
    // negative capacity therefore becomes a huge request and malloc fails.
    (*state).buffer = ffi::malloc(capacity as isize as usize) as *mut c_char;

    if (*state).buffer.is_null() {
        ffi::printf(cstr(FMT_ERR_BUFFER_ALLOC));
        ffi::free(state as *mut c_void);
        return ptr::null_mut();
    }

    ffi::snprintf(
        (*state).buffer,
        capacity as isize as usize,
        cstr(FMT_STATE_MODE),
        initial_val,
        (*state).flags.mode() as c_int,
    );

    state
}

// ---------------------------------------------------------------------------
// void destroy_state(ProcessState* state)
// ---------------------------------------------------------------------------

/// ```c
/// void destroy_state(ProcessState* state) { ... }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destroy_state(state: *mut ProcessState) {
    if !state.is_null() {
        if !(*state).buffer.is_null() {
            ffi::free((*state).buffer as *mut c_void);
        }
        ffi::free(state as *mut c_void);
    }
}

// ---------------------------------------------------------------------------
// int process_buffer(ProcessState* state, char target)
// ---------------------------------------------------------------------------

/// ```c
/// int process_buffer(ProcessState* state, char target) { ... }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn process_buffer(state: *mut ProcessState, target: c_char) -> c_int {
    if state.is_null() || (*state).buffer.is_null() {
        ffi::printf(cstr(FMT_ERR_NULL_PROCESS_BUFFER));
        return -1;
    }

    let mut count: c_int = 0;
    let mut ptr_cur: *mut c_char = (*state).buffer;
    // `size_t remaining`
    let mut remaining: usize = ffi::strlen((*state).buffer);

    while remaining > 0 {
        // `target` is promoted to `int`; memchr then compares the low
        // `unsigned char` of it, so sign-extension here is the faithful
        // conversion.
        let found = ffi::memchr(ptr_cur as *const c_void, target as c_int, remaining) as *mut c_char;

        if found.is_null() {
            break;
        }

        count = count.wrapping_add(1);
        ffi::printf(cstr(FMT_LOG_MEMCHR_FOUND), count);

        // `remaining -= (found - ptr + 1);` — unsigned (size_t) arithmetic.
        remaining = remaining.wrapping_sub((found.offset_from(ptr_cur) as usize).wrapping_add(1));
        ptr_cur = found.add(1);
    }

    count
}

// ---------------------------------------------------------------------------
// void update_flags(ProcessState* state, int param)
// ---------------------------------------------------------------------------

/// ```c
/// void update_flags(ProcessState* state, int param) { ... }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn update_flags(state: *mut ProcessState, param: c_int) {
    if state.is_null() {
        return;
    }

    let flags = &mut (*state).flags;

    // 5-bit counter
    let next_counter = (flags.counter().wrapping_add(1)) & 0x1F;
    flags.set_counter(next_counter);
    flags.set_flag1((param & 1) as u32);
    flags.set_flag2(((param & 2) >> 1) as u32);
    flags.set_flag3(((param & 4) >> 2) as u32);
    // Arithmetic right shift, matching gcc's `>>` on a signed int.
    flags.set_mode(((param >> 3) & 0x7) as u32);

    ffi::printf(cstr(FMT_DEBUG_COUNTER), flags.counter() as c_int);
    ffi::printf(
        cstr(FMT_BIT_FIELDS),
        flags.flag1() as c_int,
        flags.flag2() as c_int,
        flags.flag3() as c_int,
        flags.mode() as c_int,
    );
}

// ---------------------------------------------------------------------------
// int confuse_types(ProcessState* state, int operation)
// ---------------------------------------------------------------------------

/// ```c
/// int confuse_types(ProcessState* state, int operation) { ... }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn confuse_types(state: *mut ProcessState, operation: c_int) -> c_int {
    if state.is_null() {
        return 0;
    }

    let mut result: c_int = 0;

    match operation {
        0 => {
            (*state).data.int_val = 1078530011;
            ffi::printf(cstr(FMT_SET_AS_INT), (*state).data.int_val);
        }

        1 => {
            // `%f` promotes the float to double.
            ffi::printf(cstr(FMT_READ_AS_FLOAT), (*state).data.float_val as f64);
            // gcc emits `mulss` (single precision) followed by `cvttss2si`.
            result = cvttss2si((*state).data.float_val * 100.0f32);
        }

        2 => {
            ffi::printf(cstr(FMT_READ_AS_UINT), (*state).data.uint_val);
            result = ((*state).data.uint_val & 0xFF) as c_int;
        }

        3 => {
            let bytes = (*state).data.bytes;
            ffi::printf(
                cstr(FMT_READ_AS_BYTES),
                // `char` promotes to `int` (sign-extended: char is signed here).
                bytes[0] as c_int,
                bytes[1] as c_int,
                bytes[2] as c_int,
                bytes[3] as c_int,
            );
            result = (bytes[0] as c_int).wrapping_add(bytes[1] as c_int);
        }

        _ => {}
    }

    result
}

// ---------------------------------------------------------------------------
// int confusion(int param1, int param2, int param3, int param4)
// ---------------------------------------------------------------------------

/// ```c
/// int confusion(int param1, int param2, int param3, int param4) { ... }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn confusion(
    param1: c_int,
    param2: c_int,
    param3: c_int,
    param4: c_int,
) -> c_int {
    ffi::printf(cstr(FMT_DEBUG_PARAM1), param1);
    ffi::printf(cstr(FMT_DEBUG_PARAM2), param2);
    ffi::printf(cstr(FMT_DEBUG_PARAM3), param3);
    ffi::printf(cstr(FMT_DEBUG_PARAM4), param4);

    let mut result: c_int = 0;

    let state = create_state(param1, 128);

    if state.is_null() {
        return -1;
    }

    update_flags(state, param2);

    // `char search_char = '0' + (param3 % 10);` — the int result is truncated
    // to `char`.
    let search_char = (b'0' as c_int).wrapping_add(param3 % 10) as c_char;
    let found_count = process_buffer(state, search_char);
    result = result.wrapping_add(found_count.wrapping_mul(10));

    let confusion_result = confuse_types(state, param4 % 4);
    result = result.wrapping_add(confusion_result);

    result = result.wrapping_add(((*state).flags.counter() as c_int).wrapping_mul(5));
    result = result.wrapping_add(((*state).flags.mode() as c_int).wrapping_mul(3));

    ffi::printf(cstr(FMT_FINAL_RESULT), result);

    destroy_state(state);

    result
}
