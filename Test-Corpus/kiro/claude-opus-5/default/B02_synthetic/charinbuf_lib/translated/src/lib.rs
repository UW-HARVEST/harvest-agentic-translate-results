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

use std::ffi::{CString, c_char, c_int, c_void};
use std::sync::atomic::{AtomicI32, Ordering};

// ---------------------------------------------------------------------------
// libc bindings
//
// Output goes through C's `printf` so that stdio buffering (and the flush at
// process exit) behaves exactly as it does in the original C library.  Buffers
// handed out by `create_buffer` come from `malloc`, since callers are expected
// to release them with `free`.
// ---------------------------------------------------------------------------
unsafe extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
    fn malloc(size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
    fn memchr(s: *const c_void, c: c_int, n: usize) -> *mut c_void;
    fn strlen(s: *const c_char) -> usize;
}

/// `printf("%s", text)` — `text` must not contain interior NUL bytes.
fn print_str(text: &str) {
    if let Ok(cs) = CString::new(text) {
        unsafe {
            printf(c"%s".as_ptr(), cs.as_ptr());
        }
    }
}

/// C's `UINT16_MAX` (an `int` in this translation unit, as after promotion).
const UINT16_MAX: c_int = 65535;

// `static int counter = 0;` — file-scope mutable state shared by the
// counter operations below.
static COUNTER: AtomicI32 = AtomicI32::new(0);

fn counter_get() -> c_int {
    COUNTER.load(Ordering::Relaxed)
}

fn counter_set(value: c_int) {
    COUNTER.store(value, Ordering::Relaxed);
}

// typedef int (*operation_func)(int);
pub type OperationFunc = Option<unsafe extern "C" fn(c_int) -> c_int>;

// ---------------------------------------------------------------------------
// Counter operations
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub extern "C" fn increment_counter(value: c_int) -> c_int {
    // `counter += value;` — wrapping matches the observed behaviour of the C.
    let new = counter_get().wrapping_add(value);
    counter_set(new);
    new
}

#[unsafe(no_mangle)]
pub extern "C" fn decrement_counter(value: c_int) -> c_int {
    let new = counter_get().wrapping_sub(value);
    counter_set(new);
    new
}

#[unsafe(no_mangle)]
pub extern "C" fn multiply_counter(value: c_int) -> c_int {
    let new = counter_get().wrapping_mul(value);
    counter_set(new);
    new
}

#[unsafe(no_mangle)]
pub extern "C" fn reset_counter(value: c_int) -> c_int {
    counter_set(value);
    value
}

// ---------------------------------------------------------------------------
// String / buffer helpers
// ---------------------------------------------------------------------------

/// `int is_string_empty(const char *str)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn is_string_empty(str: *const c_char) -> c_int {
    if str.is_null() {
        return 1;
    }
    if unsafe { *str } != 0 {
        return 0;
    }
    1
}

/// `char* find_char_in_buffer(const char *buffer, size_t size, char target)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn find_char_in_buffer(
    buffer: *const c_char,
    size: usize,
    target: c_char,
) -> *mut c_char {
    if buffer.is_null() {
        return std::ptr::null_mut();
    }
    unsafe { memchr(buffer as *const c_void, target as c_int, size) as *mut c_char }
}

/// `char* create_buffer(const char *initial)`
///
/// The returned pointer comes from `malloc`, so callers may `free` it.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn create_buffer(initial: *const c_char) -> *mut c_char {
    if initial.is_null() {
        return std::ptr::null_mut();
    }

    let len = unsafe { strlen(initial) };
    let buffer = unsafe { malloc(len + 1) } as *mut c_char;

    if !buffer.is_null() {
        // strcpy(buffer, initial);
        unsafe { std::ptr::copy_nonoverlapping(initial, buffer, len + 1) };
    }

    buffer
}

/// `int validate_uint16_range(int value)`
#[unsafe(no_mangle)]
pub extern "C" fn validate_uint16_range(value: c_int) -> c_int {
    if value < 0 {
        return 0;
    }
    if value > UINT16_MAX {
        return 0;
    }
    1
}

/// `int apply_operation(operation_func op, int value)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn apply_operation(op: OperationFunc, value: c_int) -> c_int {
    match op {
        None => -1,
        Some(f) => unsafe { f(value) },
    }
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

/// `int charinbuf(int mode, int value, int opt1, int opt2)`
#[unsafe(no_mangle)]
pub extern "C" fn charinbuf(mode: c_int, value: c_int, opt1: c_int, opt2: c_int) -> c_int {
    let mut result: c_int = 0;
    let test_string = c"";
    let non_empty_string = c"Hello, World!";

    counter_set(0);

    match mode {
        0 => {
            print_str("Mode 0: UINT16_MAX validation\n");
            print_str(&format!(
                "Checking if value {value} is within uint16_t range...\n"
            ));

            if validate_uint16_range(value) != 0 {
                print_str(&format!(
                    "Value {value} is valid (0 <= value <= {UINT16_MAX})\n"
                ));
                result = value;
            } else {
                print_str(&format!("Value {value} is out of range for uint16_t\n"));
                result = -1;
            }

            print_str(&format!("UINT16_MAX constant value: {UINT16_MAX}\n"));
        }

        1 => {
            print_str("Mode 1: String empty check by dereference\n");

            if unsafe { is_string_empty(test_string.as_ptr()) } != 0 {
                print_str("Test string is empty (checked with *string)\n");
                result = 0;
            } else {
                print_str("Test string is not empty\n");
                result = 1;
            }

            if unsafe { is_string_empty(non_empty_string.as_ptr()) } != 0 {
                print_str("Non-empty string check failed!\n");
            } else {
                print_str("Non-empty string correctly identified\n");
                result += 10;
            }
        }

        2 => {
            print_str("Mode 2: Dynamic memory allocation and free\n");

            let buffer = unsafe { create_buffer(c"Testing malloc and free".as_ptr()) };

            if !buffer.is_null() {
                let contents = unsafe { std::ffi::CStr::from_ptr(buffer) };
                let len = contents.to_bytes().len();
                print_str(&format!(
                    "Buffer allocated: '{}'\n",
                    contents.to_string_lossy()
                ));
                print_str(&format!("Buffer length: {len}\n"));
                result = len as c_int;

                unsafe { free(buffer as *mut c_void) };
                print_str("Buffer freed successfully\n");
            } else {
                print_str("Failed to allocate buffer\n");
                result = -1;
            }
        }

        3 => {
            print_str("Mode 3: Function pointers with static counter\n");

            let mut current_op: OperationFunc = Some(reset_counter);
            result = unsafe { apply_operation(current_op, value) };
            print_str(&format!("Counter reset to: {result}\n"));

            current_op = Some(increment_counter);
            result = unsafe { apply_operation(current_op, opt1) };
            print_str(&format!("Counter after increment by {opt1}: {result}\n"));

            current_op = Some(multiply_counter);
            result = unsafe { apply_operation(current_op, opt2) };
            print_str(&format!("Counter after multiply by {opt2}: {result}\n"));

            current_op = Some(decrement_counter);
            result = unsafe { apply_operation(current_op, 5) };
            print_str(&format!("Counter after decrement by 5: {result}\n"));

            print_str(&format!("Final static counter value: {}\n", counter_get()));
        }

        4 => {
            print_str("Mode 4: Using memchr to find character\n");

            let buffer =
                unsafe { create_buffer(c"Search for character X in this buffer".as_ptr()) };

            if !buffer.is_null() {
                let contents = unsafe { std::ffi::CStr::from_ptr(buffer) };
                let buf_size = contents.to_bytes().len();
                let search_char: c_char = b'X' as c_char;

                print_str(&format!(
                    "Searching for '{}' in: '{}'\n",
                    search_char as u8 as char,
                    contents.to_string_lossy()
                ));
                let found_pos = unsafe { find_char_in_buffer(buffer, buf_size, search_char) };

                if !found_pos.is_null() {
                    result = unsafe { found_pos.offset_from(buffer) } as c_int;
                    print_str(&format!(
                        "Found '{}' at position: {result}\n",
                        search_char as u8 as char
                    ));
                } else {
                    print_str(&format!(
                        "Character '{}' not found\n",
                        search_char as u8 as char
                    ));
                    result = -1;
                }

                unsafe { free(buffer as *mut c_void) };
            }
        }

        _ => {
            print_str(&format!("Invalid mode: {mode}\n"));
            result = -1;
        }
    }

    result
}
