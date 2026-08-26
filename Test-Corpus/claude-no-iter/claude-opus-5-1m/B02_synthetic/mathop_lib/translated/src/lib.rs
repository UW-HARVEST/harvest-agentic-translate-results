// Translation of c_src/src/lib.c to Rust.
// Preserves exact runtime behavior (including bugs) and produces byte-identical
// output to the C version by going through libc's printf for stdout writes.

use std::ffi::{c_char, c_int};
use std::os::raw::c_long;
use std::ptr::{self, addr_of_mut};

// ---- Operation values (matches the C enum values) -------------------------
const OP_ADD: c_int = 1;
const OP_MULTIPLY: c_int = 2;
const OP_SUBTRACT: c_int = 3;
const OP_DIVIDE: c_int = 4;
const OP_MODULO: c_int = 5;

// ---- StatusCode values ----------------------------------------------------
const STATUS_SUCCESS: c_int = 0;

// ---- ComputationResult struct (matches C layout) --------------------------
#[repr(C)]
#[derive(Copy, Clone)]
struct ComputationResult {
    value: c_int,
    timestamp: libc::time_t,
    status: c_int, // C enum is int-sized
}

// Function pointer type matching C's `int (*)(int, int, int)`
type MathOperation = fn(c_int, c_int, c_int) -> c_int;

// ---- File-scope statics for the per-process history (mirrors C statics) ---
static mut COMPUTATION_HISTORY: *mut ComputationResult = ptr::null_mut();
static mut HISTORY_COUNT: c_int = 0;

// ---- Helper functions -----------------------------------------------------

fn is_valid_operation(op_char: c_char) -> bool {
    // C: char valid = op_char && (op_char >= '1' && op_char <= '5');
    //    return valid;
    // The C `&&` yields 0/1 (int), assigned to char, then implicitly converted
    // to bool (non-zero -> true).
    let one = b'1' as c_char;
    let five = b'5' as c_char;
    let valid: c_char = if op_char != 0 && op_char >= one && op_char <= five {
        1
    } else {
        0
    };
    valid != 0
}

fn get_operation_priority(op: c_int) -> c_int {
    op.wrapping_mul(10)
}

fn add_operation(a: c_int, b: c_int, _unused_param: c_int) -> c_int {
    a.wrapping_add(b)
}

fn multiply_operation(a: c_int, b: c_int, _unused_param: c_int) -> c_int {
    a.wrapping_mul(b)
}

fn subtract_operation(a: c_int, b: c_int, _unused_param: c_int) -> c_int {
    a.wrapping_sub(b)
}

fn divide_operation(a: c_int, b: c_int, _unused_param: c_int) -> c_int {
    if b == 0 {
        0
    } else {
        a.wrapping_div(b)
    }
}

fn modulo_operation(a: c_int, b: c_int, _unused_param: c_int) -> c_int {
    if b == 0 {
        0
    } else {
        a.wrapping_rem(b)
    }
}

fn select_operation(op: c_int) -> MathOperation {
    match op {
        OP_ADD => add_operation,
        OP_MULTIPLY => multiply_operation,
        OP_SUBTRACT => subtract_operation,
        OP_DIVIDE => divide_operation,
        OP_MODULO => modulo_operation,
        _ => add_operation,
    }
}

fn get_computation_timestamp() -> libc::time_t {
    let mut current_time: libc::time_t = 0;
    unsafe {
        libc::time(&mut current_time);
    }
    // Arithmetic right shift on signed time_t (matches C `>>` on signed).
    current_time >> 29
}

fn allocate_results(count: c_int) -> *mut ComputationResult {
    unsafe {
        libc::calloc(count as libc::size_t, std::mem::size_of::<ComputationResult>())
            as *mut ComputationResult
    }
}

unsafe fn perform_computation_with_history(
    a: c_int,
    b: c_int,
    op: c_int,
    history: *mut *mut ComputationResult,
    history_count: *mut c_int,
) -> c_int {
    let math_func = select_operation(op);
    let result = math_func(a, b, 0);

    if (*history).is_null() {
        *history = allocate_results(10);
        *history_count = 0;
    }

    if *history_count < 10 {
        let entry = (*history).offset(*history_count as isize);
        (*entry).value = result;
        (*entry).timestamp = get_computation_timestamp();
        (*entry).status = STATUS_SUCCESS;
        *history_count += 1;
    }

    result
}

// ---- Public C-ABI entry point --------------------------------------------

#[unsafe(no_mangle)]
pub extern "C" fn mathop(param1: c_int, param2: c_int, param3: c_int, param4: c_int) -> c_int {
    unsafe {
        // char validation_char = (char)(param1 % 128);
        let mut validation_char: c_char = (param1 % 128) as c_char;
        let is_valid = is_valid_operation(validation_char);

        if !is_valid {
            validation_char = b'1' as c_char;
        }
        // validation_char is set but unused afterwards (matches C behavior).
        let _ = validation_char;

        // Operation selected_op = (Operation)((param3 % 5) + 1);
        let selected_op: c_int = (param3 % 5).wrapping_add(1);

        let operation_priority = get_operation_priority(selected_op);

        let intermediate_result = perform_computation_with_history(
            param1,
            param2,
            selected_op,
            addr_of_mut!(COMPUTATION_HISTORY),
            addr_of_mut!(HISTORY_COUNT),
        );

        // Operation second_op = (Operation)(((param4 + 1) % 5) + 1);
        let second_op: c_int = (param4.wrapping_add(1) % 5).wrapping_add(1);
        let mut final_result = perform_computation_with_history(
            intermediate_result,
            param4,
            second_op,
            addr_of_mut!(COMPUTATION_HISTORY),
            addr_of_mut!(HISTORY_COUNT),
        );

        final_result = final_result.wrapping_add(operation_priority);

        let computation_time = get_computation_timestamp();

        let time_modifier = (computation_time % 100) as c_int;
        final_result = final_result.wrapping_add(time_modifier);

        // Use libc's printf so output is byte-identical to the C version.
        libc::printf(
            b"Computation performed at timestamp: %ld\n\0".as_ptr() as *const c_char,
            computation_time as c_long,
        );
        libc::printf(
            b"Operation priority: %d\n\0".as_ptr() as *const c_char,
            operation_priority,
        );
        libc::printf(
            b"History entries: %d\n\0".as_ptr() as *const c_char,
            HISTORY_COUNT,
        );
        libc::printf(
            b"Final result: %d\n\0".as_ptr() as *const c_char,
            final_result,
        );

        final_result
    }
}
