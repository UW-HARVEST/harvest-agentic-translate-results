//! `charinbuf` — the single function declared in `include/lib.h` and the driver
//! that exercises the rest of the library.
//!
//! Every message is emitted through libc `printf` with the original format
//! string byte-for-byte, so stdout content, ordering and buffering behaviour are
//! identical to the C build.

use core::ffi::{c_char, c_int};
use core::ptr;

use crate::cruntime::{free, printf, strlen};
use crate::helpers::{
    OperationFunc, apply_operation, create_buffer, find_char_in_buffer, is_string_empty,
};
use crate::{UINT16_MAX, counter};

/// Convenience for turning a NUL-terminated byte literal into `*const c_char`.
macro_rules! cstr {
    ($bytes:literal) => {
        concat!($bytes, "\0").as_ptr() as *const c_char
    };
}

/// ```c
/// int charinbuf(int mode, int value, int opt1, int opt2);
/// ```
///
/// Declared in `include/lib.h`. There is no namespace macro in the header, so
/// the linker symbol is plain `charinbuf`.
#[unsafe(no_mangle)]
pub extern "C" fn charinbuf(mode: c_int, value: c_int, opt1: c_int, opt2: c_int) -> c_int {
    let mut result: c_int = 0;
    let mut buffer: *mut c_char = ptr::null_mut();
    let found_pos: *mut c_char;
    let test_string = cstr!("");
    let non_empty_string = cstr!("Hello, World!");

    let mut current_op: OperationFunc;

    counter::set(0);

    // SAFETY: every call below is a variadic libc call whose argument types
    // match the corresponding conversion specifier, or a helper invoked on a
    // pointer this function itself produced.
    unsafe {
        match mode {
            0 => {
                printf(cstr!("Mode 0: UINT16_MAX validation\n"));
                printf(
                    cstr!("Checking if value %d is within uint16_t range...\n"),
                    value,
                );

                if crate::helpers::validate_uint16_range(value) != 0 {
                    printf(
                        cstr!("Value %d is valid (0 <= value <= %u)\n"),
                        value,
                        UINT16_MAX as core::ffi::c_uint,
                    );
                    result = value;
                } else {
                    printf(cstr!("Value %d is out of range for uint16_t\n"), value);
                    result = -1;
                }

                printf(
                    cstr!("UINT16_MAX constant value: %u\n"),
                    UINT16_MAX as core::ffi::c_uint,
                );
            }

            1 => {
                printf(cstr!("Mode 1: String empty check by dereference\n"));

                if is_string_empty(test_string) != 0 {
                    printf(cstr!("Test string is empty (checked with *string)\n"));
                    result = 0;
                } else {
                    printf(cstr!("Test string is not empty\n"));
                    result = 1;
                }

                if is_string_empty(non_empty_string) != 0 {
                    printf(cstr!("Non-empty string check failed!\n"));
                } else {
                    printf(cstr!("Non-empty string correctly identified\n"));
                    result = result.wrapping_add(10);
                }
            }

            2 => {
                printf(cstr!("Mode 2: Dynamic memory allocation and free\n"));

                buffer = create_buffer(cstr!("Testing malloc and free"));

                if !buffer.is_null() {
                    printf(cstr!("Buffer allocated: '%s'\n"), buffer);
                    printf(cstr!("Buffer length: %zu\n"), strlen(buffer));
                    result = strlen(buffer) as c_int;

                    free(buffer as *mut core::ffi::c_void);
                    printf(cstr!("Buffer freed successfully\n"));
                    buffer = ptr::null_mut();
                } else {
                    printf(cstr!("Failed to allocate buffer\n"));
                    result = -1;
                }
            }

            3 => {
                printf(cstr!("Mode 3: Function pointers with static counter\n"));

                current_op = Some(counter::reset_counter);
                result = apply_operation(current_op, value);
                printf(cstr!("Counter reset to: %d\n"), result);

                current_op = Some(counter::increment_counter);
                result = apply_operation(current_op, opt1);
                printf(cstr!("Counter after increment by %d: %d\n"), opt1, result);

                current_op = Some(counter::multiply_counter);
                result = apply_operation(current_op, opt2);
                printf(cstr!("Counter after multiply by %d: %d\n"), opt2, result);

                current_op = Some(counter::decrement_counter);
                result = apply_operation(current_op, 5);
                printf(cstr!("Counter after decrement by 5: %d\n"), result);

                printf(cstr!("Final static counter value: %d\n"), counter::get());
            }

            4 => {
                printf(cstr!("Mode 4: Using memchr to find character\n"));

                buffer = create_buffer(cstr!("Search for character X in this buffer"));

                if !buffer.is_null() {
                    let buf_size = strlen(buffer);
                    let search_char: c_char = b'X' as c_char;

                    printf(
                        cstr!("Searching for '%c' in: '%s'\n"),
                        search_char as c_int,
                        buffer,
                    );
                    found_pos = find_char_in_buffer(buffer, buf_size, search_char);

                    if !found_pos.is_null() {
                        result = found_pos.offset_from(buffer) as c_int;
                        printf(
                            cstr!("Found '%c' at position: %d\n"),
                            search_char as c_int,
                            result,
                        );
                    } else {
                        printf(cstr!("Character '%c' not found\n"), search_char as c_int);
                        result = -1;
                    }

                    free(buffer as *mut core::ffi::c_void);
                    buffer = ptr::null_mut();
                }
            }

            _ => {
                printf(cstr!("Invalid mode: %d\n"), mode);
                result = -1;
            }
        }
    }

    // `buffer` is cleared on every path that allocated it, mirroring the C code;
    // the assignments exist only for parity and are otherwise dead.
    let _ = buffer;

    result
}
