use std::ffi::CStr;
use std::os::raw::{c_char, c_int};
use std::sync::atomic::{AtomicI32, Ordering};

static COUNTER: AtomicI32 = AtomicI32::new(0);

type OperationFunc = Option<fn(c_int) -> c_int>;

fn increment_counter(value: c_int) -> c_int {
    COUNTER.fetch_add(value, Ordering::SeqCst) + value
}

fn decrement_counter(value: c_int) -> c_int {
    COUNTER.fetch_sub(value, Ordering::SeqCst) - value
}

fn multiply_counter(value: c_int) -> c_int {
    let mut current = COUNTER.load(Ordering::SeqCst);
    loop {
        let new = current * value;
        match COUNTER.compare_exchange_weak(current, new, Ordering::SeqCst, Ordering::SeqCst) {
            Ok(_) => return new,
            Err(v) => current = v,
        }
    }
}

fn reset_counter(value: c_int) -> c_int {
    COUNTER.store(value, Ordering::SeqCst);
    value
}

fn is_string_empty(str: *const c_char) -> c_int {
    if str.is_null() {
        return 1;
    }
    unsafe {
        if *str != 0 {
            return 0;
        }
    }
    1
}

fn find_char_in_buffer(buffer: *const c_char, size: usize, target: c_char) -> *mut c_char {
    if buffer.is_null() {
        return std::ptr::null_mut();
    }
    let slice = unsafe { std::slice::from_raw_parts(buffer as *const u8, size) };
    if let Some(pos) = slice.iter().position(|&c| c == target as u8) {
        unsafe { buffer.add(pos) as *mut c_char }
    } else {
        std::ptr::null_mut()
    }
}

fn create_buffer(initial: *const c_char) -> *mut c_char {
    if initial.is_null() {
        return std::ptr::null_mut();
    }
    unsafe {
        let c_str = CStr::from_ptr(initial);
        let c_string = std::ffi::CString::from(c_str);
        c_string.into_raw()
    }
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

fn apply_operation(op: OperationFunc, value: c_int) -> c_int {
    if let Some(f) = op {
        f(value)
    } else {
        -1
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn charinbuf(mode: c_int, value: c_int, opt1: c_int, opt2: c_int) -> c_int {
    let mut result = 0;
    let mut buffer: *mut c_char = std::ptr::null_mut();
    let mut found_pos: *mut c_char = std::ptr::null_mut();
    let test_string = c"";
    let non_empty_string = c"Hello, World!";

    let mut current_op: OperationFunc = None;

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

            if is_string_empty(test_string.as_ptr()) != 0 {
                println!("Test string is empty (checked with *string)");
                result = 0;
            } else {
                println!("Test string is not empty");
                result = 1;
            }

            if is_string_empty(non_empty_string.as_ptr()) != 0 {
                println!("Non-empty string check failed!");
            } else {
                println!("Non-empty string correctly identified");
                result += 10;
            }
        }
        2 => {
            println!("Mode 2: Dynamic memory allocation and free");

            buffer = create_buffer(c"Testing malloc and free".as_ptr());

            if !buffer.is_null() {
                unsafe {
                    let c_str = CStr::from_ptr(buffer);
                    println!("Buffer allocated: '{}'", c_str.to_string_lossy());
                    let len = c_str.to_bytes().len();
                    println!("Buffer length: {}", len);
                    result = len as c_int;

                    let _ = std::ffi::CString::from_raw(buffer);
                }
                println!("Buffer freed successfully");
                buffer = std::ptr::null_mut();
            } else {
                println!("Failed to allocate buffer");
                result = -1;
            }
        }
        3 => {
            println!("Mode 3: Function pointers with static counter");

            current_op = Some(reset_counter);
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

            buffer = create_buffer(c"Search for character X in this buffer".as_ptr());

            if !buffer.is_null() {
                unsafe {
                    let c_str = CStr::from_ptr(buffer);
                    let buf_size = c_str.to_bytes().len();
                    let search_char = b'X' as c_char;

                    println!("Searching for '{}' in: '{}'", search_char as u8 as char, c_str.to_string_lossy());
                    found_pos = find_char_in_buffer(buffer, buf_size, search_char);

                    if !found_pos.is_null() {
                        result = found_pos.offset_from(buffer) as c_int;
                        println!("Found '{}' at position: {}", search_char as u8 as char, result);
                    } else {
                        println!("Character '{}' not found", search_char as u8 as char);
                        result = -1;
                    }

                    let _ = std::ffi::CString::from_raw(buffer);
                }
                buffer = std::ptr::null_mut();
            }
        }
        _ => {
            println!("Invalid mode: {}", mode);
            result = -1;
        }
    }

    result
}
