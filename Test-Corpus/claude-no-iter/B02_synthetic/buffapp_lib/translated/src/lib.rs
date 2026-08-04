// Copyright 2025 MIT Lincoln Laboratory
// Translated from C to Rust.

use std::ffi::CString;
use std::os::raw::{c_char, c_int};

unsafe extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
}

fn get_operation_name(op_code: c_int) -> &'static str {
    match op_code {
        0 => "add",
        1 => "subtract",
        2 => "multiply",
        3 => "divide",
        _ => "unknown",
    }
}

fn perform_operation(a: c_int, b: c_int, operation: &str) -> c_int {
    match operation {
        "add" => a.wrapping_add(b),
        "subtract" => a.wrapping_sub(b),
        "multiply" => a.wrapping_mul(b),
        "divide" => {
            if b != 0 {
                // Match C: signed division. C-style: truncate toward zero.
                // Note: a / b in C with INT_MIN / -1 is UB; Rust would panic.
                // Use wrapping_div which matches C behavior except for that case.
                a.wrapping_div(b)
            } else {
                0
            }
        }
        _ => 0,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn buffapp(param1: c_int, param2: c_int, param3: c_int, param4: c_int) -> c_int {
    let mut log_buffer = String::new();
    let mut result: c_int = 0;

    // sprintf(temp, "Starting computation with %d parameters\n", 4);
    log_buffer.push_str(&format!("Starting computation with {} parameters\n", 4));

    // op1 = get_operation_name(param1 % 4)
    // C's % for negative values produces negative remainder; Rust's % does too.
    let op1 = get_operation_name(param1 % 4);
    log_buffer.push_str(&format!(
        "Operation 1: {}({}, {})\n",
        op1, param1, param2
    ));

    let intermediate1 = perform_operation(param1, param2, op1);
    result = result.wrapping_add(intermediate1);

    let op2 = get_operation_name(param3 % 4);
    log_buffer.push_str(&format!(
        "Operation 2: {}({}, {})\n",
        op2, param3, param4
    ));

    let intermediate2 = perform_operation(param3, param4, op2);
    result = result.wrapping_add(intermediate2);

    let op3 = "multiply";
    log_buffer.push_str(&format!(
        "Operation 3: {}({}, {})\n",
        op3, intermediate1, intermediate2
    ));

    let intermediate3 = perform_operation(intermediate1, intermediate2, op3);

    if intermediate3 != 0 {
        result = result.wrapping_div(intermediate3);
    } else {
        result = param1
            .wrapping_add(param2)
            .wrapping_add(param3)
            .wrapping_add(param4);
    }

    log_buffer.push_str(&format!("Final result: {}\n", result));

    // printf("Computation Log:\n%s\n", log_buffer->data);
    let fmt = CString::new("Computation Log:\n%s\n").unwrap();
    let data = CString::new(log_buffer).unwrap();
    unsafe {
        printf(fmt.as_ptr(), data.as_ptr());
    }

    result
}
