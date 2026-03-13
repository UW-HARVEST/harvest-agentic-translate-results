use std::os::raw::c_int;

static mut COUNTER: c_int = 0;

type OperationFunc = fn(c_int) -> c_int;

fn increment_counter(value: c_int) -> c_int {
    unsafe {
        COUNTER += value;
        COUNTER
    }
}

fn decrement_counter(value: c_int) -> c_int {
    unsafe {
        COUNTER -= value;
        COUNTER
    }
}

fn multiply_counter(value: c_int) -> c_int {
    unsafe {
        COUNTER *= value;
        COUNTER
    }
}

fn reset_counter(value: c_int) -> c_int {
    unsafe {
        COUNTER = value;
        COUNTER
    }
}

fn is_string_empty(s: Option<&[u8]>) -> c_int {
    match s {
        None => 1,
        Some(b) => {
            if !b.is_empty() && b[0] != 0 {
                0
            } else {
                1
            }
        }
    }
}

fn find_char_in_buffer(buffer: &[u8], target: u8) -> Option<usize> {
    buffer.iter().position(|&b| b == target)
}

fn validate_uint16_range(value: c_int) -> c_int {
    if value < 0 {
        return 0;
    }
    if value > u16::MAX as c_int {
        return 0;
    }
    1
}

fn apply_operation(op: Option<OperationFunc>, value: c_int) -> c_int {
    match op {
        None => -1,
        Some(f) => f(value),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn charinbuf(mode: c_int, value: c_int, opt1: c_int, opt2: c_int) -> c_int {
    let mut result: c_int = 0;
    let test_string = b"";
    let non_empty_string = b"Hello, World!";

    unsafe {
        COUNTER = 0;
    }

    match mode {
        0 => {
            print!("Mode 0: UINT16_MAX validation\n");
            print!("Checking if value {} is within uint16_t range...\n", value);

            if validate_uint16_range(value) != 0 {
                print!("Value {} is valid (0 <= value <= {})\n", value, u16::MAX);
                result = value;
            } else {
                print!("Value {} is out of range for uint16_t\n", value);
                result = -1;
            }

            print!("UINT16_MAX constant value: {}\n", u16::MAX);
        }
        1 => {
            print!("Mode 1: String empty check by dereference\n");

            if is_string_empty(Some(test_string)) != 0 {
                print!("Test string is empty (checked with *string)\n");
                result = 0;
            } else {
                print!("Test string is not empty\n");
                result = 1;
            }

            if is_string_empty(Some(non_empty_string)) != 0 {
                print!("Non-empty string check failed!\n");
            } else {
                print!("Non-empty string correctly identified\n");
                result += 10;
            }
        }
        2 => {
            print!("Mode 2: Dynamic memory allocation and free\n");

            let initial = b"Testing malloc and free";
            let buffer = initial.to_vec();

            print!("Buffer allocated: '{}'\n", std::str::from_utf8(&buffer).unwrap());
            print!("Buffer length: {}\n", buffer.len());
            result = buffer.len() as c_int;

            print!("Buffer freed successfully\n");
        }
        3 => {
            print!("Mode 3: Function pointers with static counter\n");

            let current_op: Option<OperationFunc> = Some(reset_counter);
            result = apply_operation(current_op, value);
            print!("Counter reset to: {}\n", result);

            let current_op: Option<OperationFunc> = Some(increment_counter);
            result = apply_operation(current_op, opt1);
            print!("Counter after increment by {}: {}\n", opt1, result);

            let current_op: Option<OperationFunc> = Some(multiply_counter);
            result = apply_operation(current_op, opt2);
            print!("Counter after multiply by {}: {}\n", opt2, result);

            let current_op: Option<OperationFunc> = Some(decrement_counter);
            result = apply_operation(current_op, 5);
            print!("Counter after decrement by 5: {}\n", result);

            unsafe {
                print!("Final static counter value: {}\n", COUNTER);
            }
        }
        4 => {
            print!("Mode 4: Using memchr to find character\n");

            let buffer = b"Search for character X in this buffer";
            let search_char = b'X';

            print!(
                "Searching for '{}' in: '{}'\n",
                search_char as char,
                std::str::from_utf8(buffer).unwrap()
            );

            match find_char_in_buffer(buffer, search_char) {
                Some(pos) => {
                    result = pos as c_int;
                    print!("Found '{}' at position: {}\n", search_char as char, result);
                }
                None => {
                    print!("Character '{}' not found\n", search_char as char);
                    result = -1;
                }
            }
        }
        _ => {
            print!("Invalid mode: {}\n", mode);
            result = -1;
        }
    }

    result
}
