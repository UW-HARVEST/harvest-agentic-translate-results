// Rust translation of c_src/src/lib.c
//
// Original copyright notice from the C source:
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

#![allow(non_snake_case)]
#![allow(dead_code)]

use std::ffi::{c_char, c_double, c_int, c_uint, c_void};

// ---------------------------------------------------------------------------
// libc bindings.
//
// The C library performs all of its I/O through `printf`/`snprintf` and all of
// its memory management through `malloc`/`free`.  We bind directly to the same
// libc entry points so that formatting, stdout buffering and allocation
// failure behaviour are bit-for-bit identical to the original.
// ---------------------------------------------------------------------------
unsafe extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
    fn strlen(s: *const c_char) -> usize;
    fn printf(fmt: *const c_char, ...) -> c_int;
    fn snprintf(s: *mut c_char, n: usize, fmt: *const c_char, ...) -> c_int;
}

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// ```c
/// typedef struct {
///     unsigned int flag1 : 1;
///     unsigned int flag2 : 1;
///     unsigned int flag3 : 1;
///     unsigned int counter : 5;
///     unsigned int mode : 3;
///     unsigned int status : 5;
///     unsigned int reserved : 16;
/// } PackedFlags;
/// ```
///
/// The System V x86-64 ABI (as implemented by gcc/clang) allocates these
/// bit-fields from the least significant bit of a single 4-byte storage unit:
///
/// | field    | bits  | mask       |
/// |----------|-------|------------|
/// | flag1    | 0     | 0x00000001 |
/// | flag2    | 1     | 0x00000002 |
/// | flag3    | 2     | 0x00000004 |
/// | counter  | 3..7  | 0x000000f8 |
/// | mode     | 8..10 | 0x00000700 |
/// | status   | 11..15| 0x0000f800 |
/// | reserved | 16..31| 0xffff0000 |
#[repr(C)]
#[derive(Copy, Clone, Default)]
struct PackedFlags {
    raw: c_uint,
}

macro_rules! bitfield {
    ($get:ident, $set:ident, $shift:expr, $width:expr) => {
        #[inline]
        fn $get(&self) -> c_uint {
            (self.raw >> $shift) & ((1u32 << $width) - 1)
        }
        #[inline]
        fn $set(&mut self, value: c_uint) {
            let mask: c_uint = ((1u32 << $width) - 1) << $shift;
            self.raw = (self.raw & !mask) | ((value << $shift) & mask);
        }
    };
}

impl PackedFlags {
    bitfield!(flag1, set_flag1, 0, 1);
    bitfield!(flag2, set_flag2, 1, 1);
    bitfield!(flag3, set_flag3, 2, 1);
    bitfield!(counter, set_counter, 3, 5);
    bitfield!(mode, set_mode, 8, 3);
    bitfield!(status, set_status, 11, 5);
    bitfield!(reserved, set_reserved, 16, 16);
}

/// ```c
/// typedef union {
///     int int_val;
///     float float_val;
///     unsigned int uint_val;
///     char bytes[4];
/// } TypeConfusion;
/// ```
///
/// All members are 4 bytes wide, so the union is modelled as a single 4-byte
/// storage unit that is reinterpreted on access.
#[repr(C)]
#[derive(Copy, Clone, Default)]
struct TypeConfusion {
    raw: c_uint,
}

impl TypeConfusion {
    #[inline]
    fn int_val(&self) -> c_int {
        self.raw as c_int
    }
    #[inline]
    fn set_int_val(&mut self, value: c_int) {
        self.raw = value as c_uint;
    }
    #[inline]
    fn float_val(&self) -> f32 {
        f32::from_bits(self.raw)
    }
    #[inline]
    fn uint_val(&self) -> c_uint {
        self.raw
    }
    /// `bytes[i]` — `char` is signed on x86-64 Linux, and the target is
    /// little-endian, so byte `i` is bits `8*i .. 8*i+7`.
    #[inline]
    fn byte(&self, i: usize) -> c_char {
        (self.raw >> (8 * i)) as u8 as c_char
    }
}

/// ```c
/// typedef struct {
///     PackedFlags flags;
///     TypeConfusion data;
///     char* buffer;
///     int capacity;
/// } ProcessState;
/// ```
///
/// Verified layout: size 24, offsets flags=0, data=4, buffer=8, capacity=16.
#[repr(C)]
pub struct ProcessState {
    flags: PackedFlags,
    data: TypeConfusion,
    buffer: *mut c_char,
    capacity: c_int,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Emulates the x86 `cvttss2si` instruction that gcc emits for
/// `(int)some_float`.  Truncates toward zero; values that are NaN or outside
/// the range of `int` yield the "integer indefinite" value `INT_MIN`.
///
/// Rust's `as` cast saturates instead, so it cannot be used directly.
#[inline]
fn f32_to_c_int_trunc(v: f32) -> c_int {
    if v.is_nan() {
        return c_int::MIN;
    }
    let t = v.trunc();
    // -2^31 is exactly representable as f32; 2^31 is the first f32 above the
    // representable range of `int`.
    if t >= -2147483648.0f32 && t < 2147483648.0f32 {
        t as c_int
    } else {
        c_int::MIN
    }
}

// ---------------------------------------------------------------------------
// Public ABI
// ---------------------------------------------------------------------------

/// ```c
/// ProcessState* create_state(int initial_val, int capacity);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn create_state(initial_val: c_int, capacity: c_int) -> *mut ProcessState {
    let state = unsafe { malloc(size_of::<ProcessState>()) } as *mut ProcessState;

    if state.is_null() {
        unsafe {
            printf(c"Error: Failed to allocate memory for state\n".as_ptr());
        }
        return std::ptr::null_mut();
    }

    let s = unsafe { &mut *state };

    // The C code assigns every bit-field of `flags`, which together cover all
    // 32 bits of the storage unit, so the resulting value is fully defined.
    s.flags = PackedFlags { raw: 0 };
    s.flags.set_flag1(1);
    s.flags.set_flag2(0);
    s.flags.set_flag3(1);
    s.flags.set_counter(0);
    s.flags.set_mode(3);
    s.flags.set_status(15);
    s.flags.set_reserved(0);

    s.data = TypeConfusion { raw: 0 };
    s.data.set_int_val(initial_val);

    s.capacity = capacity;
    // `malloc(capacity)` — a negative `int` converts to a huge `size_t`, which
    // makes the allocation fail.  Sign-extending `as usize` reproduces that.
    s.buffer = unsafe { malloc(capacity as usize) } as *mut c_char;

    if s.buffer.is_null() {
        unsafe {
            printf(c"Error: Failed to allocate buffer\n".as_ptr());
            free(state as *mut c_void);
        }
        return std::ptr::null_mut();
    }

    unsafe {
        snprintf(
            s.buffer,
            capacity as usize,
            c"State:%d:Mode:%d".as_ptr(),
            initial_val,
            s.flags.mode() as c_int,
        );
    }

    state
}

/// ```c
/// void destroy_state(ProcessState* state);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destroy_state(state: *mut ProcessState) {
    if !state.is_null() {
        let s = unsafe { &mut *state };
        if !s.buffer.is_null() {
            unsafe { free(s.buffer as *mut c_void) };
        }
        unsafe { free(state as *mut c_void) };
    }
}

/// ```c
/// int process_buffer(ProcessState* state, char target);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn process_buffer(state: *mut ProcessState, target: c_char) -> c_int {
    if state.is_null() || unsafe { (*state).buffer }.is_null() {
        unsafe {
            printf(c"Error: Null pointer in process_buffer\n".as_ptr());
        }
        return -1;
    }

    let s = unsafe { &mut *state };

    let mut count: c_int = 0;
    let mut ptr = s.buffer;
    let mut remaining = unsafe { strlen(s.buffer) };

    // `memchr` compares bytes as `unsigned char`.
    let needle = target as u8;

    while remaining > 0 {
        let haystack = unsafe { std::slice::from_raw_parts(ptr as *const u8, remaining) };

        let found = match haystack.iter().position(|&b| b == needle) {
            Some(idx) => unsafe { ptr.add(idx) },
            None => break,
        };

        count = count.wrapping_add(1);
        unsafe {
            printf(
                c"Operation: memchr_found with value %d\n".as_ptr(),
                count,
            );
        }

        let consumed = unsafe { found.offset_from(ptr) } as usize + 1;
        remaining -= consumed;
        ptr = unsafe { found.add(1) };
    }

    count
}

/// ```c
/// void update_flags(ProcessState* state, int param);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn update_flags(state: *mut ProcessState, param: c_int) {
    if state.is_null() {
        return;
    }

    let s = unsafe { &mut *state };

    // 5-bit counter
    s.flags
        .set_counter((s.flags.counter().wrapping_add(1)) & 0x1F);
    s.flags.set_flag1((param & 1) as c_uint);
    s.flags.set_flag2(((param & 2) >> 1) as c_uint);
    s.flags.set_flag3(((param & 4) >> 2) as c_uint);
    // `param >> 3` is an arithmetic shift for signed `int` on gcc.
    s.flags.set_mode(((param >> 3) & 0x7) as c_uint);

    unsafe {
        printf(
            c"Debug: state->flags.counter = %d\n".as_ptr(),
            s.flags.counter() as c_int,
        );
        printf(
            c"Bit fields - flag1:%d flag2:%d flag3:%d mode:%d\n".as_ptr(),
            s.flags.flag1() as c_int,
            s.flags.flag2() as c_int,
            s.flags.flag3() as c_int,
            s.flags.mode() as c_int,
        );
    }
}

/// ```c
/// int confuse_types(ProcessState* state, int operation);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn confuse_types(state: *mut ProcessState, operation: c_int) -> c_int {
    if state.is_null() {
        return 0;
    }

    let s = unsafe { &mut *state };

    let mut result: c_int = 0;

    match operation {
        0 => {
            s.data.set_int_val(1078530011);
            unsafe {
                printf(c"Set as int: %d\n".as_ptr(), s.data.int_val());
            }
        }

        1 => {
            unsafe {
                printf(
                    c"Read as float: %f\n".as_ptr(),
                    s.data.float_val() as c_double,
                );
            }
            result = f32_to_c_int_trunc(s.data.float_val() * 100.0f32);
        }

        2 => {
            unsafe {
                printf(c"Read as uint: %u\n".as_ptr(), s.data.uint_val());
            }
            result = (s.data.uint_val() & 0xFF) as c_int;
        }

        3 => {
            unsafe {
                printf(
                    c"Read as bytes: [%d, %d, %d, %d]\n".as_ptr(),
                    s.data.byte(0) as c_int,
                    s.data.byte(1) as c_int,
                    s.data.byte(2) as c_int,
                    s.data.byte(3) as c_int,
                );
            }
            result = (s.data.byte(0) as c_int).wrapping_add(s.data.byte(1) as c_int);
        }

        _ => {}
    }

    result
}

/// ```c
/// int confusion(int param1, int param2, int param3, int param4);
/// ```
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

    let s = unsafe { &*state };
    result = result.wrapping_add((s.flags.counter() as c_int).wrapping_mul(5));
    result = result.wrapping_add((s.flags.mode() as c_int).wrapping_mul(3));

    unsafe {
        printf(c"Final result: %d\n".as_ptr(), result);
    }

    unsafe { destroy_state(state) };

    result
}
