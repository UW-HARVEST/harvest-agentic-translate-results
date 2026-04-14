use std::fmt::Write as _;
use std::os::raw::c_int;

struct StringBuffer {
    data: String,
    capacity: usize,
}

fn create_buffer(initial_capacity: c_int) -> Option<StringBuffer> {
    let capacity = if initial_capacity <= 0 {
        0
    } else {
        initial_capacity as usize
    };
    let mut data = String::with_capacity(capacity);
    data.push('\0');
    data.pop();
    Some(StringBuffer { data, capacity })
}

fn append_to_buffer(buffer: &mut StringBuffer, s: &str) -> c_int {
    let required_capacity = buffer.data.len() + s.len() + 1;
    if required_capacity > buffer.capacity {
        buffer.capacity = required_capacity * 2;
        if buffer.data.capacity() < buffer.capacity {
            buffer.data.reserve(buffer.capacity - buffer.data.capacity());
        }
    }
    buffer.data.push_str(s);
    0
}

fn destroy_buffer(_buffer: StringBuffer) {}

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
        "add" => a + b,
        "subtract" => a - b,
        "multiply" => a * b,
        "divide" => {
            if b != 0 {
                a / b
            } else {
                0
            }
        }
        _ => 0,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn buffapp(param1: c_int, param2: c_int, param3: c_int, param4: c_int) -> c_int {
    let mut log_buffer = match create_buffer(32) {
        Some(buffer) => buffer,
        None => return 0,
    };
    let mut result = 0;
    let mut temp = String::with_capacity(64);

    log_buffer.data.clear();

    temp.clear();
    let _ = write!(temp, "Starting computation with {} parameters\n", 4);
    append_to_buffer(&mut log_buffer, &temp);

    let op1 = get_operation_name(param1 % 4);
    temp.clear();
    let _ = write!(temp, "Operation 1: {}({}, {})\n", op1, param1, param2);
    append_to_buffer(&mut log_buffer, &temp);

    let intermediate1 = perform_operation(param1, param2, op1);
    result += intermediate1;

    let op2 = get_operation_name(param3 % 4);
    temp.clear();
    let _ = write!(temp, "Operation 2: {}({}, {})\n", op2, param3, param4);
    append_to_buffer(&mut log_buffer, &temp);

    let intermediate2 = perform_operation(param3, param4, op2);
    result += intermediate2;

    let op3 = "multiply";
    temp.clear();
    let _ = write!(temp, "Operation 3: {}({}, {})\n", op3, intermediate1, intermediate2);
    append_to_buffer(&mut log_buffer, &temp);

    let intermediate3 = perform_operation(intermediate1, intermediate2, op3);

    if intermediate3 != 0 {
        result /= intermediate3;
    } else {
        result = param1 + param2 + param3 + param4;
    }

    temp.clear();
    let _ = write!(temp, "Final result: {}\n", result);
    append_to_buffer(&mut log_buffer, &temp);

    println!("Computation Log:\n{}", log_buffer.data);

    destroy_buffer(log_buffer);

    result
}
