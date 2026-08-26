use std::io::{self, Read, Write};

const READ_PERM: i32 = 0o400;
const WRITE_PERM: i32 = 0o200;
#[allow(dead_code)]
const EXEC_PERM: i32 = 0o100;

struct Result_ {
    value: i32,
    operation: String,
    #[allow(dead_code)]
    permissions: i32,
}

fn create_result_string(op: &str, val: i32) -> Option<String> {
    // mimic snprintf with 64 byte buffer, truncating to fit (including NUL)
    let full = format!("Operation: {}, Value: {}", op, val);
    let bytes = full.as_bytes();
    // 64 byte buffer means up to 63 chars + NUL
    let max_len = 63;
    if bytes.len() <= max_len {
        Some(full)
    } else {
        // Truncate at byte level (safe since input here is ASCII)
        Some(String::from_utf8_lossy(&bytes[..max_len]).into_owned())
    }
}

fn check_permissions(perms: i32, required: i32) -> bool {
    (perms & required) == required
}

fn safe_add(a: i32, b: i32, perms: i32) -> i32 {
    if !check_permissions(perms, READ_PERM | WRITE_PERM) {
        println!("Insufficient permissions for addition");
        return 0;
    }
    a.wrapping_add(b)
}

fn multiply_with_log(a: i32, b: i32, log_msg: &mut Option<String>) -> i32 {
    let product = a.wrapping_mul(b);
    *log_msg = create_result_string("multiply", product);
    if log_msg.is_none() {
        return 0;
    }
    product
}

fn copy_and_sum(src: &[i32]) -> i32 {
    // mimic original: copy, sum all elements
    let dest: Vec<i32> = src.to_vec();
    let mut sum: i32 = 0;
    for &v in &dest {
        sum = sum.wrapping_add(v);
    }
    sum
}

fn complexmode(mode: i32, value1: i32, value2: i32, value3: i32) -> i32 {
    let result: i32;
    let mut log_message: Option<String> = None;

    let permissions: i32 = 0o644; // rw-r--r--

    let mut res_tracker = Result_ {
        value: 0,
        operation: String::from("none"),
        permissions,
    };

    match mode {
        1 => {
            res_tracker.operation = String::from("addition");
            result = safe_add(value1, value2, permissions);
            res_tracker.value = result;

            println!("Mode 1: Addition");
            println!("Result: {}", result);
        }
        2 => {
            res_tracker.operation = String::from("multiplication");
            result = multiply_with_log(value1, value2, &mut log_message);
            res_tracker.value = result;

            match &log_message {
                None => {
                    println!("Log message creation failed");
                }
                Some(s) if s.is_empty() => {
                    println!("Log message creation failed");
                }
                Some(s) => {
                    println!("Mode 2: {}", s);
                }
            }
        }
        3 => {
            res_tracker.operation = String::from("array_sum");
            let values = [value1, value2, value3];
            result = copy_and_sum(&values);
            res_tracker.value = result;

            println!("Mode 3: Array Sum");
            println!("Result: {}", result);
        }
        4 => {
            res_tracker.operation = String::from("complex");

            if check_permissions(permissions, 0o100) {
                result = value1.wrapping_mul(value2).wrapping_add(value3);
            } else {
                result = value1
                    .wrapping_add(value2)
                    .wrapping_add(value3);
            }

            res_tracker.value = result;
            println!("Mode 4: Complex Calculation");
            println!("Result: {}", result);
        }
        _ => {
            println!("Invalid mode");
            result = -1;
        }
    }

    if res_tracker.operation != "none" {
        println!("Operation performed: {}", res_tracker.operation);
    }

    result
}

// Reads integers from stdin, mimicking scanf("%d", ...) behavior:
// skip leading whitespace (incl. newlines), parse a base-10 integer with optional sign.
struct ScanfReader {
    buf: Vec<u8>,
    pos: usize,
}

impl ScanfReader {
    fn new() -> Self {
        let mut buf = Vec::new();
        io::stdin().read_to_end(&mut buf).ok();
        ScanfReader { buf, pos: 0 }
    }

    fn read_int(&mut self) -> Option<i32> {
        // Skip whitespace
        while self.pos < self.buf.len()
            && (self.buf[self.pos] as char).is_ascii_whitespace()
        {
            self.pos += 1;
        }
        if self.pos >= self.buf.len() {
            return None;
        }
        let start = self.pos;
        if self.buf[self.pos] == b'+' || self.buf[self.pos] == b'-' {
            self.pos += 1;
        }
        let digits_start = self.pos;
        while self.pos < self.buf.len()
            && (self.buf[self.pos] as char).is_ascii_digit()
        {
            self.pos += 1;
        }
        if self.pos == digits_start {
            return None;
        }
        let s = std::str::from_utf8(&self.buf[start..self.pos]).ok()?;
        // scanf wraps on overflow technically undefined; use wrapping parse
        match s.parse::<i64>() {
            Ok(n) => Some(n as i32),
            Err(_) => None,
        }
    }
}

fn main() {
    let mut reader = ScanfReader::new();
    let mode = reader.read_int().unwrap_or(0);
    let v1 = reader.read_int().unwrap_or(0);
    let v2 = reader.read_int().unwrap_or(0);
    let v3 = reader.read_int().unwrap_or(0);

    let _ = complexmode(mode, v1, v2, v3);

    // Ensure stdout is flushed
    let _ = io::stdout().flush();
}
