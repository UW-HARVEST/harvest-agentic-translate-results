use std::os::raw::c_int;

const READ_PERM: c_int = 0o400;
const WRITE_PERM: c_int = 0o200;
#[allow(dead_code)]
const EXEC_PERM: c_int = 0o100;

struct ResultTracker {
    #[allow(dead_code)]
    value: c_int,
    operation: String,
    #[allow(dead_code)]
    permissions: c_int,
}

#[allow(dead_code)]
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
    a.wrapping_add(b)
}

fn multiply_with_log(a: c_int, b: c_int) -> (c_int, String) {
    let val = a.wrapping_mul(b);
    let log_msg = create_result_string("multiply", val);
    (val, log_msg)
}

fn copy_and_sum(src: &[c_int]) -> c_int {
    let dest = src.to_vec();
    let mut sum = 0;
    for &val in &dest {
        sum = sum.wrapping_add(val);
    }
    sum
}

#[allow(dead_code)]
fn compare_operations(op1: Option<&str>, op2: Option<&str>) -> c_int {
    match (op1, op2) {
        (Some(s1), Some(s2)) => {
            use std::cmp::Ordering;
            match s1.cmp(s2) {
                Ordering::Less => -1,
                Ordering::Equal => 0,
                Ordering::Greater => 1,
            }
        }
        _ => {
            println!("One or both operation strings are NULL");
            -1
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn complexmode(mode: c_int, value1: c_int, value2: c_int, value3: c_int) -> c_int {
    let mut result = 0;
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
            let (res, log_message) = multiply_with_log(value1, value2);
            result = res;
            res_tracker.value = result;

            if log_message.is_empty() {
                println!("Log message creation failed");
            } else {
                println!("Mode 2: {}", log_message);
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
                result = value1.wrapping_add(value2).wrapping_add(value3);
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
