use std::ffi::{c_char, c_int, c_long, c_void};
use std::ptr;

#[cfg(target_arch = "x86_64")]
use std::arch::asm;

pub type Operation = c_int;
pub type StatusCode = c_int;
pub type MathOperation = unsafe extern "C" fn(c_int, c_int, c_int) -> c_int;

const OP_ADD: Operation = 1;
const OP_MULTIPLY: Operation = 2;
const OP_SUBTRACT: Operation = 3;
const OP_DIVIDE: Operation = 4;
const OP_MODULO: Operation = 5;
const STATUS_SUCCESS: StatusCode = 0;

#[repr(C)]
pub struct ComputationResult {
    pub value: c_int,
    pub timestamp: c_long,
    pub status: StatusCode,
}

unsafe extern "C" {
    fn calloc(count: usize, size: usize) -> *mut c_void;
    fn printf(format: *const c_char, ...) -> c_int;
    fn time(timer: *mut c_long) -> c_long;
}

#[unsafe(no_mangle)]
pub extern "C" fn is_valid_operation(op_char: c_char) -> bool {
    op_char != 0 && op_char >= b'1' as c_char && op_char <= b'5' as c_char
}

#[unsafe(no_mangle)]
pub extern "C" fn get_operation_priority(op: Operation) -> c_int {
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

#[cfg(target_arch = "x86_64")]
fn divide_with_c_machine_behavior(a: c_int, b: c_int) -> (c_int, c_int) {
    let quotient;
    let remainder;
    unsafe {
        asm!(
            "cdq",
            "idiv {divisor:e}",
            divisor = in(reg) b,
            inout("eax") a => quotient,
            out("edx") remainder,
            options(nomem, nostack),
        );
    }
    (quotient, remainder)
}

#[cfg(not(target_arch = "x86_64"))]
fn divide_with_c_machine_behavior(a: c_int, b: c_int) -> (c_int, c_int) {
    (a.wrapping_div(b), a.wrapping_rem(b))
}

#[unsafe(no_mangle)]
pub extern "C" fn divide_operation(a: c_int, b: c_int, _unused_param: c_int) -> c_int {
    if b == 0 {
        0
    } else {
        divide_with_c_machine_behavior(a, b).0
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn modulo_operation(a: c_int, b: c_int, _unused_param: c_int) -> c_int {
    if b == 0 {
        0
    } else {
        divide_with_c_machine_behavior(a, b).1
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn select_operation(op: Operation) -> MathOperation {
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
pub unsafe extern "C" fn get_computation_timestamp() -> c_long {
    let mut current_time = 0;
    unsafe {
        time(&mut current_time);
    }
    current_time >> 29
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn allocate_results(count: c_int) -> *mut ComputationResult {
    unsafe { calloc(count as usize, std::mem::size_of::<ComputationResult>()).cast() }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn perform_computation_with_history(
    a: c_int,
    b: c_int,
    op: Operation,
    history: *mut *mut ComputationResult,
    history_count: *mut c_int,
) -> c_int {
    let math_func = select_operation(op);
    let result = unsafe { math_func(a, b, 0) };

    unsafe {
        if (*history).is_null() {
            *history = allocate_results(10);
            *history_count = 0;
        }

        if *history_count < 10 {
            let entry = (*history).add(*history_count as usize);
            (*entry).value = result;
            (*entry).timestamp = get_computation_timestamp();
            (*entry).status = STATUS_SUCCESS;
            *history_count = (*history_count).wrapping_add(1);
        }
    }

    result
}

static mut COMPUTATION_HISTORY: *mut ComputationResult = ptr::null_mut();
static mut HISTORY_COUNT: c_int = 0;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn mathop(
    param1: c_int,
    param2: c_int,
    param3: c_int,
    param4: c_int,
) -> c_int {
    let mut validation_char = (param1 % 128) as c_char;
    let is_valid = is_valid_operation(validation_char);

    if !is_valid {
        validation_char = b'1' as c_char;
    }
    std::hint::black_box(validation_char);

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

    let computation_time = unsafe { get_computation_timestamp() };
    let time_modifier = (computation_time % 100) as c_int;
    final_result = final_result.wrapping_add(time_modifier);

    unsafe {
        printf(
            c"Computation performed at timestamp: %ld\n".as_ptr(),
            computation_time,
        );
        printf(c"Operation priority: %d\n".as_ptr(), operation_priority);
        printf(c"History entries: %d\n".as_ptr(), HISTORY_COUNT);
        printf(c"Final result: %d\n".as_ptr(), final_result);
    }

    final_result
}
