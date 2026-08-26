use std::ffi::{c_char, c_int};
use std::process::ExitCode;

use driver::process_strings;

const MAX_BUFFER_SIZE: usize = 1024;

unsafe extern "C" {
    fn scanf(format: *const c_char, ...) -> c_int;
    fn printf(format: *const c_char, ...) -> c_int;
}

fn scan_i32(value: &mut i32) -> bool {
    unsafe { scanf(c"%d".as_ptr(), value) == 1 }
}

fn scan_u32(value: &mut u32) -> bool {
    unsafe { scanf(c"%u".as_ptr(), value) == 1 }
}

fn scan_usize(value: &mut usize) -> bool {
    unsafe { scanf(c"%zu".as_ptr(), value) == 1 }
}

fn fail(message: &str) -> ExitCode {
    eprintln!("{message}");
    ExitCode::FAILURE
}

fn main() -> ExitCode {
    let mut operation = 0;
    let mut flags = 0;
    let mut input_len = 0;
    let mut ref_len = 0;
    let mut input_buffer = [0u8; MAX_BUFFER_SIZE + 1];
    let mut ref_buffer = [0u8; MAX_BUFFER_SIZE + 1];

    if !scan_i32(&mut operation) {
        return fail("Error reading operation");
    }

    if !scan_u32(&mut flags) {
        return fail("Error reading flags");
    }

    if !scan_usize(&mut input_len) {
        return fail("Error reading input length");
    }

    if input_len > MAX_BUFFER_SIZE {
        eprintln!("Error: input length {input_len} exceeds maximum {MAX_BUFFER_SIZE}");
        return ExitCode::FAILURE;
    }

    for (index, byte) in input_buffer[..input_len].iter_mut().enumerate() {
        let mut value = 0u32;
        if !scan_u32(&mut value) {
            eprintln!("Error reading input byte {index}");
            return ExitCode::FAILURE;
        }
        *byte = value as u8;
    }

    if !scan_usize(&mut ref_len) {
        return fail("Error reading reference length");
    }

    if ref_len > MAX_BUFFER_SIZE {
        eprintln!("Error: reference length {ref_len} exceeds maximum {MAX_BUFFER_SIZE}");
        return ExitCode::FAILURE;
    }

    for (index, byte) in ref_buffer[..ref_len].iter_mut().enumerate() {
        let mut value = 0u32;
        if !scan_u32(&mut value) {
            eprintln!("Error reading reference byte {index}");
            return ExitCode::FAILURE;
        }
        *byte = value as u8;
    }

    let result = unsafe {
        process_strings(
            input_buffer.as_mut_ptr().cast(),
            input_len,
            ref_buffer.as_ptr().cast(),
            ref_len,
            operation,
            flags,
        )
    };

    unsafe {
        printf(c"%d\n".as_ptr(), result);
    }

    ExitCode::SUCCESS
}
