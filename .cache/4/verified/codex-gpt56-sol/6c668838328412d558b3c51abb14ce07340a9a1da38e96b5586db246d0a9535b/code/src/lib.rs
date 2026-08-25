use std::ffi::{c_char, c_int};
use std::mem::size_of;

const SCAN_INT_FORMAT: &[u8] = b"%d\0";
const PRINT_STRING_FORMAT: &[u8] = b"%s\n\0";
const PRINT_INT_FORMAT: &[u8] = b"%d\n\0";

extern "C" {
    fn scanf(format: *const c_char, ...) -> c_int;
    fn printf(format: *const c_char, ...) -> c_int;
}

#[no_mangle]
pub unsafe extern "C" fn printLine(line: *const c_char) {
    if !line.is_null() {
        printf(PRINT_STRING_FORMAT.as_ptr().cast(), line);
    }
}

#[no_mangle]
pub unsafe extern "C" fn printIntLine(int_number: c_int) {
    printf(PRINT_INT_FORMAT.as_ptr().cast(), int_number);
}

#[no_mangle]
pub unsafe extern "C" fn bad() {
    const LOGICAL_ALLOCATION_BYTES: usize = 10;
    const SOURCE_LENGTH: usize = 10;
    const MODELED_STACK_BYTES: usize = SOURCE_LENGTH * size_of::<c_int>();
    const OVERFLOW_BYTES: usize = MODELED_STACK_BYTES - LOGICAL_ALLOCATION_BYTES;

    struct ModeledStackAllocation {
        allocated: [u8; LOGICAL_ALLOCATION_BYTES],
        overflow: [u8; OVERFLOW_BYTES],
    }

    let mut data = ModeledStackAllocation {
        allocated: [0; LOGICAL_ALLOCATION_BYTES],
        overflow: [0; OVERFLOW_BYTES],
    };
    let source = [0 as c_int; SOURCE_LENGTH];

    for (i, value) in source.iter().enumerate() {
        let offset = i * size_of::<c_int>();

        for (byte_offset, byte) in value.to_ne_bytes().into_iter().enumerate() {
            let position = offset + byte_offset;
            if position < data.allocated.len() {
                data.allocated[position] = byte;
            } else {
                data.overflow[position - data.allocated.len()] = byte;
            }
        }
    }

    let first = c_int::from_ne_bytes(
        data.allocated[..size_of::<c_int>()]
            .try_into()
            .expect("an int occupies a fixed number of bytes"),
    );
    printIntLine(first);
}

#[no_mangle]
pub unsafe extern "C" fn good() {
    let mut data = [0 as c_int; 10];
    let source = [0 as c_int; 10];

    for (destination, value) in data.iter_mut().zip(source) {
        *destination = value;
    }

    printIntLine(data[0]);
}

#[cfg_attr(not(test), no_mangle)]
pub unsafe extern "C" fn main() -> c_int {
    let mut x: c_int = 0;

    scanf(SCAN_INT_FORMAT.as_ptr().cast(), &mut x);

    if x != 0 {
        good();
    } else {
        bad();
    }

    0
}
