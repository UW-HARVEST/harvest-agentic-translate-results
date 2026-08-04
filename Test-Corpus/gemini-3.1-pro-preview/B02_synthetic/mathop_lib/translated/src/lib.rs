use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum Operation {
    Add = 1,
    Multiply = 2,
    Subtract = 3,
    Divide = 4,
    Modulo = 5,
}

impl From<i32> for Operation {
    fn from(val: i32) -> Self {
        match val {
            1 => Operation::Add,
            2 => Operation::Multiply,
            3 => Operation::Subtract,
            4 => Operation::Divide,
            5 => Operation::Modulo,
            _ => Operation::Add,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum StatusCode {
    Success = 0,
    Error = -1,
    Warning = 1,
}

#[derive(Clone, Copy)]
pub struct ComputationResult {
    pub value: i32,
    pub timestamp: i64,
    pub status: StatusCode,
}

type MathOperation = fn(i32, i32, i32) -> i32;

fn is_valid_operation(op_char: u8) -> bool {
    op_char != 0 && op_char >= b'1' && op_char <= b'5'
}

fn get_operation_priority(op: Operation) -> i32 {
    (op as i32) * 10
}

fn add_operation(a: i32, b: i32, _unused: i32) -> i32 {
    a.wrapping_add(b)
}

fn multiply_operation(a: i32, b: i32, _unused: i32) -> i32 {
    a.wrapping_mul(b)
}

fn subtract_operation(a: i32, b: i32, _unused: i32) -> i32 {
    a.wrapping_sub(b)
}

fn divide_operation(a: i32, b: i32, _unused: i32) -> i32 {
    if b == 0 {
        return 0;
    }
    a.wrapping_div(b)
}

fn modulo_operation(a: i32, b: i32, _unused: i32) -> i32 {
    if b == 0 {
        return 0;
    }
    a.wrapping_rem(b)
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

fn perform_computation_with_history(
    a: i32,
    b: i32,
    op: Operation,
    history: &mut Vec<ComputationResult>,
) -> i32 {
    let math_func = select_operation(op);
    let result = math_func(a, b, 0);

    if history.len() < 10 {
        history.push(ComputationResult {
            value: result,
            timestamp: get_computation_timestamp(),
            status: StatusCode::Success,
        });
    }

    result
}

static COMPUTATION_HISTORY: Mutex<Vec<ComputationResult>> = Mutex::new(Vec::new());

#[unsafe(no_mangle)]
pub extern "C" fn mathop(param1: i32, param2: i32, param3: i32, param4: i32) -> i32 {
    let mut _validation_char = (param1 % 128) as u8;
    let is_valid = is_valid_operation(_validation_char);

    if !is_valid {
        _validation_char = b'1';
    }

    let selected_op = Operation::from((param3 % 5) + 1);
    let operation_priority = get_operation_priority(selected_op);

    let mut history = COMPUTATION_HISTORY.lock().unwrap();

    let intermediate_result = perform_computation_with_history(
        param1, param2, selected_op, &mut history
    );

    let second_op = Operation::from(((param4 + 1) % 5) + 1);
    let mut final_result = perform_computation_with_history(
        intermediate_result, param4, second_op, &mut history
    );

    final_result = final_result.wrapping_add(operation_priority);

    let computation_time = get_computation_timestamp();
    let time_modifier = (computation_time % 100) as i32;
    final_result = final_result.wrapping_add(time_modifier);

    println!("Computation performed at timestamp: {}", computation_time);
    println!("Operation priority: {}", operation_priority);
    println!("History entries: {}", history.len());
    println!("Final result: {}", final_result);

    final_result
}
