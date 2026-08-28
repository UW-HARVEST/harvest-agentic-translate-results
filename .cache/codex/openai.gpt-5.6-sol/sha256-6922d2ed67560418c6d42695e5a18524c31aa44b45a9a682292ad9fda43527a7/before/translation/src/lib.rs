use std::ffi::{c_char, c_int, c_void};

type OperationFn = extern "C" fn(c_int, c_int) -> c_int;

static mut ACCUMULATOR: c_int = 0;
static mut MULTIPLIER: c_int = 1;
static mut OPERATION_COUNT: c_int = 0;

unsafe extern "C" {
    fn sprintf(dest: *mut c_char, format: *const c_char, ...) -> c_int;
    fn strcpy(dest: *mut c_char, src: *const c_char) -> *mut c_char;
    fn memchr(ptr: *const c_void, value: c_int, len: usize) -> *mut c_void;
    fn strlen(str: *const c_char) -> usize;
}

#[unsafe(no_mangle)]
pub extern "C" fn add_to_accumulator(a: c_int, b: c_int) -> c_int {
    unsafe {
        ACCUMULATOR = ACCUMULATOR.wrapping_add(a.wrapping_add(b));
        OPERATION_COUNT = OPERATION_COUNT.wrapping_add(1);
        ACCUMULATOR
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn multiply_with_multiplier(a: c_int, b: c_int) -> c_int {
    unsafe {
        MULTIPLIER = MULTIPLIER.wrapping_mul(a.wrapping_mul(b));
        OPERATION_COUNT = OPERATION_COUNT.wrapping_add(1);
        MULTIPLIER
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn subtract_from_accumulator(a: c_int, b: c_int) -> c_int {
    unsafe {
        ACCUMULATOR = ACCUMULATOR.wrapping_sub(a.wrapping_sub(b));
        OPERATION_COUNT = OPERATION_COUNT.wrapping_add(1);
        ACCUMULATOR
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn divide_multiplier(_a: c_int, b: c_int) -> c_int {
    unsafe {
        if b != 0 {
            MULTIPLIER = MULTIPLIER.wrapping_div(b);
        }
        OPERATION_COUNT = OPERATION_COUNT.wrapping_add(1);
        MULTIPLIER
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn process_octal_string(dest: *mut c_char, octal_val: c_int) {
    const FORMAT: &[u8] = b"Octal: 0%o, Decimal: %d\0";
    let mut buffer = [0 as c_char; 50];

    unsafe {
        sprintf(
            buffer.as_mut_ptr(),
            FORMAT.as_ptr().cast::<c_char>(),
            octal_val,
            octal_val,
        );
        strcpy(dest, buffer.as_ptr());
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn find_and_replace_char(str: *mut c_char, search_char: c_int) {
    unsafe {
        let found = memchr(
            str.cast::<c_void>(),
            search_char,
            strlen(str.cast_const()),
        )
        .cast::<c_char>();
        if !found.is_null() {
            found.write(b'X' as c_char);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn validate_and_normalize(value: c_int) -> c_int {
    let is_nonzero = value != 0;
    let _is_zero = value == 0;

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

static OPERATIONS: [OperationFn; 4] = [
    add_to_accumulator,
    multiply_with_multiplier,
    subtract_from_accumulator,
    divide_multiplier,
];

#[unsafe(no_mangle)]
pub unsafe extern "C" fn findrep(
    param1: c_int,
    param2: c_int,
    param3: c_int,
    param4: c_int,
) -> c_int {
    const SEARCH_TEXT: &[u8] = b"Function pointer example with static vars\0";

    let mut result: c_int = 0;

    let p1_valid = c_int::from(param1 != 0);
    let p2_valid = c_int::from(param2 != 0);
    let p3_valid = c_int::from(param3 != 0);
    let p4_valid = c_int::from(param4 != 0);

    let active_params = p1_valid + p2_valid + p3_valid + p4_valid;

    let mode_add = 0o1;
    let mode_multiply = 0o2;

    let normalized_p1 = validate_and_normalize(param1);
    let normalized_p2 = validate_and_normalize(param2);
    let normalized_p3 = validate_and_normalize(param3);
    let normalized_p4 = validate_and_normalize(param4);

    let mut message = [0 as c_char; 100];
    let mut search_buffer = [0 as c_char; 100];

    unsafe {
        process_octal_string(message.as_mut_ptr(), 0o123);
        strcpy(
            search_buffer.as_mut_ptr(),
            SEARCH_TEXT.as_ptr().cast::<c_char>(),
        );

        let found_char = memchr(
            search_buffer.as_ptr().cast::<c_void>(),
            b'p' as c_int,
            strlen(search_buffer.as_ptr()),
        )
        .cast::<c_char>();
        if !found_char.is_null() {
            result = result.wrapping_add(
                found_char
                    .offset_from(search_buffer.as_ptr())
                    .try_into()
                    .unwrap_unchecked(),
            );
        }

        if active_params >= mode_add {
            result = result.wrapping_add(OPERATIONS[0](normalized_p1, normalized_p2));
        }

        if active_params >= mode_multiply {
            result = result.wrapping_add(OPERATIONS[1](normalized_p3, normalized_p4));
        }

        if ACCUMULATOR > 0o150 {
            let subtract_result = OPERATIONS[2](normalized_p1, normalized_p3);
            result = result.wrapping_add(subtract_result);
        }

        find_and_replace_char(message.as_mut_ptr(), b'O' as c_int);

        let mut final_message = [0 as c_char; 100];
        strcpy(final_message.as_mut_ptr(), message.as_ptr());

        let has_accumulator = ACCUMULATOR != 0;
        let has_multiplier = MULTIPLIER != 0;
        let both_active = has_accumulator && has_multiplier;

        if both_active {
            result = result.wrapping_add(ACCUMULATOR.wrapping_add(MULTIPLIER));
        }

        if MULTIPLIER > 0o100 {
            OPERATIONS[3](MULTIPLIER, 2);
        }

        result = result.wrapping_add(OPERATION_COUNT.wrapping_mul(0o10));
    }

    if result == 0 {
        result = 0o777;
    }

    result
}
