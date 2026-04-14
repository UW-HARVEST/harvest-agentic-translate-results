use std::os::raw::c_int;
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

#[repr(i32)]
#[derive(Copy, Clone)]
enum Operation {
    Add = 1,
    Multiply = 2,
    Subtract = 3,
    Divide = 4,
    Modulo = 5,
}

#[repr(i32)]
#[derive(Copy, Clone)]
enum StatusCode {
    Success = 0,
    Error = -1,
    Warning = 1,
}

#[derive(Copy, Clone)]
struct ComputationResult {
    value: c_int,
    timestamp: i64,
    status: StatusCode,
}

type MathOperation = fn(c_int, c_int, c_int) -> c_int;

fn is_valid_operation(op_char: i8) -> bool {
    op_char != 0 && (b'1' as i8..=b'5' as i8).contains(&op_char)
}

fn get_operation_priority(op: Operation) -> c_int {
    (op as c_int) * 10
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
    } else if a == i32::MIN && b == -1 {
        a
    } else {
        a / b
    }
}

fn modulo_operation(a: c_int, b: c_int, _unused_param: c_int) -> c_int {
    if b == 0 {
        0
    } else if a == i32::MIN && b == -1 {
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
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    current_time >> 29
}

fn allocate_results(count: usize) -> Vec<ComputationResult> {
    vec![
        ComputationResult {
            value: 0,
            timestamp: 0,
            status: StatusCode::Success,
        };
        count
    ]
}

fn perform_computation_with_history(
    a: c_int,
    b: c_int,
    op: Operation,
    history: &mut Option<Vec<ComputationResult>>,
    history_count: &mut usize,
) -> c_int {
    let math_func = select_operation(op);
    let result = math_func(a, b, 0);

    if history.is_none() {
        *history = Some(allocate_results(10));
        *history_count = 0;
    }

    if *history_count < 10 {
        if let Some(entries) = history.as_mut() {
            entries[*history_count].value = result;
            entries[*history_count].timestamp = get_computation_timestamp();
            entries[*history_count].status = StatusCode::Success;
            *history_count += 1;
        }
    }

    result
}

struct State {
    computation_history: Option<Vec<ComputationResult>>,
    history_count: usize,
}

fn global_state() -> &'static Mutex<State> {
    static STATE: OnceLock<Mutex<State>> = OnceLock::new();
    STATE.get_or_init(|| {
        Mutex::new(State {
            computation_history: None,
            history_count: 0,
        })
    })
}

fn operation_from_value(value: c_int) -> Operation {
    match value {
        1 => Operation::Add,
        2 => Operation::Multiply,
        3 => Operation::Subtract,
        4 => Operation::Divide,
        _ => Operation::Modulo,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn mathop(param1: c_int, param2: c_int, param3: c_int, param4: c_int) -> c_int {
    let mut state = global_state().lock().unwrap();

    let mut validation_char = (param1 % 128) as i8;
    let is_valid = is_valid_operation(validation_char);

    if !is_valid {
        validation_char = b'1' as i8;
    }

    let selected_op = operation_from_value((param3 % 5) + 1);
    let operation_priority = get_operation_priority(selected_op);

    let intermediate_result = perform_computation_with_history(
        param1,
        param2,
        selected_op,
        &mut state.computation_history,
        &mut state.history_count,
    );

    let second_op = operation_from_value(((param4 + 1) % 5) + 1);
    let mut final_result = perform_computation_with_history(
        intermediate_result,
        param4,
        second_op,
        &mut state.computation_history,
        &mut state.history_count,
    );

    final_result = final_result.wrapping_add(operation_priority);

    let computation_time = get_computation_timestamp();
    let time_modifier = (computation_time % 100) as c_int;
    final_result = final_result.wrapping_add(time_modifier);

    println!("Computation performed at timestamp: {}", computation_time);
    println!("Operation priority: {}", operation_priority);
    println!("History entries: {}", state.history_count as c_int);
    println!("Final result: {}", final_result);

    let _ = validation_char;
    let _ = StatusCode::Error;
    let _ = StatusCode::Warning;

    final_result
}
