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

use std::ffi::{c_char, c_int, c_void};
use std::ptr;

// ---------------------------------------------------------------------------
// libc bindings.
//
// The C translation unit uses malloc/realloc/free for the StringBuffer that it
// hands back to callers across the public ABI, and stdio's printf for its
// output. We bind those directly rather than using Rust's allocator / Rust's
// `println!` so that:
//
//   * a caller may `free()` a pointer obtained from `create_buffer`, and
//     `destroy_buffer` may `free()` a pointer obtained from `malloc`, exactly
//     as with the C library;
//   * `buffapp`'s output shares the C `stdout` buffer, so ordering and
//     flush-at-exit behaviour (and therefore the captured bytes) are identical.
// ---------------------------------------------------------------------------
unsafe extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn realloc(ptr: *mut c_void, size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);

    fn strlen(s: *const c_char) -> usize;
    fn strcpy(dst: *mut c_char, src: *const c_char) -> *mut c_char;
    fn strcmp(a: *const c_char, b: *const c_char) -> c_int;

    #[link_name = "sprintf"]
    fn c_sprintf(buf: *mut c_char, fmt: *const c_char, ...) -> c_int;
    #[link_name = "printf"]
    fn c_printf(fmt: *const c_char, ...) -> c_int;
}

/// typedef struct { char *data; int capacity; int length; } StringBuffer;
///
/// Opaque to C callers (it is declared inside lib.c, not in lib.h), but the
/// layout must match byte for byte: `char*` at offset 0, `int` at 8, `int` at
/// 12, size 16, align 8.
#[repr(C)]
pub struct StringBuffer {
    pub data: *mut c_char,
    pub capacity: c_int,
    pub length: c_int,
}

// ---------------------------------------------------------------------------
// Static, NUL-terminated string literals mirroring the C string constants.
// ---------------------------------------------------------------------------
macro_rules! cstr {
    ($s:expr) => {
        concat!($s, "\0").as_ptr() as *const c_char
    };
}

const OP_ADD: *const c_char = cstr!("add");
const OP_SUBTRACT: *const c_char = cstr!("subtract");
const OP_MULTIPLY: *const c_char = cstr!("multiply");
const OP_DIVIDE: *const c_char = cstr!("divide");
const OP_UNKNOWN: *const c_char = cstr!("unknown");

const FMT_STARTING: *const c_char = cstr!("Starting computation with %d parameters\n");
const FMT_OP1: *const c_char = cstr!("Operation 1: %s(%d, %d)\n");
const FMT_OP2: *const c_char = cstr!("Operation 2: %s(%d, %d)\n");
const FMT_OP3: *const c_char = cstr!("Operation 3: %s(%d, %d)\n");
const FMT_FINAL: *const c_char = cstr!("Final result: %d\n");
const FMT_LOG: *const c_char = cstr!("Computation Log:\n%s\n");

// ---------------------------------------------------------------------------
// StringBuffer* create_buffer(int initial_capacity)
// ---------------------------------------------------------------------------
#[unsafe(no_mangle)]
pub unsafe extern "C" fn create_buffer(initial_capacity: c_int) -> *mut StringBuffer {
    unsafe {
        let buffer = malloc(size_of::<StringBuffer>()) as *mut StringBuffer;
        if buffer.is_null() {
            return ptr::null_mut();
        }

        // `malloc(initial_capacity)`: the int argument is converted to size_t,
        // which sign-extends. A negative capacity therefore becomes a huge
        // request and malloc fails, exactly as in C. `as usize` on i32
        // sign-extends too.
        (*buffer).data = malloc(initial_capacity as usize) as *mut c_char;
        if (*buffer).data.is_null() {
            free(buffer as *mut c_void);
            return ptr::null_mut();
        }

        (*buffer).capacity = initial_capacity;
        (*buffer).length = 0;
        // buffer->data[0] = '\0';
        *(*buffer).data = 0;

        buffer
    }
}

// ---------------------------------------------------------------------------
// int append_to_buffer(StringBuffer *buffer, const char *str)
// ---------------------------------------------------------------------------
#[unsafe(no_mangle)]
pub unsafe extern "C" fn append_to_buffer(buffer: *mut StringBuffer, str_: *const c_char) -> c_int {
    unsafe {
        // `int str_len = strlen(str);` truncates size_t to int.
        let str_len = strlen(str_) as c_int;
        let required_capacity = (*buffer)
            .length
            .wrapping_add(str_len)
            .wrapping_add(1);

        if required_capacity > (*buffer).capacity {
            let new_capacity = required_capacity.wrapping_mul(2);
            let new_data =
                realloc((*buffer).data as *mut c_void, new_capacity as usize) as *mut c_char;

            if new_data.is_null() {
                return -1;
            }

            (*buffer).data = new_data;
            (*buffer).capacity = new_capacity;
        }

        strcpy((*buffer).data.offset((*buffer).length as isize), str_);
        (*buffer).length = (*buffer).length.wrapping_add(str_len);

        0
    }
}

// ---------------------------------------------------------------------------
// void destroy_buffer(StringBuffer *buffer)
// ---------------------------------------------------------------------------
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destroy_buffer(buffer: *mut StringBuffer) {
    unsafe {
        if !buffer.is_null() {
            if !(*buffer).data.is_null() {
                free((*buffer).data as *mut c_void);
            }
            free(buffer as *mut c_void);
        }
    }
}

// ---------------------------------------------------------------------------
// const char* get_operation_name(int op_code)
// ---------------------------------------------------------------------------
#[unsafe(no_mangle)]
pub extern "C" fn get_operation_name(op_code: c_int) -> *const c_char {
    match op_code {
        0 => OP_ADD,
        1 => OP_SUBTRACT,
        2 => OP_MULTIPLY,
        3 => OP_DIVIDE,
        _ => OP_UNKNOWN,
    }
}

// ---------------------------------------------------------------------------
// int perform_operation(int a, int b, const char *operation)
// ---------------------------------------------------------------------------
#[unsafe(no_mangle)]
pub unsafe extern "C" fn perform_operation(
    a: c_int,
    b: c_int,
    operation: *const c_char,
) -> c_int {
    unsafe {
        // Error/branch order preserved exactly: add, subtract, multiply, divide.
        if strcmp(operation, OP_ADD) == 0 {
            return a.wrapping_add(b);
        } else if strcmp(operation, OP_SUBTRACT) == 0 {
            return a.wrapping_sub(b);
        } else if strcmp(operation, OP_MULTIPLY) == 0 {
            return a.wrapping_mul(b);
        } else if strcmp(operation, OP_DIVIDE) == 0 {
            if b != 0 {
                return a.wrapping_div(b);
            }
            return 0;
        }
        0
    }
}

// ---------------------------------------------------------------------------
// int buffapp(int param1, int param2, int param3, int param4)
//
// NOTE: the C code does not check `create_buffer`'s result before writing
// through it, and does not check `append_to_buffer`'s return value. That
// behaviour is reproduced verbatim (no bug fixes).
// ---------------------------------------------------------------------------
#[unsafe(no_mangle)]
pub unsafe extern "C" fn buffapp(
    param1: c_int,
    param2: c_int,
    param3: c_int,
    param4: c_int,
) -> c_int {
    unsafe {
        let log_buffer = create_buffer(32);
        let mut result: c_int = 0;
        let mut temp = [0 as c_char; 64];

        // Unconditional deref, as in the C (no NULL check).
        (*log_buffer).length = 0;

        c_sprintf(temp.as_mut_ptr(), FMT_STARTING, 4 as c_int);
        append_to_buffer(log_buffer, temp.as_ptr());

        // `param1 % 4`: C's % truncates toward zero, and so does Rust's.
        let op1 = get_operation_name(param1.wrapping_rem(4));
        c_sprintf(temp.as_mut_ptr(), FMT_OP1, op1, param1, param2);
        append_to_buffer(log_buffer, temp.as_ptr());

        let intermediate1 = perform_operation(param1, param2, op1);
        result = result.wrapping_add(intermediate1);

        let op2 = get_operation_name(param3.wrapping_rem(4));
        c_sprintf(temp.as_mut_ptr(), FMT_OP2, op2, param3, param4);
        append_to_buffer(log_buffer, temp.as_ptr());

        let intermediate2 = perform_operation(param3, param4, op2);
        result = result.wrapping_add(intermediate2);

        let op3 = OP_MULTIPLY;
        c_sprintf(
            temp.as_mut_ptr(),
            FMT_OP3,
            op3,
            intermediate1,
            intermediate2,
        );
        append_to_buffer(log_buffer, temp.as_ptr());

        let intermediate3 = perform_operation(intermediate1, intermediate2, op3);

        if intermediate3 != 0 {
            result = result.wrapping_div(intermediate3);
        } else {
            result = param1
                .wrapping_add(param2)
                .wrapping_add(param3)
                .wrapping_add(param4);
        }

        c_sprintf(temp.as_mut_ptr(), FMT_FINAL, result);
        append_to_buffer(log_buffer, temp.as_ptr());

        c_printf(FMT_LOG, (*log_buffer).data);

        destroy_buffer(log_buffer);

        result
    }
}
