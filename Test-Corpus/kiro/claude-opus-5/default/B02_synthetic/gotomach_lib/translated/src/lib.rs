// Rust translation of c_src/src/lib.c
//
// Behaviour-preserving port: same public C ABI, same stdout bytes, same
// return codes, same order of validation checks. Bugs / quirks of the
// original C are reproduced rather than fixed.

use std::ffi::{c_char, c_int, c_void};

unsafe extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
}

/// `UINT16_MAX` from <stdint.h>.
const UINT16_MAX: c_int = 65535;

/// Emulates `LOG_MSG(level, msg)` == `printf("[" #level "] " msg "\n")`.
///
/// The messages contain no `%` characters, but routing them through `"%s"`
/// keeps the call unambiguously format-string safe while producing the exact
/// same bytes on the same C `stdout` stream (so buffering/interleaving with
/// any C caller is preserved).
fn log_msg(level: &str, msg: &str) {
    let line = format!("[{level}] {msg}\n\0");
    unsafe {
        printf(c"%s".as_ptr(), line.as_ptr() as *const c_char);
    }
}

/// `typedef int (*operation_fn)(int value, int unused_param, void *unused_context);`
type OperationFn = unsafe extern "C" fn(c_int, c_int, *mut c_void) -> c_int;

struct ProcessorState {
    results: Vec<c_int>,
    capacity: usize,
    count: usize,
    operation: Option<OperationFn>,
    status: c_char,
}

fn is_valid_state(state: &ProcessorState) -> bool {
    if state.status != 0 {
        return state.count < state.capacity;
    }
    false
}

fn check_char_flag(flag: c_char) -> bool {
    flag != 0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn process_value(
    value: c_int,
    _unused_param: c_int,
    _unused_context: *mut c_void,
) -> c_int {
    value.wrapping_add(10)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn double_value(
    value: c_int,
    _unused_param: c_int,
    _unused_context: *mut c_void,
) -> c_int {
    value.wrapping_mul(2)
}

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
/// Returns `None` where the C returns `NULL` (allocation failure).
fn init_processor(capacity: usize, op: Option<OperationFn>) -> Option<ProcessorState> {
    let mut results: Vec<c_int> = Vec::new();
    if results.try_reserve_exact(capacity).is_err() {
        return None;
    }

    Some(ProcessorState {
        results,
        capacity,
        count: 0,
        operation: op,
        status: 1,
    })
}

#[unsafe(no_mangle)]
// `result` is initialised to 0 as in the C source even though every path
// reassigns it before the final read.
#[allow(unused_assignments)]
pub unsafe extern "C" fn gotomach(
    iterations: c_int,
    seed: c_int,
    mode: c_int,
    threshold: c_int,
) -> c_int {
    let mut state: Option<ProcessorState> = None;
    let mut temp_buffer: Option<Vec<c_int>> = None;
    let mut result: c_int = 0;
    let selected_op: Option<OperationFn>;

    log_msg("INFO", "Starting gotomach function");

    // Single exit path, mirroring the C `goto cleanup` structure.
    'cleanup: {
        if iterations < 0 || iterations > UINT16_MAX {
            log_msg("ERROR", "Invalid iteration count");
            result = -1;
            break 'cleanup;
        }

        if seed < 0 || seed > UINT16_MAX {
            log_msg("ERROR", "Invalid seed value");
            result = -2;
            break 'cleanup;
        }

        match mode {
            0 => selected_op = Some(process_value),
            1 => selected_op = Some(double_value),
            2 => selected_op = Some(triple_value),
            _ => {
                log_msg("WARNING", "Invalid mode, using default");
                selected_op = Some(process_value);
            }
        }

        let iterations_usize = iterations as usize;

        state = init_processor(iterations_usize, selected_op);
        if state.is_none() {
            log_msg("ERROR", "Failed to initialize processor");
            result = -3;
            break 'cleanup;
        }
        let state_ref = state.as_mut().unwrap();

        let mut buf: Vec<c_int> = Vec::new();
        if buf.try_reserve_exact(iterations_usize).is_err() {
            log_msg("ERROR", "Failed to allocate temporary buffer");
            result = -4;
            break 'cleanup;
        }
        buf.resize(iterations_usize, 0);
        temp_buffer = Some(buf);
        let temp = temp_buffer.as_mut().unwrap();

        if !check_char_flag(state_ref.status) {
            log_msg("ERROR", "Invalid state status");
            result = -5;
            break 'cleanup;
        }

        let mut current_value: c_int = seed;
        let mut i: c_int = 0;
        while i < iterations {
            if !is_valid_state(state_ref) {
                log_msg("ERROR", "State became invalid during processing");
                result = -6;
                break 'cleanup;
            }

            let idx = i as usize;
            let op = state_ref.operation.expect("operation is always set");
            temp[idx] = unsafe { op(current_value, 0, std::ptr::null_mut()) };

            if temp[idx] < threshold {
                state_ref.results.push(temp[idx]);
                state_ref.count += 1;
            }

            // C's `%` truncates toward zero; Rust's `%` on i32 does the same.
            current_value = temp[idx] % 1000;

            if state_ref.count >= UINT16_MAX as usize {
                log_msg("WARNING", "Reached maximum count");
                break;
            }

            i += 1;
        }

        result = 0;
        for i in 0..state_ref.count {
            result = result.wrapping_add(state_ref.results[i]);
        }

        log_msg("INFO", "Processing completed successfully");
    }

    // cleanup: temp_buffer then state (order matches the C).
    drop(temp_buffer);
    drop(state);

    result
}
