use std::ffi::c_int;

static mut COUNTER: c_int = 0;

type OperationFunc = Option<extern "C" fn(c_int) -> c_int>;

#[unsafe(no_mangle)]
pub extern "C" fn increment_counter(value: c_int) -> c_int {
    unsafe {
        COUNTER += value;
        COUNTER
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn decrement_counter(value: c_int) -> c_int {
    unsafe {
        COUNTER -= value;
        COUNTER
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn multiply_counter(value: c_int) -> c_int {
    unsafe {
        COUNTER *= value;
        COUNTER
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn reset_counter(value: c_int) -> c_int {
    unsafe {
        COUNTER = value;
        COUNTER
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn is_string_empty(s: *const u8) -> c_int {
    if s.is_null() {
        return 1;
    }
    unsafe {
        if *s != 0 {
            return 0;
        }
    }
    1
}

#[unsafe(no_mangle)]
pub extern "C" fn find_char_in_buffer(buffer: *const u8, size: usize, target: u8) -> *mut u8 {
    if buffer.is_null() {
        return std::ptr::null_mut();
    }
    unsafe {
        let slice = std::slice::from_raw_parts(buffer, size);
        match slice.iter().position(|&b| b == target) {
            Some(pos) => buffer.add(pos) as *mut u8,
            None => std::ptr::null_mut(),
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn create_buffer(initial: *const u8) -> *mut u8 {
    if initial.is_null() {
        return std::ptr::null_mut();
    }
    unsafe {
        let len = libc_strlen(initial);
        let buffer = libc::malloc(len + 1) as *mut u8;
        if !buffer.is_null() {
            std::ptr::copy_nonoverlapping(initial, buffer, len + 1);
        }
        buffer
    }
}

unsafe fn libc_strlen(s: *const u8) -> usize {
    let mut len = 0;
    while *s.add(len) != 0 {
        len += 1;
    }
    len
}

#[unsafe(no_mangle)]
pub extern "C" fn validate_uint16_range(value: c_int) -> c_int {
    if value < 0 {
        return 0;
    }
    if value > 65535 {
        return 0;
    }
    1
}

#[unsafe(no_mangle)]
pub extern "C" fn apply_operation(op: OperationFunc, value: c_int) -> c_int {
    match op {
        None => -1,
        Some(f) => f(value),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn charinbuf(mode: c_int, value: c_int, opt1: c_int, opt2: c_int) -> c_int {
    let mut result: c_int = 0;
    let mut buffer: *mut u8 = std::ptr::null_mut();
    let test_string = b"\0";
    let non_empty_string = b"Hello, World!\0";

    let mut current_op: OperationFunc = None;

    unsafe {
        COUNTER = 0;
    }

    match mode {
        0 => {
            print!("Mode 0: UINT16_MAX validation\n");
            print!("Checking if value {} is within uint16_t range...\n", value);

            if validate_uint16_range(value) != 0 {
                print!("Value {} is valid (0 <= value <= {})\n", value, 65535u32);
                result = value;
            } else {
                print!("Value {} is out of range for uint16_t\n", value);
                result = -1;
            }

            print!("UINT16_MAX constant value: {}\n", 65535u32);
        }
        1 => {
            print!("Mode 1: String empty check by dereference\n");

            if is_string_empty(test_string.as_ptr()) != 0 {
                print!("Test string is empty (checked with *string)\n");
                result = 0;
            } else {
                print!("Test string is not empty\n");
                result = 1;
            }

            if is_string_empty(non_empty_string.as_ptr()) != 0 {
                print!("Non-empty string check failed!\n");
            } else {
                print!("Non-empty string correctly identified\n");
                result += 10;
            }
        }
        2 => {
            print!("Mode 2: Dynamic memory allocation and free\n");

            buffer = create_buffer(b"Testing malloc and free\0".as_ptr());

            if !buffer.is_null() {
                let len = unsafe { libc_strlen(buffer) };
                // Print buffer as C string
                let s = unsafe { std::slice::from_raw_parts(buffer, len) };
                let s = std::str::from_utf8(s).unwrap_or("");
                print!("Buffer allocated: '{}'\n", s);
                print!("Buffer length: {}\n", len);
                result = len as c_int;

                unsafe {
                    libc::free(buffer as *mut libc::c_void);
                }
                print!("Buffer freed successfully\n");
                buffer = std::ptr::null_mut();
            } else {
                print!("Failed to allocate buffer\n");
                result = -1;
            }
        }
        3 => {
            print!("Mode 3: Function pointers with static counter\n");

            current_op = Some(reset_counter);
            result = apply_operation(current_op, value);
            print!("Counter reset to: {}\n", result);

            current_op = Some(increment_counter);
            result = apply_operation(current_op, opt1);
            print!("Counter after increment by {}: {}\n", opt1, result);

            current_op = Some(multiply_counter);
            result = apply_operation(current_op, opt2);
            print!("Counter after multiply by {}: {}\n", opt2, result);

            current_op = Some(decrement_counter);
            result = apply_operation(current_op, 5);
            print!("Counter after decrement by 5: {}\n", result);

            print!("Final static counter value: {}\n", unsafe { COUNTER });
        }
        4 => {
            print!("Mode 4: Using memchr to find character\n");

            buffer = create_buffer(b"Search for character X in this buffer\0".as_ptr());

            if !buffer.is_null() {
                let buf_size = unsafe { libc_strlen(buffer) };
                let search_char = b'X';

                let s = unsafe { std::slice::from_raw_parts(buffer, buf_size) };
                let s_str = std::str::from_utf8(s).unwrap_or("");
                print!("Searching for '{}' in: '{}'\n", search_char as char, s_str);
                let found_pos = find_char_in_buffer(buffer, buf_size, search_char);

                if !found_pos.is_null() {
                    result = (found_pos as usize - buffer as usize) as c_int;
                    print!("Found '{}' at position: {}\n", search_char as char, result);
                } else {
                    print!("Character '{}' not found\n", search_char as char);
                    result = -1;
                }

                unsafe {
                    libc::free(buffer as *mut libc::c_void);
                }
                buffer = std::ptr::null_mut();
            }
        }
        _ => {
            print!("Invalid mode: {}\n", mode);
            result = -1;
        }
    }

    let _ = buffer; // suppress unused warning
    result
}
