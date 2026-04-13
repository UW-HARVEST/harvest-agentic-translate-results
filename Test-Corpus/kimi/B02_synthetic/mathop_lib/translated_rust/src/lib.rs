use std::os::raw::{c_char, c_int, c_long, c_void};
use std::time::{SystemTime, UNIX_EPOCH};

#[repr(C)]
#[derive(Clone, Copy)]
pub enum Operation {
    Add = 1,
    Multiply = 2,
    Subtract = 3,
    Divide = 4,
    Modulo = 5,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub enum StatusCode {
    Success = 0,
    Error = -1,
    Warning = 1,
}

#[repr(C)]
pub struct ComputationResult {
    pub value: c_int,
    pub timestamp: i64,
    pub status: StatusCode,
}

type MathOperation = fn(c_int, c_int, c_int) -> c_int;

fn is_valid_operation(op_char: c_char) -> bool {
    op_char != 0 && (op_char >= b'1' as c_char && op_char <= b'5' as c_char)
}

fn get_operation_priority(op: Operation) -> c_int {
    (op as c_int) * 10
}

fn add_operation(a: c_int, b: c_int, _unused_param: c_int) -> c_int {
    a + b
}

fn multiply_operation(a: c_int, b: c_int, _unused_param: c_int) -> c_int {
    a * b
}

fn subtract_operation(a: c_int, b: c_int, _unused_param: c_int) -> c_int {
    a - b
}

fn divide_operation(a: c_int, b: c_int, _unused_param: c_int) -> c_int {
    if b == 0 {
        0
    } else {
        a / b
    }
}

fn modulo_operation(a: c_int, b: c_int, _unused_param: c_int) -> c_int {
    if b == 0 {
        0
    } else {
        a % b
    }
}

fn select_operation(op: Operation) -> MathOperation {
    match op {
        Operation::Add => add_operation,
        Operation::Multiply => multiply_operation,
        Operation::Subtract => subtract_operation,
        Operation::Divide => divide_operation,
        Operation::Modulo => modulo_operation,
    }
}

fn get_computation_timestamp() -> i64 {
    let current_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    current_time >> 29
}

fn allocate_results(count: c_int) -> *mut ComputationResult {
    let size = std::mem::size_of::<ComputationResult>() * count as usize;
    let layout = std::alloc::Layout::from_size_align(size, std::mem::align_of::<ComputationResult>()).unwrap();
    unsafe {
        let ptr = std::alloc::alloc_zeroed(layout) as *mut ComputationResult;
        ptr
    }
}

fn perform_computation_with_history(
    a: c_int,
    b: c_int,
    op: Operation,
    history: &mut *mut ComputationResult,
    history_count: &mut c_int,
) -> c_int {
    let math_func = select_operation(op);
    let result = math_func(a, b, 0);

    if history.is_null() {
        *history = allocate_results(10);
        *history_count = 0;
    }

    if *history_count < 10 {
        unsafe {
            let entry = &mut (*history.add(*history_count as usize));
            entry.value = result;
            entry.timestamp = get_computation_timestamp();
            entry.status = StatusCode::Success;
        }
        *history_count += 1;
    }

    result
}

static mut COMPUTATION_HISTORY: *mut ComputationResult = std::ptr::null_mut();
static mut HISTORY_COUNT: c_int = 0;

#[unsafe(no_mangle)]
pub extern "C" fn mathop(param1: c_int, param2: c_int, param3: c_int, param4: c_int) -> c_int {
    unsafe {
        let validation_char = (param1 % 128) as c_char;
        let is_valid = is_valid_operation(validation_char);

        let _validation_char = if !is_valid { b'1' as c_char } else { validation_char };

        let selected_op = std::mem::transmute::<u8, Operation>(((param3 % 5) + 1) as u8);

        let operation_priority = get_operation_priority(selected_op);

        let intermediate_result = perform_computation_with_history(
            param1,
            param2,
            selected_op,
            &mut COMPUTATION_HISTORY,
            &mut HISTORY_COUNT,
        );

        let second_op = std::mem::transmute::<u8, Operation>((((param4 + 1) % 5) + 1) as u8);
        let final_result = perform_computation_with_history(
            intermediate_result,
            param4,
            second_op,
            &mut COMPUTATION_HISTORY,
            &mut HISTORY_COUNT,
        );

        let mut final_result = final_result + operation_priority;

        let computation_time = get_computation_timestamp();

        let time_modifier = (computation_time % 100) as c_int;
        final_result += time_modifier;

        println!("Computation performed at timestamp: {}", computation_time);
        println!("Operation priority: {}", operation_priority);
        println!("History entries: {}", HISTORY_COUNT);
        println!("Final result: {}", final_result);

        final_result
    }
}
