use std::ffi::{c_int, c_long};
use std::ptr;

extern "C" {
    fn time(t: *mut i64) -> i64;
    fn calloc(nmemb: usize, size: usize) -> *mut u8;
    fn printf(format: *const u8, ...) -> c_int;
}

#[repr(C)]
#[derive(Clone, Copy)]
struct ComputationResult {
    value: c_int,
    timestamp: i64,
    status: c_int,
}

type MathOperation = fn(c_int, c_int, c_int) -> c_int;

fn is_valid_operation(op_char: u8) -> bool {
    let valid = op_char != 0 && (op_char >= b'1' && op_char <= b'5');
    valid
}

fn get_operation_priority(op: c_int) -> c_int {
    op * 10
}

fn add_operation(a: c_int, b: c_int, _unused: c_int) -> c_int {
    a + b
}

fn multiply_operation(a: c_int, b: c_int, _unused: c_int) -> c_int {
    a * b
}

fn subtract_operation(a: c_int, b: c_int, _unused: c_int) -> c_int {
    a - b
}

fn divide_operation(a: c_int, b: c_int, _unused: c_int) -> c_int {
    if b == 0 { return 0; }
    a / b
}

fn modulo_operation(a: c_int, b: c_int, _unused: c_int) -> c_int {
    if b == 0 { return 0; }
    a % b
}

fn select_operation(op: c_int) -> MathOperation {
    match op {
        1 => add_operation,
        2 => multiply_operation,
        3 => subtract_operation,
        4 => divide_operation,
        5 => modulo_operation,
        _ => add_operation,
    }
}

fn get_computation_timestamp() -> i64 {
    let mut current_time: i64;
    unsafe {
        current_time = time(ptr::null_mut());
    }
    current_time = current_time >> 29;
    current_time
}

fn allocate_results(count: c_int) -> *mut ComputationResult {
    unsafe {
        calloc(count as usize, std::mem::size_of::<ComputationResult>()) as *mut ComputationResult
    }
}

fn perform_computation_with_history(
    a: c_int,
    b: c_int,
    op: c_int,
    history: &mut *mut ComputationResult,
    history_count: &mut c_int,
) -> c_int {
    let math_func = select_operation(op);
    let result = math_func(a, b, 0);

    if (*history).is_null() {
        *history = allocate_results(10);
        *history_count = 0;
    }

    if *history_count < 10 {
        unsafe {
            let entry = &mut *(*history).offset(*history_count as isize);
            entry.value = result;
            entry.timestamp = get_computation_timestamp();
            entry.status = 0; // STATUS_SUCCESS
        }
        *history_count += 1;
    }

    result
}

#[unsafe(no_mangle)]
pub extern "C" fn mathop(param1: c_int, param2: c_int, param3: c_int, param4: c_int) -> c_int {
    static mut COMPUTATION_HISTORY: *mut ComputationResult = ptr::null_mut();
    static mut HISTORY_COUNT: c_int = 0;

    let validation_char = (param1 % 128) as u8;
    let is_valid = is_valid_operation(validation_char);

    if !is_valid {
        // validation_char reassigned but unused — matches C behavior
    }

    let selected_op = (param3 % 5) + 1;
    let operation_priority = get_operation_priority(selected_op);

    let (intermediate_result, final_result, history_count_val, computation_time);
    unsafe {
        intermediate_result = perform_computation_with_history(
            param1, param2, selected_op, &mut COMPUTATION_HISTORY, &mut HISTORY_COUNT,
        );

        let second_op = ((param4 + 1) % 5) + 1;
        let mut result = perform_computation_with_history(
            intermediate_result, param4, second_op, &mut COMPUTATION_HISTORY, &mut HISTORY_COUNT,
        );

        result += operation_priority;

        computation_time = get_computation_timestamp();
        let time_modifier = (computation_time % 100) as c_int;
        result += time_modifier;

        final_result = result;
        history_count_val = HISTORY_COUNT;
    }

    unsafe {
        printf(
            b"Computation performed at timestamp: %ld\n\0".as_ptr(),
            computation_time as c_long,
        );
        printf(b"Operation priority: %d\n\0".as_ptr(), operation_priority);
        printf(b"History entries: %d\n\0".as_ptr(), history_count_val);
        printf(b"Final result: %d\n\0".as_ptr(), final_result);
    }

    final_result
}
