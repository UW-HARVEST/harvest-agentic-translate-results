use std::ffi::{c_char, c_int, c_void};

type Operation = unsafe extern "C" fn(c_int, c_int) -> c_int;

static mut ACCUMULATOR: c_int = 0;
static mut MULTIPLIER: c_int = 1;
static mut OPERATION_COUNT: c_int = 0;

mod libc {
    use super::{c_char, c_int, c_void};

    unsafe extern "C" {
        pub fn memchr(s: *const c_void, c: c_int, n: usize) -> *mut c_void;
        pub fn sprintf(s: *mut c_char, format: *const c_char, ...) -> c_int;
        pub fn strcpy(dest: *mut c_char, src: *const c_char) -> *mut c_char;
        pub fn strlen(s: *const c_char) -> usize;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn add_to_accumulator(a: c_int, b: c_int) -> c_int {
    unsafe {
        ACCUMULATOR = ACCUMULATOR.wrapping_add(a.wrapping_add(b));
        OPERATION_COUNT = OPERATION_COUNT.wrapping_add(1);
        ACCUMULATOR
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn multiply_with_multiplier(a: c_int, b: c_int) -> c_int {
    unsafe {
        MULTIPLIER = MULTIPLIER.wrapping_mul(a.wrapping_mul(b));
        OPERATION_COUNT = OPERATION_COUNT.wrapping_add(1);
        MULTIPLIER
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn subtract_from_accumulator(a: c_int, b: c_int) -> c_int {
    unsafe {
        ACCUMULATOR = ACCUMULATOR.wrapping_sub(a.wrapping_sub(b));
        OPERATION_COUNT = OPERATION_COUNT.wrapping_add(1);
        ACCUMULATOR
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn divide_multiplier(_a: c_int, b: c_int) -> c_int {
    unsafe {
        if b != 0 {
            MULTIPLIER /= b;
        }
        OPERATION_COUNT = OPERATION_COUNT.wrapping_add(1);
        MULTIPLIER
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn process_octal_string(dest: *mut c_char, octal_val: c_int) {
    let mut buffer = [0 as c_char; 50];
    unsafe {
        libc::sprintf(
            buffer.as_mut_ptr(),
            b"Octal: 0%o, Decimal: %d\0".as_ptr().cast(),
            octal_val,
            octal_val,
        );
        libc::strcpy(dest, buffer.as_ptr());
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn find_and_replace_char(str_: *mut c_char, search_char: c_int) {
    unsafe {
        let found = libc::memchr(str_.cast(), search_char, libc::strlen(str_.cast_const()))
            .cast::<c_char>();
        if !found.is_null() {
            *found = b'X' as c_char;
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn validate_and_normalize(value: c_int) -> c_int {
    let is_nonzero = value != 0;
    let lower_threshold = 0o100;
    let upper_threshold = 0o777;

    if is_nonzero && value > 0 {
        if value < lower_threshold {
            return lower_threshold;
        } else if value > upper_threshold {
            return upper_threshold;
        }
    }

    value
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn findrep(
    param1: c_int,
    param2: c_int,
    param3: c_int,
    param4: c_int,
) -> c_int {
    let mut result: c_int = 0;

    let p1_valid = c_int::from(param1 != 0);
    let p2_valid = c_int::from(param2 != 0);
    let p3_valid = c_int::from(param3 != 0);
    let p4_valid = c_int::from(param4 != 0);
    let active_params = p1_valid + p2_valid + p3_valid + p4_valid;

    let normalized_p1 = validate_and_normalize(param1);
    let normalized_p2 = validate_and_normalize(param2);
    let normalized_p3 = validate_and_normalize(param3);
    let normalized_p4 = validate_and_normalize(param4);

    let mut message = [0 as c_char; 100];
    let mut search_buffer = [0 as c_char; 100];

    unsafe {
        process_octal_string(message.as_mut_ptr(), 0o123);
        libc::strcpy(
            search_buffer.as_mut_ptr(),
            b"Function pointer example with static vars\0"
                .as_ptr()
                .cast(),
        );

        let found_char = libc::memchr(
            search_buffer.as_ptr().cast(),
            b'p' as c_int,
            libc::strlen(search_buffer.as_ptr()),
        )
        .cast::<c_char>();
        if !found_char.is_null() {
            result = result.wrapping_add(found_char.offset_from(search_buffer.as_ptr()) as c_int);
        }

        let operations: [Operation; 4] = [
            add_to_accumulator,
            multiply_with_multiplier,
            subtract_from_accumulator,
            divide_multiplier,
        ];

        if active_params >= 0o1 {
            result = result.wrapping_add(operations[0](normalized_p1, normalized_p2));
        }

        if active_params >= 0o2 {
            result = result.wrapping_add(operations[1](normalized_p3, normalized_p4));
        }

        if ACCUMULATOR > 0o150 {
            let subtract_result = operations[2](normalized_p1, normalized_p3);
            result = result.wrapping_add(subtract_result);
        }

        find_and_replace_char(message.as_mut_ptr(), b'O' as c_int);

        let mut final_message = [0 as c_char; 100];
        libc::strcpy(final_message.as_mut_ptr(), message.as_ptr());

        let has_accumulator = ACCUMULATOR != 0;
        let has_multiplier = MULTIPLIER != 0;
        if has_accumulator && has_multiplier {
            result = result.wrapping_add(ACCUMULATOR.wrapping_add(MULTIPLIER));
        }

        if MULTIPLIER > 0o100 {
            operations[3](MULTIPLIER, 2);
        }

        result = result.wrapping_add(OPERATION_COUNT.wrapping_mul(0o10));
    }

    if result == 0 { 0o777 } else { result }
}
