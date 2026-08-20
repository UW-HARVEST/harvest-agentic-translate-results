// Translation of `int charinbuf(int mode, int value, int opt1, int opt2)`, the
// single function declared in include/lib.h and the library's demo driver.
//
// Every `printf` is forwarded to C's `printf` with the original format string
// and argument types, so the emitted bytes -- and the interleaving with any
// other C-side output -- match exactly.

use core::ffi::{c_char, c_int, c_uint};
use core::ptr;

use crate::counter;
use crate::cstd;
use crate::helpers::{
    self, OperationFunc, UINT16_MAX, create_buffer, find_char_in_buffer, is_string_empty,
    validate_uint16_range,
};

#[unsafe(no_mangle)]
pub extern "C" fn charinbuf(mode: c_int, value: c_int, opt1: c_int, opt2: c_int) -> c_int {
    let mut result: c_int = 0;
    let mut buffer: *mut c_char = ptr::null_mut();
    let found_pos: *mut c_char;
    let test_string: *const c_char = c"".as_ptr();
    let non_empty_string: *const c_char = c"Hello, World!".as_ptr();

    // `operation_func current_op = NULL;` -- reassigned before every use.
    let mut current_op: Option<OperationFunc>;

    counter::set(0);

    unsafe {
        match mode {
            0 => {
                cstd::printf(c"Mode 0: UINT16_MAX validation\n".as_ptr());
                cstd::printf(
                    c"Checking if value %d is within uint16_t range...\n".as_ptr(),
                    value,
                );

                if validate_uint16_range(value) != 0 {
                    cstd::printf(
                        c"Value %d is valid (0 <= value <= %u)\n".as_ptr(),
                        value,
                        UINT16_MAX as c_uint,
                    );
                    result = value;
                } else {
                    cstd::printf(c"Value %d is out of range for uint16_t\n".as_ptr(), value);
                    result = -1;
                }

                cstd::printf(
                    c"UINT16_MAX constant value: %u\n".as_ptr(),
                    UINT16_MAX as c_uint,
                );
            }

            1 => {
                cstd::printf(c"Mode 1: String empty check by dereference\n".as_ptr());

                if is_string_empty(test_string) != 0 {
                    cstd::printf(c"Test string is empty (checked with *string)\n".as_ptr());
                    result = 0;
                } else {
                    cstd::printf(c"Test string is not empty\n".as_ptr());
                    result = 1;
                }

                if is_string_empty(non_empty_string) != 0 {
                    cstd::printf(c"Non-empty string check failed!\n".as_ptr());
                } else {
                    cstd::printf(c"Non-empty string correctly identified\n".as_ptr());
                    result += 10;
                }
            }

            2 => {
                cstd::printf(c"Mode 2: Dynamic memory allocation and free\n".as_ptr());

                buffer = create_buffer(c"Testing malloc and free".as_ptr());

                if !buffer.is_null() {
                    cstd::printf(c"Buffer allocated: '%s'\n".as_ptr(), buffer);
                    cstd::printf(c"Buffer length: %zu\n".as_ptr(), cstd::strlen(buffer));
                    result = cstd::strlen(buffer) as c_int;

                    cstd::free(buffer.cast());
                    cstd::printf(c"Buffer freed successfully\n".as_ptr());
                    buffer = ptr::null_mut();
                } else {
                    cstd::printf(c"Failed to allocate buffer\n".as_ptr());
                    result = -1;
                }
            }

            3 => {
                cstd::printf(c"Mode 3: Function pointers with static counter\n".as_ptr());

                current_op = Some(counter::reset_counter);
                result = helpers::apply_operation(current_op, value);
                cstd::printf(c"Counter reset to: %d\n".as_ptr(), result);

                current_op = Some(counter::increment_counter);
                result = helpers::apply_operation(current_op, opt1);
                cstd::printf(
                    c"Counter after increment by %d: %d\n".as_ptr(),
                    opt1,
                    result,
                );

                current_op = Some(counter::multiply_counter);
                result = helpers::apply_operation(current_op, opt2);
                cstd::printf(
                    c"Counter after multiply by %d: %d\n".as_ptr(),
                    opt2,
                    result,
                );

                current_op = Some(counter::decrement_counter);
                result = helpers::apply_operation(current_op, 5);
                cstd::printf(c"Counter after decrement by 5: %d\n".as_ptr(), result);

                cstd::printf(c"Final static counter value: %d\n".as_ptr(), counter::get());
            }

            4 => {
                cstd::printf(c"Mode 4: Using memchr to find character\n".as_ptr());

                buffer = create_buffer(c"Search for character X in this buffer".as_ptr());

                if !buffer.is_null() {
                    let buf_size = cstd::strlen(buffer);
                    let search_char: c_char = b'X' as c_char;

                    cstd::printf(
                        c"Searching for '%c' in: '%s'\n".as_ptr(),
                        c_int::from(search_char),
                        buffer,
                    );
                    found_pos = find_char_in_buffer(buffer, buf_size, search_char);

                    if !found_pos.is_null() {
                        result = found_pos.offset_from(buffer) as c_int;
                        cstd::printf(
                            c"Found '%c' at position: %d\n".as_ptr(),
                            c_int::from(search_char),
                            result,
                        );
                    } else {
                        cstd::printf(
                            c"Character '%c' not found\n".as_ptr(),
                            c_int::from(search_char),
                        );
                        result = -1;
                    }

                    cstd::free(buffer.cast());
                    buffer = ptr::null_mut();
                }
                // Note: when allocation fails the C code leaves `result` at 0
                // and prints nothing. Preserved.
            }

            _ => {
                cstd::printf(c"Invalid mode: %d\n".as_ptr(), mode);
                result = -1;
            }
        }
    }

    // `buffer` is always NULL by here, exactly as in the C original; the
    // assignments above are kept so the control flow mirrors the source.
    let _ = buffer;

    result
}
