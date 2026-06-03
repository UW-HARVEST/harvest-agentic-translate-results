// Copyright 2025 MIT Lincoln Laboratory
// Permission is hereby granted, free of charge,
// to any person obtaining a copy of this software
// and associated documentation files (the "Software"),
// to deal in the Software without restriction,
// including without limitation the rights to use, copy,
// modify, merge, publish, distribute, sublicense,
// and/or sell copies of the Software,
// and to permit persons to whom the Software is furnished to do so,
// subject to the following conditions:
//
// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
// EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
// THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
// IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE
// FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT,
// TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE
// OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

use std::os::raw::{c_int, c_long};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Operation {
    OpAdd = 1,
    OpMultiply = 2,
    OpSubtract = 3,
    OpDivide = 4,
    OpModulo = 5,
}

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum StatusCode {
    StatusSuccess = 0,
    StatusError = -1,
    StatusWarning = 1,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ComputationResult {
    pub value: c_int,
    pub timestamp: TimeT,
    pub status: StatusCode,
}

// `time_t` on most Linux systems is a 64-bit signed integer.
pub type TimeT = c_long;

type MathOperation = fn(c_int, c_int, c_int) -> c_int;

impl Default for ComputationResult {
    fn default() -> Self {
        ComputationResult {
            value: 0,
            timestamp: 0,
            status: StatusCode::StatusSuccess,
        }
    }
}

pub fn is_valid_operation(op_char: u8) -> bool {
    // The C code computed: char valid = op_char && (op_char >= '1' && op_char <= '5');
    // op_char is a char (signed in some platforms). Truthiness of a char in C: nonzero -> true.
    op_char != 0 && (op_char >= b'1' && op_char <= b'5')
}

pub fn get_operation_priority(op: Operation) -> c_int {
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
        return 0;
    }
    a.wrapping_div(b)
}

fn modulo_operation(a: c_int, b: c_int, _unused_param: c_int) -> c_int {
    if b == 0 {
        return 0;
    }
    a.wrapping_rem(b)
}

pub fn select_operation(op: Operation) -> MathOperation {
    match op {
        Operation::OpAdd => add_operation,
        Operation::OpMultiply => multiply_operation,
        Operation::OpSubtract => subtract_operation,
        Operation::OpDivide => divide_operation,
        Operation::OpModulo => modulo_operation,
    }
}

pub fn get_computation_timestamp() -> TimeT {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as TimeT)
        .unwrap_or(0);
    secs >> 29
}

pub fn allocate_results(count: usize) -> Vec<ComputationResult> {
    vec![ComputationResult::default(); count]
}

pub fn perform_computation_with_history(
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

    if let Some(h) = history.as_mut() {
        if *history_count < 10 {
            h[*history_count].value = result;
            h[*history_count].timestamp = get_computation_timestamp();
            h[*history_count].status = StatusCode::StatusSuccess;
            *history_count += 1;
        }
    }

    result
}

fn op_from_int(value: c_int) -> Operation {
    match value {
        1 => Operation::OpAdd,
        2 => Operation::OpMultiply,
        3 => Operation::OpSubtract,
        4 => Operation::OpDivide,
        5 => Operation::OpModulo,
        // Default fallback (mirrors `select_operation`'s default to add).
        _ => Operation::OpAdd,
    }
}

struct MathOpState {
    history: Option<Vec<ComputationResult>>,
    history_count: usize,
}

static MATHOP_STATE: Mutex<MathOpState> = Mutex::new(MathOpState {
    history: None,
    history_count: 0,
});

pub fn mathop_rs(param1: c_int, param2: c_int, param3: c_int, param4: c_int) -> c_int {
    let mut state = MATHOP_STATE.lock().unwrap();

    // char validation_char = (char)(param1 % 128);
    let validation_char_signed = (param1.wrapping_rem(128)) as i8;
    let validation_char_unsigned = validation_char_signed as u8;
    let is_valid = is_valid_operation(validation_char_unsigned);

    let _validation_char = if !is_valid { b'1' } else { validation_char_unsigned };

    // Operation selected_op = (Operation)((param3 % 5) + 1);
    let selected_op = op_from_int(param3.wrapping_rem(5).wrapping_add(1));
    let operation_priority = get_operation_priority(selected_op);

    // Split-borrow the two fields of `state` so we can pass mutable references
    // to both into helper functions simultaneously.
    let MathOpState {
        history: history_ref,
        history_count: history_count_ref,
    } = &mut *state;

    let intermediate_result = perform_computation_with_history(
        param1,
        param2,
        selected_op,
        history_ref,
        history_count_ref,
    );

    // Operation second_op = (Operation)(((param4 + 1) % 5) + 1);
    let second_op =
        op_from_int(param4.wrapping_add(1).wrapping_rem(5).wrapping_add(1));

    let final_result_intermediate = perform_computation_with_history(
        intermediate_result,
        param4,
        second_op,
        history_ref,
        history_count_ref,
    );

    let mut final_result = final_result_intermediate.wrapping_add(operation_priority);

    let computation_time = get_computation_timestamp();
    let time_modifier = (computation_time.rem_euclid(100)) as c_int;
    final_result = final_result.wrapping_add(time_modifier);

    println!(
        "Computation performed at timestamp: {}",
        computation_time as i64
    );
    println!("Operation priority: {}", operation_priority);
    println!("History entries: {}", state.history_count);
    println!("Final result: {}", final_result);

    final_result
}

/// C ABI entry point matching `int mathop(int a, int b, int c, int d);`.
#[unsafe(no_mangle)]
pub extern "C" fn mathop(a: c_int, b: c_int, c: c_int, d: c_int) -> c_int {
    mathop_rs(a, b, c, d)
}
