







extern "C" {
    fn sprintf(
        __s: *mut ::core::ffi::c_char,
        __format: *const ::core::ffi::c_char,
        ...
    ) -> ::core::ffi::c_int;
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
pub type operation_func =
    Option<unsafe extern "C" fn(::core::ffi::c_int, ::core::ffi::c_int) -> ::core::ffi::c_int>;
static mut accumulator: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
static mut multiplier: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
static mut operation_count: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
#[no_mangle]
pub extern "C" fn add_to_accumulator(a: ::core::ffi::c_int, b: ::core::ffi::c_int) -> ::core::ffi::c_int {
    unsafe {
        accumulator += a + b;
        operation_count += 1;
        accumulator
    }
}

#[no_mangle]
pub unsafe extern "C" fn multiply_with_multiplier(
    a: ::core::ffi::c_int,
    b: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    multiplier *= a * b;
    operation_count += 1;
    multiplier
}

#[no_mangle]
pub unsafe extern "C" fn subtract_from_accumulator(
    a: ::core::ffi::c_int,
    b: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    accumulator -= a - b;
    operation_count += 1;
    accumulator
}

#[no_mangle]
pub unsafe extern "C" fn divide_multiplier(_a: ::core::ffi::c_int, b: ::core::ffi::c_int) -> ::core::ffi::c_int {
    if b != 0 {
        multiplier /= b;
    }
    operation_count += 1;
    multiplier
}

#[no_mangle]
pub fn process_octal_string(dest: &mut [::core::ffi::c_char], octal_val: ::core::ffi::c_int) {
    let s = format!("Octal: 0{:o}, Decimal: {}", octal_val, octal_val);
    let bytes = s.as_bytes();
    let len = bytes.len().min(dest.len().saturating_sub(1));

    for i in 0..len {
        dest[i] = bytes[i] as ::core::ffi::c_char;
    }

    if !dest.is_empty() {
        dest[len] = 0;
    }
}

#[no_mangle]
pub fn find_and_replace_char(s: &mut [::core::ffi::c_char], search_char: ::core::ffi::c_int) {
    let target = search_char as ::core::ffi::c_char;
    if let Some(found) = s.iter_mut().take_while(|c| **c != 0).find(|c| **c == target) {
        *found = 'X' as ::core::ffi::c_char;
    }
}

#[no_mangle]
pub fn validate_and_normalize(value: i32) -> i32 {
    let lower_threshold = 0o100;
    let upper_threshold = 0o777;

    if value > 0 {
        if value < lower_threshold {
            lower_threshold
        } else if value > upper_threshold {
            upper_threshold
        } else {
            value
        }
    } else {
        value
    }
}

static mut operations: [operation_func; 4] = unsafe {
    [
        Some(
            add_to_accumulator
                as unsafe extern "C" fn(
                    ::core::ffi::c_int,
                    ::core::ffi::c_int,
                ) -> ::core::ffi::c_int,
        ),
        Some(
            multiply_with_multiplier
                as unsafe extern "C" fn(
                    ::core::ffi::c_int,
                    ::core::ffi::c_int,
                ) -> ::core::ffi::c_int,
        ),
        Some(
            subtract_from_accumulator
                as unsafe extern "C" fn(
                    ::core::ffi::c_int,
                    ::core::ffi::c_int,
                ) -> ::core::ffi::c_int,
        ),
        Some(
            divide_multiplier
                as unsafe extern "C" fn(
                    ::core::ffi::c_int,
                    ::core::ffi::c_int,
                ) -> ::core::ffi::c_int,
        ),
    ]
};
#[no_mangle]
pub fn findrep(param1: i32, param2: i32, param3: i32, param4: i32) -> i32 {
    let mut result = 0;

    let p1_valid = (param1 != 0) as i32;
    let p2_valid = (param2 != 0) as i32;
    let p3_valid = (param3 != 0) as i32;
    let p4_valid = (param4 != 0) as i32;
    let active_params = p1_valid + p2_valid + p3_valid + p4_valid;

    let mode_add = 0o1;
    let mode_multiply = 0o2;

    let normalized_p1 = validate_and_normalize(param1);
    let normalized_p2 = validate_and_normalize(param2);
    let normalized_p3 = validate_and_normalize(param3);
    let normalized_p4 = validate_and_normalize(param4);

    let mut message = [0; 100];
    let mut search_buffer = [0; 100];

    process_octal_string(&mut message, 0o123);

    let search_text = b"Function pointer example with static vars\0";
    for (dst, src) in search_buffer.iter_mut().zip(search_text.iter()) {
        *dst = *src as ::core::ffi::c_char;
    }

    let search_len = search_buffer
        .iter()
        .position(|&c| c == 0)
        .unwrap_or(search_buffer.len());

    if let Some(found_pos) = search_buffer[..search_len]
        .iter()
        .position(|&c| c == b'p' as ::core::ffi::c_char)
    {
        result += found_pos as i32;
    }

    if active_params >= mode_add {
        if let Some(op) = unsafe { operations[0] } {
            result += unsafe { op(normalized_p1, normalized_p2) };
        }
    }

    if active_params >= mode_multiply {
        if let Some(op) = unsafe { operations[1] } {
            result += unsafe { op(normalized_p3, normalized_p4) };
        }
    }

    if unsafe { accumulator } > 0o150 {
        if let Some(op) = unsafe { operations[2] } {
            let subtract_result = unsafe { op(normalized_p1, normalized_p3) };
            result += subtract_result;
        }
    }

    find_and_replace_char(&mut message, 'O' as i32);

    let mut final_message = [0; 100];
    final_message.copy_from_slice(&message);

    let has_accumulator = (unsafe { accumulator } != 0) as i32;
    let has_multiplier = (unsafe { multiplier } != 0) as i32;
    let both_active = (has_accumulator != 0 && has_multiplier != 0) as i32;

    if both_active != 0 {
        result += unsafe { accumulator + multiplier };
    }

    if unsafe { multiplier } > 0o100 {
        if let Some(op) = unsafe { operations[3] } {
            let _ = unsafe { op(multiplier, 2) };
        }
    }

    result += unsafe { operation_count } * 0o10;

    if result == 0 {
        result = 0o777;
    }

    result
}

