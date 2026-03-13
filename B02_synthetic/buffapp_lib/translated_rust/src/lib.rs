use std::ffi::c_int;

struct StringBuffer {
    data: Vec<u8>,
}

impl StringBuffer {
    fn new(_initial_capacity: usize) -> Self {
        StringBuffer { data: Vec::new() }
    }

    fn append(&mut self, s: &str) {
        self.data.extend_from_slice(s.as_bytes());
    }

    fn as_cstr(&self) -> &[u8] {
        &self.data
    }

    fn clear(&mut self) {
        self.data.clear();
    }
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
                // C integer division truncates toward zero, same as Rust
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
    let mut log_buffer = StringBuffer::new(32);
    let mut result: c_int = 0;

    log_buffer.clear();

    log_buffer.append(&format!("Starting computation with {} parameters\n", 4));

    let op1 = get_operation_name(param1 % 4);
    log_buffer.append(&format!("Operation 1: {}({}, {})\n", op1, param1, param2));

    let intermediate1 = perform_operation(param1, param2, op1);
    result = result.wrapping_add(intermediate1);

    let op2 = get_operation_name(param3 % 4);
    log_buffer.append(&format!("Operation 2: {}({}, {})\n", op2, param3, param4));

    let intermediate2 = perform_operation(param3, param4, op2);
    result = result.wrapping_add(intermediate2);

    let op3 = "multiply";
    log_buffer.append(&format!(
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

    log_buffer.append(&format!("Final result: {}\n", result));

    // Match C printf("Computation Log:\n%s\n", log_buffer->data);
    let log_str = std::str::from_utf8(log_buffer.as_cstr()).unwrap_or("");
    print!("Computation Log:\n{}\n", log_str);

    result
}
