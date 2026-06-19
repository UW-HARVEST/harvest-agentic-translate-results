use core::cell::UnsafeCell;
use core::ffi::{c_char, c_int};

type OperationFunc = extern "C" fn(c_int, c_int) -> c_int;

struct GlobalI32(UnsafeCell<c_int>);

unsafe impl Sync for GlobalI32 {}

impl GlobalI32 {
    const fn new(value: c_int) -> Self {
        Self(UnsafeCell::new(value))
    }

    fn get(&self) -> c_int {
        unsafe { *self.0.get() }
    }

    fn set(&self, value: c_int) {
        unsafe {
            *self.0.get() = value;
        }
    }
}

static ACCUMULATOR: GlobalI32 = GlobalI32::new(0);
static MULTIPLIER: GlobalI32 = GlobalI32::new(1);
static OPERATION_COUNT: GlobalI32 = GlobalI32::new(0);

static OPERATIONS: [OperationFunc; 4] = [
    add_to_accumulator,
    multiply_with_multiplier,
    subtract_from_accumulator,
    divide_multiplier,
];

unsafe fn c_strlen(ptr: *const c_char) -> usize {
    let mut len = 0usize;
    unsafe {
        while *ptr.add(len) != 0 {
            len += 1;
        }
    }
    len
}

unsafe fn c_memchr(ptr: *const c_char, needle: c_int, len: usize) -> *mut c_char {
    let target = needle as u8;
    let mut idx = 0usize;
    unsafe {
        while idx < len {
            if *ptr.add(idx).cast::<u8>() == target {
                return ptr.add(idx) as *mut c_char;
            }
            idx += 1;
        }
    }
    core::ptr::null_mut()
}

unsafe fn c_strcpy(dest: *mut c_char, src: *const c_char) {
    let mut idx = 0usize;
    unsafe {
        loop {
            let value = *src.add(idx);
            *dest.add(idx) = value;
            if value == 0 {
                break;
            }
            idx += 1;
        }
    }
}

unsafe fn write_bytes_with_nul(dest: *mut c_char, bytes: &[u8]) {
    let mut idx = 0usize;
    unsafe {
        while idx < bytes.len() {
            *dest.add(idx).cast::<u8>() = bytes[idx];
            idx += 1;
        }
        *dest.add(idx) = 0;
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn add_to_accumulator(a: c_int, b: c_int) -> c_int {
    let value = ACCUMULATOR.get().wrapping_add(a.wrapping_add(b));
    ACCUMULATOR.set(value);
    OPERATION_COUNT.set(OPERATION_COUNT.get().wrapping_add(1));
    value
}

#[unsafe(no_mangle)]
pub extern "C" fn multiply_with_multiplier(a: c_int, b: c_int) -> c_int {
    let product = a.wrapping_mul(b);
    let value = MULTIPLIER.get().wrapping_mul(product);
    MULTIPLIER.set(value);
    OPERATION_COUNT.set(OPERATION_COUNT.get().wrapping_add(1));
    value
}

#[unsafe(no_mangle)]
pub extern "C" fn subtract_from_accumulator(a: c_int, b: c_int) -> c_int {
    let value = ACCUMULATOR.get().wrapping_sub(a.wrapping_sub(b));
    ACCUMULATOR.set(value);
    OPERATION_COUNT.set(OPERATION_COUNT.get().wrapping_add(1));
    value
}

#[unsafe(no_mangle)]
pub extern "C" fn divide_multiplier(_a: c_int, b: c_int) -> c_int {
    if b != 0 {
        let value = MULTIPLIER.get().wrapping_div(b);
        MULTIPLIER.set(value);
    }
    OPERATION_COUNT.set(OPERATION_COUNT.get().wrapping_add(1));
    MULTIPLIER.get()
}

#[unsafe(no_mangle)]
pub extern "C" fn process_octal_string(dest: *mut c_char, octal_val: c_int) {
    let mut buffer = [0u8; 50];
    let rendered = format!(
        "Octal: 0{:o}, Decimal: {}",
        octal_val as u32, octal_val
    );

    unsafe {
        write_bytes_with_nul(buffer.as_mut_ptr().cast::<c_char>(), rendered.as_bytes());
        c_strcpy(dest, buffer.as_ptr().cast::<c_char>());
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn find_and_replace_char(str_ptr: *mut c_char, search_char: c_int) {
    unsafe {
        let found = c_memchr(str_ptr, search_char, c_strlen(str_ptr));
        if !found.is_null() {
            *found = b'X' as c_char;
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn validate_and_normalize(value: c_int) -> c_int {
    let is_nonzero = (value != 0) as c_int;

    let lower_threshold = 0o100;
    let upper_threshold = 0o777;

    if is_nonzero != 0 && value > 0 {
        if value < lower_threshold {
            return lower_threshold;
        } else if value > upper_threshold {
            return upper_threshold;
        }
    }

    value
}

#[unsafe(no_mangle)]
pub extern "C" fn findrep(param1: c_int, param2: c_int, param3: c_int, param4: c_int) -> c_int {
    let mut result: c_int = 0;

    let p1_valid = (param1 != 0) as c_int;
    let p2_valid = (param2 != 0) as c_int;
    let p3_valid = (param3 != 0) as c_int;
    let p4_valid = (param4 != 0) as c_int;

    let active_params = p1_valid + p2_valid + p3_valid + p4_valid;

    let mode_add = 0o1;
    let mode_multiply = 0o2;

    let normalized_p1 = validate_and_normalize(param1);
    let normalized_p2 = validate_and_normalize(param2);
    let normalized_p3 = validate_and_normalize(param3);
    let normalized_p4 = validate_and_normalize(param4);

    let mut message = [0 as c_char; 100];
    let mut search_buffer = [0 as c_char; 100];

    process_octal_string(message.as_mut_ptr(), 0o123);

    unsafe {
        write_bytes_with_nul(
            search_buffer.as_mut_ptr(),
            b"Function pointer example with static vars",
        );
    }

    unsafe {
        let found_char = c_memchr(search_buffer.as_ptr(), b'p' as c_int, c_strlen(search_buffer.as_ptr()));
        if !found_char.is_null() {
            result = result.wrapping_add(found_char.offset_from(search_buffer.as_ptr()) as c_int);
        }
    }

    let mut selected_op: OperationFunc;

    if active_params >= mode_add {
        selected_op = OPERATIONS[0];
        result = result.wrapping_add(selected_op(normalized_p1, normalized_p2));
    }

    if active_params >= mode_multiply {
        selected_op = OPERATIONS[1];
        result = result.wrapping_add(selected_op(normalized_p3, normalized_p4));
    }

    if ACCUMULATOR.get() > 0o150 {
        selected_op = OPERATIONS[2];
        let subtract_result = selected_op(normalized_p1, normalized_p3);
        result = result.wrapping_add(subtract_result);
    }

    find_and_replace_char(message.as_mut_ptr(), b'O' as c_int);

    let mut final_message = [0 as c_char; 100];
    unsafe {
        c_strcpy(final_message.as_mut_ptr(), message.as_ptr());
    }

    let has_accumulator = (ACCUMULATOR.get() != 0) as c_int;
    let has_multiplier = (MULTIPLIER.get() != 0) as c_int;
    let both_active = has_accumulator & has_multiplier;

    if both_active != 0 {
        result = result
            .wrapping_add(ACCUMULATOR.get())
            .wrapping_add(MULTIPLIER.get());
    }

    if MULTIPLIER.get() > 0o100 {
        selected_op = OPERATIONS[3];
        selected_op(MULTIPLIER.get(), 2);
    }

    result = result.wrapping_add(OPERATION_COUNT.get().wrapping_mul(0o10));

    let result_exists = (result != 0) as c_int;
    if result_exists == 0 {
        result = 0o777;
    }

    let _ = final_message;

    result
}
