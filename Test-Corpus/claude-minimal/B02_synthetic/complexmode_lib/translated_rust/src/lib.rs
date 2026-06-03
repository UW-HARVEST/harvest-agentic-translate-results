// Translated from c_src/src/lib.c
// Original copyright: 2025 MIT Lincoln Laboratory

const READ_PERM: i32 = 0o400;
const WRITE_PERM: i32 = 0o200;
#[allow(dead_code)]
const EXEC_PERM: i32 = 0o100;

struct Result {
    value: i32,
    operation: String,
    #[allow(dead_code)]
    permissions: i32,
}

fn create_result_string(op: &str, val: i32) -> Option<String> {
    Some(format!("Operation: {}, Value: {}", op, val))
}

fn check_permissions(perms: i32, required: i32) -> bool {
    (perms & required) == required
}

fn safe_add(a: i32, b: i32, perms: i32) -> i32 {
    if !check_permissions(perms, READ_PERM | WRITE_PERM) {
        println!("Insufficient permissions for addition");
        return 0;
    }
    a + b
}

fn multiply_with_log(a: i32, b: i32, log_msg: &mut Option<String>) -> i32 {
    *log_msg = create_result_string("multiply", a * b);
    if log_msg.is_none() {
        return 0;
    }
    a * b
}

fn copy_and_sum(src: Option<&[i32]>) -> i32 {
    let src = match src {
        Some(s) => s,
        None => {
            println!("Source pointer is NULL");
            return -1;
        }
    };

    // Mirror the C code's allocation + copy behavior with a Vec
    let dest: Vec<i32> = src.to_vec();

    let mut sum: i32 = 0;
    for v in dest.iter() {
        sum += *v;
    }

    sum
}

#[allow(dead_code)]
fn compare_operations(op1: Option<&str>, op2: Option<&str>) -> i32 {
    match (op1, op2) {
        (Some(a), Some(b)) => {
            if a == b {
                0
            } else if a < b {
                -1
            } else {
                1
            }
        }
        _ => {
            println!("One or both operation strings are NULL");
            -1
        }
    }
}

pub fn complexmode(mode: i32, value1: i32, value2: i32, value3: i32) -> i32 {
    let result: i32;
    let mut log_message: Option<String> = None;

    let permissions: i32 = 0o644; // rw-r--r--

    let mut res_tracker = Result {
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
                Some(msg) if msg.is_empty() => {
                    println!("Log message creation failed");
                }
                Some(msg) => {
                    println!("Mode 2: {}", msg);
                }
            }
        }
        3 => {
            res_tracker.operation = String::from("array_sum");
            let values: [i32; 3] = [value1, value2, value3];
            result = copy_and_sum(Some(&values));
            res_tracker.value = result;

            println!("Mode 3: Array Sum");
            println!("Result: {}", result);
        }
        4 => {
            res_tracker.operation = String::from("complex");

            if check_permissions(permissions, 0o100) {
                result = (value1 * value2) + value3;
            } else {
                result = value1 + value2 + value3;
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

#[no_mangle]
pub extern "C" fn complexmode_c(a: i32, b: i32, c: i32, d: i32) -> i32 {
    complexmode(a, b, c, d)
}
