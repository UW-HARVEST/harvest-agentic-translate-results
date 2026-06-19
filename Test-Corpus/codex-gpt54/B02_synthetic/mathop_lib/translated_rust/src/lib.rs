use std::ffi::{c_int, c_long, c_void};
use std::ptr;

#[repr(i32)]
#[allow(dead_code)]
#[derive(Copy, Clone)]
enum StatusCode {
    Success = 0,
    Error = -1,
    Warning = 1,
}

#[repr(C)]
#[derive(Copy, Clone)]
struct ComputationResult {
    value: c_int,
    timestamp: libc::time_t,
    status: StatusCode,
}

type MathOperation = extern "C" fn(c_int, c_int, c_int) -> c_int;
type Operation = c_int;

static mut COMPUTATION_HISTORY: *mut ComputationResult = ptr::null_mut();
static mut HISTORY_COUNT: c_int = 0;

const OP_ADD: Operation = 1;
const OP_MULTIPLY: Operation = 2;
const OP_SUBTRACT: Operation = 3;
const OP_DIVIDE: Operation = 4;
const OP_MODULO: Operation = 5;

const TIMESTAMP_FORMAT: &[u8] = b"Computation performed at timestamp: %ld\n\0";
const PRIORITY_FORMAT: &[u8] = b"Operation priority: %d\n\0";
const HISTORY_FORMAT: &[u8] = b"History entries: %d\n\0";
const RESULT_FORMAT: &[u8] = b"Final result: %d\n\0";

fn is_valid_operation(op_char: i8) -> bool {
    let valid = (op_char != 0 && (b'1' as i8..=b'5' as i8).contains(&op_char)) as i8;
    valid != 0
}

fn get_operation_priority(op: Operation) -> c_int {
    op.wrapping_mul(10)
}

extern "C" fn add_operation(a: c_int, b: c_int, _unused_param: c_int) -> c_int {
    a.wrapping_add(b)
}

extern "C" fn multiply_operation(a: c_int, b: c_int, _unused_param: c_int) -> c_int {
    a.wrapping_mul(b)
}

extern "C" fn subtract_operation(a: c_int, b: c_int, _unused_param: c_int) -> c_int {
    a.wrapping_sub(b)
}

extern "C" fn divide_operation(a: c_int, b: c_int, _unused_param: c_int) -> c_int {
    if b == 0 {
        return 0;
    }
    a / b
}

extern "C" fn modulo_operation(a: c_int, b: c_int, _unused_param: c_int) -> c_int {
    if b == 0 {
        return 0;
    }
    a % b
}

fn select_operation(op: Operation) -> MathOperation {
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
    current_time >> 29
}

fn allocate_results(count: c_int) -> *mut ComputationResult {
    unsafe {
        libc::calloc(count as usize, std::mem::size_of::<ComputationResult>()) as *mut ComputationResult
    }
}

unsafe fn perform_computation_with_history(
    a: c_int,
    b: c_int,
    op: Operation,
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
        let entry = (*history).add(*history_count as usize);
        (*entry).value = result;
        (*entry).timestamp = get_computation_timestamp();
        (*entry).status = StatusCode::Success;
        *history_count += 1;
    }

    result
}

#[unsafe(no_mangle)]
pub extern "C" fn mathop(param1: c_int, param2: c_int, param3: c_int, param4: c_int) -> c_int {
    let mut validation_char = (param1 % 128) as i8;
    let is_valid = is_valid_operation(validation_char);

    if !is_valid {
        validation_char = b'1' as i8;
    }

    let _ = validation_char;

    let selected_op = (param3 % 5) + 1;
    let operation_priority = get_operation_priority(selected_op);

    let intermediate_result = unsafe {
        perform_computation_with_history(
            param1,
            param2,
            selected_op,
            ptr::addr_of_mut!(COMPUTATION_HISTORY),
            ptr::addr_of_mut!(HISTORY_COUNT),
        )
    };

    let second_op = ((param4 + 1) % 5) + 1;
    let mut final_result = unsafe {
        perform_computation_with_history(
            intermediate_result,
            param4,
            second_op,
            ptr::addr_of_mut!(COMPUTATION_HISTORY),
            ptr::addr_of_mut!(HISTORY_COUNT),
        )
    };

    final_result = final_result.wrapping_add(operation_priority);

    let computation_time = get_computation_timestamp();
    let time_modifier = (computation_time % 100) as c_int;
    final_result = final_result.wrapping_add(time_modifier);

    unsafe {
        libc::printf(TIMESTAMP_FORMAT.as_ptr().cast(), computation_time as c_long);
        libc::printf(PRIORITY_FORMAT.as_ptr().cast(), operation_priority);
        libc::printf(HISTORY_FORMAT.as_ptr().cast(), HISTORY_COUNT);
        libc::printf(RESULT_FORMAT.as_ptr().cast(), final_result);
    }

    final_result
}

#[allow(dead_code)]
fn _keep_status_variants(status: StatusCode) -> c_int {
    status as c_int
}

#[allow(dead_code)]
fn _keep_c_void(_: *mut c_void) {}
