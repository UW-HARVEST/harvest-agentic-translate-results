use std::os::raw::c_int;

const READ_PERM: c_int = 0o400;
const WRITE_PERM: c_int = 0o200;
const EXEC_PERM: c_int = 0o100;

struct ResultTracker {
    value: c_int,
    operation: String,
    permissions: c_int,
}

fn create_result_string(op: &str, val: c_int) -> String {
    format!("Operation: {}, Value: {}", op, val)
}

fn check_permissions(perms: c_int, required: c_int) -> bool {
    (perms & required) == required
}

fn safe_add(a: c_int, b: c_int, perms: c_int) -> c_int {
    if !check_permissions(perms, READ_PERM | WRITE_PERM) {
        println!("Insufficient permissions for addition");
        return 0;
    }
    a + b
}

fn multiply_with_log(a: c_int, b: c_int) -> (c_int, Option<String>) {
    let result = a * b;
    (result, Some(create_result_string("multiply", result)))
}

fn copy_and_sum(src: &[c_int]) -> c_int {
    let dest = src.to_vec();
    let mut sum = 0;
    for value in dest {
        sum += value;
    }
    sum
}

fn compare_operations(op1: Option<&str>, op2: Option<&str>) -> c_int {
    match (op1, op2) {
        (Some(a), Some(b)) => match a.cmp(b) {
            std::cmp::Ordering::Less => -1,
            std::cmp::Ordering::Equal => 0,
            std::cmp::Ordering::Greater => 1,
        },
        _ => {
            println!("One or both operation strings are NULL");
            -1
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn complexmode(mode: c_int, value1: c_int, value2: c_int, value3: c_int) -> c_int {
    let mut result = 0;
    let mut log_message: Option<String> = None;

    let permissions = 0o644;

    let mut res_tracker = ResultTracker {
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
            let (mul_result, msg) = multiply_with_log(value1, value2);
            result = mul_result;
            log_message = msg;
            res_tracker.value = result;

            match log_message {
                Some(ref msg) if !msg.is_empty() => {
                    println!("Mode 2: {}", msg);
                }
                _ => {
                    println!("Log message creation failed");
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

            if check_permissions(permissions, EXEC_PERM) {
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

    if compare_operations(Some(&res_tracker.operation), Some("none")) != 0 {
        println!("Operation performed: {}", res_tracker.operation);
    }

    let _ = res_tracker.permissions;

    result
}
