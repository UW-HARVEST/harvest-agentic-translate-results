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

use core::ffi::{c_char, c_int, c_void};

// ---------------------------------------------------------------------------
// libc bindings.
//
// The C translation unit uses malloc/free for all of its allocations and emits
// its log lines through stdio. We bind to the very same libc entry points so
// that allocation-failure semantics (including malloc(0) returning a unique
// non-NULL pointer on glibc) and stdout buffering/interleaving are identical to
// the C library.
//
// Note: the C source writes its log lines with
//     printf("[" #level "] " msg "\n")
// which has no conversion specifiers, so the compiler lowers each call to
// `puts`. `nm -D` on the C .so confirms an undefined reference to `puts` and
// none to `printf`. We call `puts` directly for the same reason.
// ---------------------------------------------------------------------------
extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
    fn puts(s: *const c_char) -> c_int;
}

/// Equivalent of the C source's `LOG_MSG(level, msg)` macro.
///
/// `LOG_MSG(INFO, "text")` expands to `printf("[INFO] text\n")`, which the C
/// compiler turns into `puts("[INFO] text")`.
macro_rules! LOG_MSG {
    ($level:ident, $msg:literal) => {{
        const S: &[u8] = concat!("[", stringify!($level), "] ", $msg, "\0").as_bytes();
        unsafe {
            puts(S.as_ptr() as *const c_char);
        }
    }};
}

/// `UINT16_MAX` from <stdint.h>.
const UINT16_MAX: c_int = 65535;

/// `typedef int (*operation_fn)(int value, int unused_param, void *unused_context);`
type operation_fn = Option<unsafe extern "C" fn(c_int, c_int, *mut c_void) -> c_int>;

/// ```c
/// typedef struct {
///     int *results;
///     size_t capacity;
///     size_t count;
///     operation_fn operation;
///     char status;
/// } ProcessorState;
/// ```
#[repr(C)]
struct ProcessorState {
    results: *mut c_int,
    capacity: usize,
    count: usize,
    operation: operation_fn,
    status: c_char,
}

// ---------------------------------------------------------------------------
// Internal (static) helpers. These are `static` in the C source and therefore
// are NOT part of the exported ABI.
// ---------------------------------------------------------------------------

/// ```c
/// static bool is_valid_state(ProcessorState *state) {
///     if (state->status) {
///         return state->count < state->capacity;
///     }
///     return false;
/// }
/// ```
unsafe fn is_valid_state(state: *mut ProcessorState) -> bool {
    if (*state).status != 0 {
        return (*state).count < (*state).capacity;
    }
    false
}

/// ```c
/// static bool check_char_flag(char flag) {
///     return flag;
/// }
/// ```
fn check_char_flag(flag: c_char) -> bool {
    flag != 0
}

/// ```c
/// static ProcessorState* init_processor(size_t capacity, operation_fn op);
/// ```
unsafe fn init_processor(capacity: usize, op: operation_fn) -> *mut ProcessorState {
    let state = malloc(core::mem::size_of::<ProcessorState>()) as *mut ProcessorState;
    if state.is_null() {
        return core::ptr::null_mut();
    }

    // state->results = malloc(capacity * sizeof(int));
    (*state).results = malloc(capacity.wrapping_mul(core::mem::size_of::<c_int>())) as *mut c_int;
    if (*state).results.is_null() {
        free(state as *mut c_void);
        return core::ptr::null_mut();
    }

    (*state).capacity = capacity;
    (*state).count = 0;
    (*state).operation = op;
    (*state).status = 1;

    state
}

/// ```c
/// static void cleanup_processor(ProcessorState *state);
/// ```
unsafe fn cleanup_processor(state: *mut ProcessorState) {
    if !state.is_null() {
        if !(*state).results.is_null() {
            free((*state).results as *mut c_void);
        }
        free(state as *mut c_void);
    }
}

// ---------------------------------------------------------------------------
// Exported ABI.
//
// The C header only declares `gotomach`, but `process_value`, `double_value`
// and `triple_value` are non-static definitions in lib.c and therefore also
// appear in the shared library's dynamic symbol table. There are no
// function-renaming preprocessor macros in effect for any of them
// (MAKE_FUNC_NAME / CREATE_LABEL are defined but never used), so the linker
// symbols equal the source-level names.
// ---------------------------------------------------------------------------

/// ```c
/// int process_value(int value, int unused_param, void *unused_context) {
///     (void)unused_param;
///     (void)unused_context;
///     return value + 10;
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn process_value(
    value: c_int,
    _unused_param: c_int,
    _unused_context: *mut c_void,
) -> c_int {
    value.wrapping_add(10)
}

/// ```c
/// int double_value(int value, int unused_param, void *unused_context) {
///     (void)unused_param;
///     (void)unused_context;
///     return value * 2;
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn double_value(
    value: c_int,
    _unused_param: c_int,
    _unused_context: *mut c_void,
) -> c_int {
    value.wrapping_mul(2)
}

/// ```c
/// int triple_value(int value, int unused_param, void *unused_context) {
///     (void)unused_param;
///     (void)unused_context;
///     return value * 3;
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn triple_value(
    value: c_int,
    _unused_param: c_int,
    _unused_context: *mut c_void,
) -> c_int {
    value.wrapping_mul(3)
}

/// ```c
/// int gotomach(int iterations, int seed, int mode, int threshold);
/// ```
///
/// The C body is a single function with a `goto cleanup` epilogue. The
/// translation keeps the exact order of the log messages, the validation
/// checks and their return codes, and performs the same two frees at the
/// `cleanup` label on every exit path.
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
    let selected_op: operation_fn;

    LOG_MSG!(INFO, "Starting gotomach function");

    // The `'cleanup` block reproduces the C `goto cleanup` control flow: every
    // `break 'cleanup` corresponds to a `goto cleanup` in the original.
    'cleanup: {
        if iterations < 0 || iterations > UINT16_MAX {
            LOG_MSG!(ERROR, "Invalid iteration count");
            result = -1;
            break 'cleanup;
        }

        if seed < 0 || seed > UINT16_MAX {
            LOG_MSG!(ERROR, "Invalid seed value");
            result = -2;
            break 'cleanup;
        }

        match mode {
            0 => selected_op = Some(process_value),
            1 => selected_op = Some(double_value),
            2 => selected_op = Some(triple_value),
            _ => {
                LOG_MSG!(WARNING, "Invalid mode, using default");
                selected_op = Some(process_value);
            }
        }

        state = init_processor(iterations as usize, selected_op);
        if state.is_null() {
            LOG_MSG!(ERROR, "Failed to initialize processor");
            result = -3;
            break 'cleanup;
        }

        // temp_buffer = malloc(iterations * sizeof(int));
        temp_buffer = malloc((iterations as usize).wrapping_mul(core::mem::size_of::<c_int>()))
            as *mut c_int;
        if temp_buffer.is_null() {
            LOG_MSG!(ERROR, "Failed to allocate temporary buffer");
            result = -4;
            break 'cleanup;
        }

        if !check_char_flag((*state).status) {
            LOG_MSG!(ERROR, "Invalid state status");
            result = -5;
            break 'cleanup;
        }

        let mut current_value: c_int = seed;
        let mut i: c_int = 0;
        while i < iterations {
            if !is_valid_state(state) {
                LOG_MSG!(ERROR, "State became invalid during processing");
                result = -6;
                break 'cleanup;
            }

            let op = (*state).operation.unwrap_unchecked();
            let produced = op(current_value, 0, core::ptr::null_mut());
            *temp_buffer.offset(i as isize) = produced;

            if produced < threshold {
                let count = (*state).count;
                *(*state).results.add(count) = produced;
                (*state).count = count + 1;
            }

            current_value = produced.wrapping_rem(1000);

            if (*state).count >= UINT16_MAX as usize {
                LOG_MSG!(WARNING, "Reached maximum count");
                break;
            }

            i += 1;
        }

        result = 0;
        let count = (*state).count;
        let mut j: usize = 0;
        while j < count {
            result = result.wrapping_add(*(*state).results.add(j));
            j += 1;
        }

        LOG_MSG!(INFO, "Processing completed successfully");
    }

    // cleanup:
    if !temp_buffer.is_null() {
        free(temp_buffer as *mut c_void);
    }
    cleanup_processor(state);

    result
}
