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

use std::ffi::{c_char, c_int};
use std::sync::atomic::{AtomicI32, Ordering};

// ---------------------------------------------------------------------------
// File-scope (`static`) mutable state from the C translation unit.
//
// C:
//     static int accumulator = 0;
//     static int multiplier = 1;
//     static int operation_count = 0;
//
// Modelled with relaxed atomics so the translation contains no `static mut`.
// Every access is a plain load / store in program order, which reproduces the
// C semantics exactly for the single-threaded use the C code assumes.
// ---------------------------------------------------------------------------
static ACCUMULATOR: AtomicI32 = AtomicI32::new(0);
static MULTIPLIER: AtomicI32 = AtomicI32::new(1);
static OPERATION_COUNT: AtomicI32 = AtomicI32::new(0);

#[inline]
fn acc_get() -> i32 {
    ACCUMULATOR.load(Ordering::Relaxed)
}

#[inline]
fn acc_set(v: i32) {
    ACCUMULATOR.store(v, Ordering::Relaxed);
}

#[inline]
fn mul_get() -> i32 {
    MULTIPLIER.load(Ordering::Relaxed)
}

#[inline]
fn mul_set(v: i32) {
    MULTIPLIER.store(v, Ordering::Relaxed);
}

#[inline]
fn op_count_get() -> i32 {
    OPERATION_COUNT.load(Ordering::Relaxed)
}

#[inline]
fn op_count_inc() {
    // C: operation_count++ (wrapping is the practical behaviour of signed
    // overflow on the targets this library is built for).
    OPERATION_COUNT.store(op_count_get().wrapping_add(1), Ordering::Relaxed);
}

// ---------------------------------------------------------------------------
// typedef int (*operation_func)(int, int);
// ---------------------------------------------------------------------------
type OperationFunc = extern "C" fn(c_int, c_int) -> c_int;

// ---------------------------------------------------------------------------
// Small libc-equivalent helpers, kept byte-for-byte faithful to the C ones.
// ---------------------------------------------------------------------------

/// `strlen(s)`
unsafe fn c_strlen(s: *const c_char) -> usize {
    let mut n = 0usize;
    unsafe {
        while *s.add(n) != 0 {
            n += 1;
        }
    }
    n
}

/// `strcpy(dest, src_bytes)` where `src_bytes` carries no NUL terminator; the
/// terminator is appended, exactly like `strcpy` copying a C string.
unsafe fn c_strcpy_from_slice(dest: *mut c_char, src: &[u8]) {
    unsafe {
        for (i, b) in src.iter().enumerate() {
            *dest.add(i) = *b as c_char;
        }
        *dest.add(src.len()) = 0;
    }
}

/// `memchr(haystack, needle, n)` returning the index of the first match.
fn c_memchr_index(haystack: &[u8], needle: u8) -> Option<usize> {
    haystack.iter().position(|&b| b == needle)
}

/// `sprintf(buf, "Octal: 0%o, Decimal: %d", v, v)`
///
/// `%o` consumes the argument as `unsigned int`, so a negative `int` is printed
/// as its two's-complement bit pattern; `%d` prints it as signed.
fn format_octal_message(v: i32) -> Vec<u8> {
    format!("Octal: 0{:o}, Decimal: {}", v as u32, v).into_bytes()
}

// ---------------------------------------------------------------------------
// int add_to_accumulator(int a, int b)
// ---------------------------------------------------------------------------
#[unsafe(no_mangle)]
pub extern "C" fn add_to_accumulator(a: c_int, b: c_int) -> c_int {
    let v = acc_get().wrapping_add(a.wrapping_add(b));
    acc_set(v);
    op_count_inc();
    v
}

// ---------------------------------------------------------------------------
// int multiply_with_multiplier(int a, int b)
// ---------------------------------------------------------------------------
#[unsafe(no_mangle)]
pub extern "C" fn multiply_with_multiplier(a: c_int, b: c_int) -> c_int {
    let v = mul_get().wrapping_mul(a.wrapping_mul(b));
    mul_set(v);
    op_count_inc();
    v
}

// ---------------------------------------------------------------------------
// int subtract_from_accumulator(int a, int b)
// ---------------------------------------------------------------------------
#[unsafe(no_mangle)]
pub extern "C" fn subtract_from_accumulator(a: c_int, b: c_int) -> c_int {
    let v = acc_get().wrapping_sub(a.wrapping_sub(b));
    acc_set(v);
    op_count_inc();
    v
}

// ---------------------------------------------------------------------------
// int divide_multiplier(int a, int b)
//
// `a` is unused in the C original; the guard is only on `b`.
// ---------------------------------------------------------------------------
#[unsafe(no_mangle)]
pub extern "C" fn divide_multiplier(_a: c_int, b: c_int) -> c_int {
    if b != 0 {
        // wrapping_div reproduces INT_MIN / -1 == INT_MIN without panicking.
        mul_set(mul_get().wrapping_div(b));
    }
    op_count_inc();
    mul_get()
}

// ---------------------------------------------------------------------------
// void process_octal_string(char* dest, int octal_val)
// ---------------------------------------------------------------------------
#[unsafe(no_mangle)]
pub unsafe extern "C" fn process_octal_string(dest: *mut c_char, octal_val: c_int) {
    // C uses a 50-byte stack buffer for the sprintf result and then strcpy's
    // it into `dest`.  The formatted text is at most 41 characters, so the
    // intermediate buffer never overflows and the copy is a plain strcpy.
    let buffer = format_octal_message(octal_val);
    unsafe {
        c_strcpy_from_slice(dest, &buffer);
    }
}

// ---------------------------------------------------------------------------
// void find_and_replace_char(char* str, int search_char)
// ---------------------------------------------------------------------------
#[unsafe(no_mangle)]
pub unsafe extern "C" fn find_and_replace_char(str_: *mut c_char, search_char: c_int) {
    unsafe {
        let len = c_strlen(str_ as *const c_char);
        let bytes = std::slice::from_raw_parts(str_ as *const u8, len);
        // memchr converts the search value to `unsigned char`.
        if let Some(idx) = c_memchr_index(bytes, search_char as u8) {
            *str_.add(idx) = b'X' as c_char;
        }
    }
}

// ---------------------------------------------------------------------------
// int validate_and_normalize(int value)
// ---------------------------------------------------------------------------
#[unsafe(no_mangle)]
pub extern "C" fn validate_and_normalize(value: c_int) -> c_int {
    let is_nonzero = (value != 0) as c_int;
    let _is_zero = (value == 0) as c_int; // computed but unused in the C source

    let lower_threshold: c_int = 0o100; // 64
    let upper_threshold: c_int = 0o777; // 511

    if is_nonzero != 0 && value > 0 {
        if value < lower_threshold {
            return lower_threshold;
        } else if value > upper_threshold {
            return upper_threshold;
        }
    }

    value
}

// ---------------------------------------------------------------------------
// static operation_func operations[4] = { ... };
// ---------------------------------------------------------------------------
static OPERATIONS: [OperationFunc; 4] = [
    add_to_accumulator,
    multiply_with_multiplier,
    subtract_from_accumulator,
    divide_multiplier,
];

// ---------------------------------------------------------------------------
// int findrep(int param1, int param2, int param3, int param4)
// ---------------------------------------------------------------------------
#[unsafe(no_mangle)]
pub extern "C" fn findrep(
    param1: c_int,
    param2: c_int,
    param3: c_int,
    param4: c_int,
) -> c_int {
    let mut result: c_int = 0;

    let p1_valid = (param1 != 0) as c_int;
    let p2_valid = (param2 != 0) as c_int;
    let p3_valid = (param3 != 0) as c_int;
    let p4_valid = (param4 != 0) as c_int;

    let active_params = p1_valid + p2_valid + p3_valid + p4_valid;

    let mode_add: c_int = 0o1;
    let mode_multiply: c_int = 0o2;
    let _mode_subtract: c_int = 0o3; // unused in the C source
    let _mode_divide: c_int = 0o4; // unused in the C source

    let normalized_p1 = validate_and_normalize(param1);
    let normalized_p2 = validate_and_normalize(param2);
    let normalized_p3 = validate_and_normalize(param3);
    let normalized_p4 = validate_and_normalize(param4);

    let mut message: [c_char; 100] = [0; 100];
    let mut search_buffer: [c_char; 100] = [0; 100];

    unsafe {
        process_octal_string(message.as_mut_ptr(), 0o123);
        c_strcpy_from_slice(
            search_buffer.as_mut_ptr(),
            b"Function pointer example with static vars",
        );
    }

    // char* found_char = memchr(search_buffer, 'p', strlen(search_buffer));
    // if (found_char) result += (int)(found_char - search_buffer);
    let search_len = unsafe { c_strlen(search_buffer.as_ptr()) };
    let search_bytes =
        unsafe { std::slice::from_raw_parts(search_buffer.as_ptr() as *const u8, search_len) };
    if let Some(idx) = c_memchr_index(search_bytes, b'p') {
        result = result.wrapping_add(idx as c_int);
    }

    let mut selected_op: OperationFunc;

    if active_params >= mode_add {
        selected_op = OPERATIONS[0];
        result = result.wrapping_add(selected_op(normalized_p1, normalized_p2));
    }

    if active_params >= mode_multiply {
        selected_op = OPERATIONS[1];
        result = result.wrapping_add(selected_op(normalized_p3, normalized_p4));
    }

    if acc_get() > 0o150 {
        selected_op = OPERATIONS[2];
        let subtract_result = selected_op(normalized_p1, normalized_p3);
        result = result.wrapping_add(subtract_result);
    }

    unsafe {
        find_and_replace_char(message.as_mut_ptr(), b'O' as c_int);
    }

    // char final_message[100]; strcpy(final_message, message);  -- unused
    let mut final_message: [c_char; 100] = [0; 100];
    unsafe {
        let len = c_strlen(message.as_ptr());
        let src = std::slice::from_raw_parts(message.as_ptr() as *const u8, len);
        c_strcpy_from_slice(final_message.as_mut_ptr(), src);
    }
    let _ = &final_message;

    let has_accumulator = (acc_get() != 0) as c_int;
    let has_multiplier = (mul_get() != 0) as c_int;
    let both_active = (has_accumulator != 0 && has_multiplier != 0) as c_int;

    if both_active != 0 {
        result = result.wrapping_add(acc_get().wrapping_add(mul_get()));
    }

    if mul_get() > 0o100 {
        selected_op = OPERATIONS[3];
        selected_op(mul_get(), 2);
    }

    result = result.wrapping_add(op_count_get().wrapping_mul(0o10));

    let result_exists = (result != 0) as c_int;
    if result_exists == 0 {
        result = 0o777;
    }

    result
}
