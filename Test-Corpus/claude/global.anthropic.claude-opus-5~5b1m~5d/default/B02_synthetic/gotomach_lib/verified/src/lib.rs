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
// stdio interop
//
// The C code logs through `printf`, so we route the log messages through the
// very same libc `printf` in order to share the C runtime's stdout buffer and
// therefore produce byte-identical (and identically ordered) output.
// ---------------------------------------------------------------------------
unsafe extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
}

/// Equivalent of the C macro
/// `#define LOG_MSG(level, msg) printf("[" #level "] " msg "\n")`
///
/// The macro stringifies `level` and concatenates the literals at compile time,
/// producing a single format-string argument to `printf`.
macro_rules! log_msg {
    ($level:ident, $msg:literal) => {{
        // "[" #level "] " msg "\n" followed by the NUL terminator.
        const MSG: &[u8] = concat!("[", stringify!($level), "] ", $msg, "\n\0").as_bytes();
        unsafe {
            printf(MSG.as_ptr() as *const c_char);
        }
    }};
}

/// `typedef int (*operation_fn)(int value, int unused_param, void *unused_context);`
type operation_fn = unsafe extern "C" fn(value: c_int, unused_param: c_int, unused_context: *mut c_void) -> c_int;

/// ```c
/// typedef struct {
///     int *results;
///     size_t capacity;
///     size_t count;
///     operation_fn operation;
///     char status;
/// } ProcessorState;
/// ```
struct ProcessorState {
    results: Vec<c_int>,
    capacity: usize,
    count: usize,
    operation: Option<operation_fn>,
    status: c_char,
}

/// `static bool is_valid_state(ProcessorState *state)`
fn is_valid_state(state: &ProcessorState) -> bool {
    if state.status != 0 {
        return state.count < state.capacity;
    }
    false
}

/// `static bool check_char_flag(char flag)`
fn check_char_flag(flag: c_char) -> bool {
    flag != 0
}

/// `int process_value(int value, int unused_param, void *unused_context)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn process_value(
    value: c_int,
    _unused_param: c_int,
    _unused_context: *mut c_void,
) -> c_int {
    value.wrapping_add(10)
}

/// `int double_value(int value, int unused_param, void *unused_context)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn double_value(
    value: c_int,
    _unused_param: c_int,
    _unused_context: *mut c_void,
) -> c_int {
    value.wrapping_mul(2)
}

/// `int triple_value(int value, int unused_param, void *unused_context)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn triple_value(
    value: c_int,
    _unused_param: c_int,
    _unused_context: *mut c_void,
) -> c_int {
    value.wrapping_mul(3)
}

/// `static ProcessorState* init_processor(size_t capacity, operation_fn op)`
///
/// The C version allocates the state struct and the `results` array with
/// `malloc`, returning `NULL` on failure. `malloc(0)` returns a non-NULL
/// pointer on glibc, so a zero capacity is *not* treated as a failure here
/// either.
fn init_processor(capacity: usize, op: operation_fn) -> Option<Box<ProcessorState>> {
    let mut results: Vec<c_int> = Vec::new();
    if results.try_reserve_exact(capacity).is_err() {
        return None;
    }
    // `malloc` leaves the array uninitialized; the algorithm only ever reads
    // entries it has previously written, so zero-filling is behaviourally
    // equivalent and keeps the Rust side safe.
    results.resize(capacity, 0);

    Some(Box::new(ProcessorState {
        results,
        capacity,
        count: 0,
        operation: Some(op),
        status: 1,
    }))
}

/// `static void cleanup_processor(ProcessorState *state)`
///
/// Handled by `Drop` in Rust; kept as an explicit no-op-shaped helper for a
/// one-to-one reading of the original control flow.
fn cleanup_processor(state: Option<Box<ProcessorState>>) {
    drop(state);
}

/// `int gotomach(int iterations, int seed, int mode, int threshold);`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gotomach(
    iterations: c_int,
    seed: c_int,
    mode: c_int,
    threshold: c_int,
) -> c_int {
    let mut state: Option<Box<ProcessorState>> = None;
    let mut temp_buffer: Option<Vec<c_int>> = None;
    let mut result: c_int = 0;
    let selected_op: operation_fn;

    log_msg!(INFO, "Starting gotomach function");

    // Emulates the `goto cleanup` control flow of the C original: every early
    // exit sets `result` and falls through to the shared cleanup below.
    'cleanup: {
        if iterations < 0 || iterations > u16::MAX as c_int {
            log_msg!(ERROR, "Invalid iteration count");
            result = -1;
            break 'cleanup;
        }

        if seed < 0 || seed > u16::MAX as c_int {
            log_msg!(ERROR, "Invalid seed value");
            result = -2;
            break 'cleanup;
        }

        match mode {
            0 => {
                selected_op = process_value;
            }
            1 => {
                selected_op = double_value;
            }
            2 => {
                selected_op = triple_value;
            }
            _ => {
                log_msg!(WARNING, "Invalid mode, using default");
                selected_op = process_value;
            }
        }

        state = init_processor(iterations as usize, selected_op);
        if state.is_none() {
            log_msg!(ERROR, "Failed to initialize processor");
            result = -3;
            break 'cleanup;
        }
        let st = state.as_mut().unwrap();

        // `malloc(iterations * sizeof(int))`
        {
            let mut buf: Vec<c_int> = Vec::new();
            if buf.try_reserve_exact(iterations as usize).is_err() {
                log_msg!(ERROR, "Failed to allocate temporary buffer");
                result = -4;
                break 'cleanup;
            }
            buf.resize(iterations as usize, 0);
            temp_buffer = Some(buf);
        }
        let tb = temp_buffer.as_mut().unwrap();

        if !check_char_flag(st.status) {
            log_msg!(ERROR, "Invalid state status");
            result = -5;
            break 'cleanup;
        }

        let mut current_value: c_int = seed;
        let mut broke_early = false;
        let mut i: c_int = 0;
        while i < iterations {
            if !is_valid_state(st) {
                log_msg!(ERROR, "State became invalid during processing");
                result = -6;
                broke_early = true;
                break;
            }

            let op = st.operation.unwrap();
            tb[i as usize] = unsafe { op(current_value, 0, std::ptr::null_mut()) };

            if tb[i as usize] < threshold {
                let count = st.count;
                st.results[count] = tb[i as usize];
                st.count = count + 1;
            }

            current_value = tb[i as usize] % 1000;

            if st.count >= u16::MAX as usize {
                log_msg!(WARNING, "Reached maximum count");
                break;
            }

            i += 1;
        }

        if broke_early {
            break 'cleanup;
        }

        result = 0;
        for idx in 0..st.count {
            result = result.wrapping_add(st.results[idx]);
        }

        log_msg!(INFO, "Processing completed successfully");
    }

    // cleanup:
    drop(temp_buffer);
    cleanup_processor(state);

    result
}
