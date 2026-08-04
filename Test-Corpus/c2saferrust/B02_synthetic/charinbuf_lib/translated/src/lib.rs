









use std::ffi::{CStr, CString};

extern "C" {
    fn printf(__format: *const ::core::ffi::c_char, ...) -> ::core::ffi::c_int;
    fn malloc(__size: size_t) -> *mut ::core::ffi::c_void;
    fn free(__ptr: *mut ::core::ffi::c_void);
    fn memchr(
        __s: *const ::core::ffi::c_void,
        __c: ::core::ffi::c_int,
        __n: size_t,
    ) -> *mut ::core::ffi::c_void;
    fn strcpy(
        __dest: *mut ::core::ffi::c_char,
        __src: *const ::core::ffi::c_char,
    ) -> *mut ::core::ffi::c_char;
    fn strlen(__s: *const ::core::ffi::c_char) -> size_t;
}
pub type size_t = usize;
pub type operation_func = Option<unsafe extern "C" fn(::core::ffi::c_int) -> ::core::ffi::c_int>;
pub const NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<::core::ffi::c_void>();
pub const UINT16_MAX: ::core::ffi::c_int = 65535 as ::core::ffi::c_int;
static mut counter: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
#[no_mangle]
pub unsafe extern "C" fn increment_counter(mut value: ::core::ffi::c_int) -> ::core::ffi::c_int {
    counter += value;
    return counter;
}
#[no_mangle]
pub unsafe extern "C" fn decrement_counter(value: ::core::ffi::c_int) -> ::core::ffi::c_int {
    counter -= value;
    counter
}

#[no_mangle]
pub unsafe extern "C" fn multiply_counter(mut value: ::core::ffi::c_int) -> ::core::ffi::c_int {
    counter *= value;
    return counter;
}
#[no_mangle]
pub unsafe extern "C" fn reset_counter(mut value: ::core::ffi::c_int) -> ::core::ffi::c_int {
    counter = value;
    return counter;
}
#[no_mangle]
pub unsafe extern "C" fn is_string_empty(
    mut str: *const ::core::ffi::c_char,
) -> ::core::ffi::c_int {
    if str.is_null() {
        return 1 as ::core::ffi::c_int;
    }
    if *str != 0 {
        return 0 as ::core::ffi::c_int;
    }
    return 1 as ::core::ffi::c_int;
}
#[no_mangle]
pub fn find_char_in_buffer(
    buffer: Option<&CStr>,
    size: usize,
    target: ::core::ffi::c_char,
) -> Option<usize> {
    let bytes = match buffer {
        Some(s) => s.to_bytes(),
        None => return None,
    };

    let search_len = size.min(bytes.len());
    let target = target as u8;

    bytes[..search_len].iter().position(|&b| b == target)
}

#[no_mangle]
pub fn create_buffer(initial: &CStr) -> *mut ::core::ffi::c_char {
    initial.to_owned().into_raw()
}

#[no_mangle]
pub fn validate_uint16_range(value: i32) -> i32 {
    if value < 0 {
        return 0;
    }
    if value > UINT16_MAX {
        return 0;
    }
    1
}

#[no_mangle]
pub fn apply_operation(
    op: Option<unsafe extern "C" fn(::core::ffi::c_int) -> ::core::ffi::c_int>,
    value: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    match op {
        Some(f) => unsafe { f(value) },
        None => -1,
    }
}

#[no_mangle]
pub unsafe extern "C" fn charinbuf(
    mut mode: ::core::ffi::c_int,
    mut value: ::core::ffi::c_int,
    mut opt1: ::core::ffi::c_int,
    mut opt2: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    let mut result: i32 = 0;
let mut buffer: *mut ::core::ffi::c_char = ::core::ptr::null_mut();
let mut found_pos: *mut ::core::ffi::c_char = ::core::ptr::null_mut();
let mut test_string: *const ::core::ffi::c_char = b"\0" as *const u8 as *const ::core::ffi::c_char;
let mut non_empty_string: *const ::core::ffi::c_char =
    b"Hello, World!\0" as *const u8 as *const ::core::ffi::c_char;
let mut current_op: Option<unsafe extern "C" fn(i32) -> i32> = None;

counter = 0;

match mode {
    0 => {
        println!("Mode 0: UINT16_MAX validation");
println!("Checking if value {} is within uint16_t range...", value);

if validate_uint16_range(value) != 0 {
    println!("Value {} is valid (0 <= value <= {})", value, UINT16_MAX);
    result = value;
} else {
    println!("Value {} is out of range for uint16_t", value);
    result = -1;
}

println!("UINT16_MAX constant value: {}", UINT16_MAX);


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

let initial = CString::new("Testing malloc and free").expect("string literal contains no NUL bytes");
buffer = create_buffer(initial.as_c_str());

if buffer.is_null() {
    println!("Failed to allocate buffer");
    result = -1;
} else {
    let buffer_str = unsafe { CStr::from_ptr(buffer) }.to_string_lossy();
    println!("Buffer allocated: '{}'", buffer_str);
    println!("Buffer length: {}", buffer_str.len());
    result = buffer_str.len() as i32;
    free(buffer.cast());
    println!("Buffer freed successfully");
    buffer = std::ptr::null_mut();
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

println!("Final static counter value: {}", counter);


    }
    4 => {
        println!("Mode 4: Using memchr to find character");

let initial = CString::new("Search for character X in this buffer").unwrap();
buffer = create_buffer(initial.as_c_str());

if !buffer.is_null() {
    let search_char: ::core::ffi::c_char = b'X' as ::core::ffi::c_char;
    let buffer_cstr = unsafe { CStr::from_ptr(buffer) };

    println!(
        "Searching for '{}' in: '{}'",
        search_char as u8 as char,
        buffer_cstr.to_string_lossy()
    );

    let found_pos = find_char_in_buffer(
        Some(buffer_cstr),
        buffer_cstr.to_bytes().len(),
        search_char,
    );

    if let Some(pos) = found_pos {
        result = pos as i32;
        println!("Found '{}' at position: {}", search_char as u8 as char, result);
    } else {
        println!("Character '{}' not found", search_char as u8 as char);
        result = -1;
    }

    free(buffer as *mut ::core::ffi::c_void);
    buffer = ::core::ptr::null_mut();
}


    }
    _ => {
        eprintln!("Invalid mode: {}", mode);
        result = -1;
    }
}

result

}
