use std::ffi::c_int;

const READ_PERM: c_int = 0o400;
const WRITE_PERM: c_int = 0o200;

fn check_permissions(perms: c_int, required: c_int) -> bool {
    (perms & required) == required
}

fn safe_add(a: c_int, b: c_int, perms: c_int) -> c_int {
    if !check_permissions(perms, READ_PERM | WRITE_PERM) {
        print!("Insufficient permissions for addition\n");
        return 0;
    }
    a + b
}

fn multiply_with_log(a: c_int, b: c_int) -> (c_int, Option<String>) {
    let product = a * b;
    let msg = format!("Operation: multiply, Value: {}", product);
    (product, Some(msg))
}

fn copy_and_sum(src: &[c_int]) -> c_int {
    let mut sum: c_int = 0;
    for &v in src {
        sum += v;
    }
    sum
}

#[unsafe(no_mangle)]
pub extern "C" fn complexmode(mode: c_int, value1: c_int, value2: c_int, value3: c_int) -> c_int {
    let result: c_int;
    let permissions: c_int = 0o644;
    let mut operation = "none";

    match mode {
        1 => {
            operation = "addition";
            result = safe_add(value1, value2, permissions);
            print!("Mode 1: Addition\n");
            print!("Result: {}\n", result);
        }
        2 => {
            operation = "multiplication";
            let (prod, log_message) = multiply_with_log(value1, value2);
            result = prod;
            match log_message {
                None => {
                    print!("Log message creation failed\n");
                }
                Some(ref msg) if msg.is_empty() => {
                    print!("Log message creation failed\n");
                }
                Some(ref msg) => {
                    print!("Mode 2: {}\n", msg);
                }
            }
        }
        3 => {
            operation = "array_sum";
            let values = [value1, value2, value3];
            result = copy_and_sum(&values);
            print!("Mode 3: Array Sum\n");
            print!("Result: {}\n", result);
        }
        4 => {
            operation = "complex";
            if check_permissions(permissions, 0o100) {
                result = (value1 * value2) + value3;
            } else {
                result = value1 + value2 + value3;
            }
            print!("Mode 4: Complex Calculation\n");
            print!("Result: {}\n", result);
        }
        _ => {
            print!("Invalid mode\n");
            result = -1;
        }
    }

    if operation != "none" {
        print!("Operation performed: {}\n", operation);
    }

    result
}
