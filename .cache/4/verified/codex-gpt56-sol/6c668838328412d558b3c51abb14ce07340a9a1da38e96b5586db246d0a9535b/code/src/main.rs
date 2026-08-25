use std::ffi::{c_char, c_int, CStr};
use std::mem::size_of;

const SCAN_INT_FORMAT: &[u8] = b"%d\0";
const PRINT_STRING_FORMAT: &[u8] = b"%s\n\0";
const PRINT_INT_FORMAT: &[u8] = b"%d\n\0";

extern "C" {
    fn scanf(format: *const c_char, ...) -> c_int;
    fn printf(format: *const c_char, ...) -> c_int;
}

#[allow(dead_code)]
fn print_line(line: Option<&CStr>) {
    if let Some(line) = line {
        unsafe {
            printf(PRINT_STRING_FORMAT.as_ptr().cast(), line.as_ptr());
        }
    }
}

fn print_int_line(int_number: c_int) {
    unsafe {
        printf(PRINT_INT_FORMAT.as_ptr().cast(), int_number);
    }
}

fn bad() {
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
                // Preserve the C write beyond the 10 allocated bytes in
                // modeled adjacent stack storage.
                data.overflow[position - data.allocated.len()] = byte;
            }
        }
    }

    let first = c_int::from_ne_bytes(
        data.allocated[..size_of::<c_int>()]
            .try_into()
            .expect("an int occupies a fixed number of bytes"),
    );
    print_int_line(first);
}

fn good() {
    let mut data = [0 as c_int; 10];
    let source = [0 as c_int; 10];

    for (destination, value) in data.iter_mut().zip(source) {
        *destination = value;
    }

    print_int_line(data[0]);
}

fn main() {
    let mut x: c_int = 0;

    unsafe {
        scanf(SCAN_INT_FORMAT.as_ptr().cast(), &mut x);
    }

    if x != 0 {
        good();
    } else {
        bad();
    }
}
