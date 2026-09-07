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

#![allow(clippy::missing_safety_doc)]

use core::ffi::{c_char, c_double, c_int, c_uint, c_void};
use core::mem::size_of;
use core::ptr;

// ---------------------------------------------------------------------------
// libc bindings.
//
// The C code performs all of its I/O through `printf`/`snprintf` and all of
// its allocation through `malloc`/`free`.  Calling the very same C library
// routines keeps stdout formatting (notably `%f`, `%u` and `%d`) and the
// allocator ABI byte-for-byte identical with the original library.
// ---------------------------------------------------------------------------
unsafe extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
    fn printf(fmt: *const c_char, ...) -> c_int;
    fn snprintf(s: *mut c_char, n: usize, fmt: *const c_char, ...) -> c_int;
    fn strlen(s: *const c_char) -> usize;
    fn memchr(s: *const c_void, c: c_int, n: usize) -> *mut c_void;
}

// ---------------------------------------------------------------------------
// typedef struct { unsigned int flag1:1, flag2:1, flag3:1, counter:5,
//                               mode:3, status:5, reserved:16; } PackedFlags;
//
// System V / GCC on little-endian targets allocates bit-fields from the least
// significant bit of the storage unit upwards, so the layout of the single
// 32-bit allocation unit is:
//
//   bit  0      flag1
//   bit  1      flag2
//   bit  2      flag3
//   bits 3..8   counter  (5 bits)
//   bits 8..11  mode     (3 bits)
//   bits 11..16 status   (5 bits)
//   bits 16..32 reserved (16 bits)
// ---------------------------------------------------------------------------
#[repr(C)]
#[derive(Clone, Copy)]
pub struct PackedFlags {
    bits: u32,
}

macro_rules! bitfield {
    ($get:ident, $set:ident, $offset:expr, $width:expr) => {
        #[inline]
        fn $get(&self) -> c_uint {
            ((self.bits >> $offset) & ((1u32 << $width) - 1)) as c_uint
        }
        #[inline]
        fn $set(&mut self, value: c_uint) {
            let mask: u32 = ((1u32 << $width) - 1) << $offset;
            self.bits = (self.bits & !mask) | (((value as u32) << $offset) & mask);
        }
    };
}

#[allow(dead_code)]
impl PackedFlags {
    bitfield!(flag1, set_flag1, 0, 1);
    bitfield!(flag2, set_flag2, 1, 1);
    bitfield!(flag3, set_flag3, 2, 1);
    bitfield!(counter, set_counter, 3, 5);
    bitfield!(mode, set_mode, 8, 3);
    bitfield!(status, set_status, 11, 5);
    bitfield!(reserved, set_reserved, 16, 16);
}

// ---------------------------------------------------------------------------
// typedef union { int int_val; float float_val; unsigned int uint_val;
//                 char bytes[4]; } TypeConfusion;
//
// All four members occupy the same four bytes; the union is modelled as the
// raw 32-bit storage and every member is a reinterpretation of those bytes.
// ---------------------------------------------------------------------------
#[repr(C)]
#[derive(Clone, Copy)]
pub struct TypeConfusion {
    raw: u32,
}

impl TypeConfusion {
    #[inline]
    fn int_val(&self) -> c_int {
        self.raw as c_int
    }
    #[inline]
    fn set_int_val(&mut self, value: c_int) {
        self.raw = value as u32;
    }
    #[inline]
    fn float_val(&self) -> f32 {
        f32::from_bits(self.raw)
    }
    #[inline]
    fn uint_val(&self) -> c_uint {
        self.raw as c_uint
    }
    #[inline]
    fn bytes(&self) -> [c_char; 4] {
        let b = self.raw.to_ne_bytes();
        [
            b[0] as c_char,
            b[1] as c_char,
            b[2] as c_char,
            b[3] as c_char,
        ]
    }
}

// ---------------------------------------------------------------------------
// typedef struct { PackedFlags flags; TypeConfusion data;
//                  char* buffer; int capacity; } ProcessState;
//
// offsetof: flags 0, data 4, buffer 8, capacity 16; sizeof 24, alignof 8.
// ---------------------------------------------------------------------------
#[repr(C)]
pub struct ProcessState {
    flags: PackedFlags,
    data: TypeConfusion,
    buffer: *mut c_char,
    capacity: c_int,
}

/// Reproduces the x86-64 `(int)float` conversion (`cvttss2si`) that GCC emits,
/// including the "integer indefinite" result for NaN / out-of-range values.
/// Rust's `as` would saturate instead, which does not match the C code.
#[inline]
fn f32_to_c_int(value: f32) -> c_int {
    let truncated = value.trunc();
    if truncated.is_nan()
        || truncated >= 2147483648.0_f32
        || truncated < -2147483648.0_f32
    {
        c_int::MIN
    } else {
        truncated as c_int
    }
}

// ---------------------------------------------------------------------------
// ProcessState* create_state(int initial_val, int capacity)
// ---------------------------------------------------------------------------
#[unsafe(no_mangle)]
pub unsafe extern "C" fn create_state(initial_val: c_int, capacity: c_int) -> *mut ProcessState {
    let state = unsafe { malloc(size_of::<ProcessState>()) } as *mut ProcessState;

    if state.is_null() {
        unsafe { printf(c"Error: Failed to allocate memory for state\n".as_ptr()) };
        return ptr::null_mut();
    }

    // The bit-field assignments below cover every bit of the 32-bit allocation
    // unit, so the resulting word is fully determined (0x00007b05) exactly as
    // in the C original, which read-modify-writes the malloc'd storage.
    let mut flags = PackedFlags { bits: 0 };
    flags.set_flag1(1);
    flags.set_flag2(0);
    flags.set_flag3(1);
    flags.set_counter(0);
    flags.set_mode(3);
    flags.set_status(15);
    flags.set_reserved(0);
    unsafe { ptr::write(&raw mut (*state).flags, flags) };

    let mut data = TypeConfusion { raw: 0 };
    data.set_int_val(initial_val);
    unsafe { ptr::write(&raw mut (*state).data, data) };

    unsafe { ptr::write(&raw mut (*state).capacity, capacity) };
    // `malloc(capacity)`: the int argument is converted to size_t, i.e. a
    // negative capacity becomes an enormous request (which then fails).
    let buffer = unsafe { malloc(capacity as isize as usize) } as *mut c_char;
    unsafe { ptr::write(&raw mut (*state).buffer, buffer) };

    if buffer.is_null() {
        unsafe { printf(c"Error: Failed to allocate buffer\n".as_ptr()) };
        unsafe { free(state as *mut c_void) };
        return ptr::null_mut();
    }

    unsafe {
        snprintf(
            buffer,
            capacity as isize as usize,
            c"State:%d:Mode:%d".as_ptr(),
            initial_val as c_int,
            flags.mode() as c_int,
        )
    };

    state
}

// ---------------------------------------------------------------------------
// void destroy_state(ProcessState* state)
// ---------------------------------------------------------------------------
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destroy_state(state: *mut ProcessState) {
    if !state.is_null() {
        let buffer = unsafe { (*state).buffer };
        if !buffer.is_null() {
            unsafe { free(buffer as *mut c_void) };
        }
        unsafe { free(state as *mut c_void) };
    }
}

// ---------------------------------------------------------------------------
// int process_buffer(ProcessState* state, char target)
// ---------------------------------------------------------------------------
#[unsafe(no_mangle)]
pub unsafe extern "C" fn process_buffer(state: *mut ProcessState, target: c_char) -> c_int {
    if state.is_null() || unsafe { (*state).buffer }.is_null() {
        unsafe { printf(c"Error: Null pointer in process_buffer\n".as_ptr()) };
        return -1;
    }

    let mut count: c_int = 0;
    let mut ptr_cur: *mut c_char = unsafe { (*state).buffer };
    let mut remaining: usize = unsafe { strlen((*state).buffer) };

    while remaining > 0 {
        let found = unsafe { memchr(ptr_cur as *const c_void, target as c_int, remaining) }
            as *mut c_char;

        if found.is_null() {
            break;
        }

        count = count.wrapping_add(1);
        unsafe {
            printf(
                c"Operation: memchr_found with value %d\n".as_ptr(),
                count as c_int,
            )
        };

        let consumed = (found as usize - ptr_cur as usize) + 1;
        remaining = remaining.wrapping_sub(consumed);
        ptr_cur = unsafe { found.add(1) };
    }

    count
}

// ---------------------------------------------------------------------------
// void update_flags(ProcessState* state, int param)
// ---------------------------------------------------------------------------
#[unsafe(no_mangle)]
pub unsafe extern "C" fn update_flags(state: *mut ProcessState, param: c_int) {
    if state.is_null() {
        return;
    }

    let state_ref = unsafe { &mut *state };

    // 5-bit counter
    let next_counter = (state_ref.flags.counter() as c_int).wrapping_add(1) & 0x1F;
    state_ref.flags.set_counter(next_counter as c_uint);
    state_ref.flags.set_flag1((param & 1) as c_uint);
    state_ref.flags.set_flag2(((param & 2) >> 1) as c_uint);
    state_ref.flags.set_flag3(((param & 4) >> 2) as c_uint);
    state_ref.flags.set_mode(((param >> 3) & 0x7) as c_uint);

    unsafe {
        printf(
            c"Debug: state->flags.counter = %d\n".as_ptr(),
            state_ref.flags.counter() as c_int,
        );
        printf(
            c"Bit fields - flag1:%d flag2:%d flag3:%d mode:%d\n".as_ptr(),
            state_ref.flags.flag1() as c_int,
            state_ref.flags.flag2() as c_int,
            state_ref.flags.flag3() as c_int,
            state_ref.flags.mode() as c_int,
        );
    }
}

// ---------------------------------------------------------------------------
// int confuse_types(ProcessState* state, int operation)
// ---------------------------------------------------------------------------
#[unsafe(no_mangle)]
pub unsafe extern "C" fn confuse_types(state: *mut ProcessState, operation: c_int) -> c_int {
    if state.is_null() {
        return 0;
    }

    let mut result: c_int = 0;

    let state_ref = unsafe { &mut *state };

    match operation {
        0 => {
            state_ref.data.set_int_val(1078530011);
            unsafe {
                printf(
                    c"Set as int: %d\n".as_ptr(),
                    state_ref.data.int_val() as c_int,
                )
            };
        }

        1 => {
            unsafe {
                printf(
                    c"Read as float: %f\n".as_ptr(),
                    state_ref.data.float_val() as c_double,
                )
            };
            result = f32_to_c_int(state_ref.data.float_val() * 100.0_f32);
        }

        2 => {
            unsafe {
                printf(
                    c"Read as uint: %u\n".as_ptr(),
                    state_ref.data.uint_val() as c_uint,
                )
            };
            result = (state_ref.data.uint_val() & 0xFF) as c_int;
        }

        3 => {
            let bytes = state_ref.data.bytes();
            unsafe {
                printf(
                    c"Read as bytes: [%d, %d, %d, %d]\n".as_ptr(),
                    bytes[0] as c_int,
                    bytes[1] as c_int,
                    bytes[2] as c_int,
                    bytes[3] as c_int,
                )
            };
            result = (bytes[0] as c_int).wrapping_add(bytes[1] as c_int);
        }

        _ => {}
    }

    result
}

// ---------------------------------------------------------------------------
// int confusion(int param1, int param2, int param3, int param4)
// ---------------------------------------------------------------------------
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

    unsafe { update_flags(state, param2) };

    let search_char = (b'0' as c_int).wrapping_add(param3 % 10) as c_char;
    let found_count = unsafe { process_buffer(state, search_char) };
    result = result.wrapping_add(found_count.wrapping_mul(10));

    let confusion_result = unsafe { confuse_types(state, param4 % 4) };
    result = result.wrapping_add(confusion_result);

    let state_ref = unsafe { &*state };
    result = result.wrapping_add((state_ref.flags.counter() as c_int).wrapping_mul(5));
    result = result.wrapping_add((state_ref.flags.mode() as c_int).wrapping_mul(3));

    unsafe { printf(c"Final result: %d\n".as_ptr(), result) };

    unsafe { destroy_state(state) };

    result
}
