// Rust translation of c_src/src/lib.c
//
// Original C source:
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

use std::ffi::{c_char, c_int, c_uint};
use std::sync::atomic::{AtomicI32, Ordering};

// ---------------------------------------------------------------------------
// Function pointer typedefs from the C source:
//   typedef int  (*operation_func)(int, int);
//   typedef void (*string_processor)(char*, int);
// ---------------------------------------------------------------------------
type OperationFunc = unsafe extern "C" fn(c_int, c_int) -> c_int;
#[allow(dead_code)]
type StringProcessor = unsafe extern "C" fn(*mut c_char, c_int);

// ---------------------------------------------------------------------------
// File-scope (static) mutable state.
//   static int accumulator = 0;
//   static int multiplier = 1;
//   static int operation_count = 0;
// The C library keeps this state across calls; reproduce that behaviour.
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
fn opcount_get() -> i32 {
    OPERATION_COUNT.load(Ordering::Relaxed)
}
#[inline]
fn opcount_inc() {
    OPERATION_COUNT.store(opcount_get().wrapping_add(1), Ordering::Relaxed);
}

// ---------------------------------------------------------------------------
// Minimal C string helpers (strlen / memchr / strcpy semantics).
// ---------------------------------------------------------------------------

/// Equivalent of `strlen(s)`.
unsafe fn c_strlen(s: *const c_char) -> usize {
    let mut n: usize = 0;
    while *s.add(n) != 0 {
        n += 1;
    }
    n
}

/// Equivalent of `memchr(s, ch, n)`; `ch` is converted to `unsigned char`
/// exactly like the C library does.
unsafe fn c_memchr(s: *const c_char, ch: c_int, n: usize) -> *mut c_char {
    let needle = ch as u8;
    let mut i: usize = 0;
    while i < n {
        if *(s.add(i) as *const u8) == needle {
            return s.add(i) as *mut c_char;
        }
        i += 1;
    }
    std::ptr::null_mut()
}

/// Equivalent of `strcpy(dest, src_bytes)` where `src_bytes` does not contain
/// a NUL: copies the bytes then the terminating NUL.
unsafe fn c_strcpy_from_slice(dest: *mut c_char, src: &[u8]) {
    let mut i: usize = 0;
    while i < src.len() {
        *(dest.add(i) as *mut u8) = src[i];
        i += 1;
    }
    *(dest.add(src.len()) as *mut u8) = 0;
}

/// Equivalent of `strcpy(dest, src)` for two C strings.
unsafe fn c_strcpy(dest: *mut c_char, src: *const c_char) -> *mut c_char {
    let mut i: usize = 0;
    loop {
        let b = *(src.add(i) as *const u8);
        *(dest.add(i) as *mut u8) = b;
        if b == 0 {
            break;
        }
        i += 1;
    }
    dest
}

/// Renders `sprintf(buffer, "Octal: 0%o, Decimal: %d", v, v)` into bytes.
/// `%o` formats the value as `unsigned int`, `%d` as signed `int`.
fn format_octal_message(octal_val: c_int) -> Vec<u8> {
    let unsigned = octal_val as c_uint;
    format!("Octal: 0{:o}, Decimal: {}", unsigned, octal_val).into_bytes()
}

// ---------------------------------------------------------------------------
// Public ABI
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn add_to_accumulator(a: c_int, b: c_int) -> c_int {
    acc_set(acc_get().wrapping_add(a.wrapping_add(b)));
    opcount_inc();
    acc_get()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn multiply_with_multiplier(a: c_int, b: c_int) -> c_int {
    mul_set(mul_get().wrapping_mul(a.wrapping_mul(b)));
    opcount_inc();
    mul_get()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn subtract_from_accumulator(a: c_int, b: c_int) -> c_int {
    acc_set(acc_get().wrapping_sub(a.wrapping_sub(b)));
    opcount_inc();
    acc_get()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn divide_multiplier(_a: c_int, b: c_int) -> c_int {
    if b != 0 {
        mul_set(mul_get().wrapping_div(b));
    }
    opcount_inc();
    mul_get()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn process_octal_string(dest: *mut c_char, octal_val: c_int) {
    // char buffer[50]; sprintf(buffer, "Octal: 0%o, Decimal: %d", v, v);
    // strcpy(dest, buffer);
    let rendered = format_octal_message(octal_val);
    let mut buffer = [0u8; 50];
    let n = if rendered.len() < buffer.len() {
        rendered.len()
    } else {
        buffer.len() - 1
    };
    buffer[..n].copy_from_slice(&rendered[..n]);
    c_strcpy_from_slice(dest, &buffer[..n]);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn find_and_replace_char(str_: *mut c_char, search_char: c_int) {
    let found = c_memchr(str_, search_char, c_strlen(str_));
    if !found.is_null() {
        *(found as *mut u8) = b'X';
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn validate_and_normalize(value: c_int) -> c_int {
    let is_nonzero: c_int = (value != 0) as c_int;
    let _is_zero: c_int = (value == 0) as c_int;

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

// static operation_func operations[4] = { ... };
static OPERATIONS: [OperationFunc; 4] = [
    add_to_accumulator,
    multiply_with_multiplier,
    subtract_from_accumulator,
    divide_multiplier,
];

#[unsafe(no_mangle)]
pub unsafe extern "C" fn findrep(
    param1: c_int,
    param2: c_int,
    param3: c_int,
    param4: c_int,
) -> c_int {
    let mut result: c_int = 0;

    let p1_valid: c_int = (param1 != 0) as c_int;
    let p2_valid: c_int = (param2 != 0) as c_int;
    let p3_valid: c_int = (param3 != 0) as c_int;
    let p4_valid: c_int = (param4 != 0) as c_int;

    let active_params: c_int = p1_valid + p2_valid + p3_valid + p4_valid;

    let mode_add: c_int = 0o1;
    let mode_multiply: c_int = 0o2;
    let _mode_subtract: c_int = 0o3;
    let _mode_divide: c_int = 0o4;

    let normalized_p1 = validate_and_normalize(param1);
    let normalized_p2 = validate_and_normalize(param2);
    let normalized_p3 = validate_and_normalize(param3);
    let normalized_p4 = validate_and_normalize(param4);

    let mut message = [0u8; 100];
    let mut search_buffer = [0u8; 100];

    process_octal_string(message.as_mut_ptr() as *mut c_char, 0o123);
    c_strcpy_from_slice(
        search_buffer.as_mut_ptr() as *mut c_char,
        b"Function pointer example with static vars",
    );

    let search_ptr = search_buffer.as_mut_ptr() as *mut c_char;
    let found_char = c_memchr(search_ptr, b'p' as c_int, c_strlen(search_ptr));
    if !found_char.is_null() {
        result = result.wrapping_add((found_char as isize - search_ptr as isize) as c_int);
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

    find_and_replace_char(message.as_mut_ptr() as *mut c_char, b'O' as c_int);

    let mut final_message = [0u8; 100];
    c_strcpy(
        final_message.as_mut_ptr() as *mut c_char,
        message.as_ptr() as *const c_char,
    );
    // `final_message` is unused afterwards in the C source, but the copy is
    // performed for fidelity.
    let _ = &final_message;

    let has_accumulator: c_int = (acc_get() != 0) as c_int;
    let has_multiplier: c_int = (mul_get() != 0) as c_int;
    let both_active: c_int = (has_accumulator != 0 && has_multiplier != 0) as c_int;

    if both_active != 0 {
        result = result.wrapping_add(acc_get().wrapping_add(mul_get()));
    }

    if mul_get() > 0o100 {
        selected_op = OPERATIONS[3];
        selected_op(mul_get(), 2);
    }

    result = result.wrapping_add(opcount_get().wrapping_mul(0o10));

    let result_exists: c_int = (result != 0) as c_int;
    if result_exists == 0 {
        result = 0o777;
    }

    result
}
