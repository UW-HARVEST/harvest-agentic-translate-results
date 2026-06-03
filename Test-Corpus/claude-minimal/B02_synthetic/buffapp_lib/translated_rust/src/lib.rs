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

use std::os::raw::c_int;

struct StringBuffer {
    data: Vec<u8>,
    length: usize,
}

impl StringBuffer {
    fn new(initial_capacity: usize) -> Self {
        let mut data = Vec::with_capacity(initial_capacity);
        // Mimic C behavior of writing a null terminator at index 0.
        data.push(0u8);
        StringBuffer { data, length: 0 }
    }

    fn append(&mut self, s: &str) -> i32 {
        let bytes = s.as_bytes();
        let str_len = bytes.len();
        let required_capacity = self.length + str_len + 1;

        if required_capacity > self.data.capacity() {
            let new_capacity = required_capacity * 2;
            // Mimic realloc; reserve enough capacity.
            self.data.reserve(new_capacity - self.data.len());
        }

        // Ensure the vector is at least `length` bytes long, then append the
        // bytes followed by a null terminator (matching strcpy semantics).
        self.data.truncate(self.length);
        self.data.extend_from_slice(bytes);
        self.data.push(0u8);
        self.length += str_len;

        0
    }

    fn as_str(&self) -> &str {
        // Convert the buffer's logical contents (without the trailing NUL)
        // to a UTF-8 string slice. We constructed all writes from valid
        // UTF-8 strings, so this is safe.
        std::str::from_utf8(&self.data[..self.length]).unwrap_or("")
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
        "add" => a.wrapping_add(b),
        "subtract" => a.wrapping_sub(b),
        "multiply" => a.wrapping_mul(b),
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

/// Public Rust entry point matching the C `buffapp` signature.
pub fn buffapp_rs(param1: i32, param2: i32, param3: i32, param4: i32) -> i32 {
    let mut log_buffer = StringBuffer::new(32);
    let mut result: i32 = 0;

    log_buffer.length = 0;

    let temp = format!("Starting computation with {} parameters\n", 4);
    log_buffer.append(&temp);

    // Match C behavior of `param1 % 4` (truncated division remainder).
    let op1 = get_operation_name(param1 % 4);
    let temp = format!("Operation 1: {}({}, {})\n", op1, param1, param2);
    log_buffer.append(&temp);

    let intermediate1 = perform_operation(param1, param2, op1);
    result = result.wrapping_add(intermediate1);

    let op2 = get_operation_name(param3 % 4);
    let temp = format!("Operation 2: {}({}, {})\n", op2, param3, param4);
    log_buffer.append(&temp);

    let intermediate2 = perform_operation(param3, param4, op2);
    result = result.wrapping_add(intermediate2);

    let op3 = "multiply";
    let temp = format!(
        "Operation 3: {}({}, {})\n",
        op3, intermediate1, intermediate2
    );
    log_buffer.append(&temp);

    let intermediate3 = perform_operation(intermediate1, intermediate2, op3);

    if intermediate3 != 0 {
        result /= intermediate3;
    } else {
        result = param1
            .wrapping_add(param2)
            .wrapping_add(param3)
            .wrapping_add(param4);
    }

    let temp = format!("Final result: {}\n", result);
    log_buffer.append(&temp);

    println!("Computation Log:\n{}\n", log_buffer.as_str());

    result
}

/// C-callable entry point with the same signature as the original C function.
#[no_mangle]
pub extern "C" fn buffapp(a: c_int, b: c_int, c: c_int, d: c_int) -> c_int {
    buffapp_rs(a as i32, b as i32, c as i32, d as i32) as c_int
}
