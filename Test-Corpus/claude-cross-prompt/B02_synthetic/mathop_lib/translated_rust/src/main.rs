// Rust translation of c_src/src/lib.c that produces byte-identical output.

use std::io::{self, Read, Write};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Copy, Clone, PartialEq, Eq)]
#[repr(i32)]
#[allow(dead_code)]
enum Operation {
    Add = 1,
    Multiply = 2,
    Subtract = 3,
    Divide = 4,
    Modulo = 5,
}

#[derive(Copy, Clone, PartialEq, Eq)]
#[repr(i32)]
#[allow(dead_code)]
enum StatusCode {
    Success = 0,
    Error = -1,
    Warning = 1,
}

#[derive(Copy, Clone)]
struct ComputationResult {
    value: i32,
    timestamp: i64,
    status: StatusCode,
}

impl Default for ComputationResult {
    fn default() -> Self {
        ComputationResult {
            value: 0,
            timestamp: 0,
            status: StatusCode::Success,
        }
    }
}

type MathOperation = fn(i32, i32, i32) -> i32;

fn is_valid_operation(op_char: i8) -> bool {
    // C: char valid = op_char && (op_char >= '1' && op_char <= '5');
    // op_char is non-zero AND in '1'..'5'
    op_char != 0 && (op_char >= b'1' as i8 && op_char <= b'5' as i8)
}

fn get_operation_priority(op: Operation) -> i32 {
    let priority = (op as i32) * 10;
    priority
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
    // C uses integer division, which truncates toward zero
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
    // C: time_t current_time; time(&current_time); current_time = current_time >> 29;
    // time_t is typically a signed integer of seconds since epoch.
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    // arithmetic right shift on signed value
    secs >> 29
}

fn op_from_int(v: i32) -> Operation {
    match v {
        1 => Operation::Add,
        2 => Operation::Multiply,
        3 => Operation::Subtract,
        4 => Operation::Divide,
        5 => Operation::Modulo,
        _ => Operation::Add,
    }
}

fn allocate_results(count: usize) -> Vec<ComputationResult> {
    vec![ComputationResult::default(); count]
}

fn perform_computation_with_history(
    a: i32,
    b: i32,
    op: Operation,
    history: &mut Option<Vec<ComputationResult>>,
    history_count: &mut i32,
) -> i32 {
    let math_func = select_operation(op);

    let result = math_func(a, b, 0);

    if history.is_none() {
        *history = Some(allocate_results(10));
        *history_count = 0;
    }

    if *history_count < 10 {
        let h = history.as_mut().unwrap();
        let idx = *history_count as usize;
        h[idx].value = result;
        h[idx].timestamp = get_computation_timestamp();
        h[idx].status = StatusCode::Success;
        *history_count += 1;
    }

    result
}

struct MathopState {
    computation_history: Option<Vec<ComputationResult>>,
    history_count: i32,
}

impl MathopState {
    fn new() -> Self {
        MathopState {
            computation_history: None,
            history_count: 0,
        }
    }
}

fn mathop(state: &mut MathopState, param1: i32, param2: i32, param3: i32, param4: i32) -> i32 {
    // C: char validation_char = (char)(param1 % 128);
    // In C, char may be signed/unsigned depending on platform; on x86-64 Linux char is signed.
    // param1 % 128 has the same sign as param1, range (-127..127). Cast to char keeps low 8 bits as signed.
    let rem = param1 % 128;
    // Truncate to 8 bits as signed char
    let validation_char = rem as i8;

    let is_valid = is_valid_operation(validation_char);

    let mut _validation_char = validation_char;
    if !is_valid {
        _validation_char = b'1' as i8;
    }

    // Operation selected_op = (Operation)((param3 % 5) + 1);
    // param3 % 5 in C truncates toward zero, can be negative
    let selected_op_int = (param3 % 5) + 1;
    let selected_op = op_from_int(selected_op_int);

    let operation_priority = get_operation_priority(selected_op);

    let intermediate_result = perform_computation_with_history(
        param1,
        param2,
        selected_op,
        &mut state.computation_history,
        &mut state.history_count,
    );

    // Operation second_op = (Operation)(((param4 + 1) % 5) + 1);
    let second_op_int = ((param4.wrapping_add(1)) % 5) + 1;
    let second_op = op_from_int(second_op_int);

    let mut final_result = perform_computation_with_history(
        intermediate_result,
        param4,
        second_op,
        &mut state.computation_history,
        &mut state.history_count,
    );

    final_result = final_result.wrapping_add(operation_priority);

    let computation_time = get_computation_timestamp();

    // int time_modifier = (int)(computation_time % 100);
    let time_modifier = (computation_time % 100) as i32;
    final_result = final_result.wrapping_add(time_modifier);

    // printf with %ld for long
    let stdout = io::stdout();
    let mut out = stdout.lock();
    let _ = writeln!(out, "Computation performed at timestamp: {}", computation_time);
    let _ = writeln!(out, "Operation priority: {}", operation_priority);
    let _ = writeln!(out, "History entries: {}", state.history_count);
    let _ = writeln!(out, "Final result: {}", final_result);

    final_result
}

fn read_all_stdin() -> String {
    let mut s = String::new();
    let _ = io::stdin().read_to_string(&mut s);
    s
}

/// Tokenize whitespace-separated decimal integers (mimics scanf("%d") behavior:
/// scanf skips leading whitespace including newlines).
fn parse_ints(input: &str, n: usize) -> Vec<i32> {
    let mut out = Vec::with_capacity(n);
    for tok in input.split_ascii_whitespace() {
        if out.len() >= n {
            break;
        }
        // scanf reads optional sign followed by digits
        if let Ok(v) = tok.parse::<i32>() {
            out.push(v);
        } else {
            // Try to parse leading integer prefix
            let mut chars = tok.chars();
            let mut s = String::new();
            if let Some(c) = chars.clone().next() {
                if c == '-' || c == '+' {
                    s.push(c);
                    chars.next();
                }
            }
            for c in chars {
                if c.is_ascii_digit() {
                    s.push(c);
                } else {
                    break;
                }
            }
            if let Ok(v) = s.parse::<i32>() {
                out.push(v);
            } else {
                break;
            }
        }
    }
    out
}

fn main() {
    let input = read_all_stdin();
    let ints = parse_ints(&input, 4);

    if ints.len() < 4 {
        // If there are not enough inputs, behave like uninitialized scanf reads:
        // The C library doesn't have a main, so we mirror typical executable behavior:
        // exit without calling mathop.
        return;
    }

    let mut state = MathopState::new();
    let _ = mathop(&mut state, ints[0], ints[1], ints[2], ints[3]);
}
