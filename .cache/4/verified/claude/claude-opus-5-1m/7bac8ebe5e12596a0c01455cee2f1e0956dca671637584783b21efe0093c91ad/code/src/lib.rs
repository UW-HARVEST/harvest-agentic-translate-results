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

use std::ffi::{c_char, c_int, c_void};

// ---------------------------------------------------------------------------
// libc bindings.
//
// We deliberately go through the platform C library for allocation and for
// printing so that the observable behaviour (including malloc(0) returning a
// non-NULL pointer, and stdout buffering / interleaving with any C code in the
// same process) is byte-for-byte identical to the original C library.
// ---------------------------------------------------------------------------
extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
    fn printf(fmt: *const c_char, ...) -> c_int;
}

/// `UINT16_MAX` from <stdint.h>.
const UINT16_MAX: c_int = 65535;

/// #define LOG_MSG(level, msg) printf("[" #level "] " msg "\n")
///
/// The C preprocessor stringizes `level` and concatenates the literals, so the
/// resulting call is `printf("[<LEVEL>] <msg>\n")`.  The message never contains
/// a `%`, so passing it as the format string is faithful to the original.
macro_rules! log_msg {
    ($level:ident, $msg:expr) => {{
        // Build the concatenated, NUL terminated literal at compile time.
        const S: &str = concat!("[", stringify!($level), "] ", $msg, "\n\0");
        unsafe {
            printf(S.as_ptr() as *const c_char);
        }
    }};
}

// typedef int (*operation_fn)(int value, int unused_param, void *unused_context);
type operation_fn = Option<unsafe extern "C" fn(c_int, c_int, *mut c_void) -> c_int>;

// typedef struct { ... } ProcessorState;
#[repr(C)]
struct ProcessorState {
    results: *mut c_int,
    capacity: usize,
    count: usize,
    operation: operation_fn,
    status: c_char,
}

// static bool is_valid_state(ProcessorState *state)
unsafe fn is_valid_state(state: *mut ProcessorState) -> bool {
    if (*state).status != 0 {
        return (*state).count < (*state).capacity;
    }
    false
}

// static bool check_char_flag(char flag)
fn check_char_flag(flag: c_char) -> bool {
    flag != 0
}

// int process_value(int value, int unused_param, void *unused_context)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn process_value(
    value: c_int,
    _unused_param: c_int,
    _unused_context: *mut c_void,
) -> c_int {
    value.wrapping_add(10)
}

// int double_value(int value, int unused_param, void *unused_context)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn double_value(
    value: c_int,
    _unused_param: c_int,
    _unused_context: *mut c_void,
) -> c_int {
    value.wrapping_mul(2)
}

// int triple_value(int value, int unused_param, void *unused_context)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn triple_value(
    value: c_int,
    _unused_param: c_int,
    _unused_context: *mut c_void,
) -> c_int {
    value.wrapping_mul(3)
}

// static ProcessorState* init_processor(size_t capacity, operation_fn op)
unsafe fn init_processor(capacity: usize, op: operation_fn) -> *mut ProcessorState {
    let state = malloc(core::mem::size_of::<ProcessorState>()) as *mut ProcessorState;
    if state.is_null() {
        return core::ptr::null_mut();
    }

    let results = malloc(capacity.wrapping_mul(core::mem::size_of::<c_int>())) as *mut c_int;
    core::ptr::write(core::ptr::addr_of_mut!((*state).results), results);
    if results.is_null() {
        free(state as *mut c_void);
        return core::ptr::null_mut();
    }

    (*state).capacity = capacity;
    (*state).count = 0;
    (*state).operation = op;
    (*state).status = 1;

    state
}

// static void cleanup_processor(ProcessorState *state)
unsafe fn cleanup_processor(state: *mut ProcessorState) {
    if !state.is_null() {
        if !(*state).results.is_null() {
            free((*state).results as *mut c_void);
        }
        free(state as *mut c_void);
    }
}

// int gotomach(int iterations, int seed, int mode, int threshold)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gotomach(
    iterations: c_int,
    seed: c_int,
    mode: c_int,
    threshold: c_int,
) -> c_int {
    let mut state: *mut ProcessorState = core::ptr::null_mut();
    let mut temp_buffer: *mut c_int = core::ptr::null_mut();
    let mut result: c_int = 0;
    #[allow(unused_assignments)]
    let mut selected_op: operation_fn = None;

    log_msg!(INFO, "Starting gotomach function");

    // Replicates the `goto cleanup` control flow of the C original: every
    // early exit falls through to the shared cleanup block below.
    'cleanup: loop {
        if iterations < 0 || iterations > UINT16_MAX {
            log_msg!(ERROR, "Invalid iteration count");
            result = -1;
            break 'cleanup;
        }

        if seed < 0 || seed > UINT16_MAX {
            log_msg!(ERROR, "Invalid seed value");
            result = -2;
            break 'cleanup;
        }

        match mode {
            0 => {
                selected_op = Some(process_value);
            }
            1 => {
                selected_op = Some(double_value);
            }
            2 => {
                selected_op = Some(triple_value);
            }
            _ => {
                log_msg!(WARNING, "Invalid mode, using default");
                selected_op = Some(process_value);
            }
        }

        state = init_processor(iterations as usize, selected_op);
        if state.is_null() {
            log_msg!(ERROR, "Failed to initialize processor");
            result = -3;
            break 'cleanup;
        }

        temp_buffer =
            malloc((iterations as usize).wrapping_mul(core::mem::size_of::<c_int>())) as *mut c_int;
        if temp_buffer.is_null() {
            log_msg!(ERROR, "Failed to allocate temporary buffer");
            result = -4;
            break 'cleanup;
        }

        if !check_char_flag((*state).status) {
            log_msg!(ERROR, "Invalid state status");
            result = -5;
            break 'cleanup;
        }

        let mut current_value: c_int = seed;
        let mut broke_out = false;
        let mut i: c_int = 0;
        while i < iterations {
            if !is_valid_state(state) {
                log_msg!(ERROR, "State became invalid during processing");
                result = -6;
                broke_out = true;
                break;
            }

            let op = (*state).operation.unwrap();
            let produced = op(current_value, 0, core::ptr::null_mut());
            *temp_buffer.offset(i as isize) = produced;

            if produced < threshold {
                let count = (*state).count;
                *(*state).results.add(count) = produced;
                (*state).count = count + 1;
            }

            current_value = produced % 1000;

            if (*state).count >= UINT16_MAX as usize {
                log_msg!(WARNING, "Reached maximum count");
                break;
            }

            i += 1;
        }

        if broke_out {
            break 'cleanup;
        }

        result = 0;
        let count = (*state).count;
        let mut j: usize = 0;
        while j < count {
            result = result.wrapping_add(*(*state).results.add(j));
            j += 1;
        }

        log_msg!(INFO, "Processing completed successfully");
        break 'cleanup;
    }

    // cleanup:
    if !temp_buffer.is_null() {
        free(temp_buffer as *mut c_void);
    }
    cleanup_processor(state);

    result
}
