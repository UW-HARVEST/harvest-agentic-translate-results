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

use std::ffi::{c_char, c_int};

// ---------------------------------------------------------------------------
// File-scope mutable state (C: `static int accumulator/multiplier/
// operation_count`).  The C library is not thread safe; these plain mutable
// statics reproduce that behaviour (including the shared-state coupling
// between successive calls) exactly.
// ---------------------------------------------------------------------------

static mut ACCUMULATOR: c_int = 0;
static mut MULTIPLIER: c_int = 1;
static mut OPERATION_COUNT: c_int = 0;

/// C: `typedef int (*operation_func)(int, int);`
type OperationFunc = unsafe extern "C" fn(c_int, c_int) -> c_int;

/// C: `typedef void (*string_processor)(char*, int);`
/// Declared in the C source but never used; kept for fidelity.
#[allow(dead_code)]
type StringProcessor = unsafe extern "C" fn(*mut c_char, c_int);

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// `strlen` over a NUL-terminated C string.
///
/// `ptr::read` is used rather than `*s.add(n)` deliberately: a place-expression
/// deref of a raw pointer picks up rustc's `debug_assertions` null/alignment
/// check, which turns a NULL argument into a Rust panic (`SIGABRT`) in dev
/// builds while the C segfaults (`SIGSEGV`). Reading through the intrinsic keeps
/// the dev and release `.so`s byte-for-byte behaviourally identical to the C,
/// including on the undefined-behaviour NULL input.
unsafe fn c_strlen(s: *const c_char) -> usize {
    let mut n = 0usize;
    while core::ptr::read(s.add(n)) != 0 {
        n += 1;
    }
    n
}

/// `memchr(haystack, needle, len)` where `needle` is converted to
/// `unsigned char` exactly as the C library does.  Returns the index of the
/// first match, or `None`.
unsafe fn c_memchr(haystack: *const c_char, needle: c_int, len: usize) -> Option<usize> {
    let target = needle as u8;
    for i in 0..len {
        if core::ptr::read(haystack.add(i)) as u8 == target {
            return Some(i);
        }
    }
    None
}

/// `strcpy(dest, src)` for a byte slice that does *not* include a terminator;
/// the NUL is appended here.
unsafe fn c_strcpy_bytes(dest: *mut c_char, src: &[u8]) {
    for (i, b) in src.iter().enumerate() {
        core::ptr::write(dest.add(i), *b as c_char);
    }
    core::ptr::write(dest.add(src.len()), 0);
}

/// Renders `printf("%o", v)`: the value is reinterpreted as `unsigned int`
/// and printed in octal with no leading zero.
fn format_octal(v: c_int) -> String {
    format!("{:o}", v as u32)
}

/// C's `a / b` for `int`, byte-for-byte including the corner cases.
///
/// Rust's `/` operator is *not* a faithful translation here: for
/// `INT_MIN / -1` (signed-division overflow, UB in C) rustc emits an explicit
/// check and panics, and `wrapping_div` silently yields `INT_MIN`. The C
/// compiles to a bare `idiv`, which on x86-64 raises `SIGFPE` and kills the
/// process. Emitting the same `idiv` reproduces that exactly, so the two
/// libraries are indistinguishable even on the UB input.
///
/// The caller must uphold C's own precondition `b != 0` (`lib.c:54` guards it),
/// which is also the `idiv` divide-by-zero precondition.
#[cfg(target_arch = "x86_64")]
#[inline(never)]
unsafe fn c_idiv(a: c_int, b: c_int) -> c_int {
    let quotient: c_int;
    core::arch::asm!(
        "cdq",             // sign-extend eax into edx:eax
        "idiv {divisor:e}", // edx:eax / divisor -> eax (quotient), edx (rem)
        divisor = in(reg) b,
        inout("eax") a => quotient,
        out("edx") _,
    );
    quotient
}

/// Portable fallback for non-x86-64 targets: matches C for every input except
/// the `INT_MIN / -1` overflow, where C's behaviour is target-defined anyway.
#[cfg(not(target_arch = "x86_64"))]
#[inline(never)]
unsafe fn c_idiv(a: c_int, b: c_int) -> c_int {
    a.wrapping_div(b)
}

// ---------------------------------------------------------------------------
// Public ABI
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn add_to_accumulator(a: c_int, b: c_int) -> c_int {
    ACCUMULATOR = ACCUMULATOR.wrapping_add(a.wrapping_add(b));
    OPERATION_COUNT = OPERATION_COUNT.wrapping_add(1);
    ACCUMULATOR
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn multiply_with_multiplier(a: c_int, b: c_int) -> c_int {
    MULTIPLIER = MULTIPLIER.wrapping_mul(a.wrapping_mul(b));
    OPERATION_COUNT = OPERATION_COUNT.wrapping_add(1);
    MULTIPLIER
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn subtract_from_accumulator(a: c_int, b: c_int) -> c_int {
    ACCUMULATOR = ACCUMULATOR.wrapping_sub(a.wrapping_sub(b));
    OPERATION_COUNT = OPERATION_COUNT.wrapping_add(1);
    ACCUMULATOR
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn divide_multiplier(_a: c_int, b: c_int) -> c_int {
    if b != 0 {
        // C: `multiplier /= b`. `c_idiv` reproduces the raw `idiv` the C
        // compiler emits, including the `INT_MIN / -1` hardware trap.
        MULTIPLIER = c_idiv(MULTIPLIER, b);
    }
    OPERATION_COUNT = OPERATION_COUNT.wrapping_add(1);
    MULTIPLIER
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn process_octal_string(dest: *mut c_char, octal_val: c_int) {
    // C: sprintf(buffer, "Octal: 0%o, Decimal: %d", octal_val, octal_val);
    //    strcpy(dest, buffer);
    let buffer = format!(
        "Octal: 0{}, Decimal: {}",
        format_octal(octal_val),
        octal_val
    );
    c_strcpy_bytes(dest, buffer.as_bytes());
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn find_and_replace_char(s: *mut c_char, search_char: c_int) {
    let len = c_strlen(s);
    if let Some(idx) = c_memchr(s, search_char, len) {
        core::ptr::write(s.add(idx), b'X' as c_char);
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

/// C: `static operation_func operations[4] = { ... };`
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

    // C: char message[100]; char search_buffer[100];
    let mut message: [c_char; 100] = [0; 100];
    let mut search_buffer: [c_char; 100] = [0; 100];

    process_octal_string(message.as_mut_ptr(), 0o123);
    c_strcpy_bytes(
        search_buffer.as_mut_ptr(),
        b"Function pointer example with static vars",
    );

    let sb = search_buffer.as_ptr();
    if let Some(offset) = c_memchr(sb, b'p' as c_int, c_strlen(sb)) {
        result = result.wrapping_add(offset as c_int);
    }

    let selected_op: OperationFunc;

    if active_params >= mode_add {
        selected_op = OPERATIONS[0];
        result = result.wrapping_add(selected_op(normalized_p1, normalized_p2));
    }

    if active_params >= mode_multiply {
        let selected_op = OPERATIONS[1];
        result = result.wrapping_add(selected_op(normalized_p3, normalized_p4));
    }

    if ACCUMULATOR > 0o150 {
        let selected_op = OPERATIONS[2];
        let subtract_result = selected_op(normalized_p1, normalized_p3);
        result = result.wrapping_add(subtract_result);
    }

    find_and_replace_char(message.as_mut_ptr(), b'O' as c_int);

    // C: char final_message[100]; strcpy(final_message, message);
    let mut final_message: [c_char; 100] = [0; 100];
    {
        let len = c_strlen(message.as_ptr());
        std::ptr::copy_nonoverlapping(message.as_ptr(), final_message.as_mut_ptr(), len + 1);
    }
    let _ = &final_message;

    let has_accumulator: c_int = (ACCUMULATOR != 0) as c_int;
    let has_multiplier: c_int = (MULTIPLIER != 0) as c_int;
    let both_active: c_int = (has_accumulator != 0 && has_multiplier != 0) as c_int;

    if both_active != 0 {
        result = result.wrapping_add(ACCUMULATOR.wrapping_add(MULTIPLIER));
    }

    if MULTIPLIER > 0o100 {
        let selected_op = OPERATIONS[3];
        selected_op(MULTIPLIER, 2);
    }

    result = result.wrapping_add(OPERATION_COUNT.wrapping_mul(0o10));

    let result_exists: c_int = (result != 0) as c_int;
    if result_exists == 0 {
        result = 0o777;
    }

    result
}
