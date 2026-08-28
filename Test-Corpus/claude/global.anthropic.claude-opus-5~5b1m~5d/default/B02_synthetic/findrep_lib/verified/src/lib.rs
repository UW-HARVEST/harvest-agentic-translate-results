// Rust translation of c_src/src/lib.c
//
// Original copyright header of the translated C source:
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

use std::ffi::{c_char, c_int, c_uint};

// ---------------------------------------------------------------------------
// C typedefs
//
//   typedef int (*operation_func)(int, int);
//   typedef void (*string_processor)(char*, int);
// ---------------------------------------------------------------------------

type OperationFunc = unsafe extern "C" fn(c_int, c_int) -> c_int;

#[allow(dead_code)]
type StringProcessor = unsafe extern "C" fn(*mut c_char, c_int);

// ---------------------------------------------------------------------------
// File-scope (`static`) mutable state shared by every entry point.
//
//   static int accumulator = 0;
//   static int multiplier = 1;
//   static int operation_count = 0;
// ---------------------------------------------------------------------------

static mut ACCUMULATOR: c_int = 0;
static mut MULTIPLIER: c_int = 1;
static mut OPERATION_COUNT: c_int = 0;

// ---------------------------------------------------------------------------
// Small libc helpers, reproducing the exact semantics relied upon by the C.
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

/// `strcpy(dest, src_bytes)` where `src_bytes` carries no NUL terminator.
/// Copies the bytes followed by the terminating NUL, exactly like `strcpy`.
unsafe fn c_strcpy_from(dest: *mut c_char, src: &[u8]) {
    unsafe {
        let mut i = 0usize;
        while i < src.len() {
            *dest.add(i) = src[i] as c_char;
            i += 1;
        }
        *dest.add(src.len()) = 0;
    }
}

/// `memchr(haystack, needle, len)` -> index of the first match.
/// `memchr` compares after converting `needle` to `unsigned char`.
fn c_memchr(haystack: &[u8], needle: c_int) -> Option<usize> {
    let needle = needle as u8;
    let mut i = 0usize;
    while i < haystack.len() {
        if haystack[i] == needle {
            return Some(i);
        }
        i += 1;
    }
    None
}

/// Faithful `int / int` in the sense of the compiled C.
///
/// C leaves both `x / 0` and `INT_MIN / -1` undefined. gcc lowers `multiplier /= b`
/// to a single `idiv`, and on x86-64 *both* of those operands raise the `#DE`
/// (divide error) fault, i.e. the process dies with **SIGFPE**. Returning a value
/// there — as `wrapping_div` does — would silently diverge from the C, so reproduce
/// the trap instead.
#[inline]
fn c_idiv(a: c_int, b: c_int) -> c_int {
    if b == 0 || (a == c_int::MIN && b == -1) {
        integer_division_trap();
    }
    a.wrapping_div(b)
}

/// Raise the same `#DE` fault the C's `idiv` raises.
#[cfg(target_arch = "x86_64")]
#[inline(never)]
fn integer_division_trap() -> ! {
    unsafe {
        // edx:eax = sign-extended INT_MIN, divided by -1: the true quotient 2^31 is
        // not representable in eax, so `idiv` raises #DE -> SIGFPE.
        core::arch::asm!(
            "cdq",
            "idiv {b:e}",
            b = in(reg) -1i32,
            inout("eax") c_int::MIN => _,
            out("edx") _,
            options(nostack),
        );
    }
    // #DE is a fault, not a trap: the faulting instruction is restarted rather than
    // skipped, so control never arrives here.
    unreachable!("idiv did not raise SIGFPE")
}

#[cfg(not(target_arch = "x86_64"))]
#[inline(never)]
fn integer_division_trap() -> ! {
    const SIGFPE: c_int = 8;
    unsafe extern "C" {
        fn raise(sig: c_int) -> c_int;
    }
    unsafe { raise(SIGFPE) };
    std::process::abort()
}

/// `sprintf(buffer, "Octal: 0%o, Decimal: %d", octal_val, octal_val)`
///
/// `%o` formats the argument as `unsigned int`, so negative values are printed
/// as their two's-complement 32-bit octal representation; `%d` is signed.
fn format_octal_message(octal_val: c_int) -> Vec<u8> {
    format!(
        "Octal: 0{:o}, Decimal: {}",
        octal_val as c_uint, octal_val
    )
    .into_bytes()
}

// ---------------------------------------------------------------------------
// int add_to_accumulator(int a, int b)
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn add_to_accumulator(a: c_int, b: c_int) -> c_int {
    unsafe {
        ACCUMULATOR = ACCUMULATOR.wrapping_add(a.wrapping_add(b));
        OPERATION_COUNT = OPERATION_COUNT.wrapping_add(1);
        ACCUMULATOR
    }
}

// ---------------------------------------------------------------------------
// int multiply_with_multiplier(int a, int b)
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn multiply_with_multiplier(a: c_int, b: c_int) -> c_int {
    unsafe {
        MULTIPLIER = MULTIPLIER.wrapping_mul(a.wrapping_mul(b));
        OPERATION_COUNT = OPERATION_COUNT.wrapping_add(1);
        MULTIPLIER
    }
}

// ---------------------------------------------------------------------------
// int subtract_from_accumulator(int a, int b)
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn subtract_from_accumulator(a: c_int, b: c_int) -> c_int {
    unsafe {
        ACCUMULATOR = ACCUMULATOR.wrapping_sub(a.wrapping_sub(b));
        OPERATION_COUNT = OPERATION_COUNT.wrapping_add(1);
        ACCUMULATOR
    }
}

// ---------------------------------------------------------------------------
// int divide_multiplier(int a, int b)
//
// NOTE: `a` is unused by the C implementation; the guard only rejects b == 0.
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn divide_multiplier(_a: c_int, b: c_int) -> c_int {
    unsafe {
        if b != 0 {
            MULTIPLIER = c_idiv(MULTIPLIER, b);
        }
        OPERATION_COUNT = OPERATION_COUNT.wrapping_add(1);
        MULTIPLIER
    }
}

// ---------------------------------------------------------------------------
// void process_octal_string(char* dest, int octal_val)
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn process_octal_string(dest: *mut c_char, octal_val: c_int) {
    // char buffer[50]; sprintf(buffer, ...); strcpy(dest, buffer);
    let buffer = format_octal_message(octal_val);
    unsafe { c_strcpy_from(dest, &buffer) };
}

// ---------------------------------------------------------------------------
// void find_and_replace_char(char* str, int search_char)
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn find_and_replace_char(str: *mut c_char, search_char: c_int) {
    unsafe {
        let len = c_strlen(str);
        let needle = search_char as u8;
        let mut i = 0usize;
        while i < len {
            if *str.add(i) as u8 == needle {
                *str.add(i) = b'X' as c_char;
                return;
            }
            i += 1;
        }
    }
}

// ---------------------------------------------------------------------------
// int validate_and_normalize(int value)
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn validate_and_normalize(value: c_int) -> c_int {
    let is_nonzero: c_int = (value != 0) as c_int;
    let _is_zero: c_int = (value == 0) as c_int;

    let lower_threshold: c_int = 0o100;
    let upper_threshold: c_int = 0o777;

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
pub unsafe extern "C" fn findrep(
    param1: c_int,
    param2: c_int,
    param3: c_int,
    param4: c_int,
) -> c_int {
    unsafe {
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
        c_strcpy_from(
            search_buffer.as_mut_ptr() as *mut c_char,
            b"Function pointer example with static vars",
        );

        // char* found_char = memchr(search_buffer, 'p', strlen(search_buffer));
        let search_len = c_strlen(search_buffer.as_ptr() as *const c_char);
        if let Some(offset) = c_memchr(&search_buffer[..search_len], b'p' as c_int) {
            result = result.wrapping_add(offset as c_int);
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

        if ACCUMULATOR > 0o150 {
            selected_op = OPERATIONS[2];
            let subtract_result = selected_op(normalized_p1, normalized_p3);
            result = result.wrapping_add(subtract_result);
        }

        find_and_replace_char(message.as_mut_ptr() as *mut c_char, b'O' as c_int);

        // char final_message[100]; strcpy(final_message, message);
        let mut final_message = [0u8; 100];
        {
            let message_len = c_strlen(message.as_ptr() as *const c_char);
            c_strcpy_from(
                final_message.as_mut_ptr() as *mut c_char,
                &message[..message_len],
            );
        }
        let _ = &final_message;

        let has_accumulator: c_int = (ACCUMULATOR != 0) as c_int;
        let has_multiplier: c_int = (MULTIPLIER != 0) as c_int;
        let both_active: c_int = (has_accumulator != 0 && has_multiplier != 0) as c_int;

        if both_active != 0 {
            result = result.wrapping_add(ACCUMULATOR.wrapping_add(MULTIPLIER));
        }

        if MULTIPLIER > 0o100 {
            selected_op = OPERATIONS[3];
            selected_op(MULTIPLIER, 2);
        }

        result = result.wrapping_add(OPERATION_COUNT.wrapping_mul(0o10));

        let result_exists: c_int = (result != 0) as c_int;
        if result_exists == 0 {
            result = 0o777;
        }

        result
    }
}
