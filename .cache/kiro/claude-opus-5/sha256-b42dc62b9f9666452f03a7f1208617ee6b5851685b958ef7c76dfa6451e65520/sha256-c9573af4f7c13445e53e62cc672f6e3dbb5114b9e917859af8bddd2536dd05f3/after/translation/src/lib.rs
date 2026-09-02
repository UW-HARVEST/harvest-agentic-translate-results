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

use core::ffi::{c_char, c_int, c_void};

// ---------------------------------------------------------------------------
// libc bindings.
//
// The C library writes to `stdout` via `printf` and hands out `malloc`ed
// buffers that its callers are expected to `free`.  Both of those are
// observable parts of the ABI, so the translation calls straight into libc
// rather than re-implementing formatting or allocation: that keeps stdout
// buffering/interleaving and heap ownership byte-for-byte identical.
// ---------------------------------------------------------------------------
unsafe extern "C" {
    #[link_name = "printf"]
    fn c_printf(fmt: *const c_char, ...) -> c_int;
    #[link_name = "snprintf"]
    fn c_snprintf(s: *mut c_char, n: usize, fmt: *const c_char, ...) -> c_int;
    #[link_name = "malloc"]
    fn c_malloc(size: usize) -> *mut c_void;
    #[link_name = "free"]
    fn c_free(p: *mut c_void);
    #[link_name = "memcpy"]
    fn c_memcpy(dst: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
    #[link_name = "strcmp"]
    fn c_strcmp(a: *const c_char, b: *const c_char) -> c_int;
    #[link_name = "strcpy"]
    fn c_strcpy(dst: *mut c_char, src: *const c_char) -> *mut c_char;
}

/// Helper: `b"...\0"` literal as a `*const c_char`.
macro_rules! cstr {
    ($s:literal) => {
        concat!($s, "\0").as_ptr() as *const c_char
    };
}

// #define READ_PERM 0400
const READ_PERM: c_int = 0o400;
// #define WRITE_PERM 0200
const WRITE_PERM: c_int = 0o200;
// #define EXEC_PERM 0100
#[allow(dead_code)]
const EXEC_PERM: c_int = 0o100;

// typedef struct {
//     int value;
//     char operation[32];
//     int permissions;
// } Result;
#[repr(C)]
struct Result {
    value: c_int,
    operation: [c_char; 32],
    permissions: c_int,
}

// ---------------------------------------------------------------------------
// char* create_result_string(const char* op, int val)
// ---------------------------------------------------------------------------
#[unsafe(no_mangle)]
pub unsafe extern "C" fn create_result_string(op: *const c_char, val: c_int) -> *mut c_char {
    unsafe {
        let str_ = c_malloc(64 * core::mem::size_of::<c_char>()) as *mut c_char;
        if str_.is_null() {
            return core::ptr::null_mut();
        }
        c_snprintf(str_, 64, cstr!("Operation: %s, Value: %d"), op, val);
        str_
    }
}

// ---------------------------------------------------------------------------
// int check_permissions(int perms, int required)
// ---------------------------------------------------------------------------
#[unsafe(no_mangle)]
pub extern "C" fn check_permissions(perms: c_int, required: c_int) -> c_int {
    ((perms & required) == required) as c_int
}

// ---------------------------------------------------------------------------
// int safe_add(int a, int b, int perms)
// ---------------------------------------------------------------------------
#[unsafe(no_mangle)]
pub extern "C" fn safe_add(a: c_int, b: c_int, perms: c_int) -> c_int {
    if check_permissions(perms, READ_PERM | WRITE_PERM) == 0 {
        unsafe {
            c_printf(cstr!("Insufficient permissions for addition\n"));
        }
        return 0;
    }
    a.wrapping_add(b)
}

// ---------------------------------------------------------------------------
// int multiply_with_log(int a, int b, char** log_msg)
//
// NOTE: the C code dereferences `log_msg` unconditionally (no NULL check).
// That bug is reproduced here rather than fixed.
//
// The store and the read-back deliberately go through libc `memcpy` instead of
// `*log_msg = …` / `*log_msg`. A plain raw-pointer dereference picks up
// rustc's debug-only null-and-alignment precondition assertions, which turn a
// NULL or misaligned out-pointer into a panic/`abort` (SIGABRT) instead of the
// hardware fault (SIGSEGV) the C produces. Routing the 8-byte store through
// `memcpy` keeps the failure mode identical in every profile while being
// exactly the same store for well-formed pointers.
// ---------------------------------------------------------------------------
#[unsafe(no_mangle)]
pub unsafe extern "C" fn multiply_with_log(
    a: c_int,
    b: c_int,
    log_msg: *mut *mut c_char,
) -> c_int {
    unsafe {
        const PSZ: usize = core::mem::size_of::<*mut c_char>();

        // The C evaluates the right-hand side first, so the string is created
        // (and leaked) before the faulting store when `log_msg` is NULL.
        let produced = create_result_string(cstr!("multiply"), a.wrapping_mul(b));
        c_memcpy(
            log_msg as *mut c_void,
            &produced as *const *mut c_char as *const c_void,
            PSZ,
        );

        // `if (*log_msg == NULL)` — re-read through the caller's pointer, as
        // the C does.
        let mut stored: *mut c_char = core::ptr::null_mut();
        c_memcpy(
            &mut stored as *mut *mut c_char as *mut c_void,
            log_msg as *const c_void,
            PSZ,
        );
        if stored.is_null() {
            return 0;
        }
        a.wrapping_mul(b)
    }
}

// ---------------------------------------------------------------------------
// int copy_and_sum(int* src, int count)
// ---------------------------------------------------------------------------
#[unsafe(no_mangle)]
pub unsafe extern "C" fn copy_and_sum(src: *mut c_int, count: c_int) -> c_int {
    unsafe {
        if src.is_null() {
            c_printf(cstr!("Source pointer is NULL\n"));
            return -1;
        }

        // `count * sizeof(int)`: in C `count` is converted to size_t first, so a
        // negative count becomes a huge allocation request (and malloc fails).
        let nbytes = (count as isize as usize).wrapping_mul(core::mem::size_of::<c_int>());

        let dest = c_malloc(nbytes) as *mut c_int;
        if dest.is_null() {
            c_printf(cstr!("Memory allocation failed\n"));
            return -1;
        }

        c_memcpy(dest as *mut c_void, src as *const c_void, nbytes);

        let mut sum: c_int = 0;
        let mut i: c_int = 0;
        while i < count {
            sum = sum.wrapping_add(*dest.offset(i as isize));
            i += 1;
        }

        c_free(dest as *mut c_void);
        sum
    }
}

// ---------------------------------------------------------------------------
// int compare_operations(const char* op1, const char* op2)
// ---------------------------------------------------------------------------
#[unsafe(no_mangle)]
pub unsafe extern "C" fn compare_operations(op1: *const c_char, op2: *const c_char) -> c_int {
    unsafe {
        if op1.is_null() || op2.is_null() {
            c_printf(cstr!("One or both operation strings are NULL\n"));
            return -1;
        }

        c_strcmp(op1, op2)
    }
}

// ---------------------------------------------------------------------------
// int complexmode(int mode, int value1, int value2, int value3)
// ---------------------------------------------------------------------------
#[unsafe(no_mangle)]
// `int result = 0;` in the C is dead on every path (each switch arm, including
// `default:`, assigns before the read). Kept verbatim for faithfulness.
#[allow(unused_assignments)]
pub unsafe extern "C" fn complexmode(
    mode: c_int,
    value1: c_int,
    value2: c_int,
    value3: c_int,
) -> c_int {
    unsafe {
        let mut result: c_int = 0;
        let mut log_message: *mut c_char = core::ptr::null_mut();

        let permissions: c_int = 0o644; // rw-r--r--

        let res_tracker = c_malloc(core::mem::size_of::<Result>()) as *mut Result;
        if res_tracker.is_null() {
            c_printf(cstr!("Failed to allocate result tracker\n"));
            return -1;
        }

        (*res_tracker).value = 0;
        (*res_tracker).permissions = permissions;
        let operation = core::ptr::addr_of_mut!((*res_tracker).operation) as *mut c_char;
        c_strcpy(operation, cstr!("none"));

        match mode {
            1 => {
                c_strcpy(operation, cstr!("addition"));
                result = safe_add(value1, value2, permissions);
                (*res_tracker).value = result;

                c_printf(cstr!("Mode 1: Addition\n"));
                c_printf(cstr!("Result: %d\n"), result);
            }

            2 => {
                c_strcpy(operation, cstr!("multiplication"));
                result = multiply_with_log(value1, value2, &mut log_message);
                (*res_tracker).value = result;

                if log_message.is_null() || c_strcmp(log_message, cstr!("")) == 0 {
                    c_printf(cstr!("Log message creation failed\n"));
                } else {
                    c_printf(cstr!("Mode 2: %s\n"), log_message);
                    c_free(log_message as *mut c_void);
                }
            }

            3 => {
                c_strcpy(operation, cstr!("array_sum"));
                let mut values: [c_int; 3] = [value1, value2, value3];
                result = copy_and_sum(values.as_mut_ptr(), 3);
                (*res_tracker).value = result;

                c_printf(cstr!("Mode 3: Array Sum\n"));
                c_printf(cstr!("Result: %d\n"), result);
            }

            4 => {
                c_strcpy(operation, cstr!("complex"));

                if check_permissions(permissions, 0o100) != 0 {
                    result = value1.wrapping_mul(value2).wrapping_add(value3);
                } else {
                    result = value1.wrapping_add(value2).wrapping_add(value3);
                }

                (*res_tracker).value = result;
                c_printf(cstr!("Mode 4: Complex Calculation\n"));
                c_printf(cstr!("Result: %d\n"), result);
            }

            _ => {
                c_printf(cstr!("Invalid mode\n"));
                result = -1;
            }
        }

        if c_strcmp(operation, cstr!("none")) != 0 {
            c_printf(cstr!("Operation performed: %s\n"), operation);
        }

        c_free(res_tracker as *mut c_void);

        result
    }
}
