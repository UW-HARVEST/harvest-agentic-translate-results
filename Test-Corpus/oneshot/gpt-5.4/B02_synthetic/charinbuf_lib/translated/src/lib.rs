use std::os::raw::c_int;
use std::sync::atomic::{AtomicI32, Ordering};

static COUNTER: AtomicI32 = AtomicI32::new(0);

type OperationFunc = extern "C" fn(c_int) -> c_int;

#[unsafe(no_mangle)]
pub extern "C" fn increment_counter(value: c_int) -> c_int {
    COUNTER.fetch_add(value, Ordering::SeqCst) + value
}

#[unsafe(no_mangle)]
pub extern "C" fn decrement_counter(value: c_int) -> c_int {
    COUNTER.fetch_sub(value, Ordering::SeqCst) - value
}

#[unsafe(no_mangle)]
pub extern "C" fn multiply_counter(value: c_int) -> c_int {
    let result = COUNTER.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |current| {
        Some(current.wrapping_mul(value))
    });
    match result {
        Ok(previous) => previous.wrapping_mul(value),
        Err(current) => current,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn reset_counter(value: c_int) -> c_int {
    COUNTER.store(value, Ordering::SeqCst);
    value
}

#[unsafe(no_mangle)]
pub extern "C" fn is_string_empty(str_ptr: *const i8) -> c_int {
    if str_ptr.is_null() {
        return 1;
    }
    let first = unsafe { *str_ptr };
    if first != 0 { 0 } else { 1 }
}

#[unsafe(no_mangle)]
pub extern "C" fn find_char_in_buffer(buffer: *const i8, size: usize, target: i8) -> *mut i8 {
    if buffer.is_null() {
        return std::ptr::null_mut();
    }
    let slice = unsafe { std::slice::from_raw_parts(buffer as *const u8, size) };
    match slice.iter().position(|&b| b == target as u8) {
        Some(pos) => unsafe { buffer.add(pos) as *mut i8 },
        None => std::ptr::null_mut(),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn create_buffer(initial: *const i8) -> *mut i8 {
    initial as *mut i8
}

#[unsafe(no_mangle)]
pub extern "C" fn validate_uint16_range(value: c_int) -> c_int {
    if value < 0 {
        return 0;
    }
    if value > u16::MAX as c_int {
        return 0;
    }
    1
}

#[unsafe(no_mangle)]
pub extern "C" fn apply_operation(op: Option<OperationFunc>, value: c_int) -> c_int {
    match op {
        Some(f) => f(value),
        None => -1,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn charinbuf(mode: c_int, value: c_int, opt1: c_int, opt2: c_int) -> c_int {
    let mut result = 0;
    let test_string = b"\0";
    let non_empty_string = b"Hello, World!\0";

    COUNTER.store(0, Ordering::SeqCst);

    match mode {
        0 => {
            println!("Mode 0: UINT16_MAX validation");
            println!("Checking if value {} is within uint16_t range...", value);

            if validate_uint16_range(value) != 0 {
                println!("Value {} is valid (0 <= value <= {})", value, u16::MAX);
                result = value;
            } else {
                println!("Value {} is out of range for uint16_t", value);
                result = -1;
            }

            println!("UINT16_MAX constant value: {}", u16::MAX);
        }
        1 => {
            println!("Mode 1: String empty check by dereference");

            if is_string_empty(test_string.as_ptr() as *const i8) != 0 {
                println!("Test string is empty (checked with *string)");
                result = 0;
            } else {
                println!("Test string is not empty");
                result = 1;
            }

            if is_string_empty(non_empty_string.as_ptr() as *const i8) != 0 {
                println!("Non-empty string check failed!");
            } else {
                println!("Non-empty string correctly identified");
                result += 10;
            }
        }
        2 => {
            println!("Mode 2: Dynamic memory allocation and free");

            let buffer = "Testing malloc and free";

            println!("Buffer allocated: '{}'", buffer);
            println!("Buffer length: {}", buffer.len());
            result = buffer.len() as c_int;

            println!("Buffer freed successfully");
        }
        3 => {
            println!("Mode 3: Function pointers with static counter");

            let mut current_op: Option<OperationFunc> = Some(reset_counter);
            result = apply_operation(current_op, value);
            println!("Counter reset to: {}", result);

            current_op = Some(increment_counter);
            result = apply_operation(current_op, opt1);
            println!("Counter after increment by {}: {}", opt1, result);

            current_op = Some(multiply_counter);
            result = apply_operation(current_op, opt2);
            println!("Counter after multiply by {}: {}", opt2, result);

            current_op = Some(decrement_counter);
            result = apply_operation(current_op, 5);
            println!("Counter after decrement by 5: {}", result);

            println!("Final static counter value: {}", COUNTER.load(Ordering::SeqCst));
        }
        4 => {
            println!("Mode 4: Using memchr to find character");

            let buffer = "Search for character X in this buffer";
            let search_char = 'X';

            println!("Searching for '{}' in: '{}'", search_char, buffer);

            match buffer.as_bytes().iter().position(|&b| b == search_char as u8) {
                Some(pos) => {
                    result = pos as c_int;
                    println!("Found '{}' at position: {}", search_char, result);
                }
                None => {
                    println!("Character '{}' not found", search_char);
                    result = -1;
                }
            }
        }
        _ => {
            println!("Invalid mode: {}", mode);
            result = -1;
        }
    }

    result
}
