use std::ffi::{c_char, c_int, c_long, c_void};
use std::mem::size_of;
use std::ptr;

const OP_ADD: c_int = 1;
const OP_MULTIPLY: c_int = 2;
const OP_SUBTRACT: c_int = 3;
const OP_DIVIDE: c_int = 4;
const OP_MODULO: c_int = 5;

const STATUS_SUCCESS: c_int = 0;

type TimeT = c_long;
type MathOperation = extern "C" fn(c_int, c_int, c_int) -> c_int;

#[repr(C)]
pub struct ComputationResult {
    value: c_int,
    timestamp: TimeT,
    status: c_int,
}

unsafe extern "C" {
    fn time(timer: *mut TimeT) -> TimeT;
    fn calloc(nmemb: usize, size: usize) -> *mut c_void;
    fn printf(format: *const c_char, ...) -> c_int;
}

static mut COMPUTATION_HISTORY: *mut ComputationResult = ptr::null_mut();
static mut HISTORY_COUNT: c_int = 0;

#[unsafe(no_mangle)]
pub extern "C" fn is_valid_operation(op_char: c_char) -> bool {
    let valid: c_char = if op_char != 0 && op_char >= b'1' as c_char && op_char <= b'5' as c_char {
        1
    } else {
        0
    };
    valid != 0
}

#[unsafe(no_mangle)]
pub extern "C" fn get_operation_priority(op: c_int) -> c_int {
    op.wrapping_mul(10)
}

#[unsafe(no_mangle)]
pub extern "C" fn add_operation(a: c_int, b: c_int, _unused_param: c_int) -> c_int {
    a.wrapping_add(b)
}

#[unsafe(no_mangle)]
pub extern "C" fn multiply_operation(a: c_int, b: c_int, _unused_param: c_int) -> c_int {
    a.wrapping_mul(b)
}

#[unsafe(no_mangle)]
pub extern "C" fn subtract_operation(a: c_int, b: c_int, _unused_param: c_int) -> c_int {
    a.wrapping_sub(b)
}

#[unsafe(no_mangle)]
pub extern "C" fn divide_operation(a: c_int, b: c_int, _unused_param: c_int) -> c_int {
    if b == 0 {
        return 0;
    }
    a / b
}

#[unsafe(no_mangle)]
pub extern "C" fn modulo_operation(a: c_int, b: c_int, _unused_param: c_int) -> c_int {
    if b == 0 {
        return 0;
    }
    a % b
}

#[unsafe(no_mangle)]
pub extern "C" fn select_operation(op: c_int) -> MathOperation {
    match op {
        OP_ADD => add_operation,
        OP_MULTIPLY => multiply_operation,
        OP_SUBTRACT => subtract_operation,
        OP_DIVIDE => divide_operation,
        OP_MODULO => modulo_operation,
        _ => add_operation,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn get_computation_timestamp() -> TimeT {
    let mut current_time: TimeT = 0;
    unsafe {
        time(&mut current_time);
    }
    current_time >> 29
}

#[unsafe(no_mangle)]
pub extern "C" fn allocate_results(count: c_int) -> *mut ComputationResult {
    unsafe { calloc(count as usize, size_of::<ComputationResult>()) as *mut ComputationResult }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn perform_computation_with_history(
    a: c_int,
    b: c_int,
    op: c_int,
    history: *mut *mut ComputationResult,
    history_count: *mut c_int,
) -> c_int {
    let math_func = select_operation(op);
    let result = math_func(a, b, 0);

    unsafe {
        if (*history).is_null() {
            *history = allocate_results(10);
            *history_count = 0;
        }

        if *history_count < 10 {
            let slot = (*history).add(*history_count as usize);
            (*slot).value = result;
            (*slot).timestamp = get_computation_timestamp();
            (*slot).status = STATUS_SUCCESS;
            *history_count = (*history_count).wrapping_add(1);
        }
    }

    result
}

#[unsafe(no_mangle)]
pub extern "C" fn mathop(param1: c_int, param2: c_int, param3: c_int, param4: c_int) -> c_int {
    let mut validation_char = (param1 % 128) as c_char;
    let is_valid = is_valid_operation(validation_char);

    if !is_valid {
        validation_char = b'1' as c_char;
    }

    let selected_op = (param3 % 5).wrapping_add(1);
    let operation_priority = get_operation_priority(selected_op);

    let intermediate_result = unsafe {
        perform_computation_with_history(
            param1,
            param2,
            selected_op,
            &raw mut COMPUTATION_HISTORY,
            &raw mut HISTORY_COUNT,
        )
    };

    let second_op = (param4.wrapping_add(1) % 5).wrapping_add(1);
    let mut final_result = unsafe {
        perform_computation_with_history(
            intermediate_result,
            param4,
            second_op,
            &raw mut COMPUTATION_HISTORY,
            &raw mut HISTORY_COUNT,
        )
    };

    final_result = final_result.wrapping_add(operation_priority);

    let computation_time = get_computation_timestamp();
    let time_modifier = (computation_time % 100) as c_int;
    final_result = final_result.wrapping_add(time_modifier);

    unsafe {
        printf(
            c"Computation performed at timestamp: %ld\n".as_ptr(),
            computation_time as c_long,
        );
        printf(c"Operation priority: %d\n".as_ptr(), operation_priority);
        printf(c"History entries: %d\n".as_ptr(), HISTORY_COUNT);
        printf(c"Final result: %d\n".as_ptr(), final_result);
    }

    let _ = validation_char;
    final_result
}
