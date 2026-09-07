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

#![allow(non_camel_case_types)]

use core::ffi::{c_char, c_int, c_void, CStr};

// ---------------------------------------------------------------------------
// libc bindings.
//
// The C translation unit is compiled against the platform libc; using the very
// same allocator, string routines and stdio streams keeps the observable
// behaviour (heap interoperability with C callers, stdout buffering, and the
// exact bytes written by the `printf` family) bit-for-bit identical.
// ---------------------------------------------------------------------------
extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn realloc(ptr: *mut c_void, size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);

    fn strlen(s: *const c_char) -> usize;
    fn strcpy(dst: *mut c_char, src: *const c_char) -> *mut c_char;
    fn strcmp(a: *const c_char, b: *const c_char) -> c_int;

    fn sprintf(buf: *mut c_char, fmt: *const c_char, ...) -> c_int;
    fn printf(fmt: *const c_char, ...) -> c_int;
}

/// `typedef struct { char *data; int capacity; int length; } StringBuffer;`
#[repr(C)]
pub struct StringBuffer {
    pub data: *mut c_char,
    pub capacity: c_int,
    pub length: c_int,
}

/// C converts a (possibly negative) `int` argument to the `size_t` parameter of
/// `malloc`/`realloc` by sign-extending it to pointer width.
#[inline]
fn int_to_size(v: c_int) -> usize {
    v as isize as usize
}

/// C's `a / b` for a non-zero divisor.
///
/// For every representable quotient this is plain truncating division, matching
/// Rust's `/`. The one remaining case, `INT_MIN / -1`, is undefined in C; the
/// code gcc actually emits is a bare `idiv`, which raises `SIGFPE` because the
/// quotient does not fit in `eax`. Rust's `/` would instead panic, so the raw
/// instruction is issued to keep the observable behaviour identical.
#[inline]
unsafe fn c_div(a: c_int, b: c_int) -> c_int {
    #[cfg(target_arch = "x86_64")]
    {
        let mut quot: c_int = a;
        let mut _rem: c_int;
        core::arch::asm!(
            "cdq",
            "idiv {divisor:e}",
            divisor = in(reg) b,
            inout("eax") quot,
            out("edx") _rem,
            options(nostack),
        );
        quot
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        a.wrapping_div(b)
    }
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

    (*buffer).data = malloc(int_to_size(initial_capacity)) as *mut c_char;
    if (*buffer).data.is_null() {
        free(buffer as *mut c_void);
        return core::ptr::null_mut();
    }

    (*buffer).capacity = initial_capacity;
    (*buffer).length = 0;
    // buffer->data[0] = '\0';
    *(*buffer).data = 0;

    buffer
}

// ---------------------------------------------------------------------------
// int append_to_buffer(StringBuffer *buffer, const char *str)
// ---------------------------------------------------------------------------
#[unsafe(no_mangle)]
pub unsafe extern "C" fn append_to_buffer(buffer: *mut StringBuffer, str_: *const c_char) -> c_int {
    // `int str_len = strlen(str);` -- size_t truncated to int.
    let str_len: c_int = strlen(str_) as c_int;
    let required_capacity: c_int = (*buffer)
        .length
        .wrapping_add(str_len)
        .wrapping_add(1);

    if required_capacity > (*buffer).capacity {
        let new_capacity: c_int = required_capacity.wrapping_mul(2);
        let new_data = realloc((*buffer).data as *mut c_void, int_to_size(new_capacity)) as *mut c_char;

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
static OP_ADD: &CStr = c"add";
static OP_SUBTRACT: &CStr = c"subtract";
static OP_MULTIPLY: &CStr = c"multiply";
static OP_DIVIDE: &CStr = c"divide";
static OP_UNKNOWN: &CStr = c"unknown";

#[unsafe(no_mangle)]
pub unsafe extern "C" fn get_operation_name(op_code: c_int) -> *const c_char {
    match op_code {
        0 => OP_ADD.as_ptr(),
        1 => OP_SUBTRACT.as_ptr(),
        2 => OP_MULTIPLY.as_ptr(),
        3 => OP_DIVIDE.as_ptr(),
        _ => OP_UNKNOWN.as_ptr(),
    }
}

// ---------------------------------------------------------------------------
// int perform_operation(int a, int b, const char *operation)
// ---------------------------------------------------------------------------
#[unsafe(no_mangle)]
pub unsafe extern "C" fn perform_operation(a: c_int, b: c_int, operation: *const c_char) -> c_int {
    if strcmp(operation, OP_ADD.as_ptr()) == 0 {
        a.wrapping_add(b)
    } else if strcmp(operation, OP_SUBTRACT.as_ptr()) == 0 {
        a.wrapping_sub(b)
    } else if strcmp(operation, OP_MULTIPLY.as_ptr()) == 0 {
        a.wrapping_mul(b)
    } else if strcmp(operation, OP_DIVIDE.as_ptr()) == 0 {
        if b != 0 {
            c_div(a, b)
        } else {
            0
        }
    } else {
        0
    }
}

// ---------------------------------------------------------------------------
// int buffapp(int param1, int param2, int param3, int param4)
// ---------------------------------------------------------------------------
static FMT_START: &CStr = c"Starting computation with %d parameters\n";
static FMT_OP1: &CStr = c"Operation 1: %s(%d, %d)\n";
static FMT_OP2: &CStr = c"Operation 2: %s(%d, %d)\n";
static FMT_OP3: &CStr = c"Operation 3: %s(%d, %d)\n";
static FMT_FINAL: &CStr = c"Final result: %d\n";
static FMT_LOG: &CStr = c"Computation Log:\n%s\n";

#[unsafe(no_mangle)]
pub unsafe extern "C" fn buffapp(
    param1: c_int,
    param2: c_int,
    param3: c_int,
    param4: c_int,
) -> c_int {
    let log_buffer: *mut StringBuffer = create_buffer(32);
    let mut result: c_int = 0;
    let mut temp: [c_char; 64] = [0; 64];
    let temp_ptr = temp.as_mut_ptr();

    // The C code dereferences `log_buffer` unconditionally.
    (*log_buffer).length = 0;

    sprintf(temp_ptr, FMT_START.as_ptr(), 4 as c_int);
    append_to_buffer(log_buffer, temp_ptr as *const c_char);

    let op1 = get_operation_name(param1.wrapping_rem(4));
    sprintf(
        temp_ptr,
        FMT_OP1.as_ptr(),
        op1,
        param1,
        param2,
    );
    append_to_buffer(log_buffer, temp_ptr as *const c_char);

    let intermediate1 = perform_operation(param1, param2, op1);
    result = result.wrapping_add(intermediate1);

    let op2 = get_operation_name(param3.wrapping_rem(4));
    sprintf(
        temp_ptr,
        FMT_OP2.as_ptr(),
        op2,
        param3,
        param4,
    );
    append_to_buffer(log_buffer, temp_ptr as *const c_char);

    let intermediate2 = perform_operation(param3, param4, op2);
    result = result.wrapping_add(intermediate2);

    let op3 = OP_MULTIPLY.as_ptr();
    sprintf(
        temp_ptr,
        FMT_OP3.as_ptr(),
        op3,
        intermediate1,
        intermediate2,
    );
    append_to_buffer(log_buffer, temp_ptr as *const c_char);

    let intermediate3 = perform_operation(intermediate1, intermediate2, op3);

    if intermediate3 != 0 {
        result = c_div(result, intermediate3);
    } else {
        result = param1
            .wrapping_add(param2)
            .wrapping_add(param3)
            .wrapping_add(param4);
    }

    sprintf(temp_ptr, FMT_FINAL.as_ptr(), result);
    append_to_buffer(log_buffer, temp_ptr as *const c_char);

    printf(
        FMT_LOG.as_ptr(),
        (*log_buffer).data as *const c_char,
    );

    destroy_buffer(log_buffer);

    result
}
