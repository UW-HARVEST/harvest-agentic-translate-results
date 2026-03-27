use std::ffi::c_int;
use std::fmt::Write;

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
            if b != 0 { a.wrapping_div(b) } else { 0 }
        }
        _ => 0,
    }
}

/// # Safety
/// Direct translation of C `buffapp` function.
#[unsafe(no_mangle)]
pub extern "C" fn buffapp(param1: c_int, param2: c_int, param3: c_int, param4: c_int) -> c_int {
    let mut log_buffer = String::with_capacity(32);
    let mut result: c_int;

    let _ = writeln!(log_buffer, "Starting computation with {} parameters", 4);

    let op1 = get_operation_name(param1 % 4);
    let _ = writeln!(log_buffer, "Operation 1: {}({}, {})", op1, param1, param2);

    let intermediate1 = perform_operation(param1, param2, op1);
    result = intermediate1;

    let op2 = get_operation_name(param3 % 4);
    let _ = writeln!(log_buffer, "Operation 2: {}({}, {})", op2, param3, param4);

    let intermediate2 = perform_operation(param3, param4, op2);
    result = result.wrapping_add(intermediate2);

    let op3 = "multiply";
    let _ = writeln!(
        log_buffer,
        "Operation 3: {}({}, {})",
        op3, intermediate1, intermediate2
    );

    let intermediate3 = perform_operation(intermediate1, intermediate2, op3);

    if intermediate3 != 0 {
        result = result.wrapping_div(intermediate3);
    } else {
        result = param1
            .wrapping_add(param2)
            .wrapping_add(param3)
            .wrapping_add(param4);
    }

    let _ = writeln!(log_buffer, "Final result: {}", result);

    print!("Computation Log:\n{}\n", log_buffer);

    result
}
