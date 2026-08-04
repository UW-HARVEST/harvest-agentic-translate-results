use std::ffi::{CStr, c_char, c_int};
use std::io::Write;

struct StringBuffer {
    data: Vec<u8>,
    capacity: usize,
    length: usize,
}

impl StringBuffer {
    fn new(initial_capacity: usize) -> Option<Self> {
        let mut data = Vec::with_capacity(initial_capacity);
        data.push(0);
        Some(StringBuffer {
            data,
            capacity: initial_capacity,
            length: 0,
        })
    }

    fn append(&mut self, str: &str) -> Result<(), ()> {
        let str_len = str.len();
        let required_capacity = self.length + str_len + 1;

        if required_capacity > self.capacity {
            let new_capacity = required_capacity * 2;
            self.data.reserve(new_capacity - self.data.capacity());
            self.capacity = new_capacity;
        }

        self.data.truncate(self.length);
        self.data.extend_from_slice(str.as_bytes());
        self.data.push(0);
        self.length += str_len;

        Ok(())
    }

    fn as_c_str(&self) -> *const c_char {
        self.data.as_ptr() as *const c_char
    }
}

fn get_operation_name(op_code: i32) -> &'static str {
    match op_code {
        0 => "add",
        1 => "subtract",
        2 => "multiply",
        3 => "divide",
        _ => "unknown",
    }
}

fn perform_operation(a: i32, b: i32, operation: &str) -> i32 {
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
    let mut log_buffer = StringBuffer::new(32).unwrap();
    let mut result: i32 = 0;

    let temp = format!("Starting computation with {} parameters\n", 4);
    let _ = log_buffer.append(&temp);

    let op1 = get_operation_name(param1 % 4);
    let temp = format!("Operation 1: {}({}, {})\n", op1, param1, param2);
    let _ = log_buffer.append(&temp);

    let intermediate1 = perform_operation(param1, param2, op1);
    result += intermediate1;

    let op2 = get_operation_name(param3 % 4);
    let temp = format!("Operation 2: {}({}, {})\n", op2, param3, param4);
    let _ = log_buffer.append(&temp);

    let intermediate2 = perform_operation(param3, param4, op2);
    result += intermediate2;

    let op3 = "multiply";
    let temp = format!("Operation 3: {}({}, {})\n", op3, intermediate1, intermediate2);
    let _ = log_buffer.append(&temp);

    let intermediate3 = perform_operation(intermediate1, intermediate2, op3);

    if intermediate3 != 0 {
        result = result / intermediate3;
    } else {
        result = param1 + param2 + param3 + param4;
    }

    let temp = format!("Final result: {}\n", result);
    let _ = log_buffer.append(&temp);

    unsafe {
        let c_str = CStr::from_ptr(log_buffer.as_c_str());
        let _ = std::io::stdout().write_all(b"Computation Log:\n");
        let _ = std::io::stdout().write_all(c_str.to_bytes());
        let _ = std::io::stdout().write_all(b"\n");
    }

    result
}
