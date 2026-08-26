//! Translation of `c_src/src/main.c`.

mod cstr;
mod mem;
mod scanf;
mod strcpy_fun;

use mem::{Mem, INPUT_OFF, REF_OFF};
use scanf::Scanner;
use std::io::Write;

const MAX_BUFFER_SIZE: u64 = 1024;

fn fail(message: &str) -> ! {
    let mut stderr = std::io::stderr();
    let _ = stderr.write_all(message.as_bytes());
    let _ = stderr.flush();
    std::process::exit(1);
}

fn main() {
    let mut scanner = Scanner::from_stdin();

    /* Read operation */
    let operation = match scanner.scan_int() {
        Some(value) => value,
        None => fail("Error reading operation\n"),
    };

    /* Read flags */
    let flags = match scanner.scan_uint() {
        Some(value) => value,
        None => fail("Error reading flags\n"),
    };

    /* Read input length */
    let input_len = match scanner.scan_size() {
        Some(value) => value,
        None => fail("Error reading input length\n"),
    };

    if input_len > MAX_BUFFER_SIZE {
        fail(&format!(
            "Error: input length {} exceeds maximum {}\n",
            input_len, MAX_BUFFER_SIZE
        ));
    }

    /* Read input buffer data */
    let mut input_bytes: Vec<u8> = Vec::new();
    for i in 0..input_len {
        match scanner.scan_uint() {
            Some(byte) => input_bytes.push(byte as u8),
            None => fail(&format!("Error reading input byte {}\n", i)),
        }
    }

    /* Read reference length */
    let ref_len = match scanner.scan_size() {
        Some(value) => value,
        None => fail("Error reading reference length\n"),
    };

    if ref_len > MAX_BUFFER_SIZE {
        fail(&format!(
            "Error: reference length {} exceeds maximum {}\n",
            ref_len, MAX_BUFFER_SIZE
        ));
    }

    /* Read reference buffer data */
    let mut ref_bytes: Vec<u8> = Vec::new();
    for i in 0..ref_len {
        match scanner.scan_uint() {
            Some(byte) => ref_bytes.push(byte as u8),
            None => fail(&format!("Error reading reference byte {}\n", i)),
        }
    }

    let mut memory = Mem::new(operation, flags, input_len, ref_len);
    for (i, &byte) in input_bytes.iter().enumerate() {
        memory.set(INPUT_OFF + i, byte);
    }
    for (i, &byte) in ref_bytes.iter().enumerate() {
        memory.set(REF_OFF + i, byte);
    }

    /* Call the library function */
    let result = strcpy_fun::process_strings(
        &memory,
        INPUT_OFF,
        input_len as usize,
        REF_OFF,
        ref_len as usize,
        operation,
        flags,
    );

    /* Print result to stdout */
    let mut stdout = std::io::stdout();
    let _ = stdout.write_all(format!("{}\n", result).as_bytes());
    let _ = stdout.flush();
}
