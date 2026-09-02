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
// Fidelity notes:
//  * The C allocator (malloc/realloc/free) is used so that buffers created by
//    `create_buffer` remain interchangeable with the C library's buffers and so
//    that allocation-failure behaviour matches exactly.
//  * The C formatted-output routines (`sprintf`, `printf`) are used so that the
//    emitted bytes and the stdout buffering behaviour are byte-identical.
//  * Integer arithmetic uses wrapping operations. The C code has signed
//    overflow / INT_MIN-division paths that are UB in C but wrap on the target
//    ABI; Rust would otherwise panic. Bugs in the C (missing NULL check on
//    `create_buffer`'s result in `buffapp`, `data[0]` write when
//    `initial_capacity == 0`, `int`-typed lengths) are preserved verbatim.

#![allow(clippy::missing_safety_doc)]

use core::ffi::{c_char, c_int, c_void};

extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn realloc(ptr: *mut c_void, size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);

    fn strlen(s: *const c_char) -> usize;
    fn strcpy(dest: *mut c_char, src: *const c_char) -> *mut c_char;
    fn strcmp(a: *const c_char, b: *const c_char) -> c_int;

    fn sprintf(s: *mut c_char, format: *const c_char, ...) -> c_int;
    fn printf(format: *const c_char, ...) -> c_int;
}

/// C `int / int`.
///
/// The C code guards against `b == 0`, but not against `INT_MIN / -1`, which is
/// UB in C and raises SIGFPE (#DE) on x86-64 because it is compiled to `idiv`.
/// `i32::wrapping_div` would quietly yield `INT_MIN` instead, so the raw `idiv`
/// is issued directly to keep the observable behaviour identical.
#[inline]
fn c_div(a: c_int, b: c_int) -> c_int {
    #[cfg(target_arch = "x86_64")]
    {
        let quot: c_int;
        unsafe {
            core::arch::asm!(
                "cdq",
                "idiv {divisor:e}",
                divisor = in(reg) b,
                inout("eax") a => quot,
                out("edx") _,
                options(nomem, nostack),
            );
        }
        quot
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        a.wrapping_div(b)
    }
}

/// Mirrors the anonymous C struct:
/// ```c
/// typedef struct {
///     char *data;
///     int capacity;
///     int length;
/// } StringBuffer;
/// ```
#[repr(C)]
pub struct StringBuffer {
    pub data: *mut c_char,
    pub capacity: c_int,
    pub length: c_int,
}

// ---------------------------------------------------------------------------
// StringBuffer* create_buffer(int initial_capacity)
// ---------------------------------------------------------------------------
#[unsafe(no_mangle)]
pub unsafe extern "C" fn create_buffer(initial_capacity: c_int) -> *mut StringBuffer {
    let buffer = malloc(core::mem::size_of::<StringBuffer>()) as *mut StringBuffer;
    if buffer.is_null() {
        return core::ptr::null_mut();
    }

    // `int` -> `size_t` conversion in C sign-extends; `as usize` does the same.
    let data = malloc(initial_capacity as usize) as *mut c_char;
    (*buffer).data = data;
    if data.is_null() {
        free(buffer as *mut c_void);
        return core::ptr::null_mut();
    }

    (*buffer).capacity = initial_capacity;
    (*buffer).length = 0;
    // Reproduced as-is: out of bounds when initial_capacity == 0.
    *(*buffer).data = 0;

    buffer
}

// ---------------------------------------------------------------------------
// int append_to_buffer(StringBuffer *buffer, const char *str)
// ---------------------------------------------------------------------------
#[unsafe(no_mangle)]
pub unsafe extern "C" fn append_to_buffer(buffer: *mut StringBuffer, str_: *const c_char) -> c_int {
    // C truncates size_t -> int here.
    let str_len: c_int = strlen(str_) as c_int;
    let required_capacity: c_int = (*buffer).length.wrapping_add(str_len).wrapping_add(1);

    if required_capacity > (*buffer).capacity {
        let new_capacity: c_int = required_capacity.wrapping_mul(2);
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

// ---------------------------------------------------------------------------
// void destroy_buffer(StringBuffer *buffer)
// ---------------------------------------------------------------------------
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destroy_buffer(buffer: *mut StringBuffer) {
    if !buffer.is_null() {
        if !(*buffer).data.is_null() {
            free((*buffer).data as *mut c_void);
        }
        free(buffer as *mut c_void);
    }
}

// ---------------------------------------------------------------------------
// const char* get_operation_name(int op_code)
// ---------------------------------------------------------------------------
const OP_ADD: &[u8] = b"add\0";
const OP_SUBTRACT: &[u8] = b"subtract\0";
const OP_MULTIPLY: &[u8] = b"multiply\0";
const OP_DIVIDE: &[u8] = b"divide\0";
const OP_UNKNOWN: &[u8] = b"unknown\0";

#[unsafe(no_mangle)]
pub extern "C" fn get_operation_name(op_code: c_int) -> *const c_char {
    let s: &[u8] = match op_code {
        0 => OP_ADD,
        1 => OP_SUBTRACT,
        2 => OP_MULTIPLY,
        3 => OP_DIVIDE,
        _ => OP_UNKNOWN,
    };
    s.as_ptr() as *const c_char
}

// ---------------------------------------------------------------------------
// int perform_operation(int a, int b, const char *operation)
// ---------------------------------------------------------------------------
#[unsafe(no_mangle)]
pub unsafe extern "C" fn perform_operation(a: c_int, b: c_int, operation: *const c_char) -> c_int {
    if strcmp(operation, OP_ADD.as_ptr() as *const c_char) == 0 {
        a.wrapping_add(b)
    } else if strcmp(operation, OP_SUBTRACT.as_ptr() as *const c_char) == 0 {
        a.wrapping_sub(b)
    } else if strcmp(operation, OP_MULTIPLY.as_ptr() as *const c_char) == 0 {
        a.wrapping_mul(b)
    } else if strcmp(operation, OP_DIVIDE.as_ptr() as *const c_char) == 0 {
        if b != 0 {
            return c_div(a, b);
        }
        0
    } else {
        0
    }
}

// ---------------------------------------------------------------------------
// int buffapp(int param1, int param2, int param3, int param4)
// ---------------------------------------------------------------------------
const FMT_STARTING: &[u8] = b"Starting computation with %d parameters\n\0";
const FMT_OP1: &[u8] = b"Operation 1: %s(%d, %d)\n\0";
const FMT_OP2: &[u8] = b"Operation 2: %s(%d, %d)\n\0";
const FMT_OP3: &[u8] = b"Operation 3: %s(%d, %d)\n\0";
const FMT_FINAL: &[u8] = b"Final result: %d\n\0";
const FMT_LOG: &[u8] = b"Computation Log:\n%s\n\0";

#[unsafe(no_mangle)]
pub unsafe extern "C" fn buffapp(
    param1: c_int,
    param2: c_int,
    param3: c_int,
    param4: c_int,
) -> c_int {
    let log_buffer: *mut StringBuffer = create_buffer(32);
    let mut result: c_int = 0;
    let mut temp = [0 as c_char; 64];
    let temp_ptr = temp.as_mut_ptr();

    // Reproduced as-is: the C code does not check `log_buffer` for NULL.
    (*log_buffer).length = 0;

    sprintf(temp_ptr, FMT_STARTING.as_ptr() as *const c_char, 4 as c_int);
    append_to_buffer(log_buffer, temp_ptr);

    let op1: *const c_char = get_operation_name(param1.wrapping_rem(4));
    sprintf(
        temp_ptr,
        FMT_OP1.as_ptr() as *const c_char,
        op1,
        param1,
        param2,
    );
    append_to_buffer(log_buffer, temp_ptr);

    let intermediate1: c_int = perform_operation(param1, param2, op1);
    result = result.wrapping_add(intermediate1);

    let op2: *const c_char = get_operation_name(param3.wrapping_rem(4));
    sprintf(
        temp_ptr,
        FMT_OP2.as_ptr() as *const c_char,
        op2,
        param3,
        param4,
    );
    append_to_buffer(log_buffer, temp_ptr);

    let intermediate2: c_int = perform_operation(param3, param4, op2);
    result = result.wrapping_add(intermediate2);

    let op3: *const c_char = OP_MULTIPLY.as_ptr() as *const c_char;
    sprintf(
        temp_ptr,
        FMT_OP3.as_ptr() as *const c_char,
        op3,
        intermediate1,
        intermediate2,
    );
    append_to_buffer(log_buffer, temp_ptr);

    let intermediate3: c_int = perform_operation(intermediate1, intermediate2, op3);

    if intermediate3 != 0 {
        result = c_div(result, intermediate3);
    } else {
        result = param1
            .wrapping_add(param2)
            .wrapping_add(param3)
            .wrapping_add(param4);
    }

    sprintf(temp_ptr, FMT_FINAL.as_ptr() as *const c_char, result);
    append_to_buffer(log_buffer, temp_ptr);

    printf(FMT_LOG.as_ptr() as *const c_char, (*log_buffer).data);

    destroy_buffer(log_buffer);

    result
}
