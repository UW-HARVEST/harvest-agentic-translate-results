use std::ffi::{c_char, c_int, CStr, CString};
use std::os::raw::c_void;
use std::ptr;
use std::sync::atomic::{AtomicI32, Ordering};

static COUNTER: AtomicI32 = AtomicI32::new(0);

type OperationFunc = Option<unsafe extern "C" fn(c_int) -> c_int>;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn increment_counter(value: c_int) -> c_int {
    COUNTER.fetch_add(value, Ordering::SeqCst) + value
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn decrement_counter(value: c_int) -> c_int {
    COUNTER.fetch_sub(value, Ordering::SeqCst) - value
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn multiply_counter(value: c_int) -> c_int {
    let old = COUNTER.load(Ordering::SeqCst);
    let new = old * value;
    COUNTER.store(new, Ordering::SeqCst);
    new
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn reset_counter(value: c_int) -> c_int {
    COUNTER.store(value, Ordering::SeqCst);
    value
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn is_string_empty(str: *const c_char) -> c_int {
    if str.is_null() {
        return 1;
    }
    if *str != 0 {
        return 0;
    }
    1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn find_char_in_buffer(buffer: *const c_char, size: usize, target: c_char) -> *mut c_char {
    if buffer.is_null() {
        return ptr::null_mut();
    }
    let slice = std::slice::from_raw_parts(buffer as *const u8, size);
    match slice.iter().position(|&b| b as i8 == target) {
        Some(pos) => buffer.add(pos) as *mut c_char,
        None => ptr::null_mut(),
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn create_buffer(initial: *const c_char) -> *mut c_char {
    if initial.is_null() {
        return ptr::null_mut();
    }
    let c_str = match CStr::from_ptr(initial).to_str() {
        Ok(s) => s,
        Err(_) => return ptr::null_mut(),
    };
    match CString::new(c_str) {
        Ok(cstring) => cstring.into_raw(),
        Err(_) => ptr::null_mut(),
    }
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
pub unsafe extern "C" fn apply_operation(op: OperationFunc, value: c_int) -> c_int {
    match op {
        Some(func) => func(value),
        None => -1,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn charinbuf(mode: c_int, value: c_int, opt1: c_int, opt2: c_int) -> c_int {
    let mut result: c_int = 0;
    let mut buffer: *mut c_char = ptr::null_mut();
    let mut found_pos: *mut c_char = ptr::null_mut();
    let test_string: *const c_char = b"\0".as_ptr() as *const c_char;
    let non_empty_string: *const c_char = b"Hello, World!\0".as_ptr() as *const c_char;
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

            if is_string_empty(test_string) != 0 {
                println!("Test string is empty (checked with *string)");
                result = 0;
            } else {
                println!("Test string is not empty");
                result = 1;
            }

            if is_string_empty(non_empty_string) != 0 {
                println!("Non-empty string check failed!");
            } else {
                println!("Non-empty string correctly identified");
                result += 10;
            }
        }

        2 => {
            println!("Mode 2: Dynamic memory allocation and free");

            let initial = b"Testing malloc and free\0".as_ptr() as *const c_char;
            buffer = create_buffer(initial);

            if !buffer.is_null() {
                let c_str = CStr::from_ptr(buffer);
                let len = c_str.to_bytes().len();
                println!("Buffer allocated: '{}'", c_str.to_str().unwrap_or("<invalid utf8>"));
                println!("Buffer length: {}", len);
                result = len as c_int;

                let _ = CString::from_raw(buffer);
                println!("Buffer freed successfully");
                buffer = ptr::null_mut();
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

            let initial = b"Search for character X in this buffer\0".as_ptr() as *const c_char;
            buffer = create_buffer(initial);

            if !buffer.is_null() {
                let c_str = CStr::from_ptr(buffer);
                let buf_size = c_str.to_bytes().len();
                let search_char: c_char = 'X' as c_char;

                println!("Searching for '{}' in: '{}'", 'X', c_str.to_str().unwrap_or("<invalid utf8>"));
                found_pos = find_char_in_buffer(buffer, buf_size, search_char);

                if !found_pos.is_null() {
                    result = found_pos.offset_from(buffer) as c_int;
                    println!("Found '{}' at position: {}", 'X', result);
                } else {
                    println!("Character '{}' not found", 'X');
                    result = -1;
                }

                let _ = CString::from_raw(buffer);
                buffer = ptr::null_mut();
            }
        }

        _ => {
            println!("Invalid mode: {}", mode);
            result = -1;
        }
    }

    result
}
